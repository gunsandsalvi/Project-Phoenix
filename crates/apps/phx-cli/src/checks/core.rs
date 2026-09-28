use phx_world::Inspector;

use phx_num::Missing;

use super::{Check, Observed, Outcome};
use crate::live_check;

/// Business days a month holds at least; a run shorter than a month may pay no monthly due.
const MONTH_DAYS: usize = 31;

/// Every flow the core's day made was settled, failed or committed, and over a month the core paid dues.
fn core_settles_its_flows(w: Inspector<'_>) -> Outcome {
    let days = &w.core().days;
    for d in days {
        if d.flows != d.settled + d.failed + d.committed {
            return Outcome::Fail(format!(
                "day {}: {} flows made, {} settled, {} failed, {} committed",
                d.day.get(),
                d.flows,
                d.settled,
                d.failed,
                d.committed
            ));
        }
    }
    if days.len() < MONTH_DAYS {
        return Outcome::NotYet("the run ended before a month of the core's days");
    }
    if days.iter().all(|d| d.flows == 0) {
        return Outcome::Fail(format!("the core made no flow in {} days", days.len()));
    }
    Outcome::Pass
}

pub const LC_0_62: Check = live_check! {
    id: "LC-0-62",
    title: "Every flow the core's day makes is settled, failed or committed, and the core pays its dues",
    from_step: "S1.23",
    check: core_settles_its_flows,
};

/// The core's households hold the persons they opened with, plus every person born, less every person gone, and
/// chance reached its persons over the run.
fn core_persons_live(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let (born, gone): (u64, u64) = core.pop_days.iter().fold((0, 0), |(b, g), (_, d)| (b + d.born, g + d.gone));
    let expected = core.persons_opened + born - gone;
    if core.persons_held() != expected {
        return Outcome::Fail(format!(
            "the core holds {} persons; it opened with {}, {born} were born and {gone} are gone",
            core.persons_held(),
            core.persons_opened
        ));
    }
    if core.pop_days.len() < MONTH_DAYS {
        return Outcome::NotYet("the run ended before a month of the core's days");
    }
    if gone == 0 {
        return Outcome::Fail(format!("no person on the core died in {} days", core.pop_days.len()));
    }
    Outcome::Pass
}

pub const LC_0_63: Check = live_check! {
    id: "LC-0-63",
    title: "The core's households hold their opening persons plus those born less those gone, and chance reaches them",
    from_step: "S1.23",
    check: core_persons_live,
};

/// Days an estate may wait for its country's next business day: a weekend and a holiday on either side.
const ESTATE_WAIT_DAYS: u32 = 5;

/// Every estate the core opened settles on its country's next business day, paying what it holds, and ends.
fn core_estates_settle(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let today = w.today();
    if let Some((e, c, opened)) = core.estates.iter().find(|(_, _, d)| d.get() + ESTATE_WAIT_DAYS < today.get()) {
        return Outcome::Fail(format!(
            "an estate of country {} opened on day {} is still waiting on day {} (slot {})",
            c.get(),
            opened.get(),
            today.get(),
            e.slot().get()
        ));
    }
    if core.days.iter().all(|d| d.estates == 0) {
        return Outcome::NotYet("no estate on the core has settled yet");
    }
    Outcome::Pass
}

pub const LC_0_64: Check = live_check! {
    id: "LC-0-64",
    title: "Every estate on the core settles on its country's next business day and ends",
    from_step: "S1.23",
    check: core_estates_settle,
};

/// The core's money family found no break on any day.
fn core_money_holds(w: Inspector<'_>) -> Outcome {
    match w.core().days.iter().find(|d| d.breaks > 0) {
        Some(d) => Outcome::Fail(format!("day {}: {} breaks of the core's money", d.day.get(), d.breaks)),
        None if w.core().days.is_empty() => Outcome::NotYet("the core ran no day"),
        None => Outcome::Pass,
    }
}

pub const LC_0_65: Check = live_check! {
    id: "LC-0-65",
    title: "On the core each bank owes what its customers hold, and settlement makes and loses no money",
    from_step: "S1.23",
    check: core_money_holds,
};

/// Money moves on every business day of some country; no read rises on every day of the run unless its declaration
/// names why; and the reads never all stand still to the end.
fn alive(w: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for d in w.core().days.iter().filter(|d| w.any_business(d.day)) {
        if d.settled == 0 || d.gross == 0 {
            return Outcome::Fail(format!("no money moved on business day {}", d.day.get()));
        }
    }
    for (decl, series) in o.reads.iter().zip(o.series) {
        if decl.grows.is_none() && phx_obs::rises_throughout(&series.values) {
            return Outcome::Fail(format!("read `{}` rose on every day, for no named cause", decl.id));
        }
    }
    if let Missing::Present(day) = phx_obs::still_from(o.series) {
        return Outcome::Fail(format!("every read stood still from day {} to the end", day.get()));
    }
    Outcome::Pass
}

/// Each opening distribution has its distance from the world's own, and the share of its members that moved within
/// it, read at settling's end and at the run's end.
fn drift_read(_: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for (when, drifts) in [("settling's end", o.settled), ("the run's end", o.ended)] {
        if drifts.is_empty() {
            return Outcome::Fail(format!("no opening distribution was read at {when}"));
        }
        if let Some(d) = drifts.iter().find(|d| d.distance == Missing::Absent) {
            return Outcome::Fail(format!("`{}` has no distance at {when}: a histogram empty or rebinned", d.id));
        }
        if let Some(d) = drifts.iter().find(|d| d.moved == Missing::Absent) {
            return Outcome::Fail(format!("`{}` has no member sampled in both views at {when}", d.id));
        }
    }
    Outcome::Pass
}

pub const LC_0_59: Check = live_check! {
    id: "LC-0-59",
    title: "liveness: money circulates, fails are counted by cause, no quantity grows without a named cause",
    from_step: "S0.26",
    observed: alive,
};

pub const LC_0_60: Check = live_check! {
    id: "LC-0-60",
    title: "each opening distribution's distance from the world's own and its members' moves, at settling's end and the year's",
    from_step: "S0.26",
    observed: drift_read,
};
