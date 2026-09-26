//! The central bank: money's invariants hold with the standing facilities in use, and their quantities are read.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// The money families of the audit stay clean over days the facilities were used, and each day's quantities are
/// read.
fn facilities_clean(w: Inspector<'_>) -> Outcome {
    let days = w.central_days();
    if days.iter().all(|(_, d)| d.uses == 0) {
        return Outcome::NotYet("the facilities are used once a bank's reserves stray from its target");
    }
    match w.findings().iter().find(|f| f.family == "MON.money") {
        Some(f) => Outcome::Fail(format!("with the facilities in use, {} on day {}", f.detail, f.day.get())),
        None => Outcome::Pass,
    }
}

pub const LC_1_27: super::Check = live_check! {
    id: "LC-1-27",
    title: "MON.7 and MON.9 clean with the central bank's facilities in use; facility quantities are reported daily",
    from_step: "S1.10",
    check: facilities_clean,
};
