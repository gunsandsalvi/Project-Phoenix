use phx_audit::CloseInputs;
use phx_core::StreamDef;
use phx_core::{
    AUDIT_SUBSTEP, ColumnTrace, CtxParts, EventIntent, EventStore, FactStore, IntentDef, Intents, NewEvent,
    QueuedIntent, ReadTrace, SUB_STEPS, SubStep, SubStepInfo, SubStepKind,
};
use phx_exec::Clock;
use phx_exec::site::{self, Site};
use phx_id::Day;
use phx_ledger::apply_batch::DaySettlement;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::prims::ClearedStream;
use phx_rand::{Subject, SubjectTag};
use phx_store::consts::DEFAULT_ROWS_PER_CHUNK;

use crate::metrics::{SubStepRecord, TurnRecord};
use crate::trace::{Open, traced};
use crate::world::{Settled, World};

/// The stage a sub-step belongs to: the number its label begins with.
fn stage(info: &SubStepInfo) -> &str {
    info.label.trim_end_matches(|c: char| c.is_ascii_lowercase())
}

/// Whether a stage runs on a day: every day if any of its sub-steps does, otherwise only when some country has a
/// business day.
fn stage_runs(info: &SubStepInfo, any_business: bool) -> bool {
    any_business || SUB_STEPS.iter().any(|i| stage(i) == stage(info) && !i.business_only)
}

/// Where gathered intents are applied: each stage's kernel apply, the end of 3e for stage 3, and the end of their own
/// sub-step for stages 1 and 8, for 7d and 7e, and for 10c to 10f.
const APPLY_POINTS: [SubStep; 23] = [
    SubStep::S1a,
    SubStep::S1b,
    SubStep::S1c,
    SubStep::S2f,
    SubStep::S3e,
    SubStep::S4b,
    SubStep::S5d,
    SubStep::S6d,
    SubStep::S7c,
    SubStep::S7d,
    SubStep::S7e,
    SubStep::S8a,
    SubStep::S8b,
    SubStep::S8c,
    SubStep::S8d,
    SubStep::S8e,
    SubStep::S8f,
    SubStep::S9e,
    SubStep::S10b,
    SubStep::S10c,
    SubStep::S10d,
    SubStep::S10e,
    SubStep::S10f,
];

/// Sub-steps where the kernel works though no handler runs there: 1b's marking of the lines due today, stage 2's
/// contract process at 2d, over the fails of the days since it last ran, 3b's screening of the population's cells and
/// 3e's outcomes of their hits, 9b's accounts, posting the day's settled money, and 10a's public events.
pub const KERNEL_WORK: [SubStep; 6] =
    [SubStep::S1b, SubStep::S2d, SubStep::S3b, SubStep::S3e, SubStep::S9b, SubStep::S10a];

/// The audit's sub-step, which runs every day.
pub const AUDIT_AT: SubStep = AUDIT_SUBSTEP;

/// A chunk's rows: a whole chunk, or what is left of the table in its last.
fn chunk_rows(chunk: u32, rows: u32) -> core::ops::Range<u32> {
    let start = chunk * DEFAULT_ROWS_PER_CHUNK;
    let full = start + DEFAULT_ROWS_PER_CHUNK;
    start..if full > rows { rows } else { full }
}

fn is_apply_point(info: &SubStepInfo) -> bool {
    APPLY_POINTS.contains(&info.step)
}

impl World {
    /// The last day run.
    pub fn today(&self) -> Day {
        self.today
    }

    /// Runs a turn with no observer.
    pub fn run_turn(&mut self, intents: &[QueuedIntent], clock: &dyn Clock) -> TurnRecord {
        self.run_turn_observed(intents, clock, None)
    }

