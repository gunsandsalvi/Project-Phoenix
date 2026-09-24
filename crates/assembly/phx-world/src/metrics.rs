use phx_audit::CloseRecord;
use phx_id::Day;

/// One sub-step that ran on a day: the rows and bytes it touched and the barriers it crossed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct SubStepRecord {
    pub day: Day,
    pub substep: u8,
    pub rows: u64,
    pub bytes: u64,
    pub barriers: u64,
}

/// One turn: the days it ran and its wall time by the application's clock, absent when the clock ran backwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct TurnRecord {
    pub first: Day,
    pub last: Day,
    pub days: u32,
    pub wall_ns: Option<u64>,
}

/// One save as measured: the day it was taken at the close of, each store's size compressed and before compression,
/// the run's record's size, how long it took to write and to check, and, where its files did not read back to the
/// world hash of that close, why.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct SaveMeasure {
    pub day: Day,
    pub stores: Vec<(String, u64, u64)>,
    pub run_bytes: u64,
    pub write_ns: Option<u64>,
    pub check_ns: Option<u64>,
    pub mismatch: Option<String>,
}

/// One family's injection into the run's injection save: how long the save took to load, the families that found
/// something over a full cycle of closes after it, or why the save could not take it.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct InjectionRecord {
    pub family: String,
    pub load_ns: Option<u64>,
    pub lit: Vec<String>,
    pub refused: Option<String>,
}

/// The audit's closes, saved here field by field: the audit's crate reads the world and keeps no store.
#[derive(Debug, Default)]
pub struct Closes(pub Vec<CloseRecord>);

/// A close as a save writes it: its day, families, rows checked, findings and instructions applied.
type CloseRow = ((Day, usize), (u64, usize, u64));

impl phx_store::Saved for Closes {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        let rows: Vec<CloseRow> =
            self.0.iter().map(|c| ((c.day, c.families), (c.rows_checked, c.findings, c.applied))).collect();
        rows.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Closes, phx_store::LoadError> {
        let rows: Vec<CloseRow> = phx_store::Saved::load(r)?;
        Ok(Closes(
            rows.into_iter()
                .map(|((day, families), (rows_checked, findings, applied))| CloseRecord {
                    day,
                    families,
                    rows_checked,
                    findings,
                    applied,
                })
                .collect(),
        ))
    }
}

/// The run's measures, kept outside the world: never hashed and never read by a handler.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Metrics {
    pub substeps: Vec<SubStepRecord>,
    pub turns: Vec<TurnRecord>,
    pub closes: Closes,
    pub saves: Vec<SaveMeasure>,
    pub injections: Vec<InjectionRecord>,
    /// What each day's work on the population's cells did.
    pub cells: Vec<crate::cells::CellDay>,
}
