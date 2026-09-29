//! The dataset's identities over hand-built tables of two activities, one final use and two sectors: a balanced pair
//! passes, and each break is refused by name.
#![cfg(test)]

use super::{Flows, Stocks, flows_breaks, solve, stocks_breaks};

const UNIT: f64 = 1e-9;

/// Two activities each using a fifth of the other's output a unit; households take what is left, GDP one.
fn balanced() -> Flows {
    // Output x solves x = A x + f with A = [[0, 0.2], [0.2, 0]] and f = [0.5, 0.5]: x = 0.625 each.
    Flows {
        output: vec![0.625, 0.625],
        inputs: vec![vec![0.0, 0.2], vec![0.2, 0.0]],
        taxes: vec![0.0, 0.0, 0.0],
        added: vec![vec![0.3, 0.2], vec![0.3, 0.2]],
        finals: vec![vec![0.5], vec![0.5]],
    }
}

#[test]
fn balanced_flows_pass() {
    assert_eq!(flows_breaks(&balanced(), UNIT), Vec::<String>::new());
}

#[test]
fn register_refuses_unbalanced_dataset() {
    let short = Flows { output: vec![0.625, 0.6], ..balanced() };
    let breaks = flows_breaks(&short, UNIT);
    assert!(breaks.iter().any(|b| b.starts_with("GEN.final_composition: activity 1")), "{breaks:?}");
    let untaxed = Flows { added: vec![vec![0.3, 0.2], vec![0.3, 0.1]], ..balanced() };
    assert!(flows_breaks(&untaxed, UNIT).iter().any(|b| b.starts_with("GEN.value_added_parts: activity 1")));
    let stocks = Stocks {
        financial: vec![vec![0.4, -0.4, 0.0, 0.0, 0.0], vec![0.5, 0.0, -0.4, 0.0, 0.0]],
        real: vec![vec![0.0, 0.4, 0.0, 0.0, 0.0]],
    };
    let breaks = stocks_breaks(&stocks, UNIT);
    assert!(breaks.iter().any(|b| b.starts_with("the sheet: instrument 1")), "{breaks:?}");
    assert!(breaks.iter().any(|b| b.contains("sector 2")), "banks worth what their unmatched deposits leave");
}

#[test]
fn output_is_what_the_uses_need() {
    // (I − A) x = f with the balanced pair's A and f gives its output, 0.625 each.
    let m = vec![vec![1.0, -0.2], vec![-0.2, 1.0]];
    let x = solve(&m, &[0.5, 0.5]).unwrap_or_default();
    assert!(x.iter().all(|v| (v - 0.625).abs() < 1e-12), "{x:?}");
    assert_eq!(solve(&[vec![0.0, 1.0], vec![1.0, 0.0]], &[1.0, 1.0]), None, "a nought pivot solves nothing");
}
