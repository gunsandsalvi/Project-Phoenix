//! The accounts on the core. Every party with owners — each firm and each bank — keeps an equity account, opened at
//! what its balance sheet shows at the opening and moved only by its income as it is recognised: a sale's revenue and
//! the cost its units carried on the day it is delivered, goods lost and services used up, wages and taxes as they
//! are paid or fall into arrears, interest as a loan's payment less the principal it repays, and balances written
//! off. At each close every equity account is held to what the party's balance sheet shows — its money, its goods at
//! their cost and what it is owed, less what it owes — two records kept apart, a difference a finding.

use std::collections::BTreeMap;

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_core::flows::Flow;
use phx_id::{Day, PartyKey};
use phx_macros::{clause, opening, sweep};
use phx_num::violation;
use phx_pop::kinds::KindStore;
use phx_store::SystemBacking;

use crate::account_lines::BookWords;
pub use crate::account_lines::{Line, Lines};
use crate::consts::reason::{CARRIED, REPAID, SEVERANCE, SOLD, TAXED, WAGE};
use crate::core::Core;

/// The accounts: the day they opened, and the money received for sales against the revenue recognised on their
/// delivery. Each owned party's equity at the opening and its lines since are its kind's store's.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub struct Accounts {
    pub opened: Option<Day>,
    pub sales_received: i128,
    pub revenue: i128,
}

impl Core {
    /// Whether a kind keeps books: its legal form has owners who hold its equity.
    fn kind_owned(&self, kind: u8) -> bool {
        crate::core_kinds::of(&self.declared.kinds, usize::from(kind)).has_owners
    }

    fn owned(&self, party: PartyKey) -> bool {
        self.kind_owned(party.kind())
    }

    /// Whether a party keeps an equity account now: its kind keeps books, and the accounts have opened.
    pub(crate) fn keeps_books(&self, party: PartyKey) -> bool {
        self.accounts.opened.is_some() && self.owned(party)
    }

    /// The store a kind that keeps books keeps them in, with its book words; none for a kind that keeps none.
    fn books_of(&self, kind: u8) -> Option<(&KindStore<SystemBacking>, BookWords)> {
        match (self.firms.as_ref(), self.banks.as_ref()) {
            (Some(f), _) if f.kind() == kind => Some(f.books()),
            (_, Some(b)) if b.kind() == kind => Some(b.books()),
            _ => None,
        }
    }

    fn books_mut_of(&mut self, kind: u8) -> Option<(&mut KindStore<SystemBacking>, BookWords)> {
        match (self.firms.as_mut(), self.banks.as_mut()) {
            (Some(f), _) if f.kind() == kind => Some(f.books_mut()),
            (_, Some(b)) if b.kind() == kind => Some(b.books_mut()),
            _ => None,
        }
    }

    /// An event of a party's income entered on its line, where the party keeps an equity account. A party whose kind
    /// keeps books on no store stops the run.
    pub(crate) fn recognise(&mut self, party: PartyKey, line: Line, amount: i64) {
        if amount == 0 || !self.keeps_books(party) {
            return;
        }
        let Some((store, books)) = self.books_mut_of(party.kind()) else {
            violation!(clause = "ACC.4", "a party with owners whose kind keeps no books", kind = party.kind());
        };
        books.add(store, party.slot(), line, amount);
    }

    /// A party's lines since the accounts opened; none for a party that keeps none.
    #[must_use]
    pub fn lines_of(&self, party: PartyKey) -> Option<Lines> {
        if !self.keeps_books(party) {
            return None;
        }
        let (store, books) = self.books_of(party.kind())?;
        books.lines(store, party.slot())
    }

    /// A party's equity account: its equity at the opening and the income its lines make.
    #[must_use]
    pub fn equity_of(&self, party: PartyKey) -> Option<i128> {
        if !self.keeps_books(party) {
            return None;
        }
        let (store, books) = self.books_of(party.kind())?;
        books.equity(store, party.slot())
    }

