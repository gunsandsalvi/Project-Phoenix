use phx_audit::CloseRecord;
use phx_id::Day;

/// One sub-step that ran on a day: the rows and bytes it touched and the barriers it crossed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubStepRecord {
    pub day: Day,
    pub substep: u8,
    pub rows: u64,
    pub bytes: u64,
    pub barriers: u64,
}

/// One turn: the days it ran and its wall time by the application's clock, absent when the clock ran backwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnRecord {
    pub first: Day,
    pub last: Day,
    pub days: u32,
    pub wall_ns: Option<u64>,
}

/// The run's measures, kept outside the world: never hashed and never read by a handler.
#[derive(Debug, Default)]
pub struct Metrics {
    pub substeps: Vec<SubStepRecord>,
    pub turns: Vec<TurnRecord>,
    pub closes: Vec<CloseRecord>,
}
