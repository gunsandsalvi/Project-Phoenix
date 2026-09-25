//! Outlooks and values: every party reads its own view, formed from what it observed, never a common one.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Outlooks are formed and read once households and firms decide from them and the flow of income closes.
const NO_OUTLOOKS: &str = "outlooks are formed once households decide from them (S1.12)";

fn not_yet(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_OUTLOOKS)
}

pub const LC_1_01: super::Check = live_check! {
    id: "LC-1-01",
    title: "outlooks disagree: their dispersion per variable is reported, positive where methods or histories differ, and wider after a large surprise",
    from_step: "S1.01",
    check: not_yet,
};

pub const LC_1_02: super::Check = live_check! {
    id: "LC-1-02",
    title: "no outlook is formed after the stage that uses it",
    from_step: "S1.01",
    check: not_yet,
};

pub const LC_1_03: super::Check = live_check! {
    id: "LC-1-03",
    title: "heuristic shares per series are reported and move over the run, their lead over price swings published",
    from_step: "S1.01",
    check: not_yet,
};

pub const LC_1_04: super::Check = live_check! {
    id: "LC-1-04",
    title: "no variable is read by every party as one expectation, and things two parties with different histories value have more than one value",
    from_step: "S1.01",
    check: not_yet,
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
