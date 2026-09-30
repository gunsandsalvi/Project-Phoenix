//! What a run cost: its turns' wall times, its memory a person, the cores it kept busy and the page faults it took, so
//! every step reads the budget and not only the gates.

use std::path::Path;

use phx_world::Inspector;
use serde_json::json;

/// The run's own counters, every one of which a ratchet may hold only where the run produces it.
const PRODUCED: &str = "phx_budget.";

/// The unit of the kernel's process times in `/proc`: Linux reports them in `USER_HZ`, a hundredth of a second on every
/// architecture it runs on.
const TICK_NS: u64 = 10_000_000;

/// The process's CPU time in nanoseconds and its minor page faults so far, from the kernel's own account.
#[must_use]
pub fn process_times() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // The command name may hold spaces, so the fields are counted from the parenthesis that closes it.
    let rest = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After the name the fields run from the state, the third: minor faults are the tenth, user and system time the
    // fourteenth and fifteenth.
    let field = |n: usize| -> Option<u64> { fields.get(n - 3)?.parse().ok() };
    let faults = field(10)?;
    let ticks = field(14)?.checked_add(field(15)?)?;
    Some((ticks.checked_mul(TICK_NS)?, faults))
}

/// The median and the greatest of the turns' wall times, in milliseconds; none for no turn.
#[must_use]
pub fn median_and_worst(turn_ns: &[u64]) -> Option<(u64, u64)> {
    let mut sorted = turn_ns.to_vec();
    sorted.sort_unstable();
    let median = sorted.get(sorted.len().checked_sub(1)? / 2)?;
    let worst = sorted.last()?;
    Some((median / 1_000_000, worst / 1_000_000))
}

/// The run's counts of the core's work: unit costs reckoned, meeting weights reckoned, sales made.
fn run_counts(w: Inspector<'_>) -> [u64; 3] {
    let c = &w.core().counts;
    [c.unit_costs.get(), c.weighed.get(), c.sales.get()]
}

/// The process's times, the pools' work and the core's counts when the days began.
pub struct Meter {
    started_ns: u64,
    times: Option<(u64, u64)>,
    pool: phx_exec::pool::PoolUsage,
    counts: [u64; 3],
}

impl Meter {
    #[must_use]
    pub fn start(now_ns: u64, w: Inspector<'_>) -> Meter {
        Meter { started_ns: now_ns, times: process_times(), pool: phx_exec::pool::usage(), counts: run_counts(w) }
    }

    /// The days' wall time, and the process's times, the pools' work and the core's counts over them.
    #[must_use]
    pub fn stop(self, now_ns: u64, w: Inspector<'_>) -> Span {
        let (pool, after) = (phx_exec::pool::usage(), run_counts(w));
        let delta = |a: u64, b: u64| a.checked_sub(b);
        Span {
            run_ns: now_ns.checked_sub(self.started_ns),
            before: self.times,
            after: process_times(),
            dispatches: delta(pool.dispatches, self.pool.dispatches),
            spun: delta(pool.spun, self.pool.spun),
            counts: [0, 1, 2].map(|i| after.get(i).zip(self.counts.get(i)).and_then(|(a, b)| delta(*a, *b))),
        }
    }
}

/// What the days took: their wall time, the process's CPU time and minor faults before and after them, the pools'
/// dispatches and spin rounds, and the core's counts of unit costs, meeting weights and sales over them.
#[derive(Clone, Copy)]
pub struct Span {
    pub run_ns: Option<u64>,
    before: Option<(u64, u64)>,
    after: Option<(u64, u64)>,
    dispatches: Option<u64>,
    spun: Option<u64>,
    counts: [Option<u64>; 3],
}

/// What the run measured beside the days: allocations a day after the first days, the spans of many items on too few
/// cores, and the process's baseline.
#[derive(Clone, Copy, Debug, Default)]
pub struct Beside {
    pub allocs_per_day: Option<f64>,
    pub spans_below_busy: Option<u64>,
    pub baseline_mb: Option<u64>,
}

