use phx_id::{Day, PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::calendar::Calendar;
use crate::columns::{FactColumns, KernelTable};
use crate::directory::Directory;
use crate::events::EventStore;
use crate::findings::Findings;
use crate::messages::DayMessages;
use crate::records::RecordStore;
use crate::register::Register;
use crate::substep::SubStep;
use crate::touched::TouchedRows;

/// How an audit family checks: on every apply; on the rows the day touched and wrote; or on those and, besides, one
/// slice a day of its whole domain, covering it in a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyMode {
    Streaming,
    Incremental,
    Rolling { cycle_days: u16 },
}

/// An audit family: its name, owner, clause and mode, which every family states.
#[clause("N1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyDecl {
    pub name: &'static str,
    pub owner: &'static str,
    pub clause: &'static str,
    pub mode: FamilyMode,
}

/// Where the audit reads what the day left behind: after the day's last write, before the read-only tracers and views.
pub const AUDIT_SUBSTEP: SubStep = SubStep::S10d;

/// A run of row indices, `start` included and `end` not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn iter(self) -> impl Iterator<Item = usize> {
        self.start..self.end
    }
}

/// The day's share of a rolling cycle over `len` rows in slot order: the cycle's days cut the rows into equal runs,
/// and each day takes its own.
#[must_use]
pub fn rolling_slice(len: usize, cycle_days: u16, day: Day) -> Span {
    if cycle_days == 0 {
        violation!(clause = "N1", "a rolling cycle of no days");
    }
    let cycle = usize::from(cycle_days);
    let Ok(k) = usize::try_from(day.get() % u32::from(cycle_days)) else {
        violation!(clause = "N1", "a cycle position beyond the machine's words");
    };
    Span { start: len * k / cycle, end: len * (k + 1) / cycle }
}

/// What `read-trace` found on a day, when the run traces reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadTrace {
    pub undeclared_reads: usize,
    pub later_writes: usize,
    pub duplicate_opens: usize,
}

/// Everything the audit reads at a close, as shared borrows: the world's stores, the rows the day touched, and the
/// records and events written since the last close.
#[derive(Clone, Copy, Debug)]
pub struct AuditInputs<'a> {
    pub day: Day,
    pub register: &'a Register,
    pub directory: &'a Directory,
    pub calendar: &'a Calendar,
    pub records: &'a RecordStore,
    pub events: &'a EventStore,
    pub messages: &'a DayMessages,
    pub tables: &'a [KernelTable],
    pub touched: &'a TouchedRows,
    pub trace: Option<ReadTrace>,
    pub new_records: Span,
    pub new_events: Span,
}

/// What a family may read: only through `&self`, so checking changes nothing.
#[derive(Clone, Copy, Debug)]
pub struct FamilyCtx<'a> {
    inputs: AuditInputs<'a>,
    mode: FamilyMode,
}

impl<'a> FamilyCtx<'a> {
    #[must_use]
    pub fn new(inputs: AuditInputs<'a>, mode: FamilyMode) -> FamilyCtx<'a> {
        FamilyCtx { inputs, mode }
    }

    pub fn day(&self) -> Day {
        self.inputs.day
    }

    /// The sub-step the audit runs in; everything it reads was written before it.
    #[must_use]
    pub fn substep(&self) -> SubStep {
        AUDIT_SUBSTEP
    }

    #[must_use]
    pub fn register(&self) -> &Register {
        self.inputs.register
    }

    #[must_use]
    pub fn directory(&self) -> &Directory {
        self.inputs.directory
    }

    #[must_use]
    pub fn calendar(&self) -> &Calendar {
        self.inputs.calendar
    }

    #[must_use]
    pub fn records(&self) -> &RecordStore {
        self.inputs.records
    }

    #[must_use]
    pub fn events(&self) -> &EventStore {
        self.inputs.events
    }

    #[must_use]
    pub fn messages(&self) -> &DayMessages {
        self.inputs.messages
    }

    /// A table the kernel keeps, which the family must name rightly.
    #[must_use]
    pub fn table(&self, name: &str) -> &FactColumns {
        let Some(t) = self.inputs.tables.iter().find(|t| t.name == name) else {
            violation!(clause = "N1", "a family reading a table the world does not keep");
        };
        &t.columns
    }

    /// The rows of a table the day's applies touched, in slot order.
    pub fn touched(&self, table: TableId) -> impl Iterator<Item = Slot> + '_ {
        self.inputs.touched.rows(table)
    }

    /// The day's read trace, when the run traces reads.
    #[must_use]
    pub fn trace(&self) -> Option<ReadTrace> {
        self.inputs.trace
    }

    /// The records written since the last close.
    #[must_use]
    pub fn new_records(&self) -> Span {
        self.inputs.new_records
    }

    /// The events written since the last close.
    #[must_use]
    pub fn new_events(&self) -> Span {
        self.inputs.new_events
    }

    /// The family's slice today of a domain of `len` rows; only a rolling family has one.
    #[must_use]
    pub fn rolling(&self, len: usize) -> Span {
        let FamilyMode::Rolling { cycle_days } = self.mode else {
            violation!(clause = "N1", "a rolling slice read by a family that does not roll");
        };
        rolling_slice(len, cycle_days, self.inputs.day)
    }
}

/// A save loaded apart for the audit alone, which one family's injection changes by one unit before the audit reads
/// it; never the world that runs.
pub trait InjectTarget {
    fn day(&self) -> Day;
    /// An identity the directory never handed out.
    fn unhanded_party(&self) -> PartyId;
    /// A party the directory holds live, if any.
    fn live_party(&self) -> Option<PartyId>;
    /// Adds an entry about a party to one of the save's record kinds, dated as given.
    ///
    /// # Errors
    /// When the save declares no record kind.
    fn add_record(&mut self, subject: PartyId, day: Day, substep: SubStep) -> Result<(), String>;
    /// A fact of a kernel table's row as the save holds it.
    fn fact(&self, table: &str, fact: &'static str, slot: Slot) -> Missing<i64>;
    /// Sets a fact of a kernel table's row in the save.
    ///
    /// # Errors
    /// When the save keeps no such table or row.
    fn set_fact(&mut self, table: &str, fact: &'static str, slot: Slot, value: i64) -> Result<(), String>;
}

/// An audit family: it reads the world through its context and writes only findings, never repairing, and says how
/// many rows it checked. Its injection is a discrepancy that it alone sees.
pub trait AuditFamily: Send + Sync {
    fn decl(&self) -> FamilyDecl;
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64;
    /// # Errors
    /// When the save holds nothing the injection can change.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String>;
}

/// The sink the apply routine feeds as it applies, which the audit implements and the assembly injects, so no kernel
/// crate calls the audit above it.
pub trait AuditStream {
    fn applied(&mut self, instruction: u64);
    fn touched(&mut self, table: TableId, slot: Slot);
}

#[cfg(test)]
mod tests {
    use phx_id::Day;

    use super::{Span, rolling_slice};

    #[test]
    fn rolling_slices_cover_everything() {
        for (len, cycle) in [(0, 3), (7, 3), (100, 30), (5, 30), (1000, 7)] {
            let mut covered = Vec::new();
            for d in 40..40 + u32::from(cycle) {
                covered.extend(rolling_slice(len, cycle, Day::new(d)).iter());
            }
            covered.sort_unstable();
            assert_eq!(covered, (0..len).collect::<Vec<_>>(), "{len} rows over {cycle} days");
        }
        assert_eq!(rolling_slice(10, 5, Day::new(7)), Span { start: 4, end: 6 });
    }
}
