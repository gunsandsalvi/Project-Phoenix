//! The observer's checks: the player and the public events.

use phx_core::PublicEventRule as _;
use phx_num::Missing;
use phx_world::Inspector;
use phx_world::core_player::Taken;

use super::Outcome;
use crate::live_check;

/// The player's household is seated, live, in the country the setup named; every intent queued was taken on the first
/// day its decision came for the household after it was queued, and none stays queued past such a day; on a day a
/// decision came with nothing queued, the rule took it where the player delegates, and no one where the player keeps
/// its decisions.
fn player_decides(w: Inspector<'_>) -> Outcome {
    let p = w.player();
    let Some(key) = p.household else { return Outcome::Fail("no player was seated at the opening".to_owned()) };
    let core = w.core();
    let Some(store) = core.kinds.get(usize::from(key.kind())) else {
        return Outcome::Fail("the player's household is of no kind".to_owned());
    };
    if store.parties.at(key.slot()).is_none() {
        return Outcome::Fail(format!("the player's household {} is not live", key.word()));
    }
    let region = core.declared.household.as_ref().and_then(|d| match d.sited_by {
        Missing::Present(at) => store.record(key.slot()).get(at).map(|v| v.get()),
        Missing::Absent => None,
    });
    let country = match region {
        Some(Missing::Present(r)) => usize::try_from(r).ok().and_then(|r| w.regions().get(r)).map(|c| c.get()),
        _ => None,
    };
    let chosen = w.game().setup.player.country;
    if country.map(|c| u64::from(c) + 1) != Some(chosen) {
        return Outcome::Fail(format!("the player lives in country {country:?}, not the setup's {chosen}"));
    }
    for i in &p.queued {
        if let Some(d) = p.days.iter().find(|d| d.point == i.point && d.day > i.queued) {
            return Outcome::Fail(format!(
                "an intent for decision {} queued on day {} stayed queued past day {}, when it came",
                i.point,
                i.queued.get(),
                d.day.get()
            ));
        }
    }
    for d in &p.days {
        let wrong = match d.taken {
            Taken::Queued(q) => q >= d.day || p.days.iter().any(|e| e.point == d.point && e.day > q && e.day < d.day),
            Taken::Rule => !p.delegate,
            Taken::Kept => p.delegate,
        };
        if wrong {
            return Outcome::Fail(format!("decision {} on day {} taken as {:?}", d.point, d.day.get(), d.taken));
        }
    }
    if p.days.is_empty() {
        return Outcome::NotYet("no decision came for the player's household");
    }
    Outcome::Pass
}

pub const LC_0_57: super::Check = live_check! {
    id: "LC-0-57",
    title: "the player's queued intents are decided on the first day their point runs; with none, the rule decides",
    from_step: "S0.26",
    check: player_decides,
};

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
