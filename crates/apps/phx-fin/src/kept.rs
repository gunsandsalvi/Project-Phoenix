//! `-F kept`: today's kernels that the core keeps or replaces, filled at the design point and run through a day of
//! each type before any base lands, so each base step's gain is read against a measure. Each kernel's fill and day
//! are in their own module; this driver holds them, the pool they run on, and the keys their base steps retire.

use std::collections::BTreeMap;

use phx_exec::{Pool, PoolSpec};

use crate::compose::Leaf;
use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};
use crate::{kept_calendar, kept_flows, kept_meet, kept_search, kept_wheel};

/// The base the kept kernels are measured under.
pub const BASE: &str = "kept";

/// Every `[fin.kept]` key and the step whose own driver replaces it.
pub const RETIRERS: &[(&str, &str)] = &[
    ("flow_ns", "S1.248"),
    ("flow_h_ns", "S1.248"),
    ("due_ns", "S1.225"),
    ("purchase_ns", "S1.306"),
    ("search_ns", "S1.321"),
    ("civil_ns", "S1.178"),
    ("draw_ns", "S1.176"),
    ("mb", "S1.321"),
];

/// Where today's kernels stand in the declared day until their bases land: settlement's flows in its stage (ordinary
/// and heavy days apart, measured apart), the dues taken at the day's opening, the retail meeting among prices, and
/// hiring's search among decisions. The calendar's and the streams' reads stand in no declared leaf.
pub const LEAVES: &[Leaf] = &[
    Leaf { op: "flow", line: "7 Settle", unit: "flow", count: "flows", days: &[DayType::B], gather: true },
    Leaf { op: "flow_h", line: "7 Settle", unit: "flow", count: "flows", days: &[DayType::H], gather: true },
    Leaf { op: "due", line: "1 Open", unit: "dues", count: "dues", days: &[DayType::B, DayType::H], gather: true },
    Leaf {
        op: "purchase",
        line: "6 Prices",
        unit: "retail",
        count: "retail",
        days: &[DayType::B, DayType::Nb, DayType::H],
        gather: true,
    },
    Leaf {
        op: "search",
        line: "5 Decide",
        unit: "search",
        count: "searches",
        days: &[DayType::B, DayType::H],
        gather: false,
    },
];

/// A count of a table of the design point, refused by name where it is absent.
pub(crate) fn count(table: &BTreeMap<String, u64>, key: &str, table_name: &str) -> Result<u64, FinError> {
    table.get(key).copied().ok_or_else(|| FinError(format!("the design point has no `{table_name}.{key}`")))
}

/// A count as an index width.
pub(crate) fn index(n: u64) -> Result<usize, FinError> {
    usize::try_from(n).map_err(|e| FinError(format!("{n} rows: {e}")))
}

/// A drawn count as an amount, refused past `i64`.
pub(crate) fn whole(n: u64) -> Result<i64, FinError> {
    i64::try_from(n).map_err(|e| FinError(format!("{n}: {e}")))
}

/// A length as a count: every target the world runs on has lengths of at most 64 bits, so none is refused.
pub(crate) fn wide(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A day type's place as the day a stream draws for.
pub(crate) fn day_of(day: DayType) -> Result<u32, FinError> {
    u32::try_from(day.index()).map_err(|e| FinError(e.to_string()))
}

/// A count as a slot width.
pub(crate) fn slots(n: u64) -> Result<u32, FinError> {
    u32::try_from(n).map_err(|e| FinError(format!("{n} slots: {e}")))
}

/// The kept kernels, filled, on their pool.
#[derive(Default)]
pub struct Kept {
    pool: Option<Pool>,
    flows: kept_flows::Flows,
    wheel: kept_wheel::Wheel,
    meet: kept_meet::Meet,
    search: kept_search::Search,
    calendar: kept_calendar::Calendar,
}

impl std::fmt::Debug for Kept {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kept").finish_non_exhaustive()
    }
}

impl Kept {
    /// The kernels on a pool of the workers asked for.
    ///
    /// # Errors
    /// A pool the system will not start.
    pub fn on(spec: &PoolSpec) -> Result<Kept, FinError> {
        let pool = Pool::new(spec).map_err(|e| FinError(e.0))?;
        Ok(Kept { pool: Some(pool), ..Kept::default() })
    }

    fn pool(&mut self) -> Result<&Pool, FinError> {
        if self.pool.is_none() {
            self.pool = Some(Pool::new(&PoolSpec::detect()).map_err(|e| FinError(e.0))?);
        }
        self.pool.as_ref().ok_or_else(|| FinError("no pool".to_owned()))
    }

    /// The last retail day's wants that found no stall, and its sales.
    #[must_use]
    pub fn meeting(&self) -> (u64, u64) {
        (self.meet.unserved, self.meet.sales)
    }

    /// A digest of every filled kernel's state, the same for any workers.
    #[must_use]
    pub fn digest(&self) -> u64 {
        [self.flows.digest(), self.wheel.digest(), self.meet.digest(), self.search.digest()]
            .into_iter()
            .fold(0, |a, d| phx_exec::mix::mix64(a ^ d))
    }
}

impl FinBase for Kept {
    fn name(&self) -> &'static str {
        BASE
    }

    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let mut rows = self.flows.fill(design, streams)?;
        rows += self.wheel.fill(design, streams)?;
        rows += self.meet.fill(design, streams)?;
        rows += self.search.fill(design, streams)?;
        self.calendar.fill()?;
        Ok(Filled { rows })
    }

    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        self.pool()?;
        let pool = self.pool.as_ref();
        self.flows.day(day, counts, m, pool)?;
        self.wheel.day(counts, m, pool)?;
        self.meet.day(day, counts, m, pool)?;
        self.search.day(day, counts, m, pool)?;
        self.calendar.day(counts, m)
    }

    fn bytes(&self) -> Bytes {
        let parts = [self.flows.bytes(), self.wheel.bytes(), self.meet.bytes(), self.search.bytes()];
        Bytes { rows: parts.iter().sum(), resident: 0 }
    }
}
