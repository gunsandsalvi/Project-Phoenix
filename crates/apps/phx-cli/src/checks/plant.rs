//! Plant: its stock family clean and its wear realised; its capacity and its purchases, once firms produce and invest.

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

fn no_output(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("firms produce from their plant from S1.15")
}

pub const LC_1_11: super::Check = live_check! {
    id: "LC-1-11",
    title: "no output exceeds the capacity of the plant that made it (CAP.9)",
    from_step: "S1.04",
    check: no_output,
};

fn no_investment(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("firms invest from S1.15")
}

pub const LC_1_12: super::Check = live_check! {
    id: "LC-1-12",
    title: "every investment is a purchase from a named producer, a commitment until delivery; investment's share, \
            volatility and responses and the plant's age are reported (CAP.10)",
    from_step: "S1.04",
    check: no_investment,
};
