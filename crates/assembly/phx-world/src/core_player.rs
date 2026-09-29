//! The player on the core: a household drawn at the opening, each of its country's equally likely, whose decisions —
//! its own and its persons' — the player takes by queuing intents, and leaves to the rule on a day it queued nothing
//! only where its setup delegates. An intent is taken on the first day its decision comes for the household.

use phx_core::decisions::{Prefs, Say, Standing};
use phx_core::{QueuedIntent, Streams};
use phx_id::{Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};

use crate::core::{Core, kind_number};

/// An intent the player queued: its decision's place among the decisions, its output as words, and the day it was
/// queued on.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Intent {
    pub point: u16,
    pub words: Vec<i64>,
    pub queued: Day,
}

/// How a decision of the player's household was taken on a day: by an intent queued on a day, by the rule, or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Taken {
    Queued(Day),
    Rule,
    Kept,
}

/// A day a decision came for the player's household: the decision's place, how many times it came and how it was
/// taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct PlayerDay {
    pub day: Day,
    pub point: u16,
    pub times: u64,
    pub taken: Taken,
}

/// The player's household, whether the rule decides for it on a day it queued nothing, its intents waiting, how many
/// times each decision has come for it and had been at the last close, and each day a decision came.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct PlayerDesk {
    pub household: Option<PartyKey>,
    pub delegate: bool,
    pub queued: Vec<Intent>,
    came: Vec<phx_exec::Tally>,
    seen: Vec<u64>,
    pub days: Vec<PlayerDay>,
}

impl Core {
    /// The player's household drawn from the households of its country, each equally likely, and seated.
    ///
    /// # Errors
    /// A country with no household.
    #[clause("OBS.4", "REP.1")]
    pub(crate) fn seat_player(
        &mut self,
        (streams, today): (&Streams, Day),
        (country, delegate): (u8, bool),
        regions: &[phx_id::CountryId],
    ) -> Result<(), String> {
        let place = self.names.iter().position(|n| *n == crate::consts::PLAYER_KIND).ok_or("no household kind")?;
        let Some(Missing::Present(sited)) = self.household_decl.as_ref().map(|d| d.sited_by) else {
            return Err("households sited by no region".to_owned());
        };
        let store = self.kinds.get(place).ok_or("no household kind")?;
        let of_country: Vec<Slot> = store
            .parties
            .live_slots()
            .filter(|s| {
                let region = store.record(*s).get(sited).and_then(|w| match w.get() {
                    Missing::Present(r) => usize::try_from(r).ok(),
                    Missing::Absent => None,
                });
                region.and_then(|r| regions.get(r)).is_some_and(|c| c.get() == country)
            })
            .collect();
        let n = phx_rand::float::len_u64(of_country.len());
        if n == 0 {
            return Err(format!("no household in the player's country {country}"));
        }
        let mut draws =
            streams.open(&crate::opening::prims::PLAYER_STREAM, Subject::new(SubjectTag::World, 0), today, 0);
        let at = phx_rand::float::index(phx_rand::uniform::below_u64(&mut draws, n));
        let slot = of_country.get(at).copied().ok_or("a draw beyond the households")?;
        let came = self.decisions.len();
        self.player = crate::core_player::PlayerDesk {
            household: Some(PartyKey::new(kind_number(place), slot)),
            delegate,
            came: (0..came).map(|_| phx_exec::Tally::default()).collect(),
            seen: vec![0; came],
            ..crate::core_player::PlayerDesk::default()
        };
        Ok(())
    }

    /// The player's intents for the next turn, each for its own household and a decision the world takes.
    #[clause("OBS.4")]
    pub(crate) fn queue(&mut self, intents: &[QueuedIntent], today: Day) {
        let household = self.player.household.and_then(|k| self.kinds.get(usize::from(k.kind()))?.parties.id(k.slot()));
        for intent in intents {
            if Some(intent.party) != household {
                violation!(
                    clause = "OBS.4",
                    "an intent queued for a party not the player's",
                    party = intent.party.get()
                );
            }
            let Some(point) = self.decisions.position(intent.point).and_then(|p| u16::try_from(p).ok()) else {
                violation!(clause = "MND.20", "an intent for a decision the world does not take");
            };
            self.player.queued.push(Intent { point, words: intent.words.clone(), queued: today });
        }
    }

    /// What the decider says for a decision at `at` a party takes, where the party is the player's household: its
    /// queued intent, the rule where it delegates, or nothing; none for any other party.
    #[clause("OBS.4", "MND.20")]
    pub(crate) fn say(&self, at: usize, party: PartyKey, prefs: impl FnOnce() -> Prefs) -> Option<Say> {
        if self.player.household != Some(party) {
            return None;
        }
        self.decisions.count(at, Standing::Player);
        if let Some(c) = self.player.came.get(at) {
            c.add(1);
        }
        Some(match self.player.queued.iter().find(|i| usize::from(i.point) == at) {
            Some(i) => Say::Queued(i.words.clone()),
            None if self.player.delegate => Say::Rule(prefs()),
            None => Say::Kept,
        })
    }

    /// At a close, each decision that came for the player's household today recorded with how it was taken, and the
    /// intent it took gone from the queue.
    #[clause("OBS.4")]
    pub(crate) fn player_day(&mut self, day: Day) {
        let p = &mut self.player;
        for (at, (c, seen)) in p.came.iter().zip(p.seen.iter_mut()).enumerate() {
            let now = c.get();
            let (Some(times), Ok(point)) = (now.checked_sub(*seen).filter(|t| *t > 0), u16::try_from(at)) else {
                continue;
            };
            *seen = now;
            let taken = match p.queued.iter().position(|i| i.point == point) {
                Some(i) => Taken::Queued(p.queued.remove(i).queued),
                None if p.delegate => Taken::Rule,
                None => Taken::Kept,
            };
            p.days.push(PlayerDay { day, point, times, taken });
        }
    }
}
