//! What a run cost: its turns' wall times, its memory a person, the cores it kept busy and the page faults it took, so
//! every step reads the budget and not only the gates.

use std::path::Path;

use phx_world::Inspector;
use serde_json::json;

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

/// The process's times when the days began.
pub struct Meter {
    started_ns: u64,
    times: Option<(u64, u64)>,
}

impl Meter {
    #[must_use]
    pub fn start(now_ns: u64) -> Meter {
        Meter { started_ns: now_ns, times: process_times() }
    }

    /// The days' wall time and the process's times before and after them.
    #[must_use]
    pub fn stop(self, now_ns: u64) -> Span {
        Span { run_ns: now_ns.checked_sub(self.started_ns), before: self.times, after: process_times() }
    }
}

/// What the days took: their wall time, and the process's CPU time and minor faults before and after them.
#[derive(Clone, Copy)]
pub struct Span {
    pub run_ns: Option<u64>,
    before: Option<(u64, u64)>,
    after: Option<(u64, u64)>,
}

/// The run's cost as the budget reads it: the counters its ratchets hold, and the report's block.
pub struct Cost {
    pub counters: Vec<(&'static str, u64)>,
    pub block: serde_json::Value,
}

/// The cost of the days run, from the turns' wall times, the peak resident memory over the world's persons, and the
/// process times taken before and after the days.
#[must_use]
pub fn cost(w: Inspector<'_>, peak: Option<u64>, span: &Span) -> Cost {
    let Span { run_ns, before, after } = *span;
    let turn_ns: Vec<u64> = w.turns().iter().filter_map(|t| t.wall_ns).collect();
    let days: u64 = w.turns().iter().map(|t| u64::from(t.days)).sum();
    let persons = w.core().persons_opened;
    let mut counters = Vec::new();
    let turns = median_and_worst(&turn_ns);
    if let Some((median, worst)) = turns {
        counters.push(("phx_budget.turn_ms_median", median));
        counters.push(("phx_budget.turn_ms_worst", worst));
    }
    let bytes_per_person = peak.and_then(|p| p.checked_div(persons));
    if let Some(b) = bytes_per_person {
        counters.push(("phx_budget.bytes_per_person", b));
    }
    let spent = before.zip(after).and_then(|((c0, f0), (c1, f1))| Some((c1.checked_sub(c0)?, f1.checked_sub(f0)?)));
    // Cores busy in hundredths, so the ratchet holds a whole number.
    let busy = spent.zip(run_ns).and_then(|((cpu, _), wall)| (cpu * 100).checked_div(wall));
    if let Some(b) = busy {
        counters.push(("phx_budget.busy_core_hundredths", b));
    }
    let faults = spent.and_then(|(_, f)| f.checked_div(days));
    if let Some(f) = faults {
        counters.push(("phx_budget.faults_per_day", f));
    }
    let block = json!({
        "persons": persons,
        "turn_ms_median": turns.map(|t| t.0),
        "turn_ms_worst": turns.map(|t| t.1),
        "bytes_per_person": bytes_per_person,
        "busy_core_hundredths": busy,
        "faults_per_day": faults,
    });
    Cost { counters, block }
}

/// The run's cost judged against the budget's ratchets, when a file of them is given: the report's block, with each
/// ratchet the cost broke, and whether it broke none. Each failure is printed as it is found.
pub fn judge(
    w: Inspector<'_>,
    peak: Option<u64>,
    span: &Span,
    ratchets: Option<&Path>,
) -> Result<(serde_json::Value, bool), String> {
    let cost = cost(w, peak, span);
    let failures = match ratchets {
        Some(path) => crate::run::check_ratchets(path, &cost.counters)?,
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
    use super::median_and_worst;

    #[test]
    fn budget_block_reads_median_and_worst() {
        let ms = 1_000_000;
        assert_eq!(median_and_worst(&[3 * ms, ms, 7 * ms, 2 * ms]), Some((2, 7)), "the lower middle of four");
        assert_eq!(median_and_worst(&[5 * ms]), Some((5, 5)));
        assert_eq!(median_and_worst(&[]), None, "no turn, no budget");
    }
}