/// The run's cost as the budget reads it: the counters its ratchets hold, and the report's block.
pub struct Cost {
    pub counters: Vec<(&'static str, f64)>,
    pub block: serde_json::Value,
}

/// A count over another as a rate; none where either is missing or the whole is none.
#[must_use]
pub fn per(part: Option<u64>, whole: Option<u64>) -> Option<f64> {
    let (part, whole) = (crate::run::real(part?), crate::run::real(whole.filter(|w| *w > 0)?));
    Some(part / whole)
}

/// The firms the core holds now, the party kind its unit costs are reckoned for.
fn firms(w: Inspector<'_>) -> Option<u64> {
    use phx_store::StoreStats;
    let core = w.core();
    core.names.iter().zip(&core.kinds).find(|(n, _)| **n == "firm").map(|(_, k)| k.rows_live())
}

/// The cost of the days run, from the turns' wall times, the peak resident memory over the world's persons, the
/// process times, the pools' work and the core's counts taken before and after the days, and what was measured beside.
#[must_use]
pub fn cost(w: Inspector<'_>, peak: Option<u64>, span: &Span, beside: &Beside) -> Cost {
    let Span { run_ns, before, after, dispatches, spun, counts } = *span;
    let turn_ns: Vec<u64> = w.turns().iter().filter_map(|t| t.wall_ns).collect();
    let days: u64 = w.turns().iter().map(|t| u64::from(t.days)).sum();
    let persons = w.core().persons_opened;
    let mut counters: Vec<(&'static str, f64)> = Vec::new();
    let mut put = |name: &'static str, v: Option<f64>| {
        if let Some(v) = v {
            counters.push((name, v));
        }
        v
    };
    let whole = |n: Option<u64>| n.map(crate::run::real);
    let turns = median_and_worst(&turn_ns);
    put("phx_budget.turn_ms_median", whole(turns.map(|t| t.0)));
    put("phx_budget.turn_ms_worst", whole(turns.map(|t| t.1)));
    let bytes_per_person = peak.and_then(|p| p.checked_div(persons));
    put("phx_budget.bytes_per_person", whole(bytes_per_person));
    let spent = before.zip(after).and_then(|((c0, f0), (c1, f1))| Some((c1.checked_sub(c0)?, f1.checked_sub(f0)?)));
    // Cores busy in hundredths, so the ratchet holds a whole number.
    let busy = spent.zip(run_ns).and_then(|((cpu, _), wall)| (cpu * 100).checked_div(wall));
    put("phx_budget.busy_core_hundredths", whole(busy));
    let faults = spent.and_then(|(_, f)| f.checked_div(days));
    put("phx_budget.faults_per_day", whole(faults));
    let [unit_costs, weighed, sales] = counts;
    let firm_days = firms(w).and_then(|f| f.checked_mul(days));
    let unit_cost = put("phx_budget.unit_cost_per_firm_day", per(unit_costs, firm_days));
    let meeting = put("phx_budget.meeting_work_per_sale", per(weighed, sales));
    let allocs = put("phx_budget.allocs_per_day", beside.allocs_per_day);
    let below = put("phx_budget.spans_below_busy", whole(beside.spans_below_busy));
    let barriers = put("phx_budget.barriers_per_day", per(dispatches, Some(days)));
    let spin_rate = put("phx_budget.spin_rounds_per_day", per(spun, Some(days)));
    let baseline = put("phx_budget.baseline_mb", whole(beside.baseline_mb));
    let block = json!({
        "persons": persons,
        "turn_ms_median": turns.map(|t| t.0),
        "turn_ms_worst": turns.map(|t| t.1),
        "bytes_per_person": bytes_per_person,
        "busy_core_hundredths": busy,
        "faults_per_day": faults,
        "unit_cost_per_firm_day": unit_cost,
        "meeting_work_per_sale": meeting,
        "allocs_per_day": allocs,
        "spans_below_busy": below,
        "barriers_per_day": barriers,
        "spin_rounds_per_day": spin_rate,
        "baseline_mb": baseline,
    });
    Cost { counters, block }
}

/// The run's cost judged against the budget's ratchets, when a file of them is given: the report's block, with each
/// ratchet the cost broke, and whether it broke none. Each failure is printed as it is found.
pub fn judge(
    w: Inspector<'_>,
    peak: Option<u64>,
    (span, beside): (&Span, &Beside),
    ratchets: Option<&Path>,
) -> Result<(serde_json::Value, bool), String> {
    let cost = cost(w, peak, span, beside);
    let failures = match ratchets {
        Some(path) => crate::run::check_ratchets(path, &cost.counters, Some(PRODUCED))?,
        None => Vec::new(),
    };
    for f in &failures {
        println!("budget: {f}");
    }
    let mut block = cost.block;
    if let Some(map) = block.as_object_mut() {
        map.insert("failures".to_owned(), json!(failures));
    }
    Ok((block, failures.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::{median_and_worst, per};
    use crate::run::judge_counters;

    #[test]
    fn unit_cost_ratio_arithmetic() {
        assert_eq!(per(Some(3_000), Some(1_000)), Some(3.0), "three reckonings a firm-day");
        assert_eq!(per(Some(3_000), Some(0)), None, "no firm-days, no rate");
        assert_eq!(per(None, Some(10)), None, "a count not taken is missing");
    }

    #[test]
    fn new_counters_are_judged() {
        let text = "[[ratchet]]\ncounter = \"phx_budget.meeting_work_per_sale\"\nvalue = 40.0\ndirection = \"down\"\n\
                    [[ratchet]]\ncounter = \"phx_budget.allocs_per_day\"\nvalue = 10\ndirection = \"down\"\n\
                    [[ratchet]]\ncounter = \"fin.day.b_core_ms\"\nvalue = 1.5\ndirection = \"down\"\n";
        let failures =
            judge_counters(text, &[("phx_budget.meeting_work_per_sale", 41.5)], Some("phx_budget.")).unwrap();
        assert_eq!(
            failures,
            [
                "`phx_budget.meeting_work_per_sale` is 41.5; its ratchet allows 40",
                "`phx_budget.allocs_per_day` was not produced by the run"
            ]
        );
    }

    #[test]
    fn budget_block_reads_median_and_worst() {
        let ms = 1_000_000;
        assert_eq!(median_and_worst(&[3 * ms, ms, 7 * ms, 2 * ms]), Some((2, 7)), "the lower middle of four");
        assert_eq!(median_and_worst(&[5 * ms]), Some((5, 5)));
        assert_eq!(median_and_worst(&[]), None, "no turn, no budget");
    }
}
