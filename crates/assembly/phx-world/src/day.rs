use phx_core::{Intents, QueuedIntent, SUB_STEPS, SubStep, SubStepInfo, SubStepKind};
use phx_exec::Clock;
use phx_exec::site::{self, Site};
use phx_id::Day;
use phx_macros::clause;
use phx_num::violation;

use crate::metrics::{SubStepRecord, TurnRecord};
use crate::world::World;

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

fn is_apply_point(info: &SubStepInfo) -> bool {
    APPLY_POINTS.contains(&info.step)
}

impl World {
    /// The last day run.
    pub fn today(&self) -> Day {
        self.today
    }

    /// Runs a turn: the player's intents are queued for 1c, and every day from the day after the last turn up to the
    /// next day that is a business day in some country runs, each as its own day.
    #[clause("TIME.6", "N8.2")]
    pub fn run_turn(&mut self, intents: &[QueuedIntent], clock: &dyn Clock) -> TurnRecord {
        let start = clock.now_ns();
        for intent in intents {
            self.queue.push(intent.clone());
        }
        let first = self.today.succ();
        let last = self.calendar.next_turn_day(self.today);
        let mut days = 0_u32;
        loop {
            let day = self.today.succ();
            self.run_day(day);
            self.today = day;
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
    fn run_day(&mut self, day: Day) {
        let any_business = self.calendar.any_business(day);
        self.day_messages.lapse();
        let mut pending = Intents::default();
        for info in &SUB_STEPS {
            let has_handlers = self.graph.at(info.step).next().is_some();
            let runs = if info.kind == SubStepKind::KernelApply {
                stage_runs(info, any_business)
            } else {
                has_handlers && (any_business || !info.business_only)
            };
            if !runs {
                continue;
            }
            site::enter(Site { day: day.get(), substep: info.step.ordinal(), handler: 0, chunk: 0 });
            if let Some((id, handler)) = self.graph.at(info.step).next() {
                violation!(
                    clause = "TIME.6",
                    "a handler registered before its table can be traversed",
                    handler = id.0,
                    reads = handler.reads.len()
                );
            }
            if is_apply_point(info) {
                apply(&mut pending);
            }
            site::leave();
            self.metrics.substeps.push(SubStepRecord {
                day,
                substep: info.step.ordinal(),
                rows: 0,
                bytes: 0,
                barriers: 0,
            });
        }
        let date = self.calendar.date(day);
        if date.month() == 1 && date.day() == 1 {
            self.calendar.move_window(date.year());
        }
        if self.read_trace {
            self.trace.close_day();
        }
    }
}

/// The one apply routine: until settlement exists it refuses every intent, since none can be applied yet.
fn apply(pending: &mut Intents) {
    if !pending.is_empty() {
        violation!(clause = "TIME.6", "an intent before the apply routine can settle it", intents = pending.len());
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{SUB_STEPS, SubStepKind};

    use super::{is_apply_point, stage_runs};

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
