//! The seeder's arithmetic over a small hand-written design point and budget.
#![cfg(test)]

use crate::seed::{BEGIN, END, seed, within_design, write};

const DESIGN: &str = r#"
[point]
persons = 1_000
[phone]
memory_mb = 4_000
turn_ms = [1_000, 2_000]
turn_headroom = 0.1
k_compute = 1.0
[day.bc]
extra_core_ms = 20.0
[stage]
"1 Open" = [1.0, 1.0, 1.0]
day = [100.0, 40.0, 150.0]
[ledger]
1 = { name = "process", mb = 300.0 }
2 = { name = "accounts and side rows", mb = 20.0 }
total_mb = 320.0
[fin.fixed.open]
b_core_ms = 1.2
nb_core_ms = 0.8
h_core_ms = 1.5
[fin.fixed.open.shares]
"S1.178" = [1.2, 0.8, 1.5]
[unit]
spend = 215
handler = 30
[store.contracts]
"LAB.employment" = 30
[bytes]
person = 66
"#;

const BUDGET: &str =
    "# the budget\n\n[[ratchet]]\ncounter = \"fin.kept.flow_ns\"\nvalue = 1195\ndirection = \"down\"\n";

fn counters() -> Vec<(String, f64)> {
    seed(DESIGN).unwrap().into_iter().map(|s| (s.counter, s.value)).collect()
}

#[test]
fn seed_writes_every_key() {
    let got = counters();
    let at = |k: &str| got.iter().find(|(c, _)| c == k).map(|(_, v)| *v);
    assert_eq!(at("fin.day.bc_core_ms"), Some(120.0), "B′ is B and its extra");
    assert_eq!(
        (at("fin.turn.median_ms"), at("fin.turn.worst_ms")),
        (Some(900.0), Some(1800.0)),
        "10 % below the phone's"
    );
    assert_eq!(at("fin.mem.baseline_mb"), Some(300.0));
    assert_eq!(at("fin.mem.accounts_and_side_rows_mb"), Some(20.0));
    assert!((at("fin.mem.bytes_per_person").unwrap() - 320.0 * 1_048_576.0 / 1_000.0).abs() < 1e-6);
    assert_eq!(at("fin.fixed.open.nb_core_ms"), Some(0.8));
    assert_eq!((at("fin.unit.spend_ns"), at("fin.decide.handler_mind_ns")), (Some(215.0), Some(30.0)));
    assert_eq!(at("fin.contracts.lab_employment_rows"), Some(30.0));
    assert_eq!(at("fin.bytes.person"), Some(66.0));
    assert_eq!(got.len(), 4 + 2 + 3 + 2 + 3 + 2 + 2 + 1 + 1, "{got:?}");
}

#[test]
fn fin_within_design() {
    let seeded = seed(DESIGN).unwrap();
    let written = write(BUDGET, &seeded).unwrap();
    assert!(within_design(&written, &seeded).unwrap().is_empty());
    let loose = written
        .replace("counter = \"fin.day.b_core_ms\"\nvalue = 100.0", "counter = \"fin.day.b_core_ms\"\nvalue = 130.0");
    assert_eq!(within_design(&loose, &seeded).unwrap().len(), 1, "a key looser than its seed");
    let tighter = written
        .replace("counter = \"fin.day.b_core_ms\"\nvalue = 100.0", "counter = \"fin.day.b_core_ms\"\nvalue = 80.0");
    let rewritten = write(&tighter, &seeded).unwrap();
    assert!(rewritten.contains("counter = \"fin.day.b_core_ms\"\nvalue = 80.0"), "a measure below its seed stands");
    assert_eq!(rewritten.matches(BEGIN).count(), 1, "the section is written anew, never twice");
}

#[test]
fn fin_section_parses_and_every_key_is_directed() {
    let written = write(BUDGET, &seed(DESIGN).unwrap()).unwrap();
    assert!(written.starts_with("# the budget") && written.contains("fin.kept.flow_ns") && written.contains(END));
    let file: toml::Table = written.parse().unwrap();
    let ratchets = file["ratchet"].as_array().unwrap();
    assert!(
        ratchets
            .iter()
            .all(|r| r["direction"].as_str() == Some("down") && r["value"].is_float() || r["value"].is_integer())
    );
    for key in ["fin.day.h_core_ms", "fin.day.bc_core_ms", "fin.turn.median_ms", "fin.turn.worst_ms"] {
        assert!(ratchets.iter().any(|r| r["counter"].as_str() == Some(key)), "{key}");
    }
}

#[test]
fn seed_refuses_a_missing_figure() {
    let refused = seed(&DESIGN.replace("turn_headroom = 0.1\n", "")).unwrap_err();
    assert_eq!(refused.0, "the design point has no `phone.turn_headroom` to seed from");
    assert!(seed(&DESIGN.replace("h_core_ms = 1.5\n", "")).is_err(), "a fixed line without its heavy day");
}
