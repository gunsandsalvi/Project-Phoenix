//! The finished-volume measure: each base filled at the design point with seeded draws and run through its own kernels
//! on each day type, never a world; the day's and the turns' lines composed on the phone model and held to their
//! ratchets. A base's driver lives in its own module and joins the registry.

pub mod budget;
pub mod compose;
pub mod counters;
pub mod design;
pub mod fill;
pub mod kept;
pub mod kept_calendar;
pub mod kept_flows;
pub mod kept_meet;
pub mod kept_search;
#[path = "kept_tests.rs"]
mod kept_tests;
pub mod kept_wheel;
pub mod measure;
pub mod report;
pub mod seed;
#[path = "seed_tests.rs"]
mod seed_tests;
#[path = "tests.rs"]
mod tests;

use std::collections::BTreeMap;

use design::Design;
use fill::Streams;
use measure::{Measure, Measures};
use phx_exec::Clock;
use report::Report;

/// Bytes to MiB, a shift.
const MIB_SHIFT: u32 = 20;

/// A refusal of the harness: a key the design point lacks, a base it does not know, a capacity or a driver's failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinError(pub String);

impl std::fmt::Display for FinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The day types a base is run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum DayType {
    B,
    Nb,
    H,
    Bc,
}

impl DayType {
    pub const ALL: [DayType; 4] = [DayType::B, DayType::Nb, DayType::H, DayType::Bc];

    /// The day's table in the design point.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            DayType::B => "b",
            DayType::Nb => "nb",
            DayType::H => "h",
            DayType::Bc => "bc",
        }
    }

    /// The day's place among a line's figures.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            DayType::B | DayType::Bc => 0,
            DayType::Nb => 1,
            DayType::H => 2,
        }
    }

    /// A day type by the name `-D` gives it.
    ///
    /// # Errors
    /// A name that is no day type.
    pub fn parse(name: &str) -> Result<DayType, FinError> {
        DayType::ALL
            .into_iter()
            .find(|d| d.key().eq_ignore_ascii_case(name))
            .ok_or_else(|| FinError(format!("`{name}` is not a day type (B, NB, H, BC)")))
    }
}

/// A base's rows after its fill.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filled {
    pub rows: u64,
}

/// A base's bytes after its fill: its rows' and what it holds beside them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bytes {
    pub rows: u64,
    pub resident: u64,
}

/// One base's driver: filled at the design point, then run through its own kernels for a day of each type.
pub trait FinBase {
    fn name(&self) -> &'static str;
    /// # Errors
    /// A fill the design point does not describe.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError>;
    /// # Errors
    /// A kernel's refusal.
    fn day(
        &mut self,
        day: DayType,
        counts: &BTreeMap<String, u64>,
        measures: &mut Measures<'_>,
    ) -> Result<(), FinError>;
    fn bytes(&self) -> Bytes;
}

/// Every driver, in the order the bases depend on one another; each base's step adds its own.
pub const REGISTRY: &[fn() -> Box<dyn FinBase>] =
    &[|| Box::new(counters::Counters::default()), || Box::new(kept::Kept::default())];

/// What a run fills, runs and reads.
#[derive(Debug, Clone)]
pub struct Args {
    pub design: String,
    pub budget: String,
    /// The bases named, or `None` for every driver.
    pub bases: Option<Vec<String>>,
    pub days: Vec<DayType>,
}

/// Every store the capacity table holds whose rows fall short of the design point's count for it.
#[must_use]
pub fn capacities_short(design: &Design) -> Vec<String> {
    phx_core::capacity::table()
        .filter_map(|c| {
            let count = design.store.get(c.store)?;
            (u64::from(c.rows) < *count)
                .then(|| format!("the store `{}` holds {} rows, short of the design point's {count}", c.store, c.rows))
        })
        .collect()
}

/// Fills and runs the drivers named, warmed a day of each type before the measured one, composes the day and the
/// turns and checks every ratchet and capacity.
///
/// # Errors
/// A key the design point lacks, a base with no driver, a driver's refusal.
pub fn run(args: &Args, drivers: &[fn() -> Box<dyn FinBase>], clock: &dyn Clock) -> Result<Report, FinError> {
    let design = Design::parse(&args.design)?;
    let ratchets = budget::read(&args.budget)?;
    let streams = Streams::new(design.point.seed);
    let mut chosen: Vec<Box<dyn FinBase>> = drivers.iter().map(|make| make()).collect();
    if let Some(names) = &args.bases {
        if let Some(unknown) = names.iter().find(|n| *n != "all" && !chosen.iter().any(|d| d.name() == n.as_str())) {
            return Err(FinError(format!("no driver measures the base `{unknown}` yet")));
        }
        if !names.iter().any(|n| n == "all") {
            chosen.retain(|d| names.iter().any(|n| n == d.name()));
        }
    }
    let mut measures = Measures::new(clock);
    let mut sizes = Vec::new();
    for driver in &mut chosen {
        let filled = driver.fill(&design, &streams)?;
        let bytes = driver.bytes();
        sizes.push((driver.name().to_owned(), filled.rows, (bytes.rows + bytes.resident) >> MIB_SHIFT));
        for day in &args.days {
            let counts =
                design.day(*day).ok_or_else(|| FinError(format!("the design point has no `day.{}`", day.key())))?;
            driver.day(*day, counts, &mut Measures::new(clock))?;
            driver.day(*day, counts, &mut measures)?;
        }
    }
    let mut per_op: BTreeMap<String, f64> = measures
        .iter()
        .filter_map(|((base, op), m)| Some((format!("fin.{base}.{op}_ns"), m.ns_per_op()?.to_string().parse().ok()?)))
        .collect();
    for (base, _, mb) in &sizes {
        if let Ok(mb) = mb.to_string().parse() {
            per_op.insert(format!("fin.{base}.mb"), mb);
        }
    }
    let mut lines = compose::declared(&design);
    compose::substitute(&mut lines, &design, (kept::BASE, kept::LEAVES), &|key| per_op.get(key).copied());
    let days = compose::days(&lines, &design);
    let mut misses = capacities_short(&design);
    // A budget holding the seeded section is held to it: no key looser than its seed, none missing.
    if args.budget.contains(seed::BEGIN) {
        misses.extend(seed::within_design(&args.budget, &seed::seed(&args.design)?)?);
    }
    misses.extend(budget::misses(&ratchets, &|key| per_op.get(key).copied()));
    Ok(Report {
        bases: chosen.iter().map(|d| d.name().to_owned()).collect(),
        ops: measures
            .iter()
            .map(|((b, o), m): (&(String, String), &Measure)| (b.clone(), o.clone(), m.items, m.ns_per_op()))
            .collect(),
        sizes,
        lines,
        days,
        misses,
    })
}
