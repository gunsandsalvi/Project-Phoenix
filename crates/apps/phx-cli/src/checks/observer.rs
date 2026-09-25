//! The observer's checks: the player, the public events, the world's liveness and the opening's distributions against
//! the world's own.

use std::collections::BTreeMap;

use phx_core::{PublicEventRule, Resolved, SubStep};
use phx_ledger::check::FailCause;
use phx_num::Missing;
use phx_world::Inspector;

use super::{Observed, Outcome};
use crate::live_check;

/// The player's household is seated, lives as an agent of one of the household kind in the country the setup named,
/// kept its identity, and left no intent queued past the points it was queued for: with no decision point yet, none is
/// queued, and the rule decides nothing for it.
fn player_seated(w: Inspector<'_>) -> Outcome {
    let Missing::Present(player) = w.player_queue().player() else {
        return Outcome::Fail("no player was seated at the opening".to_owned());
    };
    if let Some(point) = w.player_queue().queued().next() {
        return Outcome::Fail(format!("an intent for `{point}` stayed queued past the turn"));
    }
    let Resolved::Live(party, row) = w.books().parties.directory().resolve(player.party) else {
        return Outcome::Fail(format!("the player's party {} is not live", player.party.get()));
    };
    if party != player.party {
        return Outcome::Fail(format!("the player's party {} resolves to {}", player.party.get(), party.get()));
    }
    let kinds = &w.population().kinds;
    let Some((k, kd)) = kinds.iter().enumerate().find(|(_, k)| k.decl.kind == phx_world::consts::PLAYER_KIND) else {
        return Outcome::Fail("the world keeps no household kind".to_owned());
    };
    let table = w.agent_table(k);
    if table.id() != row.table {
        return Outcome::Fail("the player's party is not a household".to_owned());
    }
    if table.multiplicity(row.slot).get() != 1 {
        return Outcome::Fail("the player's household stands for more than one".to_owned());
    }
    let Missing::Present(sited) = kd.decl.sited_by else {
        return Outcome::Fail("households are sited by no region".to_owned());
    };
    let region = table.attr(row.slot, sited);
    let country = usize::try_from(region).ok().and_then(|r| w.geo().map.regions.get(r)).map(|r| r.country.get());
    let chosen = w.game().setup.player.country;
    if country.map(|c| u64::from(c) + 1) != Some(chosen) {
        return Outcome::Fail(format!("the player lives in country {country:?}, not the setup's {chosen}"));
    }
    Outcome::Pass
}

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
    check: player_seated,
};

pub const LC_0_58: super::Check = live_check! {
    id: "LC-0-58",
    title: "every public event was made public by the declared rule, with a date and subjects",
    from_step: "S0.26",
    check: public_events_from_the_rule,
};

/// Money moves on every business day; each day's fails tallied by their causes number the fails recorded, and match
/// them cause by cause while the day still holds them; no read
/// rises on every day of the run unless its declaration names why; and the reads never all stand still to the end.
fn alive(w: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for s in w.settlements().iter().filter(|s| w.any_business(s.day)) {
        if s.dues.settled == 0 || s.dues.gross == 0 {
            return Outcome::Fail(format!("no money moved on business day {}", s.day.get()));
        }
        if s.measure.fails.values().sum::<u64>() != s.recorded {
            return Outcome::Fail(format!("day {}'s fails by cause do not number the fails recorded", s.day.get()));
        }
        let mut tally: BTreeMap<FailCause, u64> = BTreeMap::new();
        for f in &s.fails {
            *tally.entry(f.cause).or_insert(0) += 1;
        }
        if s.unrecorded == Missing::Absent && tally != s.measure.fails {
            return Outcome::Fail(format!("day {}'s fails by cause are not the fails recorded", s.day.get()));
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

/// Each opening distribution has its distance from the world's own read at settling's end and at the run's end.
fn drift_read(_: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for (when, drifts) in [("settling's end", o.settled), ("the run's end", o.ended)] {
        if drifts.is_empty() {
            return Outcome::Fail(format!("no opening distribution was read at {when}"));
        }
        if let Some(d) = drifts.iter().find(|d| d.distance == Missing::Absent) {
            return Outcome::Fail(format!("`{}` has no distance at {when}: a histogram empty or rebinned", d.id));
        }
    }
    Outcome::Pass
}

pub const LC_0_59: super::Check = live_check! {
    id: "LC-0-59",
    title: "liveness: money circulates, fails are counted by cause, no quantity grows without a named cause",
    from_step: "S0.26",
    observed: alive,
};

pub const LC_0_60: super::Check = live_check! {
    id: "LC-0-60",
    title: "each opening distribution's distance from the world's own, at settling's end and the year's",
    from_step: "S0.26",
    observed: drift_read,
};
