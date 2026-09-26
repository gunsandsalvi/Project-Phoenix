//! Technology: every unit made came by a way its maker knew, from the inputs the way states.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// The production family ran and found nothing, and something was made by a way.
fn productions_by_way(w: Inspector<'_>) -> Outcome {
    if let Some(f) = w.findings().iter().find(|f| f.family == "TEC.production") {
        return Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail));
    }
    if w.goods_days().iter().all(|(_, g)| g.made == 0) {
        return Outcome::NotYet("no firm has made anything by its way yet");
    }
    Outcome::Pass
}

pub const LC_1_05: super::Check = live_check! {
    id: "LC-1-05",
    title: "every production names a way its producer knew, with the inputs it consumed as the way states",
    from_step: "S1.02",
    check: productions_by_way,
};
