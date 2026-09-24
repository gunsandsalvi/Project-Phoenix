use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// The world keeps no cell until the households and small firms are opened.
const NO_CELLS: &str = "the world keeps no cell before the households and small firms are opened (S0.25)";

/// In every cell, each profile group counts every member of its role, the weight.
fn profiles_sum_to_weights(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// Every cell's landing key is the one its key, positions and kinks give now.
fn landing_keys_current(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

pub const LC_0_37: Check = live_check! {
    id: "LC-0-37",
    title: "In every cell, the profile counts sum to the weight in each role, every close",
    from_step: "S0.21",
    check: profiles_sum_to_weights,
};

pub const LC_0_38: Check = live_check! {
    id: "LC-0-38",
    title: "Every cell's landing key equals the one recomputed from its key, positions and kinks",
    from_step: "S0.21",
    check: landing_keys_current,
};

/// Every hazard's realised hit rate over the run within its sampling error of its declared rate.
fn realised_rates(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// No cell visited on a day it had no agenda entry.
fn only_the_active(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// Every hazard occurrence's event recorded at the sub-step that drew it.
fn hazards_recorded(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// Carried needs and notices decided on the first day their decision point ran.
fn carried_decided(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

pub const LC_0_39: Check = live_check! {
    id: "LC-0-39",
    title: "Every hazard's realised hit rate is within its sampling error of its declared rate",
    from_step: "S0.22",
    check: realised_rates,
};

pub const LC_0_40: Check = live_check! {
    id: "LC-0-40",
    title: "No cell is visited on a day it had no agenda entry",
    from_step: "S0.22",
    check: only_the_active,
};

pub const LC_0_41: Check = live_check! {
    id: "LC-0-41",
    title: "Every hazard occurrence has its event recorded at the sub-step that drew it",
    from_step: "S0.22",
    check: hazards_recorded,
};

pub const LC_0_42: Check = live_check! {
    id: "LC-0-42",
    title: "Carried needs and notices are decided on the first day their decision point runs",
    from_step: "S0.22",
    check: carried_decided,
};

/// The Representation family's close: weights sum to each population, attachments within members, every close.
fn representation_whole(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// Sampled landings re-checked against both sides' kinks from their records.
fn no_kink_crossed(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// Sampled landings' straight rules' totals unchanged at the moment of landing, to a smallest unit per member.
fn straight_rules_exact(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

/// The representation's costs reported each day: dispersion erased per landing and per pooled flow, and splits,
/// landings and new cells.
fn costs_reported(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_CELLS)
}

pub const LC_0_43: Check = live_check! {
    id: "LC-0-43",
    title: "Weights sum to each population, lines' sides are equal and attachments reconcile with profiles, every close",
    from_step: "S0.23",
    check: representation_whole,
};

pub const LC_0_44: Check = live_check! {
    id: "LC-0-44",
    title: "No landing crossed a kink: sampled landings re-checked against both sides' kinks",
    from_step: "S0.23",
    check: no_kink_crossed,
};

pub const LC_0_45: Check = live_check! {
    id: "LC-0-45",
    title: "Sampled landings leave every straight rule's total unchanged to a smallest unit per member",
    from_step: "S0.23",
    check: straight_rules_exact,
};

pub const LC_0_46: Check = live_check! {
    id: "LC-0-46",
    title: "The representation's costs are reported per day: dispersion erased, splits, landings and new cells",
    from_step: "S0.23",
    check: costs_reported,
};
