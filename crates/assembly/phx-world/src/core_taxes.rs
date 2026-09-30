//! The taxes' collectors on the core. A tax arises on its base's settled payment — withheld by an employer from the
//! wage it pays, charged by a seller in a final sale's price — and from then it is its collector's debt to its
//! treasury: a contract remitted in full on the collection day of the month after, asked again each month while
//! it is unpaid. Each employer kind owes in its own collectors' family, as it pays wages in its own wage family. The
//! treasury's own payroll collects to itself. At each close what arose is held to what was remitted, what collectors
//! still owe and what they owed when their debts closed.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::calendar::period::ScheduleDates;
use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_core::flows::Flow;
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_ledger::algebra::{Leg, Schedule};
use phx_ledger::levy::Withholding;
use phx_macros::{clause, opening};
use phx_num::{Missing, violation};
use phx_store::AddressSpace;

use crate::consts::AGENT_ROWS_PER_CHUNK;
use crate::consts::reason::{TAXED, WAGE};
use crate::core::{Core, kind_number};
use crate::core_accounts::Line;
use crate::core_day::{DatedFamily, Due};
use phx_core::capacity::{AGENT_ROWS, KIND_ROWS};

/// The bases a tax arises on.
pub const INCOME: u8 = 0;
pub const CONSUMPTION: u8 = 1;
pub const BASES: usize = 2;

/// A flow as settlement's failures name it: payer, payee, amount, reason and source.
pub type FlowKey = (PartyKey, PartyKey, i64, u8, u32);

/// A tax that arises if its base's payment settles: its collector, the party that bears it, its base, its amount and
/// currency, and the payment it arises on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Arising {
    pub collector: PartyKey,
    pub payer: PartyKey,
    pub base: u8,
    pub tax: i64,
    pub ccy: u8,
    pub on: FlowKey,
}

/// The taxes: those waiting on their bases' settlement, each collector's debt open for a base and a collection day,
/// what arose by base and how often, what was remitted, what estates paid of the debts lost when their collectors
/// ended, and a sample of the income tax withheld, with the gross wage it was withheld from, for the levies' check.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Taxes {
    pub(crate) arising: Vec<Arising>,
    open: BTreeMap<(PartyKey, u8, Day), u32>,
    pub arisen: [i128; BASES],
    pub arisings: [u64; BASES],
    pub remitted: i128,
    pub recovered: i128,
    pub sample: Vec<(i64, i64, u8)>,
}

/// The collectors' family of a kind: the one of the families, each given by its reason and its first side's kind,
/// that collects taxes and whose first side is that kind.
#[must_use]
pub(crate) fn collected_family(families: &[(u8, u8)], kind: u8) -> Option<usize> {
    families.iter().position(|&(reason, first)| reason == TAXED && first == kind)
}

/// Each kind's collectors' family, for every kind a key can name.
#[must_use]
#[opening]
pub(crate) fn collectors_index(families: &[(u8, u8)], kinds: usize) -> Vec<Option<usize>> {
    (0..kinds).map(|k| u8::try_from(k).ok().and_then(|k| collected_family(families, k))).collect()
}

/// The family a collector owes its taxes in; a collector of a kind no family collects for breaks the taxes' identity.
#[clause("TAX.5")]
pub(crate) fn collector_family(index: &[Option<usize>], collector: PartyKey) -> usize {
    let Some(Some(family)) = index.get(usize::from(collector.kind())).copied() else {
        violation!(clause = "TAX.5", "a tax withheld by a kind with no collectors' family", kind = collector.kind());
    };
    family
}

