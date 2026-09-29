//! What the run measures of itself, outside the world: each turn's days and time, each save's sizes and times, and
//! each family's injection.

use phx_id::Day;

/// A turn: its first and last day, the days it ran and its wall time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct TurnRecord {
    pub first: Day,
    pub last: Day,
    pub days: u32,
    pub wall_ns: Option<u64>,
}

/// A save as measured: the day it closed, each store's name and its bytes compressed and before, the run's record's
/// bytes, the time it took to write and to read back, and why it did not read back to its close's hash, if it did not.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct SaveMeasure {
    pub day: Day,
    pub stores: Vec<(String, u64, u64)>,
    pub run_bytes: u64,
    pub write_ns: Option<u64>,
    pub check_ns: Option<u64>,
    pub mismatch: Option<String>,
}

/// A family's injection into a save loaded apart: the family, the load's time, the families the audit lit over it, and
/// why the injection was refused, if it was.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct InjectionRecord {
    pub family: String,
    pub load_ns: Option<u64>,
    pub lit: Vec<String>,
    pub refused: Option<String>,
}

/// The run's measures.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Metrics {
    pub turns: Vec<TurnRecord>,
    pub saves: Vec<SaveMeasure>,
    pub injections: Vec<InjectionRecord>,
}