    /// Runs a turn: the player's intents are queued for 1c, and every day from the day after the last turn up to the
    /// next day that is a business day in some country runs, each as its own day, the observer reading each once it
    /// has ended; its reading is inside the turn's time, as the phone's views are.
    #[clause("TIME.6", "N8.2", "Law 17")]
    pub fn run_turn_observed(
        &mut self,
        intents: &[QueuedIntent],
        clock: &dyn Clock,
        mut observer: Option<&mut dyn crate::observe::Observer>,
    ) -> TurnRecord {
        let start = clock.now_ns();
        for intent in intents {
            self.queue.push(intent.clone());
        }
        let first = self.today.succ();
        let last = self.calendar.next_turn_day(self.today);
        let mut days = 0_u32;
        loop {
            let day = self.today.succ();
            let traced = observer.as_deref().map(|o| -> &dyn phx_core::TracedCells { o });
            self.run_day(day, traced, clock);
            self.today = day;
            if let Some(o) = observer.as_deref_mut() {
                o.day_closed(crate::Inspector::new(self));
            }
            // The parties ended today were held for the day's legs and the observer's reading; the day is over.
            self.books.parties.cells_mut().1.close_day();
            days += 1;
            if day == last {
                break;
            }
        }
        let wall_ns = clock.now_ns().checked_sub(start);
        let record = TurnRecord { first, last, days, wall_ns };
        self.metrics.turns.push(record);
        record
    }

    /// Runs one day: each sub-step in the table's order, skipping one with no handlers unless it is a kernel apply of
    /// a stage that runs, and one that runs only on business days when no country has one.
    #[clause("TIME.6", "TIME.8")]
    fn run_day(&mut self, day: Day, traced: Option<&dyn phx_core::TracedCells>, clock: &dyn Clock) {
        let any_business = self.calendar.any_business(day);
        self.split_log.splits.clear();
        self.day_messages.lapse();
        let mut pending: Vec<(SubStep, Intents)> = Vec::new();
        let mut dues = DaySettlement::default();
        for info in &SUB_STEPS {
            let has_handlers = self.graph.at(info.step).next().is_some();
            let runs = if info.step == AUDIT_AT {
                true
            } else if info.kind == SubStepKind::KernelApply {
                stage_runs(info, any_business)
            } else if KERNEL_WORK.contains(&info.step) {
                any_business || !info.business_only
            } else {
                has_handlers && (any_business || !info.business_only)
            };
            if !runs {
                continue;
            }
            let started = clock.now_ns();
            let rows = self.dispatch(day, info.step, &mut pending);
            site::enter(Site { day: day.get(), substep: info.step.ordinal(), handler: 0, chunk: 0 });
            if info.step == SubStep::S1b {
                self.due = self.books.ledger.mark_due(day, &self.calendar);
            }
            if info.step == SubStep::S3b {
                self.catastrophe_losses(day);
                self.cells_screen(day);
            }
            if info.step == SubStep::S3e {
                self.cells_outcomes(day);
            }
            if info.step == SubStep::S10a {
                // Yesterday's events recorded after its 10a are judged today; the rest are judged again the same way.
                let from = day.get().checked_sub(1).map_or(day, Day::new);
                self.events.publish(from, &self.news);
            }
            if info.step == SubStep::S10b {
                self.cells_settle(day, traced);
            }
            if info.step == SubStep::S2d {
                self.process_fails(day);
            }
            if is_apply_point(info) {
                apply(day, &mut pending, &mut self.events, self.event_kinds.len());
            }
            if info.step == SubStep::S7c {
                let streams = &self.streams;
                let draws_of = |line: phx_id::LineId| {
                    let subject = Subject::new(SubjectTag::Line, u64::from(line.get()));
                    streams.open(&ClearedStream::DECL, subject, day, SubStep::S7b.ordinal())
                };
                dues =
                    self.books.settle_day(&self.due, day, &self.calendar, &self.closed, &draws_of, self.audit.stream());
                self.estates_settle(day);
            }
            if info.step == SubStep::S9b {
                let period = crate::registry::period_of(&self.calendar, day);
                self.accounts.post_day(self.books.ledger.day_book(), period);
            }
            if info.step == AUDIT_AT {
                self.close(day, dues);
            }
            site::leave();
            self.metrics.substeps.push(SubStepRecord {
                day,
                substep: info.step.ordinal(),
                rows,
                bytes: 0,
                barriers: 0,
                wall_ns: clock.now_ns().checked_sub(started),
            });
        }
        let date = self.calendar.date(day);
        if date.month() == 1 && date.day() == 1 {
            self.calendar.move_window(date.year());
        }
    }

