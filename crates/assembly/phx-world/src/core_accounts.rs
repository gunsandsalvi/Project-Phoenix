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
use phx_macros::clause;

use crate::consts::reason::{REPAID, SEVERANCE, SOLD, TAXED, WAGE};
use crate::core::Core;

/// A party's income since the opening, by line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Income {
    pub revenue: i128,
    pub cost_of_sales: i128,
    pub goods_lost: i128,
    pub services_used: i128,
    pub wages: i128,
    pub taxes: i128,
    pub interest_paid: i128,
    pub interest_received: i128,
    pub written_off: i128,
}

impl Income {
    /// The income the lines make.
    #[must_use]
    pub fn net(&self) -> i128 {
        self.revenue + self.interest_received
            - self.cost_of_sales
            - self.goods_lost
            - self.services_used
            - self.wages
            - self.taxes
            - self.interest_paid
            - self.written_off
    }
}

/// A line of income an event moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Revenue,
    CostOfSales,
    GoodsLost,
    ServicesUsed,
    Wages,
    Taxes,
    InterestPaid,
    InterestReceived,
    WrittenOff,
}

/// The accounts: the day they opened, each owned party's equity account at the opening and its income since, and
/// the money received for sales against the revenue recognised on their delivery.
#[derive(Clone, Debug, Default)]
pub struct Accounts {
    pub opened: Option<Day>,
    pub opening: BTreeMap<PartyKey, i128>,
    pub income: BTreeMap<PartyKey, Income>,
    pub sales_received: i128,
    pub revenue: i128,
}

impl Accounts {
    /// A party's equity account.
    #[must_use]
    pub fn equity(&self, party: PartyKey) -> Option<i128> {
        let opening = self.opening.get(&party)?;
        Some(opening + self.income.get(&party).map_or(0, Income::net))
    }
}

impl Core {
    /// Whether a party keeps an equity account: a firm or a bank, whose owners it has.
    fn owned(&self, party: PartyKey) -> bool {
        let kind = usize::from(party.kind());
        self.names.get(kind).is_some_and(|n| *n == "firm" || *n == "bank")
    }

    /// An event of a party's income entered on its line, where the party keeps an equity account.
    pub(crate) fn recognise(&mut self, party: PartyKey, line: Line, amount: i64) {
        if amount == 0 || !self.accounts.opening.contains_key(&party) {
            return;
        }
        let i = self.accounts.income.entry(party).or_default();
        let a = i128::from(amount);
        match line {
            Line::Revenue => i.revenue += a,
            Line::CostOfSales => i.cost_of_sales += a,
            Line::GoodsLost => i.goods_lost += a,
            Line::ServicesUsed => i.services_used += a,
            Line::Wages => i.wages += a,
            Line::Taxes => i.taxes += a,
            Line::InterestPaid => i.interest_paid += a,
            Line::InterestReceived => i.interest_received += a,
            Line::WrittenOff => i.written_off += a,
        }
    }

    /// What a party's balance sheet shows: its money, its goods at their cost and the arrears it is owed, with, for a
    /// bank, the balances its loans owe it; less the arrears it owes and the balances it has to repay, and, for a bank,
    /// its customers' deposits.
    #[must_use]
    pub fn net_assets(&self, party: PartyKey, deposits: &[i64]) -> Option<i128> {
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
        Some(money + goods + owed_to - owes - deposits)
    }

    /// Each bank's customers' deposits, by its slot.
    fn deposits(&self) -> Vec<i64> {
        let banks = self.bank_kind.and_then(|b| self.kinds.get(usize::from(b))).map_or(0, |k| k.parties.high_water());
        phx_core::store::deposits_of(self.kinds.iter(), usize::try_from(banks).unwrap_or(0))
    }

    /// The accounts opened: every firm's and bank's equity account at what its balance sheet shows.
    #[clause("ACC.4")]
    pub fn open_accounts(&mut self, today: Day) {
        let deposits = self.deposits();
        let mut opening = BTreeMap::new();
        for (k, store) in self.kinds.iter().enumerate() {
            let Ok(kind) = u8::try_from(k) else { continue };
            for slot in store.parties.live_slots() {
                let party = PartyKey::new(kind, slot);
                if self.owned(party)
                    && let Some(worth) = self.net_assets(party, &deposits)
                {
                    opening.insert(party, worth);
                }
            }
        }
        self.accounts = Accounts { opened: Some(today), opening, ..Accounts::default() };
    }

    /// The day's settled flows entered as income: wages, severance and taxes paid; what a loan's payments paid
    /// beyond the principal its dues repay, paid by the borrower and received by the creditor; and the money received
    /// for sales, against which the revenue recognised at their delivery is held.
    #[clause("ACC.13", "FRM.13", "FRM.17")]
    pub(crate) fn account_flows(&mut self, flows: &[Flow], failed: &[Flow]) {
        let mut unpaid: BTreeMap<(PartyKey, PartyKey, i64, u8, u32), u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        for f in flows.iter().filter(|f| f.denomination.is_money()) {
            if let Some(n) = unpaid.get_mut(&(f.payer, f.payee, f.amount, f.reason, f.source)).filter(|n| **n > 0) {
                *n -= 1;
                continue;
            }
            match f.reason {
                WAGE | SEVERANCE => self.recognise(f.payer, Line::Wages, f.amount),
                TAXED => self.recognise(f.payer, Line::Taxes, f.amount),
                REPAID => {
                    self.recognise(f.payer, Line::InterestPaid, f.amount);
                    self.recognise(f.payee, Line::InterestReceived, f.amount);
                }
                SOLD if self.owned(f.payee) => self.accounts.sales_received += i128::from(f.amount),
                _ => {}
            }
        }
    }

    /// The day's moves on contracts entered as income: a principal repaid is no interest, an arrears' change is a
    /// wage or interest accrued or given up, a balance written off a loss to its creditor.
    #[clause("ACC.13", "BNK.12")]
    pub(crate) fn account_moves(&mut self) {
        let mut events: Vec<(PartyKey, Line, i64)> = Vec::new();
        for family in &self.families {
            for &(payer, payee, a) in &family.moves.repaid {
                events.push((payer, Line::InterestPaid, -a));
                events.push((payee, Line::InterestReceived, -a));
            }
            for &(creditor, a) in &family.moves.written_off {
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
    pub(crate) fn audit_accounts(&mut self, day: Day) {
        let parties: Vec<PartyKey> = self.accounts.opening.keys().copied().collect();
        let deposits = self.deposits();
        let mut found = Vec::new();
        let mut ended = Vec::new();
        for party in parties {
            let live = self.kinds.get(usize::from(party.kind())).is_some_and(|k| k.parties.id(party.slot()).is_some());
            if !live {
                ended.push(party);
                continue;
            }
            let (Some(equity), Some(worth)) = (self.accounts.equity(party), self.net_assets(party, &deposits)) else {
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
        for party in ended {
            self.accounts.opening.remove(&party);
            self.accounts.income.remove(&party);
        }
        self.found.extend(found);
    }
}
