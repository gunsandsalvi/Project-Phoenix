//! Goods spoiled in stock at their holders' visits: each system's declared rates, realised by the kernel on each row a
//! declared visit visits, over each lot's days held since the row's last visit, whole for a twin. One instruction a
//! row, of transformation legs under `spoiled`.

use phx_core::{Cadence, Declarations, Register, RunsOn, SubStep};
use phx_id::{Day, Slot};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::world::World;

/// A declared spoilage as the world realises it: the visit it follows, its period in days, and each product's yearly
/// rate of loss, by its place.
#[derive(Clone, Debug)]
pub(crate) struct SpoilBound {
    pub visit: usize,
    pub period: i64,
    pub rates: Vec<f64>,
}

/// Every declared spoilage bound to its visit, which must run every period of calendar days so the days between two
/// are the period's; every refusal at once.
pub(crate) fn bind(
    d: &Declarations,
    visits: &[crate::visits::Bound],
    register: &Register,
) -> Result<Vec<SpoilBound>, Vec<String>> {
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, s) in &d.spoilage {
        let Some(visit) = visits.iter().position(|b| b.decl.handler == s.visit) else {
            errors.push(format!("{system}'s spoilage follows `{}`, no declared visit", s.visit));
            continue;
        };
        let Some(b) = visits.get(visit) else { continue };
        let Cadence::Schedule { days, runs_on: RunsOn::Any } = b.decl.cadence else {
            errors.push(format!("`{}` is not a schedule of calendar days, which spoilage is realised over", s.visit));
            continue;
        };
        let period = match register.count(days).map(i64::try_from) {
            Ok(Ok(p)) => p,
            Ok(Err(_)) => {
                errors.push(format!("`{days}` beyond a count of days"));
                continue;
            }
            Err(e) => {
                errors.push(format!("{system}'s spoilage: {e}"));
                continue;
            }
        };
        match (s.rates)(register) {
            Ok(rates) => out.push(SpoilBound { visit, period, rates }),
            Err(e) => errors.push(format!("{system}'s spoilage: {e}")),
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

impl World {
    /// The spoilage of every good a row holds, realised for the rows a visit visited, one instruction a row. Returns
    /// the rows whose goods spoiled.
    #[clause("GDS.8", "GDS.10")]
    pub(crate) fn spoil_after(&mut self, visit: usize, slots: &[Slot], (day, step): (Day, SubStep)) -> u64 {
        let spoils: Vec<SpoilBound> = self.spoils.iter().filter(|s| s.visit == visit).cloned().collect();
        let Some(b) = self.visits.get(visit).copied() else { return 0 };
        let rows = crate::goods::Rows { place: b.table.get(), individuals: b.individuals };
        let mut spoiled = 0;
        for s in &spoils {
            for slot in slots {
                let legs = self.spoiled_legs(rows, *slot, s, day);
                if legs.is_empty() {
                    continue;
                }
                let instruction = Instruction {
                    id: self.books.ledger.next_id(day),
                    reason: self.books.dues.spoiled,
                    trade_day: day,
                    settle_day: day,
                    legs,
                    pays: Missing::Absent,
                    covers: Vec::new(),
                };
                if let Err(f) = self.books.apply(ApplyAt::Day(step), instruction, self.audit.stream()) {
                    violation!(clause = "GDS.8", "spoiled units a holding could not give", party = f.party.get());
                }
                spoiled += 1;
            }
        }
        spoiled
    }

    /// A row's spoilage legs: each good it holds that spoils, its twin's whole units lost over each lot's days held
    /// within the period, times its twins.
    fn spoiled_legs(&self, rows: crate::goods::Rows, slot: Slot, s: &SpoilBound, day: Day) -> Vec<LegRec> {
        let Some(row) = self.goods_row(rows, slot) else { return Vec::new() };
        let (place, at) = self.books.parties.row(row.party);
        let arenas = self.books.parties.holder(place);
        let mut legs = Vec::new();
        for (instrument, _) in phx_ledger::holding::bases(arenas, at) {
            let Missing::Present(key) = self.books.ledger.goods.key(instrument) else { continue };
            let Some(rate) = s.rates.get(usize::from(key.product)).copied() else { continue };
            let lots: Vec<(i64, i64)> = phx_ledger::holding::lots(arenas, at, instrument)
                .iter()
                .map(|l| (l.quantity.raw(), i64::from(day.get()) - i64::from(l.acquired.get())))
                .collect();
            let Some(lost) = phx_core::spoilage::lost(&lots, s.period, rate, row.twins, phx_core::consts::DAYS_365)
            else {
                violation!(clause = "GDS.8", "a stock's spoilage beyond an integer", party = row.party.get());
            };
            if lost == 0 {
                continue;
            }
            let Some(qty) = lost.checked_mul(row.twins) else {
                phx_num::capacity_exceeded!("units spoiled for every twin", i64::MAX, lost);
            };
            legs.push(LegRec {
                party: row.party,
                account: AccountRef::Instrument(instrument),
                qty: -qty,
                denom: Denom::Unit(self.books.ledger.instruments.get(instrument).unit),
                kind: LegKind::Transformation { source: Source::Spoilage(s.period.cast_unsigned()), cost: 0 },
            });
        }
        legs
    }
}
