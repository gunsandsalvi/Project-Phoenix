//! Outlooks and values: every party reads its own view, formed from what it observed, never a common one.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// What the outlooks' reads over time need: a record of each heuristic's share and of each decision's first change
/// after a surprise, which the run does not yet keep.
const NO_RECORD: &str = "the run keeps no record of heuristic shares or of decisions after surprises (F-120)";

fn not_yet(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_RECORD)
}

/// Each public series' outlooks by method, where it has more than one.
fn outlooks_by_method(w: Inspector<'_>) -> Vec<Vec<i64>> {
    w.markets().public.values().filter(|s| s.outlooks.len() > 1).map(|s| s.outlooks.clone()).collect()
}

/// Some series' methods disagree: their outlooks are not all one.
fn disagree(w: Inspector<'_>) -> Outcome {
    let series = outlooks_by_method(w);
    if series.is_empty() {
        return Outcome::NotYet("no public series has two methods' outlooks in the run");
    }
    if series.iter().any(|o| o.iter().any(|x| Some(x) != o.first())) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("every method's outlook agrees on each of {} series", series.len()))
    }
}

/// Every public series' outlook was formed on or before today, at 5a, before the decisions that read it.
fn formed_before_use(w: Inspector<'_>) -> Outcome {
    let public = &w.markets().public;
    if public.is_empty() {
        return Outcome::NotYet("no public series was formed in the run");
    }
    match public.iter().find(|(_, s)| s.day > w.today()) {
        Some((m, s)) => {
            Outcome::Fail(format!("market {}'s outlook formed on day {}, after today", m.get(), s.day.get()))
        }
        None => Outcome::Pass,
    }
}

pub const LC_1_01: super::Check = live_check! {
    id: "LC-1-01",
    title: "outlooks disagree: their dispersion per variable is reported, positive where methods or histories differ, and wider after a large surprise",
    from_step: "S1.01",
    check: disagree,
};

pub const LC_1_02: super::Check = live_check! {
    id: "LC-1-02",
    title: "no outlook is formed after the stage that uses it",
    from_step: "S1.01",
    check: formed_before_use,
};

/// The firms' shares by heuristic are kept each day and move over the run as their stances are reconsidered.
fn shares_move(w: Inspector<'_>) -> Outcome {
    let days = w.stance_days();
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
            first.by_heuristic,
            first.day.get(),
            last.day.get()
        ))
    }
}

pub const LC_1_03: super::Check = live_check! {
    id: "LC-1-03",
    title: "heuristic shares per series are reported and move over the run, their lead over price swings published",
    from_step: "S1.01",
    check: shares_move,
};

pub const LC_1_04: super::Check = live_check! {
    id: "LC-1-04",
    title: "no variable is read by every party as one expectation, and things two parties with different histories value have more than one value",
    from_step: "S1.01",
    check: disagree,
};

pub const LC_1_43: super::Check = live_check! {
    id: "LC-1-43",
    title: "each method's lag behind each turning point of a published series, by memory type and heuristic mix",
    from_step: "S1.01",
    check: not_yet,
};

pub const LC_1_44: super::Check = live_check! {
    id: "LC-1-44",
    title: "after each large surprise, the days until each stance's first changed decision, ranked by its surprise",
    from_step: "S1.01",
    check: not_yet,
};
