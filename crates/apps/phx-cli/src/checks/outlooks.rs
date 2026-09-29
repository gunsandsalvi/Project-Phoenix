//! Outlooks on the core: every method's own outlook of each public series, never one shared, formed before it is
//! read; and the firms' stances, whose shares move as the heuristics' records do.

use phx_num::Missing;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Each series' outlooks of its next print by every method that has formed one.
fn outlooks(w: Inspector<'_>) -> Vec<Vec<f64>> {
    w.core()
        .goods
        .outlooks
        .series
        .values()
        .map(|s| {
            s.methods
                .iter()
                .flat_map(|v| v.outlook.iter())
                .filter_map(|o| match o {
                    Missing::Present(x) => Some(*x),
                    Missing::Absent => None,
                })
                .collect::<Vec<f64>>()
        })
        .filter(|o| o.len() > 1)
        .collect()
}

/// Some series' methods disagree: their outlooks are not all one.
fn disagree(w: Inspector<'_>) -> Outcome {
    let series = outlooks(w);
    if series.is_empty() {
        return Outcome::NotYet("no public series has two methods' outlooks in the run");
    }
    if series.iter().any(|o| o.iter().any(|x| Some(x) != o.first())) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("every method's outlook agrees on each of {} series", series.len()))
    }
}

/// Every series' outlooks were formed on or before today.
fn formed_before_use(w: Inspector<'_>) -> Outcome {
    let series = &w.core().goods.outlooks.series;
    if series.is_empty() {
        return Outcome::NotYet("no public series printed in the run");
    }
    match series.iter().find(|(_, s)| s.day > w.today()) {
        Some(((p, r), s)) => {
            Outcome::Fail(format!("product {p} at region {r}: its outlooks formed on day {}, after today", s.day.get()))
        }
        None => Outcome::Pass,
    }
}

/// The firms' shares by heuristic are kept each day and move over the run as their stances are reconsidered.
fn shares_move(w: Inspector<'_>) -> Outcome {
    let days = &w.core().goods.outlooks.days;
    let (Some(first), Some(last)) = (days.first(), days.last()) else {
        return Outcome::NotYet("no day's stances were counted in the run");
    };
    if first.by_heuristic.iter().sum::<u64>() == 0 {
        return Outcome::NotYet("no firm held a stance in the run");
    }
    if days.iter().any(|d| d.by_heuristic != first.by_heuristic) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!(
            "the firms' heuristics stood at {:?} on every day from day {} to {}",
            first.by_heuristic, first.day, last.day
        ))
    }
}

pub const LC_1_01: Check = live_check! {
    id: "LC-1-01",
    title: "outlooks disagree: their dispersion per variable is reported, positive where methods or histories differ, and wider after a large surprise",
    from_step: "S1.01",
    check: disagree,
};

pub const LC_1_02: Check = live_check! {
    id: "LC-1-02",
    title: "no outlook is formed after the stage that uses it",
    from_step: "S1.01",
    check: formed_before_use,
};

pub const LC_1_03: Check = live_check! {
    id: "LC-1-03",
    title: "heuristic shares per series are reported and move over the run, their lead over price swings published",
    from_step: "S1.01",
    check: shares_move,
};
