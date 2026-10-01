//! `-F stages`: the day runner's own cost — the stage table walked for the day type in each mode, every slot it runs
//! taken as a span around no work — so what the runner adds to a day is measured apart from what its slots do.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::stages::{DAY_TABLE, Mode, StageTable, compile};

use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the runner is measured under.
pub const BASE: &str = "stages";

/// The table compiled once, as assembly does.
#[derive(Debug, Default)]
pub struct Stages {
    table: Option<StageTable>,
    folded: usize,
}

/// The day's walk: each slot the day runs as a span around no work, and the barriers after them counted.
fn walk(table: &StageTable, mode: Mode, any_business: bool) -> usize {
    let mut ran = 0;
    for slot in table.walk(mode, any_business) {
        ran += phx_exec::trace::span("slot", || black_box(slot.runs.len()));
    }
    ran + table.barriers(mode, any_business)
}

impl FinBase for Stages {
    fn name(&self) -> &'static str {
        BASE
    }

    fn fill(&mut self, _design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        let table = compile(&DAY_TABLE).map_err(|e| FinError(e.join("; ")))?;
        let rows = u64::try_from(table.slots().len()).map_err(|e| FinError(e.to_string()))?;
        self.table = Some(table);
        Ok(Filled { rows })
    }

    /// The day type's walk in the ordinary mode, and day zero's.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(table) = self.table.as_ref() else {
            return Err(FinError("the runner measured before its table".to_owned()));
        };
        let any_business = day != DayType::Nb;
        self.folded ^= m.read(BASE, "runner", 1, || walk(table, Mode::Ordinary, any_business));
        self.folded ^= m.read(BASE, "day_zero", 1, || walk(table, Mode::DayZero, any_business));
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: 0, resident: 0 }
    }
}
