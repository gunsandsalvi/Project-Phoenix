use phx_id::Slot;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

use crate::family::ReadTrace;
use crate::handler::FactStore;

/// The handler running on a traced chunk: its sub-step, its canonical id, and the facts it declares it reads and
/// writes.
#[derive(Clone, Copy, Debug)]
pub struct ColumnTrace {
    pub substep: u8,
    pub handler: u16,
    pub reads: &'static [&'static str],
    pub writes: &'static [&'static str],
}

/// A table the kernel keeps, with a column per fact its handlers read or write.
#[derive(Debug, phx_macros::Saved)]
pub struct KernelTable {
    pub name: &'static str,
    pub columns: FactColumns,
}

/// A kernel table's facts as columns, one per declared fact, each row's value present or missing; while a traced
/// chunk runs, every write is stamped with its sub-step and handler, and every read is checked against the reader's
/// declaration and against a later stamp.
#[clause("TIME.10")]
#[derive(Debug)]
pub struct FactColumns {
    rows: u32,
    columns: Vec<(&'static str, Vec<Missing<i64>>)>,
    stamps: Vec<Vec<Option<(u8, u16)>>>,
    reader: Option<ColumnTrace>,
    found: ReadTrace,
}

impl FactColumns {
    #[must_use]
    pub fn new(rows: u32, facts: &[&'static str]) -> FactColumns {
        let Ok(n) = usize::try_from(rows) else {
            capacity_exceeded!("table rows", usize::MAX, rows);
        };
        FactColumns {
            rows,
            columns: facts.iter().map(|f| (*f, vec![Missing::Absent; n])).collect(),
            stamps: facts.iter().map(|_| vec![None; n]).collect(),
            reader: None,
            found: ReadTrace::default(),
        }
    }

    #[must_use]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// The facts the table carries, in the order declared.
    pub fn facts(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.columns.iter().map(|(f, _)| *f)
    }

    fn column(&self, fact: &str) -> usize {
        let Some(i) = self.columns.iter().position(|(f, _)| *f == fact) else {
            violation!(clause = "TIME.10", "a fact the table does not carry");
        };
        i
    }

    /// The cell of a fact's column at a row, which the table must have.
    fn cell(&self, column: usize, slot: Slot) -> (usize, Missing<i64>) {
        let row = usize::try_from(slot.get()).ok();
        let found = self.columns.get(column).and_then(|(_, c)| row.and_then(|r| c.get(r).map(|v| (r, *v))));
        let Some(found) = found else {
            violation!(clause = "TIME.10", "a row beyond the table's rows", row = slot.get());
        };
        found
    }

    /// A row's value, read outside any handler: by the audit, the observer or the tests.
    pub fn value(&self, fact: &str, slot: Slot) -> Missing<i64> {
        self.cell(self.column(fact), slot).1
    }

    /// Feeds every column to the world's hash, in the order declared, a missing value apart from every present one.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        for (fact, values) in &self.columns {
            h.bytes(fact.as_bytes());
            for v in values {
                match v {
                    Missing::Present(x) => {
                        h.u64(1);
                        h.u64(x.cast_unsigned());
                    }
                    Missing::Absent => h.u64(0),
                }
            }
        }
    }

    /// Traces the reads and writes of the handler about to run on a traced chunk, or stops tracing.
    pub fn trace(&mut self, reader: Option<ColumnTrace>) {
        self.reader = reader;
    }

    /// What the trace found since it was last taken, and the day's stamps cleared.
    pub fn take_trace(&mut self) -> ReadTrace {
        for s in &mut self.stamps {
            s.fill(None);
        }
        std::mem::take(&mut self.found)
    }
}

/// A table's facts saved as their values alone: the trace's stamps and findings live for a day, and a save is taken
/// at a day's close, when they are empty.
impl phx_store::Saved for FactColumns {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        self.rows.save(w);
        self.columns.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<FactColumns, phx_store::LoadError> {
        let rows = u32::load(r)?;
        let columns: Vec<(&'static str, Vec<Missing<i64>>)> = phx_store::Saved::load(r)?;
        let names: Vec<&'static str> = columns.iter().map(|(n, _)| *n).collect();
        let mut out = FactColumns::new(rows, &names);
        for ((_, into), (_, values)) in out.columns.iter_mut().zip(columns) {
            if values.len() != into.len() {
                return Err(phx_store::LoadError::Invalid("a fact column of another length than its table".to_owned()));
            }
            *into = values;
        }
        Ok(out)
    }
}

impl FactStore for FactColumns {
    fn read(&mut self, fact: &'static str, slot: Slot) -> Missing<i64> {
        let i = self.column(fact);
        let (row, value) = self.cell(i, slot);
        if let Some(r) = self.reader {
            if !r.reads.contains(&fact) && !r.writes.contains(&fact) {
                self.found.undeclared_reads += 1;
            }
            if self.stamps.get(i).and_then(|s| s.get(row)).copied().flatten().is_some_and(|(sub, _)| sub > r.substep) {
                self.found.later_writes += 1;
            }
        }
        value
    }