    /// What a party's balance sheet shows: its money, its goods and plant at their cost, its projects at what they
    /// cost and the arrears it is owed, with, for a
    /// bank, the balances its loans owe it; less the arrears it owes and the balances it has to repay, and, for a bank,
    /// its customers' deposits.
    #[must_use]
    pub fn net_assets(
        &self,
        party: PartyKey,
        (deposits, projects): (&[i64], &BTreeMap<PartyKey, i128>),
    ) -> Option<i128> {
        let store = self.kinds.get(usize::from(party.kind()))?;
        let money = store.accounts.as_ref().map_or(0, |a| {
            i128::from(a.balance.get(party.slot()).unwrap_or(0)) + i128::from(a.pending.get(party.slot()).unwrap_or(0))
        });
        let goods: i128 = self.goods.stocks.holdings(party).map(|h| i128::from(h.cost)).sum();
        let mut owed_to = 0_i128;
        let mut owes = 0_i128;
        for family in &self.families {
            let terms_of =
                |at: u32| family.terms.get(usize::try_from(at).unwrap_or(usize::MAX)).is_some_and(Option::is_some);
            for (side, kind) in family.store.kinds.iter().enumerate() {
                if *kind != party.kind() || family.store.heads.get(side).is_none_or(Option::is_none) {
                    continue;
                }
                for edge in family.store.of(side, party.slot()) {
                    let Some(row) = family.store.edges.row(edge) else { continue };
                    let balance = if terms_of(row.schedule) { i128::from(row.amount) } else { 0 };
                    let due = balance + i128::from(row.arrears);
                    if side == 0 {
                        owes += due;
                    } else {
                        owed_to += due;
                    }
                }
            }
        }
        let deposits = if self.bank_kind == Some(party.kind()) {
            i128::from(deposits.get(usize::try_from(party.slot().get()).unwrap_or(usize::MAX)).copied().unwrap_or(0))
        } else {
            0
        };
        // A party with no project has none of their cost.
        let projects = projects.get(&party).copied().unwrap_or(0);
        Some(money + goods + projects + owed_to - owes - deposits)
    }

    /// Each bank's customers' deposits, by its slot.
    fn deposits(&self) -> Vec<i64> {
        let banks = self.bank_kind.map_or(0, |b| self.directory.high_water(b));
        phx_core::store::deposits_of(self.kinds.iter(), usize::try_from(banks).unwrap_or(0))
    }

    /// The accounts opened: every firm's and bank's equity account at what its balance sheet shows, written to its
    /// kind's store; an equity past an i64 stops the run.
    #[clause("ACC.4")]
    #[opening]
    pub fn open_accounts(&mut self, today: Day) {
        let (deposits, projects) = (self.deposits(), self.project_costs());
        let mut opening = Vec::new();
        for kind in self.directory.kind_numbers() {
            for slot in self.directory.live_slots(kind) {
                let party = PartyKey::new(kind, slot);
                if self.owned(party)
                    && let Some(worth) = self.net_assets(party, (&deposits, &projects))
                {
                    let Ok(worth) = i64::try_from(worth) else {
                        phx_num::capacity_exceeded!("a party's equity at the opening", i64::MAX, worth);
                    };
                    opening.push((party, worth));
                }
            }
        }
        for (party, worth) in opening {
            let Some((store, books)) = self.books_mut_of(party.kind()) else {
                violation!(clause = "ACC.4", "a party with owners whose kind keeps no books", kind = party.kind());
            };
            books.open(store, party.slot(), worth);
        }
        self.accounts = Accounts { opened: Some(today), ..Accounts::default() };
    }

