//! The harness's own arithmetic over a hand-written design point and a driver that counts its calls.
#![cfg(test)]

use std::collections::BTreeMap;

use crate::budget::{Direction, Ratchet, misses};
use crate::compose::{self, Line, Source, line_core_ms};
use crate::design::Design;
use crate::fill::Streams;
use crate::measure::{Measure, Measures};
use crate::{Args, Bytes, DayType, Filled, FinBase, FinError, capacities_short, run};

const DESIGN: &str = r#"
[point]
persons = 6_000_000
seed = 1
scale = 0.8
[phone]
cores = 3.0
k_compute = 1.0
k_gather = 1.3
memory_mb = 4_349
turn_ms = [1_000, 2_000]
[store]
persons = 6_000_000
[day.b]
retail = 100
[day.nb]
retail = 50
[day.h]
retail = 200
[day.bc]
applies = 10
extra_core_ms = 30.0
[stage]
"1 Open" = [300.0, 60.0, 600.0]
"2 Resolve" = [600.0, 240.0, 900.0]
day = [900.0, 300.0, 1500.0]
[resolution]
zones = 1_000
cell_m = "Missing"
"#;

fn design() -> Design {
    Design::parse(DESIGN).unwrap()
}

#[test]
fn compose_day_lines() {
    assert!((line_core_ms(500.0, 1.3, 2_000_000.0) - 1_300.0).abs() < 1e-9, "500 VM ns × 1.3 × 2 M items is 1.3 s");
    let lines = compose::declared(&design());
    assert_eq!(lines.len(), 2, "the day's own total is not a line");
    let days = compose::days(&lines, &design());
    let close = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
    assert!(close(days.core_ms, [900.0, 300.0, 1500.0, 930.0]), "{:?}", days.core_ms);
}

#[test]
fn turns_are_sums_of_days() {
    let line = Line { name: "x".to_owned(), core_ms: [90.0, 30.0, 150.0], source: Source::Measured };
    let days = compose::days(&[line], &design());
    let [first, second] = days.turns_ms;
    assert!((first - (4.0 * 30.0 + 120.0) / 3.0).abs() < 1e-9, "B' is B with its extra: {first}");
    assert!((second - (3.0 * 30.0 + 150.0) / 3.0).abs() < 1e-9, "{second}");
}

#[test]
fn ns_per_op_is_cpu_over_items() {
    let one = Measure { items: 1_000, cpu_ns: 500_000, wall_ns: 500_000, ..Measure::default() };
    let four = Measure { items: 1_000, cpu_ns: 500_000, wall_ns: 125_000, ..Measure::default() };
    assert_eq!(one.ns_per_op(), Some(500));
    assert_eq!(four.ns_per_op(), one.ns_per_op(), "the workers change the wall, never the CPU an item");
    assert_eq!(Measure::default().ns_per_op(), None, "no items, no cost an item");
}

#[test]
fn a_missing_design_key_is_refused() {
    let refused = Design::parse(&DESIGN.replace("seed = 1\n", "")).unwrap_err();
    assert_eq!(refused, FinError("`point.seed` is missing from the design point".to_owned()));
    let overflow = Design::parse(&DESIGN.replace("seed = 1", "seed = -1")).unwrap_err();
    assert_eq!(overflow, FinError("`point.seed` is not a count".to_owned()));
}

#[test]
fn missing_setting_is_refused() {
    let d = design();
    assert!(d.resolution("zones").is_ok());
    assert_eq!(d.resolution("cell_m").unwrap_err(), FinError("`resolution.cell_m` is Missing".to_owned()));
    assert!(d.resolution("horizon").is_err(), "an absent setting");
}

#[derive(Default)]
struct Counting {
    days: u64,
}

impl FinBase for Counting {
    fn name(&self) -> &'static str {
        "counting"
    }
    fn fill(&mut self, _: &Design, _: &Streams) -> Result<Filled, FinError> {
        Ok(Filled { rows: 1 })
    }
    fn day(&mut self, _: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures) -> Result<(), FinError> {
        self.days += 1;
        let items = counts.values().sum();
        m.record("counting", "op", &Measure { items, cpu_ns: items * 7, ..Measure::default() });
        Ok(())
    }
    fn bytes(&self) -> Bytes {
        Bytes::default()
    }
}

#[test]
fn warm_day_is_not_counted() {
    let args = Args { design: DESIGN.to_owned(), budget: String::new(), bases: None, days: vec![DayType::B] };
    let report = run(&args, &[|| Box::new(Counting::default())]).unwrap();
    assert_eq!(
        report.ops,
        [("counting".to_owned(), "op".to_owned(), 100, Some(7))],
        "one day measured, its warm-up not"
    );
    let unknown = Args { bases: Some(vec!["stalls".to_owned()]), ..args };
    assert!(run(&unknown, &[]).is_err(), "a base with no driver is refused");
}

#[test]
fn capacities_cover_the_design_point() {
    assert!(capacities_short(&design()).is_empty());
    let bigger =
        Design::parse(&DESIGN.replace("[store]\npersons = 6_000_000", "[store]\npersons = 60_000_000")).unwrap();
    assert_eq!(capacities_short(&bigger).len(), 1, "a store short of the design point is refused");
}

#[test]
fn ratchet_directions() {
    let ratchets = [
        Ratchet { counter: "fin.x.op_ns".to_owned(), value: 100.0, direction: Direction::Down },
        Ratchet { counter: "fin.x.busy_hundredths".to_owned(), value: 150.0, direction: Direction::Up },
        Ratchet { counter: "fin.y.op_ns".to_owned(), value: 1.0, direction: Direction::Down },
    ];
    let measured = |key: &str| match key {
        "fin.x.op_ns" => Some(120.0),
        "fin.x.busy_hundredths" => Some(140.0),
        _ => None,
    };
    assert_eq!(
        misses(&ratchets, &measured),
        ["`fin.x.op_ns` is 120; its ratchet allows 100", "`fin.x.busy_hundredths` is 140; its ratchet allows 150"],
        "a base with no measure is not read"
    );
}

#[test]
fn fill_is_seeded() {
    let digest = |seed: u64| -> Vec<u64> {
        let streams = Streams::new(seed);
        (0..4).map(|row| streams.draws("stalls", row, 1).next_u64()).collect()
    };
    assert_eq!(digest(1), digest(1));
    assert_ne!(digest(1), digest(2));
    let other = Streams::new(1).draws("offers", 0, 1).next_u64();
    assert_ne!(Some(&other), digest(1).first(), "each base has its own stream");
}
