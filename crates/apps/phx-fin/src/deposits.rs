//! `-F deposits`: the deposits register at the design point — its deposits, most finite and some without end — and a
//! day's extractions from them, drawn across the deposits, each within what remains.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_geo::deposits::{DepositId, Deposits, Extraction};
use phx_id::TileId;
use phx_num::{Fixed, Missing, UnitId};
use phx_rand::uniform::below_u64;
use phx_store::StoreStats;

use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the register is measured under.
pub const BASE: &str = "deposits";

/// The day's extractions, a deposit's opening quantity at most, and one deposit in this many without end.
const EXTRACTIONS: u64 = 100_000;
const MOST_HELD: u64 = 10_000_000;
const WITHOUT_END: u64 = 10;
/// The most units an extraction takes, well within a deposit over the measure's days.
const MOST_TAKEN: u64 = 100;
/// A deposit's grade, in thousandths.
const GRADE: i64 = 1_000;

/// The register and the day's extractions.
#[derive(Debug, Default)]
pub struct DepositsBase {
    register: Option<Deposits>,
    streams: Option<Streams>,
    count: u64,
    folded: u64,
}

fn err(e: impl std::fmt::Display) -> FinError {
    FinError(e.to_string())
}

impl FinBase for DepositsBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The design point's deposits, each on a drawn tile with its resource's and right's units, one in ten without end.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let (count, tiles) = (store("deposits")?, store("tiles")?);
        let mut d = streams.draws(BASE, 0, 0);
        let mut register = Deposits::default();
        for i in 0..count {
            let tile = TileId::new(u32::try_from(below_u64(&mut d, tiles)).map_err(err)?);
            let unit = |n: u64| u32::try_from(n).map(UnitId::new).map_err(err);
            let held = if i % WITHOUT_END == 0 { Missing::Absent } else { Missing::Present(MOST_HELD) };
            let _ = register.open(tile, (unit(i)?, unit(count + i)?), Fixed::from_raw(GRADE), held);
        }
        register.settle();
        (self.register, self.streams, self.count) = (Some(register), Some(*streams), count);
        Ok(Filled { rows: count })
    }

    /// A day's extractions drawn across the deposits, then applied as one batch.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(register), Some(streams)) = (self.register.as_mut(), self.streams) else {
            return Err(FinError("the register measured before its fill".to_owned()));
        };
        let mut d = streams.draws(BASE, 1, crate::kept::day_of(day)?);
        let items: Vec<Extraction> = (0..EXTRACTIONS)
            .map(|i| {
                let deposit = DepositId::new(u32::try_from(below_u64(&mut d, self.count)).map_err(err)?);
                let extractor = u32::try_from(i).map_err(err)?;
                Ok(Extraction { deposit, extractor, units: 1 + below_u64(&mut d, MOST_TAKEN) })
            })
            .collect::<Result<_, FinError>>()?;
        self.folded ^= m.read(BASE, "extract", EXTRACTIONS, || black_box(register.extract_batch(&items)));
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.register.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let rows = self.register.as_ref().map_or(0, StoreStats::rows_live);
        let figure = |n: u64| n.to_string().parse::<f64>().ok();
        let bytes = self.bytes().rows.checked_div(rows).and_then(figure);
        [bytes.map(|b| ("row_bytes", b)), figure(rows).map(|n| ("rows", n))].into_iter().flatten().collect()
    }
}