    /// The day's settled flows entered as income: wages, severance and taxes paid; what a loan's payments paid
    /// beyond the principal its dues repay, paid by the borrower and received by the creditor; and the money received
    /// for sales and for freight, against which the revenue recognised at their delivery or departure is held.
    #[clause("ACC.13", "FRM.13", "FRM.17")]
    pub(crate) fn account_flows<'f>(&mut self, flows: impl IntoIterator<Item = &'f Flow>, failed: &[Flow]) {
        let mut unpaid: BTreeMap<(PartyKey, PartyKey, i64, u8, u32), u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        let estate = self.bound.kinds.estate.map(crate::core::kind_number);
        for f in flows.into_iter().filter(|f| f.denomination.is_money()) {
            if let Some(n) = unpaid.get_mut(&(f.payer, f.payee, f.amount, f.reason, f.source)).filter(|n| **n > 0) {
                *n -= 1;
                continue;
            }
            match f.reason {
                WAGE | SEVERANCE => self.recognise(f.payer, Line::Wages, f.amount),
                // An estate paying a loan its borrower's ending wrote off is a recovery of that loss, not interest.
                REPAID if Some(f.payer.kind()) == estate => self.recognise(f.payee, Line::WrittenOff, -f.amount),
                REPAID => {
                    self.recognise(f.payer, Line::InterestPaid, f.amount);
                    self.recognise(f.payee, Line::InterestReceived, f.amount);
                }
                SOLD | CARRIED if self.owned(f.payee) => self.accounts.sales_received += i128::from(f.amount),
                _ => {}
            }
        }
    }

    /// The day's moves on contracts entered as income: a principal repaid is no interest, an arrears' change is a
    /// wage or interest accrued or given up, a balance written off a loss to its creditor.
    #[clause("ACC.13", "BNK.12")]
    pub(crate) fn account_moves(&mut self) {
        let mut events: Vec<(PartyKey, Line, i64)> = Vec::new();
        // A collector's debt for taxes was its tax when it arose; paying or owing it later moves no income.
        for family in self.families.iter().filter(|f| f.reason != TAXED) {
            for &(payer, payee, a) in &family.moves.repaid {
                events.push((payer, Line::InterestPaid, -a));
                events.push((payee, Line::InterestReceived, -a));
            }
            for &(creditor, a, _) in &family.moves.written_off {
                events.push((creditor, Line::WrittenOff, a));
            }
            for m in &family.moves.arrears {
                let (paid, received) = if m.terms {
                    (Line::InterestPaid, Line::InterestReceived)
                } else if m.reason == WAGE {
                    (Line::Wages, Line::Revenue)
                } else {
                    (Line::Taxes, Line::Revenue)
                };
                events.push((m.payer, paid, m.change));
                events.push((m.payee, received, m.change));
            }
        }
        for (party, line, a) in events {
            self.recognise(party, line, a);
        }
    }

    /// Every equity account held to what its party's balance sheet shows, and the revenue recognised to the money
    /// received for sales; a difference is a finding.
    #[clause("ACC.10", "ACC.11", "ACC.16", "FRM.17", "II.5")]
    #[sweep(store = books, reason = "every equity account is held to its party's books at each close")]
    pub(crate) fn audit_accounts(&mut self, day: Day) {
        if self.accounts.opened.is_none() {
            return;
        }
        let (deposits, projects) = (self.deposits(), self.project_costs());
        let mut found = Vec::new();
        let owned = self.directory.kind_numbers().filter(|k| self.kind_owned(*k));
        let parties = owned.flat_map(|k| self.directory.live_slots(k).map(move |s| PartyKey::new(k, s)));
        for party in parties {
            let (Some(equity), Some(worth)) = (self.equity_of(party), self.net_assets(party, (&deposits, &projects)))
            else {
                continue;
            };
            if equity != worth {
                found.push(Finding {
                    family: "accounts",
                    clause: "ACC.10",
                    owner: FindingOwner::Run,
                    size: worth - equity,
                    unit: Unit::Count,
                    day,
                    detail: format!(
                        "party {}: its equity account holds {equity} where its books show {worth}",
                        party.word()
                    ),
                });
            }
        }
        if self.accounts.revenue != self.accounts.sales_received {
            found.push(Finding {
                family: "revenue",
                clause: "FRM.17",
                owner: FindingOwner::Run,
                size: self.accounts.sales_received - self.accounts.revenue,
                unit: Unit::Count,
                day,
                detail: format!(
                    "the firms recognised {} of revenue where they were paid {} for sales",
                    self.accounts.revenue, self.accounts.sales_received
                ),
            });
        }
        self.found.extend(found);
    }
}
