use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::schedule::WakeKind;

/// A decision a party takes: its system, its rule — one pure function, which is also its evaluation form — its
/// schedule or the wakes that bring it on, and whether it runs on days no market opens.
#[clause("OBS.4")]
#[derive(Debug)]
pub struct DecisionPointDecl<I, O> {
    pub name: &'static str,
    pub system: &'static str,
    pub rule: fn(&I) -> O,
    pub schedule: Missing<&'static str>,
    pub wakes: &'static [WakeKind],
    pub runs_on_non_business: bool,
    pub clause: &'static str,
}

impl<I, O> DecisionPointDecl<I, O> {
    /// # Errors
    /// When the point has neither a schedule nor a wake to bring it on.
    pub fn validate(&self) -> Result<(), String> {
        if matches!(self.schedule, Missing::Absent) && self.wakes.is_empty() {
            return Err(format!("decision point `{}` has no schedule and no wake", self.name));
        }
        Ok(())
    }
}

/// Who decides for a party: its kind's rule, or the player, who may leave to the rule what they have not queued.
#[clause("OBS.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decider {
    Rule,
    Player { delegate_when_unqueued: bool },
}

/// An intent the player queued, read back as a decision's output.
pub trait QueuedPayload: Sized {
    fn decode(words: &[i64]) -> Option<Self>;
}

/// A player's intent for one decision point, as queued for the next turn.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct QueuedIntent {
    pub party: PartyId,
    pub point: &'static str,
    pub words: Vec<i64>,
}

/// The player's party, an agent of one from the start, and whether the rule decides for it on a day it queued nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Player {
    pub party: PartyId,
    pub delegate: bool,
}

/// The player, once seated at the opening, and its intents for the turn, taken as their decision points come due.
#[clause("OBS.4", "REP.1")]
#[derive(Debug, phx_macros::Saved)]
pub struct PlayerQueue {
    player: Missing<Player>,
    queued: Vec<QueuedIntent>,
}

impl PlayerQueue {
    /// A queue before the opening has seated the player.
    #[must_use]
    pub fn unseated() -> PlayerQueue {
        PlayerQueue { player: Missing::Absent, queued: Vec::new() }
    }

    /// Seats the player's party, once.
    pub fn seat(&mut self, player: Player) {
        if let Missing::Present(p) = self.player {
            violation!(clause = "OBS.4", "a second player seated", party = p.party.get());
        }
        self.player = Missing::Present(player);
    }

    pub fn player(&self) -> Missing<Player> {
        self.player
    }

    /// Who decides for a party: the player for its own party, the rule for every other.
    #[must_use]
    pub fn decider(&self, party: PartyId) -> Decider {
        match self.player {
            Missing::Present(p) if p.party == party => Decider::Player { delegate_when_unqueued: p.delegate },
            _ => Decider::Rule,
        }
    }

    /// Queues an intent; the player acts only as its own party.
    pub fn push(&mut self, intent: QueuedIntent) {
        if !matches!(self.player, Missing::Present(p) if p.party == intent.party) {
            violation!(clause = "OBS.4", "an intent queued for a party not the player's", party = intent.party.get());
        }
        self.queued.push(intent);
    }

    /// The intents still queued, by their points.
    pub fn queued(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.queued.iter().map(|q| q.point)
    }

    /// The intent queued for a party's decision point, removed from the queue.
    pub fn take(&mut self, party: PartyId, point: &str) -> Option<Vec<i64>> {
        let at = self.queued.iter().position(|q| q.party == party && q.point == point)?;
        Some(self.queued.remove(at).words)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }
}

/// A decision taken: the player's queued intent when there is one; otherwise the rule, unless the player keeps the
/// decision and queued nothing, when it is not taken that day.
#[clause("OBS.4")]
pub fn dispatch<I, O: QueuedPayload>(
    point: &DecisionPointDecl<I, O>,
    decider: Decider,
    queued: Option<&[i64]>,
    input: &I,
) -> Option<O> {
    if let Some(words) = queued {
        let Some(out) = O::decode(words) else {
            violation!(clause = "OBS.4", "a queued intent that is not its decision's output");
        };
        return Some(out);
    }
    match decider {
        Decider::Rule | Decider::Player { delegate_when_unqueued: true } => Some((point.rule)(input)),
        Decider::Player { delegate_when_unqueued: false } => None,
    }
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::Missing;

    use super::{Decider, DecisionPointDecl, Player, PlayerQueue, QueuedIntent, QueuedPayload, dispatch};
    use crate::schedule::WakeKind;

    #[derive(Debug, PartialEq)]
    struct Spend(i64);

    impl QueuedPayload for Spend {
        fn decode(words: &[i64]) -> Option<Spend> {
            words.first().map(|w| Spend(*w))
        }
    }

    fn half(household: &[i64; 2]) -> Spend {
        Spend(household[0] / 2)
    }

    const SPEND: DecisionPointDecl<[i64; 2], Spend> = DecisionPointDecl {
        name: "HH.spend",
        system: "HH",
        rule: half,
        schedule: Missing::Present("HH.monthly"),
        wakes: &[],
        runs_on_non_business: true,
        clause: "HH.1",
    };

    #[test]
    fn decision_point_needs_schedule_or_wakes() {
        assert!(SPEND.validate().is_ok());
        let unwoken = DecisionPointDecl { schedule: Missing::Absent, ..SPEND };
        assert!(unwoken.validate().is_err());
        assert!(DecisionPointDecl { wakes: &[WakeKind::Message], ..unwoken }.validate().is_ok());
    }

    #[test]
    fn decider_dispatch() {
        let player = PartyId::new(1);
        let mut queue = PlayerQueue::unseated();
        queue.seat(Player { party: player, delegate: false });
        assert_eq!(queue.decider(player), Decider::Player { delegate_when_unqueued: false });
        assert_eq!(queue.decider(PartyId::new(2)), Decider::Rule, "every other party's rule decides");
        queue.push(QueuedIntent { party: player, point: "HH.spend", words: vec![7] });
        let other = QueuedIntent { party: PartyId::new(2), point: "HH.spend", words: vec![7] };
        let mut refused = PlayerQueue::unseated();
        refused.seat(Player { party: player, delegate: true });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| refused.push(other))).is_err(),
            "the player acts only as its own party"
        );
        assert_eq!(dispatch(&SPEND, Decider::Rule, None, &[100, 0]), Some(Spend(50)), "the rule");
        let queued = queue.take(player, "HH.spend");
        let keeps = Decider::Player { delegate_when_unqueued: false };
        assert_eq!(dispatch(&SPEND, keeps, queued.as_deref(), &[100, 0]), Some(Spend(7)), "the queued intent");
        assert_eq!(
            dispatch(&SPEND, Decider::Player { delegate_when_unqueued: true }, None, &[100, 0]),
            Some(Spend(50))
        );
        assert_eq!(dispatch(&SPEND, keeps, None, &[100, 0]), None, "no decision that day");
        assert!(queue.is_empty());
    }
}
