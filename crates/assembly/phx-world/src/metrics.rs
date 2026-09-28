//! What the run measures of itself, outside the world: each turn's days and time.

use phx_id::Day;

/// A turn: its first and last day, the days it ran and its wall time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnRecord {
    pub first: Day,
    pub last: Day,
    pub days: u32,
    pub wall_ns: Option<u64>,
}

/// The run's measures.
#[derive(Debug, Default)]
pub struct Metrics {
    pub turns: Vec<TurnRecord>,
}
