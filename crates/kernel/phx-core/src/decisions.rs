use phx_id::PartyRef;
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::declare_prim;
use crate::kinds::LegalForm;
use crate::schedule::WakeKind;

declare_prim! {
    /// The concerns an option is weighed on, and for each decision the world takes who takes it — an office, a
    /// household or a person — the concerns it touches and whether it takes the best option or the first good enough.
    pub DECISIONS = "MND.decisions" {
        kind: Shape, value: Decisions, clause: "MND.19", scope: Shared,
        shape: standing("what people care about and where each decision is taken: no mechanism in the world derives the concerns or an institution's division of its decisions, so both are the accepted stand-in from the literature on preferences and on corporate governance")
    }
}

/// Who takes a decision: the holder of an office its institution's form declares, a household's adults as one, or a
/// person for itself.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum TakenIn {
    Office(String),
    Household,
    Person,
}

/// Whether a decision takes the best of its options or the first that is good enough.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Best,
    Satisfice,
}

/// A kind of decision as the register declares it: its name, its taker, the concerns it touches by their place in the
/// concern list, and its mode.
#[clause("MND.20", "MND.2", "MND.7")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionKind {
    pub name: String,
    pub taken_in: TakenIn,
    pub concerns: Vec<u8>,
    pub mode: Mode,
}

/// The concern list and every decision kind the world takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionKinds {
    pub concerns: Vec<String>,
    pub kinds: Vec<DecisionKind>,
}

impl DecisionKinds {
    /// Where a decision sits among the kinds, by its name.
    #[must_use]
    pub fn position(&self, name: &str) -> Option<usize> {
        self.kinds.iter().position(|k| k.name == name)
    }

    /// The kinds held to the decision points the systems declare, both ways, and each office a decision is taken in
    /// to the legal forms that declare it.
    ///
    /// # Errors
    /// Every point the register does not list, every listed decision no system declares, and every office no form
    /// declares.
    #[clause("MND.20", "PTY.16")]
    pub fn check(&self, points: &[&str], forms: &[LegalForm]) -> Result<(), Vec<String>> {
        let mut refused = Vec::new();
        for p in points {
            if self.position(p).is_none() {
                refused.push(format!("decision `{p}` is declared by a system but not in `MND.decisions`"));
            }
        }
        for k in &self.kinds {
            if !points.contains(&k.name.as_str()) {
                refused.push(format!("decision `{}` is in `MND.decisions` but no system declares it", k.name));
            }
            if let TakenIn::Office(office) = &k.taken_in
                && !forms.iter().any(|f| f.office(office).is_some())
            {
                refused
                    .push(format!("decision `{}` is taken in the office `{office}`, which no legal form has", k.name));
            }
        }
        if refused.is_empty() { Ok(()) } else { Err(refused) }
    }
}

/// What a decider brings to a decision's rule before minds: its memory type, switching type and stance on the
/// heuristics' menu, the return it requires a year, its management type and its age class, each absent where it holds
/// none.
#[clause("MND.20", "MND.16", "VAL.6", "VAL.7")]
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Prefs {
    pub memory: Missing<u16>,
    pub switching: Missing<u16>,
    pub stance: Missing<u16>,
    pub required_return: Missing<f64>,
    pub management: Missing<u16>,
    /// The age class whose lived years weight its outlooks of public series; none for an institution no person holds.
    pub window: Missing<u16>,
}

impl Prefs {
    /// A decider holding no preference a rule reads.
    pub const NONE: Prefs = Prefs {
        memory: Missing::Absent,
        switching: Missing::Absent,
        stance: Missing::Absent,
        required_return: Missing::Absent,
        management: Missing::Absent,
        window: Missing::Absent,
    };
}

/// Who a decision was taken by: the player, an office's holder, an institution's founding preferences for an office no
/// one holds, a household or a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Player,
    Holder,
    Founding,
    Household,
    Person,
}

impl Standing {
    pub const ALL: [Standing; 5] =
        [Standing::Player, Standing::Holder, Standing::Founding, Standing::Household, Standing::Person];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Standing::Player => "player",
            Standing::Holder => "holder",
            Standing::Founding => "founding",
            Standing::Household => "household",
            Standing::Person => "person",
        }
    }
}

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

/// An intent the player queued, read back as a decision's output.
pub trait QueuedPayload: Sized {
    fn decode(words: &[i64]) -> Option<Self>;
}

/// A yes or no, queued as one word: nothing is no, anything else yes.
impl QueuedPayload for bool {
    fn decode(words: &[i64]) -> Option<bool> {
        match words {
            [w] => Some(*w != 0),
            _ => None,
        }
    }
}

/// A number, queued as one word.
impl QueuedPayload for i64 {
    fn decode(words: &[i64]) -> Option<i64> {
        match words {
            [w] => Some(*w),
            _ => None,
        }
    }
}

/// A choice among places, queued as a word each.
impl QueuedPayload for Vec<u32> {
    fn decode(words: &[i64]) -> Option<Vec<u32>> {
        words.iter().map(|w| u32::try_from(*w).ok()).collect()
    }
}

