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
