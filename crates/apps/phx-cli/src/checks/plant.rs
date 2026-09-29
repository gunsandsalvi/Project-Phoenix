//! Plant's checks: its stock kept by what moves it and wearing, output within its capacity, and investment a purchase
//! from a named producer entering service when built.

use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// The goods family, which holds every capital unit held to what entered service, wore, was retired or destroyed,
/// found nothing, and plant wore in the run.
fn plant_kept(w: Inspector<'_>) -> Outcome {
    if let Some(f) = w.findings().iter().find(|f| f.family == "goods") {
        return Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail));
    }
    let days = &w.core().plant.days;
    if days.iter().all(|d| d.worn + d.retired == 0) {
        return Outcome::NotYet("no plant wore in the run");
    }
    Outcome::Pass
}

/// No making went beyond what its maker's plant allowed.
fn within_capacity(w: Inspector<'_>) -> Outcome {
    let days = &w.core().plant.days;
    if let Some(d) = days.iter().find(|d| d.beyond > 0) {
        return Outcome::Fail(format!("day {}: {} makings beyond their plant", d.day, d.beyond));
    }
    if days.is_empty() { Outcome::NotYet("the run closed no goods day") } else { Outcome::Pass }
}

/// Every project was bought from a producer other than its buyer, and projects entered service in the run.
fn investment_bought(w: Inspector<'_>) -> Outcome {
    let plant = &w.core().plant;
    if let Some(p) = plant.projects.iter().find(|p| p.producer == p.holder) {
        return Outcome::Fail(format!("a project of kind {} bought from its own holder", p.kind));
    }
    if plant.days.iter().all(|d| d.completed == 0) {
        return Outcome::NotYet("no project entered service in the run");
    }
    Outcome::Pass
}

pub const LC_1_10: Check = live_check! {
    id: "LC-1-10",
    title: "per owner and kind, plant next day is plant today plus completions less retirements plus transfers: \
            the family of the plant's stock (CAP.8) is clean and plant wears",
    from_step: "S1.04",
    check: plant_kept,
};

pub const LC_1_11: Check = live_check! {
    id: "LC-1-11",
    title: "no output exceeds the capacity of the plant that made it (CAP.9)",
    from_step: "S1.04",
    check: within_capacity,
};

pub const LC_1_12: Check = live_check! {
    id: "LC-1-12",
    title: "every investment is a purchase from a named producer, a commitment until delivery; investment's share, \
            volatility and responses and the plant's age are reported (CAP.10)",
    from_step: "S1.04",
    check: investment_bought,
};