    fn write(&mut self, fact: &'static str, slot: Slot, value: i64) {
        let i = self.column(fact);
        let (row, _) = self.cell(i, slot);
        if let Some(cell) = self.columns.get_mut(i).and_then(|(_, c)| c.get_mut(row)) {
            *cell = Missing::Present(value);
        }
        if let Some(r) = self.reader
            && let Some(stamp) = self.stamps.get_mut(i).and_then(|s| s.get_mut(row))
        {
            *stamp = Some((r.substep, r.handler));
        }
    }
}

/// One more move of a fact to a new value, counted by its name.
pub fn count_moved(moved: &mut Vec<(&'static str, u64)>, fact: &'static str) {
    match moved.iter_mut().find(|(f, _)| *f == fact) {
        Some((_, n)) => *n += 1,
        None => moved.push((fact, 1)),
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;
    use phx_num::Missing;

    use super::{ColumnTrace, FactColumns};
    use crate::decisions::PlayerQueue;
    use crate::handler::{Ctx, CtxParts, FactStore, HandlerDecl, Intents};
    use crate::register::RegisterBuilder;
    use crate::register::limit::Bindings;
    use crate::rules::RuleTable;
    use crate::streams::Streams;

    crate::declare_fact! {
        pub Rain = "GEO.rain" { value: Count, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3" }
    }

    crate::declare_fact! {
        pub Wind = "GEO.wind" { value: Count, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3" }
    }

    fn step<S: FactStore + ?Sized>(ctx: &mut Ctx<'_, Wet, S>, row: Slot) {
        let start = match ctx.read::<Rain>(row) {
            Missing::Present(v) => v,
            Missing::Absent => *ctx.own::<i64>(),
        };
        ctx.write::<Rain>(row, start + 1);
    }

    crate::declare_handler! {
        pub Wet = "GEO.wet" { substep: S3a, table: "region", reads: [Rain], writes: [Rain], clause: "CHN.3", body: step }
    }

    #[test]
    fn a_body_runs_over_its_chunk_rows() {
        let Missing::Present(run) = Wet::RUN else { panic!("the body is registered with the declaration") };
        let mut t = FactColumns::new(4, &["GEO.rain", "GEO.wind"]);
        t.write("GEO.rain", Slot::new(2), 40);
        let streams = Streams::new(phx_rand::Seed::new(1), &[]).unwrap();
        let register = RegisterBuilder::default().build(&[], 1).unwrap();
        let (mut intents, mut bindings, mut queue, rules) =
            (Intents::default(), Bindings::default(), PlayerQueue::unseated(), RuleTable::default());
        let parts = CtxParts {
            day: phx_id::Day::new(1),
            date: phx_id::Date::new(2025, 1, 2).unwrap(),
            streams: &streams,
            register: &register,
            own: &10_i64,
            facts: {
                let facts: &mut dyn FactStore = &mut t;
                facts
            },
            intents: &mut intents,
            bindings: &mut bindings,
            rules: &rules,
            queue: &mut queue,
            opens: None,
        };
        run(parts, 1..3);
        let rain: Vec<Missing<i64>> = (0..4).map(|r| t.value("GEO.rain", Slot::new(r))).collect();
        assert_eq!(rain, [Missing::Absent, Missing::Present(11), Missing::Present(41), Missing::Absent]);
        assert_eq!(t.value("GEO.wind", Slot::new(1)), Missing::Absent);
        let _ = Wind;
    }

    #[test]
    fn traced_reads_find_undeclared_and_later_writes() {
        let mut t = FactColumns::new(3, &["GEO.rain", "GEO.wind"]);
        let row = Slot::new(1);
        t.trace(Some(ColumnTrace { substep: 12, handler: 0, reads: &[], writes: &["GEO.rain"] }));
        t.write("GEO.rain", row, 40);
        t.trace(Some(ColumnTrace { substep: 9, handler: 1, reads: &["GEO.rain"], writes: &[] }));
        assert_eq!(t.read("GEO.rain", row), Missing::Present(40));
        assert_eq!(t.read("GEO.wind", row), Missing::Absent);
        t.trace(None);
        assert_eq!(t.read("GEO.wind", row), Missing::Absent, "untraced reads are not counted");
        let found = t.take_trace();
        assert_eq!((found.undeclared_reads, found.later_writes, found.duplicate_opens), (1, 1, 0));
        assert_eq!(t.take_trace(), super::ReadTrace::default());
        assert_eq!(t.value("GEO.rain", row), Missing::Present(40));
    }
}
