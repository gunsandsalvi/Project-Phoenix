//! The taxes' collectors on the core. A tax arises on its base's settled payment — withheld by an employer from the
//! wage it pays, charged by a seller in a final sale's price — and from then it is its collector's debt to its
//! treasury: a contract remitted in full on the collection day of the month after, asked again each month while
//! it is unpaid. The treasury's own payroll collects to itself. At each close what arose is held to what was
//! remitted, what collectors still owe and what they owed when their debts closed.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_core::flows::Flow;
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_ledger::algebra::{Leg, Schedule};
use phx_macros::clause;
use phx_num::Missing;

use crate::consts::reason::TAXED;
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, KIND_ROWS};
use crate::core::{Core, kind_number};
use crate::core_accounts::Line;
use crate::core_day::{DatedFamily, Due};

/// The collectors' debts to their treasuries.
pub const COLLECTED: &str = "TAX.collected";

/// The bases a tax arises on.
pub const INCOME: u8 = 0;
pub const CONSUMPTION: u8 = 1;
pub const BASES: usize = 2;

/// A flow as settlement's failures name it: payer, payee, amount, reason and source.
pub type FlowKey = (PartyKey, PartyKey, i64, u8, u32);

/// A tax that arises if its base's payment settles: its collector, the party that bears it, its base, its amount and
/// currency, and the payment it arises on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arising {
    pub collector: PartyKey,
    pub payer: PartyKey,
    pub base: u8,
    pub tax: i64,
    pub ccy: u8,
    pub on: FlowKey,
}

/// The taxes: those waiting on their bases' settlement, each collector's debt open for a base and a collection day,
/// what arose by base and how often, what was remitted, and a sample of the income tax withheld, with the gross wage
/// it was withheld from, for the levies' check.
#[derive(Debug, Default)]
pub struct Taxes {
    pub(crate) arising: Vec<Arising>,
    open: BTreeMap<(PartyKey, u8, Day), u32>,
    pub arisen: [i128; BASES],
    pub arisings: [u64; BASES],
    pub remitted: i128,
    pub sample: Vec<(i64, i64, u8)>,
}

impl Core {
    /// The collectors' debts' family opened: each firm's to its treasury.
    pub fn open_taxes(&mut self, today: Day) {
        let (Some(firm), Some(treasury)) =
            (self.names.iter().position(|n| *n == "firm"), self.names.iter().position(|n| *n == "treasury"))
        else {
            return;
        };
        let family = DatedFamily {
            name: COLLECTED,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(firm), kind_number(treasury)], [AGENT_ROWS, KIND_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), crate::consts::CORE_WHEEL_DAYS),
            ),
            reason: TAXED,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            moves: crate::core_day::LoanMoves::default(),
            lost: 0,
        };
        self.families.push(family);
    }

    /// Each tax whose base's payment settled today made its collector's debt: added to the debt it owes for the base
    /// on the coming collection day, or a new one, and entered as its collector's tax; and the day's remittances
    /// counted.
    #[clause("TAX.2", "TAX.5", "ACC.13")]
    pub(crate) fn accrue_taxes(&mut self, (day, calendar): (Day, &Calendar), flows: &[Flow], failed: &[Flow]) {
        let mut unpaid: BTreeMap<FlowKey, u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        let mut remitted_failed = unpaid.clone();
        for f in flows.iter().filter(|f| f.reason == TAXED && f.denomination.is_money()) {
            if let Some(n) =
                remitted_failed.get_mut(&(f.payer, f.payee, f.amount, f.reason, f.source)).filter(|n| **n > 0)
            {
                *n -= 1;
                continue;
            }
            self.taxes.remitted += i128::from(f.amount);
        }
        let Some(family) = self.families.iter().position(|f| f.name == COLLECTED) else { return };
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
            self.owe_tax(family, (a, treasury), (day, calendar, remit));
            self.recognise(a.collector, Line::Taxes, a.tax);
        }
    }

    /// A collector's debt for a tax: added to the one it owes for the base on the coming collection day, or opened.
    fn owe_tax(
        &mut self,
        family: usize,
        (a, treasury): (Arising, PartyKey),
        (day, calendar, remit): (Day, &Calendar, u32),
    ) {
        let date = calendar.date(day);
        let Some(anchor) = u8::try_from(remit).ok().and_then(|d| phx_id::Date::new(date.year(), date.month(), d))
        else {
            phx_num::violation!(clause = "TAX.8", "a collection day no month has", day = remit);
        };
        let country = CountryId::new(a.ccy);
        let dates = phx_ledger::opening::monthly(anchor, country);
        let due = dates.nth(calendar, 1);
        let Some(f) = self.families.get_mut(family) else { return };
        if let Some(edge) = self.taxes.open.get(&(a.collector, a.base, due)).copied()
            && f.store.edges.is_open(Slot::new(edge))
            && let Some(row) = f.store.edges.rows_mut().get_mut(usize::try_from(edge).unwrap_or(usize::MAX))
        {
            row.amount += a.tax;
            f.moves.lent.push((treasury, a.tax));
            return;
        }
        let terms = phx_ledger::opening::plain_terms(
            phx_ledger::opening::currency(country),
            vec![Leg::Amortising],
            Schedule { dates, count: Missing::Present(1) },
        );
        let schedule = f.schedule_in(a.ccy, [u32::from(a.base), 0, 0], terms);
        let edge = f.store.open(
            Due { ends: [a.collector, treasury], amount: a.tax, nth: 1, schedule, person: 0, arrears: 0 },
            Some(due),
        );
        f.moves.lent.push((treasury, a.tax));
        self.taxes.open.retain(|(_, _, d), _| *d >= day);
        self.taxes.open.insert((a.collector, a.base, due), edge.get());
    }

    /// What arose held to what was remitted, what collectors still owe and what their debts owed when they closed; a
    /// difference is a finding of the taxes family.
    #[clause("TAX.5", "N1", "II.5")]
    pub(crate) fn audit_taxes(&mut self, day: Day) {
        let Some(f) = self.families.iter().find(|f| f.name == COLLECTED) else { return };
        let owed: i128 = f
            .store
            .edges
            .open_slots()
            .filter_map(|e| f.store.edges.row(e))
            .map(|r| i128::from(r.amount) + i128::from(r.arrears))
            .sum();
        let arisen: i128 = self.taxes.arisen.iter().sum();
        let accounted = self.taxes.remitted + owed + f.lost;
        if arisen != accounted {
            self.found.push(Finding {
                family: "taxes",
                clause: "TAX.5",
                owner: FindingOwner::Run,
                size: arisen - accounted,
                unit: Unit::Count,
                day,
                detail: format!(
                    "{arisen} of tax arose where {} was remitted, {owed} is owed and {} was lost",
                    self.taxes.remitted, f.lost
                ),
            });
        }
    }
}
