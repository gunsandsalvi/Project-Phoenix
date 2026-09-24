use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Every save the run took read back, from its files alone, to the world hash of the close it was taken at.
fn saves_check(w: Inspector<'_>) -> Outcome {
    if w.saves().is_empty() {
        return Outcome::Fail("the run took no save".to_owned());
    }
    match w.saves().iter().find_map(|s| s.mismatch.as_ref().map(|why| (s.day, why))) {
        Some((day, why)) => Outcome::Fail(format!("the save of day {}: {why}", day.get())),
        None => Outcome::Pass,
    }
}

/// Every save's stores measured: their sizes compressed and before, and the time it took to write and to check.
fn saves_measured(w: Inspector<'_>) -> Outcome {
    if w.saves().is_empty() {
        return Outcome::Fail("the run took no save".to_owned());
    }
    let unmeasured = w.saves().iter().find(|s| s.stores.is_empty() || s.write_ns.is_none() || s.check_ns.is_none());
    match unmeasured {
        Some(s) => Outcome::Fail(format!("the save of day {} was not measured", s.day.get())),
        None => Outcome::Pass,
    }
}

pub const LC_0_35: Check = live_check! {
    id: "LC-0-35",
    title: "Every save reads back to the world hash of its close",
    from_step: "S0.20",
    check: saves_check,
};

pub const LC_0_36: Check = live_check! {
    id: "LC-0-36",
    title: "Save sizes and write times are recorded",
    from_step: "S0.20",
    check: saves_measured,
};
