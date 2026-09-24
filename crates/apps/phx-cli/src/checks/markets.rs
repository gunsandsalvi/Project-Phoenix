use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Why a market check has nothing to read on a world where no market meets.
const NO_MEETING: &str = "no market meets before the firms post their prices (S1.03)";

/// Whether any market met over the run: a print, a failure or a day's measures.
fn met(w: Inspector<'_>) -> bool {
    let m = w.markets();
    !m.tape.prints().is_empty() || !m.tape.failures().is_empty() || !m.days.is_empty()
}

/// No finding of the Prices family over the run.
fn prices_clean(w: Inspector<'_>) -> Outcome {
    if !met(w) {
        return Outcome::NotYet(NO_MEETING);
    }
    match w.findings().iter().find(|f| f.family == phx_market::audit::PRICES.name) {
        Some(f) => Outcome::Fail(format!("{} on day {}: {}", f.family, f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

/// Every print has its match set, of its market and day, trading what it says; every failure is of a meeting the
/// day's measures record.
fn traced(w: Inspector<'_>) -> Outcome {
    if !met(w) {
        return Outcome::NotYet(NO_MEETING);
    }
    let m = w.markets();
    for p in m.tape.prints() {
        let Some(set) = m.tape.set_of(p.matches()) else {
            return Outcome::Fail(format!("a print of market {} has no match set", p.market().get()));
        };
        let traded: i64 = set.matches.iter().map(|x| x.qty).sum();
        if set.market != p.market() || set.day != p.day() || traded != p.quantity() {
            return Outcome::Fail(format!(
                "a print of market {} on day {} does not trace to its matches",
                p.market().get(),
                p.day().get()
            ));
        }
    }
    for f in m.tape.failures() {
        if !m.days.iter().any(|d| d.market == f.market && d.day == f.day && d.failed > 0) {
            return Outcome::Fail(format!(
                "a failure of market {} on day {} with no meeting",
                f.market.get(),
                f.day.get()
            ));
        }
    }
    Outcome::Pass
}

/// Every market that printed or failed on a day has that day's measures.
fn measured(w: Inspector<'_>) -> Outcome {
    if !met(w) {
        return Outcome::NotYet(NO_MEETING);
    }
    let m = w.markets();
    let met_on = m
        .tape
        .prints()
        .iter()
        .map(|p| (p.market(), p.day()))
        .chain(m.tape.failures().iter().map(|f| (f.market, f.day)));
    for (market, day) in met_on {
        if !m.days.iter().any(|d| d.market == market && d.day == day) {
            return Outcome::Fail(format!("market {} met on day {} with no measures", market.get(), day.get()));
        }
    }
    Outcome::Pass
}

pub const LC_0_30: Check = live_check! {
    id: "LC-0-30",
    title: "The Prices family is clean every close",
    from_step: "S0.18",
    check: prices_clean,
};

pub const LC_0_31: Check = live_check! {
    id: "LC-0-31",
    title: "Every print traces to its match set, and every published failure to its meeting",
    from_step: "S0.18",
    check: traced,
};

pub const LC_0_32: Check = live_check! {
    id: "LC-0-32",
    title: "The markets' measures are published per market per day, for the markets that met",
    from_step: "S0.18",
    check: measured,
};
