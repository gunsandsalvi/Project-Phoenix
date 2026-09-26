//! Plant worn at its owners' visits: each system's chains of classes, the ledger's, realised by the kernel on each row
//! a declared visit visits, for the days since the row's last: from each class the units the leaving rate takes over
//! those days, whole for a twin, move to the next class carrying their cost times the ratio of the classes' values,
//! and the last class's leave, their cost written off. One instruction a row, of transformation legs under `worn`.

use phx_core::{Cadence, Declarations, Register, RunsOn, SubStep, WearSpec};
use phx_id::{Day, InstrumentId, PartyId, Slot};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::world::World;

/// A declared wear as the world realises it: the visit it follows, its period in days, and each tag's rates and values.
#[derive(Clone, Debug)]
pub(crate) struct WearBound {
    pub visit: usize,
    pub period: i64,
    pub specs: Vec<WearSpec>,
}

/// Every declared wear bound to its visit, which must run every period of calendar days so the days between two are
/// the period's; every refusal at once.
pub(crate) fn bind(
    d: &Declarations,
    visits: &[crate::visits::Bound],
    register: &Register,
) -> Result<Vec<WearBound>, Vec<String>> {
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, w) in &d.wear {
        let Some(visit) = visits.iter().position(|b| b.decl.handler == w.visit) else {
            errors.push(format!("{system}'s wear follows `{}`, no declared visit", w.visit));
            continue;
        };
        let Some(b) = visits.get(visit) else { continue };
        let Cadence::Schedule { days, runs_on: RunsOn::Any } = b.decl.cadence else {
            errors.push(format!("`{}` is not a schedule of calendar days, which wear is realised over", w.visit));
            continue;
        };
        let period = match register.count(days).map(i64::try_from) {
            Ok(Ok(p)) => p,
            Ok(Err(_)) => {
                errors.push(format!("`{days}` beyond a count of days"));
                continue;
            }
            Err(e) => {
                errors.push(format!("{system}'s wear: {e}"));
                continue;
            }
        };
        match (w.specs)(register) {
            Ok(specs) => out.push(WearBound { visit, period, specs }),
            Err(e) => errors.push(format!("{system}'s wear: {e}")),
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

impl World {
    /// The wear of every chain a row's holdings are classes of, realised for the rows a visit visited, one
    /// instruction a row. Returns the rows worn.
    #[clause("CAP.6", "REP.24", "CAP.8")]
    pub(crate) fn wear_after(&mut self, visit: usize, slots: &[Slot], (day, step): (Day, SubStep)) -> u64 {
        let wears: Vec<WearBound> = self.wears.iter().filter(|w| w.visit == visit).cloned().collect();
        if wears.is_empty() {
            return 0;
        }
        let Some(b) = self.visits.get(visit).copied() else { return 0 };
        let place = b.table.get();
        let (mut worn, mut retired, mut charged) = (0, 0_i128, 0_i128);
        for w in &wears {
            // Plant held from the opening has worn since day zero, so a row's first visit takes the days since then.
            let since = i64::from(day.get()) - i64::from(self.day_zero.get());
            let days = if since < w.period { since } else { w.period };
            for slot in slots {
                let Wear { legs, retired: r, charged: c } = self.worn_legs(place, *slot, w, days);
                retired += r;
                charged += c;
                if legs.is_empty() {
                    continue;
                }
                let instruction = Instruction {
                    id: self.books.ledger.next_id(day),
                    reason: self.books.dues.worn,
                    trade_day: day,
                    settle_day: day,
                    legs,
                    pays: Missing::Absent,
                    covers: Vec::new(),
                };
                if let Err(f) = self.books.apply(ApplyAt::Day(step), instruction, self.audit.stream()) {
                    violation!(clause = "CAP.6", "worn units that could not move", party = f.party.get());
                }
                worn += 1;
            }
        }
        self.agent_day.worn += worn;
        self.agent_day.retired += retired;
        self.agent_day.depreciation += charged;
        worn
    }

    /// A row's wear legs over `days`: per chain it holds classes of, each class's leavers out at the cost of their
    /// lots and into the next at that cost times the ratio of the two classes' values, the last's out alone.
    fn worn_legs(&self, place: u16, slot: Slot, w: &WearBound, days: i64) -> Wear {
        let arenas = self.books.parties.holder(place);
        let party = arenas.party(slot);
        let chains = &self.books.ledger.chains;
        let mut wear = Wear { legs: Vec::new(), retired: 0, charged: 0 };
        for (n, chain) in (0_u32..).zip(chains.iter()) {
            let Some(spec) = w.specs.iter().find(|s| s.tag == chain.tag) else { continue };
            if spec.values.len() != chain.classes.len() {
                violation!(clause = "CAP.1", "a chain's classes other than its spec's values", tag = chain.tag);
            }
            for (class, instrument) in chain.classes.iter().enumerate() {
                let Missing::Present(h) = phx_ledger::holding::holding(arenas, slot, *instrument) else { continue };
                let Some(leaving) = sys_leaving(h.quantity.raw(), days, spec.leaving_per_year) else {
                    violation!(clause = "CAP.6", "a class's wear beyond an integer", party = party.get());
                };
                if leaving == 0 {
                    continue;
                }
                let Missing::Present(cost) = phx_ledger::holding::first_in_cost(arenas, slot, *instrument, leaving)
                else {
                    violation!(clause = "CAP.6", "worn units a holding does not hold", party = party.get());
                };
                wear.legs.push(leg(self, (party, *instrument), -leaving, (n, 0)));
                wear.charged += i128::from(cost);
                if let Some(next) = chain.classes.get(class + 1) {
                    let (from, to) = (spec.values.get(class), spec.values.get(class + 1));
                    let (Some(from), Some(to)) = (from, to) else {
                        violation!(clause = "CAP.1", "a class with no value", tag = chain.tag);
                    };
                    let Some(carried) = phx_core::wear::carried(cost, *from, *to) else {
                        violation!(clause = "CAP.6", "a carried cost beyond an integer", party = party.get());
                    };
                    wear.legs.push(leg(self, (party, *next), leaving, (n, carried)));
                    wear.charged -= i128::from(carried);
                } else {
                    wear.retired += i128::from(leaving);
                }
            }
        }
        wear
    }
}

/// A row's wear: its legs, the units its last classes retired, and the cost its units lost, the depreciation.
struct Wear {
    legs: Vec<LegRec>,
    retired: i128,
    charged: i128,
}

/// A wear leg: units of a class out, or into the next at the cost they carry.
fn leg(world: &World, (party, instrument): (PartyId, InstrumentId), qty: i64, (chain, cost): (u32, i64)) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Instrument(instrument),
        qty,
        denom: Denom::Unit(world.books.ledger.instruments.get(instrument).unit),
        kind: LegKind::Transformation { source: Source::Wear(chain), cost },
    }
}

fn sys_leaving(held: i64, days: i64, rate: f64) -> Option<i64> {
    phx_core::wear::leaving(held, days, rate, phx_core::consts::DAYS_365)
}
