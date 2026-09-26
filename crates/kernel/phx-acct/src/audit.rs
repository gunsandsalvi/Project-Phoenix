use phx_core::{
    AccountsAudit, AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, Gap, InjectTarget, Unit,
    declare_family,
};
use phx_id::{Day, LineId, PartyId};
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::accounts::{Accounts, Closed};
use crate::equity::EquityEvent;

declare_family! { pub EQUITY = "ACC.equity" { mode: Rolling { cycle_days: 30 }, clause: "ACC.10" } }
declare_family! { pub PERIODS = "ACC.periods" { mode: Incremental, clause: "ACC.11" } }
declare_family! { pub CLAIMS = "ACC.claims" { mode: Incremental, clause: "ACC.12" } }

/// The accounts as the audit reads them, over the books they are checked against: the parties with an equity
/// account in their order, taken once at the close.
pub struct AccountsView<'a, B: Backing> {
    pub books: &'a Books<B>,
    pub accounts: &'a Accounts,
    parties: Vec<PartyId>,
}

impl<B: Backing> core::fmt::Debug for AccountsView<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccountsView").field("parties", &self.parties.len()).finish_non_exhaustive()
    }
}

impl<'a, B: Backing> AccountsView<'a, B> {
    #[must_use]
    pub fn new(books: &'a Books<B>, accounts: &'a Accounts) -> AccountsView<'a, B> {
        AccountsView { books, accounts, parties: accounts.equity.parties().collect() }
    }

    fn line_unit(&self, line: LineId) -> Unit {
        let lines = &self.books.ledger.lines;
        Unit::Money(self.books.ledger.terms.get(lines.terms(line)).ccy)
    }
}

impl<B: Backing> AccountsAudit for AccountsView<'_, B> {
    fn parties(&self) -> usize {
        self.parties.len()
    }

    #[clause("ACC.10")]
    fn equity(&self, i: usize) -> Vec<Gap> {
        let Some(party) = self.parties.get(i).copied() else {
            phx_num::violation!(clause = "N1", "an equity account read past the accounts kept", index = i);
        };
        let owner = FindingOwner::Party(party);
        let Missing::Present(account) = self.accounts.equity.of(party) else {
            phx_num::violation!(clause = "ACC.4", "a party listed without its equity account", party = party.get());
        };
        match self.accounts.net_assets(self.books, party) {
            Err(why) => vec![Gap { owner, size: 1, unit: Unit::Count, detail: format!("unreadable: {why}") }],
            Ok(net) => {
                let gap = net - i128::from(account.balance());
                if gap == 0 {
                    return Vec::new();
                }
                let detail = format!(
                    "party {}: assets less liabilities {net} against an equity account of {}",
                    party.get(),
                    account.balance()
                );
                vec![Gap { owner, size: gap, unit: Unit::Money(account.ccy()), detail }]
            }
        }
    }

    #[clause("ACC.12")]
    fn claims(&self) -> (u64, Vec<Gap>) {
        let gaps = self
            .accounts
            .claims
            .mismatches()
            .into_iter()
            .map(|(line, r, p)| Gap {
                owner: FindingOwner::Line(line),
                size: r - p,
                unit: self.line_unit(line),
                detail: format!("line {}: {r} receivable against {p} payable", line.get()),
            })
            .collect();
        (phx_rand::float::len_u64(self.accounts.claims.lines()), gaps)
    }

    #[clause("FRM.17")]
    fn income(&self) -> i128 {
        self.accounts.day_income()
    }

    #[clause("ACC.11")]
    fn periods(&self, _: Day) -> (u64, Vec<Gap>) {
        let closed = self.accounts.closed();
        let mut gaps = Vec::new();
        for c in closed {
            let change = i128::from(c.closed) - i128::from(c.opened);
            let gap = change - c.capital - c.income;
            if gap == 0 {
                continue;
            }
            let Missing::Present(account) = self.accounts.equity.of(c.party) else {
                phx_num::violation!(clause = "ACC.4", "a closed period without its account", party = c.party.get());
            };
            let detail = format!(
                "party {}: equity changed by {change} over the period against income {} and capital {}",
                c.party.get(),
                c.income,
                c.capital
            );
            gaps.push(Gap { owner: FindingOwner::Party(c.party), size: gap, unit: Unit::Money(account.ccy()), detail });
        }
        (phx_rand::float::len_u64(closed.len()), gaps)
    }
}

