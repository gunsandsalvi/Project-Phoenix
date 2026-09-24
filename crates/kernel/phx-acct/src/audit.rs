use phx_core::{
    AccountsAudit, AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, Gap, InjectTarget, Unit,
    declare_family,
};
use phx_id::{Day, LineId, PartyId};
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::accounts::Accounts;

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

/// An injection needs a save loaded apart, which the world cannot yet keep.
fn no_save() -> Result<(), String> {
    Err("the accounts are injected into a save loaded apart, which persistence brings".to_owned())
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
    fn inject(&self, _: &mut dyn InjectTarget) -> Result<(), String> {
        no_save()
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
    fn inject(&self, _: &mut dyn InjectTarget) -> Result<(), String> {
        no_save()
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
    fn inject(&self, _: &mut dyn InjectTarget) -> Result<(), String> {
        no_save()
    }
}

/// The accounts' families, which the world registers over its accounts.
#[must_use]
pub fn families() -> Vec<Box<dyn AuditFamily>> {
    vec![Box::new(Equity), Box::new(Periods), Box::new(Claims)]
}