    /// Runs a sub-step: one traversal per table, in the order the world keeps its tables, over the table's chunks in
    /// order, running every handler of the sub-step on the table on each chunk, in canonical order, with a context
    /// built for the chunk. Intents join the pending list in (table, chunk, handler) order. A traced chunk records its
    /// reads, its writes and the streams it opens. Returns the rows visited.
    #[clause("TIME.6", "TIME.10")]
    fn dispatch(&mut self, day: Day, step: SubStep, pending: &mut Vec<(SubStep, Intents)>) -> u64 {
        let mut visited = 0_u64;
        let date = self.calendar.date(day);
        for table in &mut self.tables {
            let handlers: Vec<_> = self.graph.at(step).filter(|(_, h)| h.table == table.name).collect();
            if handlers.is_empty() {
                continue;
            }
            let rows = table.columns.rows();
            for chunk in 0..rows.div_ceil(DEFAULT_ROWS_PER_CHUNK) {
                let span = chunk_rows(chunk, rows);
                visited += u64::from(span.end - span.start);
                for (id, h) in &handlers {
                    let Missing::Present(run) = h.run else {
                        violation!(clause = "TIME.6", "a handler with no body", handler = id.0);
                    };
                    let Some((_, own)) = self.own.iter().find(|(system, _)| *system == h.system) else {
                        violation!(clause = "TIME.6", "a handler whose system compiled no state", handler = id.0);
                    };
                    let first = !self.traced_first.contains(id);
                    if first {
                        self.traced_first.push(*id);
                    }
                    let is_traced = self.read_trace && traced(chunk, first, day);
                    site::enter(Site { day: day.get(), substep: step.ordinal(), handler: id.0, chunk });
                    table.columns.trace(is_traced.then_some(ColumnTrace {
                        substep: step.ordinal(),
                        handler: id.0,
                        reads: h.reads,
                        writes: h.writes,
                    }));
                    let (mut intents, mut opens) = (Intents::default(), Vec::new());
                    run(
                        CtxParts {
                            day,
                            date,
                            streams: &self.streams,
                            register: &self.register,
                            own: own.as_ref(),
                            facts: {
                                let facts: &mut dyn FactStore = &mut table.columns;
                                facts
                            },
                            intents: &mut intents,
                            bindings: &mut self.bindings,
                            rules: &self.rules,
                            queue: &mut self.queue,
                            opens: is_traced.then_some(&mut opens),
                        },
                        span.clone(),
                    );
                    table.columns.trace(None);
                    for (stream, subject) in opens {
                        self.trace.opened(Open { stream, subject, substep: step.ordinal() });
                    }
                    pending.push((step, intents));
                    site::leave();
                }
            }
        }
        visited
    }

    /// The contract process over the fails waiting since the last business day, each read against the party that
    /// holds its row now: a cell that landed since is its successor.
    #[clause("SET.3", "PTY.10")]
    fn process_fails(&mut self, day: Day) {
        let fails = std::mem::take(&mut self.unprocessed);
        let directory = self.books.parties.directory();
        let now: Vec<phx_ledger::fails::Fail> = fails
            .iter()
            .map(|f| match directory.resolve(f.party) {
                phx_core::Resolved::Live(party, _) => phx_ledger::fails::Fail { party, ..*f },
                phx_core::Resolved::Ended(_) | phx_core::Resolved::Unknown => *f,
            })
            .collect();
        self.books.contract_process(&now, day);
        let directory = self.books.parties.cells_mut().1;
        for f in &fails {
            directory.release(f.party);
        }
    }

    /// The day's close: the read trace sums the day, and every audit family reads what the day left behind.
    #[clause("N1")]
    fn close(&mut self, day: Day, dues: DaySettlement) {
        self.books.parties.compact_arenas();
        self.books.ledger.flush_outside(self.audit.stream());
        let book = self.books.close();
        self.accounts.close_day(&book);
        self.settlements.push(Settled { day, measure: book.measure(), dues, fails: book.fails.clone() });
        // A fail waits for the next business day's contract process, and its party may end before then.
        let directory = self.books.parties.cells_mut().1;
        for f in &book.fails {
            directory.retain(f.party);
        }
        self.unprocessed.extend(book.fails);
        let mut reads = ReadTrace::default();
        for t in &mut self.tables {
            let found = t.columns.take_trace();
            reads.undeclared_reads += found.undeclared_reads;
            reads.later_writes += found.later_writes;
        }
        let trace = self.read_trace.then(|| self.trace.close_day(reads));
        let record = self.audit_close(day, trace);
        self.metrics.closes.0.push(record);
    }