/// A collectors' family, empty: its first side the payers' kind, its second the treasuries'; `rows` its sides' and
/// its contracts' capacities.
#[opening]
pub(crate) fn collectors_family(
    space: &mut AddressSpace,
    (name, kinds): (&'static str, [u8; 2]),
    (rows, contracts): ([u32; 2], u32),
    today: Day,
) -> DatedFamily {
    DatedFamily {
        name,
        store: phx_core::store::Family::new(
            space,
            (kinds, rows),
            (contracts, AGENT_ROWS_PER_CHUNK),
            [true, true],
            (today.succ(), phx_core::capacity::WHEEL_DAYS),
        ),
        reason: TAXED,
        schedules: Vec::new(),
        classes: Vec::new(),
        terms: Vec::new(),
        ends_after: Vec::new(),
        finishing: Vec::new(),
        alike: BTreeMap::new(),
        alike_upto: 0,
        moves: crate::core_day::LoanMoves::default(),
        lost: 0,
    }
}

/// The income tax a wage's payer withholds under its currency's law, taken from what the wage pays, with the gross
/// wage; none where the law withholds nothing, or would take none of it or more than all of it.
#[clause("TAX.2")]
pub(crate) fn withheld(f: &mut Flow, law: Option<&Withholding>) -> Option<(Arising, i64)> {
    if f.reason != WAGE || !f.denomination.is_money() {
        return None;
    }
    let gross = f.amount;
    let tax = law?.on_payment(gross);
    if tax <= 0 || tax > gross {
        return None;
    }
    f.amount -= tax;
    let arising = Arising {
        collector: f.payer,
        payer: f.payee,
        base: INCOME,
        tax,
        ccy: f.denomination.ccy(),
        on: (f.payer, f.payee, f.amount, f.reason, f.source),
    };
    Some((arising, gross))
}

impl Taxes {
    /// A collector's debt for a tax in its family: added to the one it owes for the base on the coming collection
    /// day, or opened.
    pub(crate) fn owe(
        &mut self,
        f: &mut DatedFamily,
        (a, treasury): (Arising, PartyKey),
        (dates, due): (ScheduleDates, Day),
    ) {
        if let Some(edge) = self.open.get(&(a.collector, a.base, due)).copied()
            && f.store.edges.is_open(Slot::new(edge))
            && let Some(row) = f.store.edges.rows_mut().get_mut(usize::try_from(edge).unwrap_or(usize::MAX))
        {
            row.amount += a.tax;
            f.moves.lent.push((treasury, a.tax));
            return;
        }
        let terms = phx_ledger::opening::plain_terms(
            phx_ledger::opening::currency(CountryId::new(a.ccy)),
            vec![Leg::Amortising],
            Schedule { dates, count: Missing::Present(1) },
        );
        let schedule = f.schedule_in(a.ccy, [u32::from(a.base), 0, 0], terms);
        let edge = f.store.open(
            Due { ends: [a.collector, treasury], amount: a.tax, nth: 1, schedule, person: 0, arrears: 0 },
            Some(due),
        );
        f.moves.lent.push((treasury, a.tax));
        self.open.insert((a.collector, a.base, due), edge.get());
    }
}

/// A currency's collection dates from this month's collection day, and the coming one.
fn collection_day(ccy: u8, (day, calendar, remit): (Day, &Calendar, u32)) -> (ScheduleDates, Day) {
    let date = calendar.date(day);
    let Some(anchor) = u8::try_from(remit).ok().and_then(|d| phx_id::Date::new(date.year(), date.month(), d)) else {
        phx_num::violation!(clause = "TAX.8", "a collection day no month has", day = remit);
    };
    let dates = phx_ledger::opening::monthly(anchor, CountryId::new(ccy));
    let due = dates.nth(calendar, 1);
    (dates, due)
}

impl Core {
    /// The collectors' debts' families opened: for each wage family, one whose first side is that family's payers'
    /// kind and whose second is the treasuries'.
    #[opening]
    pub fn open_taxes(&mut self, today: Day) {
        let Some(treasury) = self.treasuries.iter().flatten().next().map(|t| t.kind()) else { return };
        for (wage, collected) in crate::consts::families::COLLECTORS {
            let Some(payer) = self.families.iter().find(|f| f.name == wage).map(|f| f.store.kinds[0]) else {
                continue;
            };
            let family = collectors_family(
                &mut self.space,
                (collected, [payer, treasury]),
                ([AGENT_ROWS, KIND_ROWS], AGENT_ROWS),
                today,
            );
            self.add_family(family);
        }
        self.index_collectors();
    }

    /// Each kind's collectors' family found among the families, at the opening and after a load.
    #[opening]
    pub(crate) fn index_collectors(&mut self) {
        let families: Vec<(u8, u8)> = self.families.iter().map(|f| (f.reason, f.store.kinds[0])).collect();
        self.collectors = collectors_index(&families, self.names.len());
    }

    /// Each tax whose base's payment settled today made its collector's debt: added to the debt it owes for the base
    /// on the coming collection day, or a new one, and entered as its collector's tax; and the day's remittances
    /// counted.
    #[clause("TAX.2", "TAX.5", "ACC.13")]
    pub(crate) fn accrue_taxes<'f>(
        &mut self,
        (day, calendar): (Day, &Calendar),
        flows: impl IntoIterator<Item = &'f Flow>,
        failed: &[Flow],
    ) {
        let mut unpaid: BTreeMap<FlowKey, u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        let mut remitted_failed = unpaid.clone();
        let estate = self.bound.kinds.estate.map(kind_number);
        for f in flows.into_iter().filter(|f| f.reason == TAXED && f.denomination.is_money()) {
            if let Some(n) =
                remitted_failed.get_mut(&(f.payer, f.payee, f.amount, f.reason, f.source)).filter(|n| **n > 0)
            {
                *n -= 1;
                continue;
            }
            // An estate pays its collector's debt, which closed as lost when the collector ended: remitted, and
            // recovered of what was lost.
            self.taxes.remitted += i128::from(f.amount);
            if Some(f.payer.kind()) == estate {
                self.taxes.recovered += i128::from(f.amount);
            }
        }
        // The debts due before today are closed, so none takes today's taxes.
        self.taxes.open.retain(|(_, _, d), _| *d >= day);
        // Every tax arising today in a currency is owed on the same collection day.
        let mut collection: BTreeMap<u8, (ScheduleDates, Day)> = BTreeMap::new();
        for a in std::mem::take(&mut self.taxes.arising) {
            if let Some(n) = unpaid.get_mut(&a.on).filter(|n| **n > 0) {
                *n -= 1;
                continue;
            }
            let base = usize::from(a.base);
            if let (Some(t), Some(n)) = (self.taxes.arisen.get_mut(base), self.taxes.arisings.get_mut(base)) {
                (*t, *n) = (*t + i128::from(a.tax), *n + 1);
            }
            let Some(Some(treasury)) = self.treasuries.get(usize::from(a.ccy)).copied() else { continue };
            // The treasury paying its own staff collects to itself, remitting at once.
            if a.collector == treasury {
                self.taxes.remitted += i128::from(a.tax);
                continue;
            }
            let Some(Some(remit)) = self.state.remit_day.get(usize::from(a.ccy)).copied() else { continue };
            let due = *collection.entry(a.ccy).or_insert_with(|| collection_day(a.ccy, (day, calendar, remit)));
            let family = collector_family(&self.collectors, a.collector);
            if let Some(f) = self.families.get_mut(family) {
                self.taxes.owe(f, (a, treasury), due);
            }
            self.recognise(a.collector, Line::Taxes, a.tax);
        }
    }

    /// What arose held to what was remitted, what collectors still owe and what their debts owed when they closed, less
    /// what their estates recovered of that; a difference is a finding of the taxes family.
    #[clause("TAX.5", "N1", "II.5")]
    pub(crate) fn audit_taxes(&mut self, day: Day) {
        let (mut owed, mut lost) = (0_i128, 0_i128);
        for f in self.families.iter().filter(|f| f.reason == TAXED) {
            owed += f
                .store
                .edges
                .open_slots()
                .filter_map(|e| f.store.edges.row(e))
                .map(|r| i128::from(r.amount) + i128::from(r.arrears))
                .sum::<i128>();
            lost += f.lost;
        }
        let arisen: i128 = self.taxes.arisen.iter().sum();
        let accounted = self.taxes.remitted + owed + lost - self.taxes.recovered;
        if arisen != accounted {
            self.found.push(Finding {
                family: "taxes",
                clause: "TAX.5",
                owner: FindingOwner::Run,
                size: arisen - accounted,
                unit: Unit::Count,
                day,
                detail: format!(
                    "{arisen} of tax arose where {} was remitted, {owed} is owed and {lost} was lost, {} of it recovered",
                    self.taxes.remitted, self.taxes.recovered
                ),
            });
        }
    }
}

#[cfg(test)]
#[path = "core_taxes_tests.rs"]
mod tests;
