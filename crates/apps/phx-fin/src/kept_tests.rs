//! The kept drivers over a scaled fixture of the design point: a few thousand rows of each store, a day's counts in
//! hundreds, the pool of a given size.
#![cfg(test)]

use phx_exec::PoolSpec;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{BASE, Kept, RETIRERS};
use crate::measure::Measures;
use crate::{DayType, FinBase};

const SCALED: &str = r"
[point]
persons = 10_000
seed = 7
scale = 1.0
[phone]
cores = 3.0
k_compute = 1.0
k_gather = 1.3
memory_mb = 100
turn_ms = [1_000, 2_000]
[store]
accounts = 3_000
banks = 4
wheel_rows = 5_000
wheel_far = 200
wheel_days = 16
stalls = 600
products = 6
zones = 40
vacancies = 400
regions = 3
[day.b]
flows = 900
dues = 300
retail = 800
searches = 150
[day.nb]
retail = 800
[day.h]
flows = 1_500
dues = 700
retail = 800
searches = 150
[day.bc]
extra_core_ms = 1.0
[stage]
day = [1.0, 1.0, 1.0]
";

/// A clock that never moves: the tests read items, not time.
struct Still;

impl phx_exec::Clock for Still {
    fn now_ns(&self) -> u64 {
        0
    }
}

fn filled(workers: usize) -> (Kept, u64) {
    let design = Design::parse(SCALED).unwrap();
    let mut kept = Kept::on(&PoolSpec::unpinned(workers)).unwrap();
    let rows = kept.fill(&design, &Streams::new(design.point.seed)).unwrap().rows;
    (kept, rows)
}

#[test]
fn kept_fill_counts_match_design() {
    let (_, rows) = filled(2);
    assert_eq!(rows, 3_000 + 4 + 5_000 + 200 + 600 + 400, "every store the fill names, at the design point's count");
}

#[test]
fn kept_fill_is_worker_independent() {
    let (one, _) = filled(1);
    let (four, _) = filled(4);
    assert_eq!(one.digest(), four.digest());
}

#[test]
fn kept_heavy_day_runs() {
    let design = Design::parse(SCALED).unwrap();
    let (mut kept, _) = filled(2);
    let clock = Still;
    let mut m = Measures::new(&clock);
    kept.day(DayType::H, design.day(DayType::H).unwrap(), &mut m).unwrap();
    let ops: Vec<(&str, u64)> = m.iter().map(|((_, op), x)| (op.as_str(), x.items)).collect();
    for (op, items) in [("flow_h", 1_500), ("due", 700), ("purchase", 800), ("search", 150), ("civil", 700)] {
        assert!(ops.contains(&(op, items)), "{op} measured over {items} items: {ops:?}");
    }
    let mut closed = Measures::new(&clock);
    kept.day(DayType::Nb, design.day(DayType::Nb).unwrap(), &mut closed).unwrap();
    assert!(
        closed.iter().all(|((_, op), _)| op == "purchase" || op == "draw"),
        "a closed day settles and takes nothing"
    );
}

#[test]
fn empty_zone_is_a_failed_want() {
    let design = Design::parse(&SCALED.replace("stalls = 600", "stalls = 6")).unwrap();
    let mut kept = Kept::on(&PoolSpec::unpinned(2)).unwrap();
    kept.fill(&design, &Streams::new(design.point.seed)).unwrap();
    let clock = Still;
    kept.day(DayType::Nb, design.day(DayType::Nb).unwrap(), &mut Measures::new(&clock)).unwrap();
    let (unserved, sales) = kept.meeting();
    assert!(unserved > 0, "a buyer whose zone has no stall goes without");
    assert!(sales > 0, "a buyer whose zone has one buys");
}

#[test]
fn kept_keys_named_for_their_retirers() {
    let budget: toml::Table = include_str!("../../../../perf/budget.toml").parse().unwrap();
    let keys: Vec<String> = budget["ratchet"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["counter"].as_str()?.strip_prefix(&format!("fin.{BASE}.")).map(str::to_owned))
        .collect();
    assert!(!keys.is_empty());
    for key in &keys {
        assert!(RETIRERS.iter().any(|(k, step)| k == key && step.starts_with("S1.")), "`{key}` names no retirer");
    }
    for (k, _) in RETIRERS {
        assert!(keys.iter().any(|x| x == k), "`{k}` has no ratchet");
    }
}