    /// Every audit family over what the world holds at a day's close, and the day's accounts then done.
    pub(crate) fn audit_close(&mut self, day: Day, trace: Option<ReadTrace>) -> phx_audit::CloseRecord {
        let lines = &self.books.ledger.lines;
        let roles = |line: phx_id::LineId, side: phx_ledger::algebra::Side| lines.side_decl(line, side).holder_roles;
        let inputs = CloseInputs {
            day,
            register: &self.register,
            directory: self.books.parties.directory(),
            calendar: &self.calendar,
            records: &self.records,
            events: &self.events,
            messages: &self.day_messages,
            tables: &self.tables,
            trace,
            books: &self.books,
            markets: &self.markets,
            accounts: &phx_acct::audit::AccountsView::new(&self.books, &self.accounts),
            cells: &self.population.view::<phx_store::SystemBacking>(self.books.parties.cells(), &roles),
            own: &self.own,
        };
        let record = self.audit.close(inputs, &mut self.findings);
        self.accounts.end_day();
        record
    }
}

/// The one apply routine, in the order the intents were gathered: an event is recorded, dated by the sub-step that
/// drew it; until settlement exists every other intent is refused, since none can be applied yet.
#[clause("TIME.6", "CHN.4")]
fn apply(day: Day, pending: &mut Vec<(SubStep, Intents)>, events: &mut EventStore, kinds: usize) {
    for (step, intents) in pending.drain(..) {
        for (name, words) in intents.iter() {
            if name != EventIntent::NAME {
                violation!(clause = "TIME.6", "an intent before the apply routine can settle it", words = words.len());
            }
            let Some(e) = EventIntent::decode(words) else {
                violation!(clause = "CHN.4", "an event intent its words do not encode", words = words.len());
            };
            if usize::from(e.kind) >= kinds {
                violation!(clause = "CHN.4", "an event of a kind never declared", kind = e.kind);
            }
            events.record(NewEvent {
                day,
                substep: step,
                kind: e.kind,
                subjects: &e.subjects,
                details: &e.details,
                develops_from: Missing::Absent,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{SUB_STEPS, SubStepKind};

    use phx_store::consts::DEFAULT_ROWS_PER_CHUNK;

    use super::{chunk_rows, is_apply_point, stage_runs};

    #[test]
    fn chunks_cover_the_table_once() {
        let rows = DEFAULT_ROWS_PER_CHUNK * 2 + 5;
        let spans: Vec<_> = (0..rows.div_ceil(DEFAULT_ROWS_PER_CHUNK)).map(|c| chunk_rows(c, rows)).collect();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[2], DEFAULT_ROWS_PER_CHUNK * 2..rows);
        assert!(spans.windows(2).all(|w| w[0].end == w[1].start) && spans[0].start == 0);
    }

    #[test]
    fn substep_table_matches_architecture() {
        let business: Vec<&str> = SUB_STEPS.iter().filter(|i| i.business_only).map(|i| i.label).collect();
        let expected = [
            "2b", "2c", "2d", "2e", "6c", "7a", "7b", "7c", "7d", "7e", "8a", "8b", "8c", "8d", "8e", "8f", "9a", "9b",
            "9c", "9d", "9e", "10c",
        ];
        assert_eq!(business, expected);
        let applies: Vec<&str> =
            SUB_STEPS.iter().filter(|i| i.kind == SubStepKind::KernelApply).map(|i| i.label).collect();
        assert_eq!(applies, ["2f", "4b", "5d", "6d", "7c", "9e", "10b"]);
        let seven_c = SUB_STEPS.iter().find(|i| i.label == "7c").unwrap();
        let two_f = SUB_STEPS.iter().find(|i| i.label == "2f").unwrap();
        assert!(!stage_runs(seven_c, false) && stage_runs(two_f, false), "stage 7 rests on a day no market opens");
        let points: Vec<&str> = SUB_STEPS.iter().filter(|i| is_apply_point(i)).map(|i| i.label).collect();
        assert!(applies.iter().all(|a| points.contains(a)), "every kernel apply is an apply point");
        let others =
            ["1a", "1b", "1c", "3e", "7d", "7e", "8a", "8b", "8c", "8d", "8e", "8f", "10c", "10d", "10e", "10f"];
        assert!(others.iter().all(|a| points.contains(a)) && points.len() == applies.len() + others.len());
    }
}
