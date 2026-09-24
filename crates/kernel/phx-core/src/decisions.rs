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

/// The player's intents for the turn, taken as their decision points come due.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct PlayerQueue {
    queued: Vec<QueuedIntent>,
}

impl PlayerQueue {
    pub fn push(&mut self, intent: QueuedIntent) {
        self.queued.push(intent);
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

    use super::{Decider, DecisionPointDecl, PlayerQueue, QueuedIntent, QueuedPayload, dispatch};
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
        let mut queue = PlayerQueue::default();
        queue.push(QueuedIntent { party: player, point: "HH.spend", words: vec![7] });
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
