//! The observer's checks: the public events.

use phx_core::PublicEventRule as _;
use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Every event judged at a close is public exactly when the declared rule says, and every public event names a
/// subject. Events of the kinds the rule makes public — the weather's and the catastrophes' — come with their events
/// on the core.
fn public_events_from_the_rule(w: Inspector<'_>) -> Outcome {
    let events = w.events();
    let (mut public, mut judged) = (0_u64, 0_u64);
    for id in (1..=events.len()).filter_map(|i| u64::try_from(i).ok()) {
        let e = events.get(id);
        if e.day > w.today() {
            return Outcome::Fail(format!("event {id} is dated after the run's last day"));
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
    match (judged, public) {
        (0, _) => Outcome::NotYet("the run recorded no event"),
        (_, 0) => {
            Outcome::NotYet("no event of a kind the rule makes public yet: the weather's and catastrophes' (S1.24 d)")
        }
        _ => Outcome::Pass,
    }
}

pub const LC_0_58: super::Check = live_check! {
    id: "LC-0-58",
    title: "every public event was made public by the declared rule, with a date and subjects",
    from_step: "S0.26",
    check: public_events_from_the_rule,
};
