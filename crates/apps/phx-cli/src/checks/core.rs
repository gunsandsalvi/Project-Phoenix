use phx_world::Inspector;

use super::{Check, Outcome};
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
