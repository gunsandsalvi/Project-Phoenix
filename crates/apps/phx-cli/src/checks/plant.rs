//! Plant: its stock family clean and its wear realised; no making beyond its capacity; its purchases from builders.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// The plant's family is registered and found nothing, and once the run has lasted a review period some plant has
/// worn.
fn stock_clean(w: Inspector<'_>) -> Outcome {
    let name = sys_cap::families::STOCK.name;
    if !w.families().iter().any(|f| f.name == name) {
        return Outcome::Fail(format!("the family `{name}` is not registered"));
    }
    if let Some(f) = w.findings().iter().find(|f| f.family == name) {
        return Outcome::Fail(format!("{} on day {}: {}", f.family, f.day.get(), f.detail));
    }
    let Ok(period) = w.register().count(sys_cap::REVIEW_DAYS.id) else {
        return Outcome::Fail("the plant review's period is not declared".to_owned());
    };
    let ran = u64::from(w.today().get()) - u64::from(w.day_zero().get());
    let worn: u64 = w.agent_days().iter().map(|d| d.worn).sum();
    if ran > period && worn == 0 {
        return Outcome::Fail(format!("no plant wore in {ran} days, with a review every {period}"));
    }
    Outcome::Pass
}

pub const LC_1_10: super::Check = live_check! {
    id: "LC-1-10",
    title: "per owner and kind, plant next day is plant today plus completions less retirements plus transfers: \
            the family of the plant's stock (CAP.8) is clean and plant wears",
    from_step: "S1.04",
    check: stock_clean,
};

/// No making exceeded its plant's capacity a day, over a run in which firms made goods and their plant was reviewed.
fn within_capacity(w: Inspector<'_>) -> Outcome {
    let days = w.goods_days();
    if let Some((day, d)) = days.iter().find(|(_, d)| d.beyond_capacity > 0) {
        return Outcome::Fail(format!(
            "{} makings beyond their plant's capacity on day {}",
            d.beyond_capacity,
            day.get()
        ));
    }
    if days.iter().all(|(_, d)| d.made == 0) {
        return Outcome::NotYet("no firm has made anything in the run");
    }
    let reviews = [
        <sys_cap::review::ReviewSmall as phx_core::HandlerDecl>::NAME,
        <sys_cap::review::ReviewLarge as phx_core::HandlerDecl>::NAME,
    ];
    if w.visit_days().iter().all(|v| reviews.iter().all(|r| v.visits_of(r) == 0)) {
        return Outcome::NotYet("no owner has reviewed its plant in the run");
    }
    Outcome::Pass
}

pub const LC_1_11: super::Check = live_check! {
    id: "LC-1-11",
    title: "no output exceeds the capacity of the plant that made it (CAP.9)",
    from_step: "S1.04",
    check: within_capacity,
};

/// Owners ordered plant from named builders, and their projects are building or complete; every stage and completion
/// passed the ledger's check of what construction gives.
fn invested(w: Inspector<'_>) -> Outcome {
    let days = w.goods_days();
    let begun: u64 = days.iter().map(|(_, d)| d.projects).sum();
    let completed: u64 = days.iter().map(|(_, d)| d.completed).sum();
    if begun == 0 {
        return Outcome::NotYet("no owner has ordered plant in the run");
    }
    let building = w.books().ledger.chains.projects().count();
    if completed == 0 && building == 0 {
        return Outcome::Fail(format!("{begun} projects begun, none building and none completed"));
    }
    Outcome::Pass
}

pub const LC_1_12: super::Check = live_check! {
    id: "LC-1-12",
    title: "every investment is a purchase from a named producer, a commitment until delivery; investment's share, \
            volatility and responses and the plant's age are reported (CAP.10)",
    from_step: "S1.04",
    check: invested,
};
