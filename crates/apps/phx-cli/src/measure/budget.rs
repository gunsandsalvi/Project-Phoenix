//! The budget as measured: the phone's turns, sub-steps and memory from its device report, the full-load bench's
//! month, and stage 7's unit costs from each phone turn's own time and counts.

use serde_json::{Value, json};

/// The median of the values, the lower of the middle two for an even count; absent for none.
fn median(mut v: Vec<u64>) -> Option<u64> {
    v.sort_unstable();
    let at = v.len().checked_sub(1)? / 2;
    v.get(at).copied()
}

fn greatest(v: &[u64]) -> Option<u64> {
    v.iter().copied().reduce(|a, b| if b > a { b } else { a })
}

fn u64_at(v: &Value, path: &[&str]) -> Option<u64> {
    path.iter().try_fold(v, |at, k| at.get(k))?.as_u64()
}

/// The phone's turns of each kind — those ending on a business day and those not — with their wall times.
fn turns(device: &Value, business: bool) -> Vec<&Value> {
    let all = device.pointer("/world/turns").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    all.iter().filter(|t| t.get("business").and_then(Value::as_bool) == Some(business)).collect()
}

/// Each sub-step's median wall time over the turns given, by its label, in the day's order.
fn substeps(turns: &[&Value]) -> Vec<(String, Option<u64>)> {
    let mut labels: Vec<String> = Vec::new();
    for t in turns {
        for s in t.get("substeps").and_then(Value::as_array).into_iter().flatten() {
            if let Some(l) = s.get("substep").and_then(Value::as_str)
                && !labels.iter().any(|x| x == l)
            {
                labels.push(l.to_owned());
            }
        }
    }
    labels
        .into_iter()
        .map(|l| {
            let times = turns
                .iter()
                .filter_map(|t| {
                    t.get("substeps")?
                        .as_array()?
                        .iter()
                        .find(|s| s.get("substep").and_then(Value::as_str) == Some(l.as_str()))
                })
                .filter_map(|s| s.get("wall_ns")?.as_u64())
                .collect();
            (l, median(times))
        })
        .collect()
}

/// Stage 7's cost per unit of a count, from the phone's own turns: in each turn that settled any, the stage's time
/// (all of it runs in 7c's block) over the turn's own count; the median of those, and how many turns gave one.
fn per_unit(turns: &[&Value], count: &str) -> Value {
    let costs: Vec<u64> = turns
        .iter()
        .filter_map(|t| {
            let n = t.get(count)?.as_u64().filter(|n| *n > 0)?;
            let ns = t
                .get("substeps")?
                .as_array()?
                .iter()
                .find(|s| s.get("substep").and_then(Value::as_str) == Some(STAGE_7))?
                .get("wall_ns")?
                .as_u64()?;
            Some(ns / n)
        })
        .collect();
    json!({ "count": count, "turns": costs.len(), "median_ns_per_unit": median(costs.clone()), "worst_ns_per_unit": greatest(&costs) })
}

/// The sub-step whose block runs all of stage 7: the stream, the fixed point and the apply.
const STAGE_7: &str = "7c";

/// The measurement: the world's turns and sub-steps on the phone, its memory, the unit costs, the full-load bench
/// against its criteria and the fundamentals' targets.
pub fn measure(build: &Value, device: &Value) -> Value {
    let wall = |ts: &[&Value]| -> Vec<u64> { ts.iter().filter_map(|t| t.get("wall_ms")?.as_u64()).collect() };
    let (open, closed) = (turns(device, true), turns(device, false));
    let (open_ms, closed_ms) = (wall(&open), wall(&closed));
    let steps = substeps(&open);
    let load = device.get("load");
    let criteria = |k: &str| load.and_then(|l| u64_at(l, &["criteria", k]));
    let within = |v: Option<u64>, k: &str| v.zip(criteria(k)).map(|(v, c)| v <= c);
    let load_median = load.and_then(|l| u64_at(l, &["median_turn_ms"]));
    let load_worst = load.and_then(|l| u64_at(l, &["worst_turn_ms"]));
    let load_peak = load.and_then(|l| u64_at(l, &["vm_hwm_bytes"]));
    let targets = device.get("targets").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let verdicts = |v: &str| targets.iter().filter(|t| t.get("verdict").and_then(Value::as_str) == Some(v)).count();
    json!({
        "commit": device.get("commit"),
        "build_run": build.get("world_hash"),
        "turns": {
            "business": { "count": open_ms.len(), "median_ms": median(open_ms.clone()), "worst_ms": greatest(&open_ms) },
            "closed": { "count": closed_ms.len(), "median_ms": median(closed_ms.clone()), "worst_ms": greatest(&closed_ms) },
        },
        "substeps_business": steps.iter().map(|(l, ns)| json!({ "substep": l, "median_ns": ns })).collect::<Vec<_>>(),
        "substeps_closed": substeps(&closed).iter().map(|(l, ns)| json!({ "substep": l, "median_ns": ns })).collect::<Vec<_>>(),
        "memory": {
            "opening_vm_hwm_bytes": device.pointer("/world/opening_vm_hwm_bytes"),
            "vm_hwm_bytes": device.pointer("/world/vm_hwm_bytes"),
        },
        "unit_costs": [per_unit(&open, "rows_scanned"), per_unit(&open, "payments")],
        "load": {
            "median_turn_ms": load_median,
            "worst_turn_ms": load_worst,
            "vm_hwm_bytes": load_peak,
            "median_within": within(load_median, "median_turn_ms"),
            "worst_within": within(load_worst, "worst_turn_ms"),
            "memory_within": within(load_peak, "memory_bytes"),
            "saves": load.and_then(|l| l.get("saves")),
        },
        "targets": { "met": verdicts("met"), "missed": verdicts("missed") },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{measure, median};

    #[test]
    fn median_takes_the_lower_middle() {
        assert_eq!((median(vec![5, 1, 3]), median(vec![4, 1, 3, 2]), median(Vec::new())), (Some(3), Some(2), None));
    }

    #[test]
    fn unit_costs_are_each_turns_stage_seven_over_its_own_counts() {
        let turn = |ns, payments| {
            json!({ "business": true, "wall_ms": 10, "payments": payments, "rows_scanned": 0,
                    "substeps": [{ "substep": "7c", "wall_ns": ns }] })
        };
        let device = json!({ "world": { "turns": [turn(300, 10), turn(900, 30), turn(50, 0), turn(800, 100)] } });
        let m = measure(&json!({}), &device);
        assert_eq!(m["turns"]["business"]["count"], 4);
        assert_eq!(
            m["unit_costs"][1],
            json!({ "count": "payments", "turns": 3, "median_ns_per_unit": 30, "worst_ns_per_unit": 30 }),
            "three turns paid anything, each 30 ns a payment but the last's 8"
        );
        assert_eq!(m["unit_costs"][0]["turns"], 0, "no rows scanned, no cost per row");
    }
}