/// A sum of money, queued as its amount in its currency's smallest unit.
impl QueuedPayload for f64 {
    fn decode(words: &[i64]) -> Option<f64> {
        match words {
            [w] => Some(phx_rand::float::from_i64(*w)),
            _ => None,
        }
    }
}

/// A choice by its place, queued as one word.
impl QueuedPayload for usize {
    fn decode(words: &[i64]) -> Option<usize> {
        match words {
            [w] => usize::try_from(*w).ok(),
            _ => None,
        }
    }
}

/// What a decision's decider says before its rule is called: the rule decides, by these preferences; the player
/// queued the decision's output; or the player keeps the decision and queued nothing, and it is not taken that day.
#[clause("OBS.4", "MND.20")]
#[derive(Clone, Debug, PartialEq)]
pub enum Say {
    Rule(Prefs),
    Queued(Vec<i64>),
    Kept,
}

impl Say {
    /// The decision taken as its decider says: the queued output, the rule's on the input built from the decider's
    /// preferences, or none.
    pub fn take<I, O: QueuedPayload>(
        self,
        point: &DecisionPointDecl<I, O>,
        input: impl FnOnce(&Prefs) -> I,
    ) -> Option<O> {
        match self {
            Say::Rule(prefs) => Some((point.rule)(&input(&prefs))),
            Say::Queued(words) => match O::decode(&words) {
                Some(out) => Some(out),
                None => violation!(clause = "OBS.4", "a queued intent that is not its decision's output"),
            },
            Say::Kept => None,
        }
    }
}

/// A player's intent for one decision point, as queued for the next turn.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct QueuedIntent {
    pub party: PartyRef,
    pub point: &'static str,
    pub words: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::{DecisionKind, DecisionKinds, DecisionPointDecl, Mode, Prefs, QueuedPayload, Say, TakenIn};
    use crate::kinds::{Feature, LegalForm};
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
    fn plain_choices_decode_from_their_words() {
        assert_eq!(bool::decode(&[0]), Some(false));
        assert_eq!(bool::decode(&[3]), Some(true));
        assert_eq!(bool::decode(&[1, 1]), None, "a yes or no is one word");
        assert_eq!(Vec::<u32>::decode(&[2, 0]), Some(vec![2, 0]));
        assert_eq!(Vec::<u32>::decode(&[-1]), None, "no place is negative");
        assert_eq!(i64::decode(&[-4]), Some(-4));
        assert_eq!(i64::decode(&[]), None, "a number is one word");
    }

    #[test]
    fn say_rule_decides() {
        assert_eq!(Say::Rule(Prefs::NONE).take(&SPEND, |_| [10, 0]), Some(Spend(5)), "the rule on its input");
        assert_eq!(f64::decode(&[250]), Some(250.0));
        assert_eq!(usize::decode(&[-1]), None, "no place is negative");
    }

    #[test]
    fn say_queued_is_output() {
        assert_eq!(Say::Queued(vec![7]).take(&SPEND, |_| [10, 0]), Some(Spend(7)), "the player's queued output");
    }

    #[test]
    fn say_kept_takes_nothing() {
        assert_eq!(Say::Kept.take(&SPEND, |_| [10, 0]), None, "kept and not queued: not taken");
    }

    #[test]
    fn undecodable_intent_stops() {
        assert!(std::panic::catch_unwind(|| Say::Queued(Vec::new()).take(&SPEND, |_| [10, 0])).is_err());
    }

    #[test]
    fn decision_point_needs_schedule_or_wakes() {
        assert!(SPEND.validate().is_ok());
        let unwoken = DecisionPointDecl { schedule: Missing::Absent, ..SPEND };
        assert!(unwoken.validate().is_err());
        assert!(DecisionPointDecl { wakes: &[WakeKind::Message], ..unwoken }.validate().is_ok());
    }

    #[test]
    fn decisions_held_to_points() {
        let kind = |name: &str, taken_in: TakenIn| DecisionKind {
            name: name.to_owned(),
            taken_in,
            concerns: vec![0],
            mode: Mode::Best,
        };
        let kinds = DecisionKinds {
            concerns: vec!["means".to_owned()],
            kinds: vec![
                kind("FRM.close", TakenIn::Office("chief_executive".to_owned())),
                kind("HH.spend", TakenIn::Household),
            ],
        };
        let company = LegalForm {
            name: "company".to_owned(),
            may_hold: Vec::new(),
            features: vec![Feature::SeparateParty],
            endings: vec!["dissolution".to_owned()],
            owners: crate::kinds::Owners::Shareholders,
            offices: vec!["chief_executive".to_owned()],
        };
        assert!(kinds.check(&["FRM.close", "HH.spend"], std::slice::from_ref(&company)).is_ok());
        let refused = kinds.check(&["FRM.close", "HH.spend", "LAB.post"], std::slice::from_ref(&company)).unwrap_err();
        assert_eq!(refused.len(), 1, "a declared point the register does not list");
        assert_eq!(
            kinds.check(&["FRM.close"], std::slice::from_ref(&company)).unwrap_err().len(),
            1,
            "a listed decision undeclared"
        );
        let officeless = LegalForm { offices: Vec::new(), ..company };
        assert_eq!(
            kinds.check(&["FRM.close", "HH.spend"], &[officeless]).unwrap_err().len(),
            1,
            "an office no form has"
        );
    }
}