fn record(decl: FamilyDecl, ctx: &FamilyCtx<'_>, gaps: Vec<Gap>, findings: &mut Findings) {
    for g in gaps {
        findings.record(Finding {
            family: decl.name,
            clause: decl.clause,
            owner: g.owner,
            size: g.size,
            unit: g.unit,
            day: ctx.day(),
            detail: g.detail,
        });
    }
}

/// The save's accounts, which an injection breaks.
fn accounts(target: &mut dyn InjectTarget) -> Result<&mut Accounts, String> {
    target.accounts().downcast_mut::<Accounts>().ok_or_else(|| "the save's accounts are not the accounts'".to_owned())
}

/// An income with no outlay against it, which the firms' revenue family alone sees.
///
/// # Errors
/// When the save's accounts are not the accounts'.
pub fn inject_income(target: &mut dyn InjectTarget) -> Result<(), String> {
    accounts(target)?.income_alone(1);
    Ok(())
}

/// The first party that keeps an equity account.
fn first_owned(accounts: &Accounts) -> Result<PartyId, String> {
    accounts.equity.parties().next().ok_or_else(|| "the save keeps no equity account".to_owned())
}

/// Every equity account against its party's assets less liabilities at their carrying values, a slice a day.
#[derive(Debug)]
pub struct Equity;

/// Each period closed today: its income against its equity's change less its capital.
#[derive(Debug)]
pub struct Periods;

/// The receivables against the payables, line by line.
#[derive(Debug)]
pub struct Claims;

impl AuditFamily for Equity {
    fn decl(&self) -> FamilyDecl {
        EQUITY
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let accounts = ctx.accounts();
        let span = ctx.rolling(accounts.parties());
        for i in span.iter() {
            record(EQUITY, ctx, accounts.equity(i), findings);
        }
        phx_rand::float::len_u64(span.end - span.start)
    }
    /// An equity account moved by income its party's assets never received.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let accounts = accounts(target)?;
        let party = first_owned(accounts)?;
        accounts.equity.post(&EquityEvent::earned(party, 1));
        Ok(())
    }
}

impl AuditFamily for Periods {
    fn decl(&self) -> FamilyDecl {
        PERIODS
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let (checked, gaps) = ctx.accounts().periods(ctx.day());
        record(PERIODS, ctx, gaps, findings);
        checked
    }
    /// A period closed today whose equity moved by more than its income and capital, apart from the account itself.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let accounts = accounts(target)?;
        let party = first_owned(accounts)?;
        accounts.close_period(Closed { party, opened: 0, closed: 1, income: 0, capital: 0 });
        Ok(())
    }
}

impl AuditFamily for Claims {
    fn decl(&self) -> FamilyDecl {
        CLAIMS
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let (checked, gaps) = ctx.accounts().claims();
        record(CLAIMS, ctx, gaps, findings);
        checked
    }
    /// A receivable with no payable to meet it, its income on the holder's equity account so its equity still
    /// agrees with its assets.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let Some(books) = target.books().downcast_ref::<Books>() else {
            return Err("the save's books are not the ledger's".to_owned());
        };
        if books.ledger.lines.is_empty() {
            return Err("the save's books hold no line".to_owned());
        }
        let accounts = accounts(target)?;
        let party = first_owned(accounts)?;
        accounts.claims.receivable_alone(LineId::new(0), party, party, 1);
        accounts.equity.post(&EquityEvent::earned(party, 1));
        Ok(())
    }
}

/// The accounts' families, which the world registers over its accounts.
#[must_use]
pub fn families() -> Vec<Box<dyn AuditFamily>> {
    vec![Box::new(Equity), Box::new(Periods), Box::new(Claims)]
}
