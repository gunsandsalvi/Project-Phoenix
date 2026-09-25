//! The observer's checks: the player, the public events, the world's liveness and the opening's distributions against
//! the world's own.

use phx_core::{PublicEventRule, SubStep};
use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Every event the rule has judged is public exactly when the rule says, and every public event is dated no later
/// than the run's last day and names a subject. The last day's events recorded after its close's rule are judged the
/// next day, so they are not read.
fn public_events_from_the_rule(w: Inspector<'_>) -> Outcome {
    let (events, today, close) = (w.events(), w.today(), SubStep::S10a.ordinal());
    let (mut public, mut judged) = (0_u64, 0_u64);
    for id in (1..=events.len()).filter_map(|i| u64::try_from(i).ok()) {
        let e = events.get(id);
        if e.day > today || (e.day == today && e.substep > close) {
            if e.public {
                return Outcome::Fail(format!("event {id} is public before the rule judged it"));
            }
            continue;
        }
        judged += 1;
        if e.public != w.news().is_public(&e) {
            return Outcome::Fail(format!("event {id} of kind {} is public {} against the rule", e.kind, e.public));
        }
        if e.public {
            public += 1;
            if e.subjects.is_empty() {
                return Outcome::Fail(format!("public event {id} names no subject"));
            }
        }
    }
    if judged > 0 && public == 0 {
        return Outcome::Fail(format!("none of {judged} events judged became public"));
    }
    Outcome::Pass
}

pub const LC_0_57: super::Check = live_check! {
    id: "LC-0-57",
    title: "the player's queued intents are decided on the first day their point runs; with none, the rule decides",
    from_step: "S0.26",
    check: |_| Outcome::NotYet("S0.26d"),
};

pub const LC_0_58: super::Check = live_check! {
    id: "LC-0-58",
    title: "every public event was made public by the declared rule, with a date and subjects",
    from_step: "S0.26",
    check: public_events_from_the_rule,
};

pub const LC_0_59: super::Check = live_check! {
    id: "LC-0-59",
    title: "liveness: money circulates, fails are counted by cause, no quantity grows without a named cause",
    from_step: "S0.26",
    check: |_| Outcome::NotYet("S0.26e"),
};

pub const LC_0_60: super::Check = live_check! {
    id: "LC-0-60",
    title: "each opening distribution's distance from the world's own, at settling's end and the year's",
    from_step: "S0.26",
    check: |_| Outcome::NotYet("S0.26e"),
};
