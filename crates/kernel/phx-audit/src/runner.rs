use phx_core::{
    AuditFamily, AuditInputs, AuditStream, Calendar, DayMessages, Directory, EventStore, FamilyCtx, FamilyDecl,
    Findings, KernelTable, ReadTrace, RecordStore, Register, Span,
};
use phx_id::Day;
use phx_macros::clause;

use crate::stream::StreamAudit;

/// What the close hands the audit to read: the world's stores as shared borrows, and the day's read trace.
#[derive(Clone, Copy, Debug)]
pub struct CloseInputs<'a> {
    pub day: Day,
    pub register: &'a Register,
    pub directory: &'a Directory,
    pub calendar: &'a Calendar,
    pub records: &'a RecordStore,
    pub events: &'a EventStore,
    pub messages: &'a DayMessages,
    pub tables: &'a [KernelTable],
    pub trace: Option<ReadTrace>,
    pub books: &'a dyn phx_core::BooksAudit,
    pub markets: &'a dyn phx_core::MarketsAudit,
}

/// One close: its day, how many families ran, the rows they checked, and the findings they recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseRecord {
    pub day: Day,
    pub families: usize,
    pub rows_checked: u64,
    pub findings: usize,
    pub applied: u64,
}

/// The audit: every declared family, run at each close over what the day left behind, and the sink the day's applies
/// fed. It reads the world and writes only findings.
#[clause("N1", "Law 17")]
pub struct Audit {
    families: Vec<Box<dyn AuditFamily>>,
    stream: StreamAudit,
    records_seen: usize,
    events_seen: usize,
}

impl std::fmt::Debug for Audit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.families.iter().map(|x| x.decl().name).collect();
        f.debug_struct("Audit").field("families", &names).field("stream", &self.stream).finish_non_exhaustive()
    }
}

impl Audit {
    /// # Errors
    /// Each family declared twice.
    pub fn new(families: Vec<Box<dyn AuditFamily>>) -> Result<Audit, Vec<String>> {
        let decls: Vec<FamilyDecl> = families.iter().map(|f| f.decl()).collect();
        let twice: Vec<String> = decls
            .iter()
            .enumerate()
            .filter(|(i, d)| decls.iter().skip(i + 1).any(|e| e.name == d.name))
            .map(|(_, d)| format!("audit family `{}` declared twice", d.name))
            .collect();
        if !twice.is_empty() {
            return Err(twice);
        }
        Ok(Audit { families, stream: StreamAudit::default(), records_seen: 0, events_seen: 0 })
    }

    /// The sink the apply routine feeds.
    pub fn stream(&mut self) -> &mut dyn AuditStream {
        &mut self.stream
    }

    pub fn families(&self) -> impl Iterator<Item = FamilyDecl> + '_ {
        self.families.iter().map(|f| f.decl())
    }

    /// Runs every family over what the day wrote and touched, then starts the next day's count.
    pub fn close(&mut self, c: CloseInputs<'_>, findings: &mut Findings) -> CloseRecord {
        let before = findings.len();
        let new_records = Span { start: self.records_seen, end: c.records.len() };
        let new_events = Span { start: self.events_seen, end: c.events.len() };
        let inputs = AuditInputs {
            day: c.day,
            register: c.register,
            directory: c.directory,
            calendar: c.calendar,
            records: c.records,
            events: c.events,
            messages: c.messages,
            tables: c.tables,
            touched: self.stream.touched_rows(),
            trace: c.trace,
            new_records,
            new_events,
            books: c.books,
            legs: self.stream.digests(),
            markets: c.markets,
        };
        let mut rows_checked = 0;
        for family in &self.families {
            let ctx = FamilyCtx::new(inputs, family.decl().mode);
            rows_checked += family.check(&ctx, findings);
        }
        let record = CloseRecord {
            day: c.day,
            families: self.families.len(),
            rows_checked,
            findings: findings.len() - before,
            applied: self.stream.applied_count(),
        };
        self.records_seen = new_records.end;
        self.events_seen = new_events.end;
        self.stream.clear();
        record
    }
}
