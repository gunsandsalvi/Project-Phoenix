//! The decision core. Every decision the world takes is taken here, each time it is: the core names its decider — an
//! office's holder, or, for an office no one holds, its institution's preferences drawn at its founding; a household;
//! a person — hands the decision's input that decider's preferences and nothing else, calls the decision's rule and
//! counts the decision by its decider.

use std::collections::BTreeMap;

use phx_core::decisions::{DecisionKinds, DecisionPointDecl, Prefs, QueuedPayload, Say, Standing, TakenIn};
use phx_core::kinds::LegalForm;
use phx_id::{PartyKey, Slot};
use phx_macros::clause;
use phx_num::{MaybeI64, Missing, violation};

use crate::core::Core;

/// A decision bound to its kind once for a pass: its place among the kinds and its point.
#[derive(Debug)]
pub(crate) struct Bound<I: 'static, O: 'static> {
    at: usize,
    point: &'static DecisionPointDecl<I, O>,
}

impl<I, O> Clone for Bound<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for Bound<I, O> {}

/// The decisions the world takes and who takes them: each decision's name and taker, the office it is taken in by
/// each party kind whose form declares it, each kind's founding preferences by slot, the offices' holders with their
/// own preferences, and the decisions taken, by kind and by the decider's standing.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Decisions {
    names: Vec<String>,
    taken_in: Vec<TakenIn>,
    office: Vec<Vec<Option<u8>>>,
    founding: Vec<Vec<Prefs>>,
    holders: BTreeMap<(PartyKey, u8), (u64, Prefs)>,
    taken: Vec<Vec<phx_exec::Tally>>,
    pub(crate) household: Option<(usize, [usize; 4])>,
}

impl Decisions {
    /// Who takes a decision in an office for a party: its holder, else its institution's founding preferences; none
    /// where the party's form does not declare the office.
    fn in_office(&self, at: usize, party: PartyKey) -> Option<(Standing, Prefs)> {
        let office = (*self.office.get(at)?.get(usize::from(party.kind()))?)?;
        if let Some((_, prefs)) = self.holders.get(&(party, office)) {
            return Some((Standing::Holder, *prefs));
        }
        Some((Standing::Founding, self.founding_of(party)))
    }

    /// Each decision's name and how many times it was taken by each standing of decider.
    #[must_use]
    pub fn taken(&self) -> Vec<(&str, [u64; Standing::ALL.len()])> {
        self.names
            .iter()
            .zip(&self.taken)
            .map(|(n, t)| (n.as_str(), std::array::from_fn(|i| t.get(i).map_or(0, phx_exec::Tally::get))))
            .collect()
    }

    /// How many decisions the world takes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// A decision's place, by its name.
    #[must_use]
    pub fn position(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// A person put in every office a decision is taken in by its party's form, bringing its own preferences.
    #[clause("PTY.16")]
    pub(crate) fn appoint(&mut self, party: PartyKey, person: u64, prefs: Prefs) {
        let kind = usize::from(party.kind());
        let mut offices: Vec<u8> = self.office.iter().filter_map(|o| o.get(kind).copied().flatten()).collect();
        offices.sort_unstable();
        offices.dedup();
        for office in offices {
            self.holders.insert((party, office), (person, prefs));
        }
    }

    /// The offices a person holds at a party left empty, or every one of its offices where no person is named.
    #[clause("PTY.16")]
    pub(crate) fn vacate(&mut self, party: PartyKey, person: Option<u64>) {
        let held: Vec<(PartyKey, u8)> = self
            .holders
            .range((party, 0)..=(party, u8::MAX))
            .filter(|(_, (holder, _))| person.is_none_or(|p| p == *holder))
            .map(|(k, _)| *k)
            .collect();
        for k in held {
            self.holders.remove(&k);
        }
    }

    /// The age class of a party's office holder, where a person holds its offices.
    pub(crate) fn set_window(&mut self, party: PartyKey, person: u64, window: Missing<u16>) {
        for (_, (holder, prefs)) in self.holders.range_mut((party, 0)..=(party, u8::MAX)) {
            if *holder == person {
                prefs.window = window;
            }
        }
    }

    /// The preferences a party's institution was founded with.
    pub(crate) fn founding_of(&self, party: PartyKey) -> Prefs {
        self.founding
            .get(usize::from(party.kind()))
            .and_then(|f| f.get(usize::try_from(party.slot().get()).unwrap_or(usize::MAX)))
            .copied()
            .unwrap_or(Prefs::NONE)
    }

    /// How many offices a person holds.
    #[must_use]
    pub fn held(&self) -> usize {
        self.holders.len()
    }

    /// A decision counted as taken by a standing of decider.
    pub(crate) fn count(&self, at: usize, standing: Standing) {
        if let Some(c) = self.taken.get(at).and_then(|t| t.get(standing_at(standing))) {
            c.add(1);
        }
    }
}

/// The place of a standing among the counts.
fn standing_at(standing: Standing) -> usize {
    Standing::ALL.iter().position(|s| *s == standing).unwrap_or(0)
}

/// A record word read as a type's index.
fn index_of(w: Option<&MaybeI64>) -> Missing<u16> {
    match w.map(|w| w.get()) {
        Some(Missing::Present(v)) => u16::try_from(v).map_or(Missing::Absent, Missing::Present),
        _ => Missing::Absent,
    }
}

impl Core {
    /// The decisions opened: each kind as the register declares it, its office found among the forms of the core's
    /// kinds, each kind's founding preferences empty until its parties are drawn, and no office held.
    #[clause("MND.20", "PTY.16")]
    pub fn open_decisions(&mut self, kinds: &DecisionKinds, forms: &[Option<&LegalForm>]) {
        let office = kinds
            .kinds
            .iter()
            .map(|k| {
                forms
                    .iter()
                    .map(|f| match (&k.taken_in, f) {
                        (TakenIn::Office(o), Some(f)) => f.office(o).and_then(|i| u8::try_from(i).ok()),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        let household = self.names.iter().position(|n| *n == "household").and_then(|place| {
            let decl = self.household_decl.as_ref()?;
            let at = |name: &str| decl.attrs.iter().position(|a| a.item.name == name);
            Some((
                place,
                [
                    at(sys_hh::MEMORY_ATTR.name)?,
                    at(sys_hh::SWITCHING_ATTR.name)?,
                    at(sys_hh::STANCE_ATTR.name)?,
                    at(sys_hh::WINDOW_ATTR.name)?,
                ],
            ))
        });
        self.decisions = Decisions {
            names: kinds.kinds.iter().map(|k| k.name.clone()).collect(),
            taken_in: kinds.kinds.iter().map(|k| k.taken_in.clone()).collect(),
            office,
            founding: vec![Vec::new(); self.names.len()],
            holders: BTreeMap::new(),
            taken: kinds
                .kinds
                .iter()
                .map(|_| Standing::ALL.iter().map(|_| phx_exec::Tally::default()).collect())
                .collect(),
            household,
        };
    }

    /// A party's preferences drawn at its founding, read by each of its offices no one holds.
    #[clause("MND.16")]
    pub(crate) fn found(&mut self, party: PartyKey, prefs: Prefs) {
        let Some(by_slot) = self.decisions.founding.get_mut(usize::from(party.kind())) else {
            violation!(
                clause = "MND.16",
                "founding preferences for a kind the core does not keep",
                kind = party.kind()
            );
        };
        let at = usize::try_from(party.slot().get()).unwrap_or(usize::MAX);
        if by_slot.len() <= at {
            by_slot.resize(at + 1, Prefs::NONE);
        }
        if let Some(p) = by_slot.get_mut(at) {
            *p = prefs;
        }
    }

    /// A decision point bound to its kind for a pass; one the register does not declare stops the run.
    pub(crate) fn bind<I, O>(&self, point: &'static DecisionPointDecl<I, O>) -> Bound<I, O> {
        let Some(at) = self.decisions.names.iter().position(|n| n == point.name) else {
            violation!(clause = "MND.20", "a decision taken that the register does not declare");
        };
        Bound { at, point }
    }

    /// A household's preferences: its record's memory and switching types, its stance and its age class.
    fn household_prefs(&self, slot: Slot) -> Prefs {
        let Some((place, [m, s, h, w])) = self.decisions.household else { return Prefs::NONE };
        let Some(store) = self.kinds.get(place) else { return Prefs::NONE };
        let record = store.record(slot);
        Prefs {
            memory: index_of(record.get(m)),
            switching: index_of(record.get(s)),
            stance: index_of(record.get(h)),
            window: index_of(record.get(w)),
            ..Prefs::NONE
        }
    }

    /// Who takes a decision for a party and the preferences it brings: the office's holder, else its institution's
    /// founding preferences; a household's own; a person's, its household's until each person holds its own. A party
    /// whose form lacks the decision's office stops the run.
    #[clause("MND.20", "MND.16")]
    pub(crate) fn decider<I, O>(&self, b: Bound<I, O>, party: PartyKey) -> (Standing, Prefs) {
        match self.decisions.taken_in.get(b.at) {
            Some(TakenIn::Household) => (Standing::Household, self.household_prefs(party.slot())),
            Some(TakenIn::Person) => (Standing::Person, self.household_prefs(party.slot())),
            Some(TakenIn::Office(_)) => match self.decisions.in_office(b.at, party) {
                Some(decider) => decider,
                None => violation!(
                    clause = "MND.20",
                    "a decision taken in an office its party's form does not declare",
                    kind = party.kind()
                ),
            },
            None => violation!(clause = "MND.20", "a decision bound beyond the kinds"),
        }
    }

    /// A decision taken: its decider named, the input built from that decider's preferences, the rule called on it,
    /// and the decision counted by its decider.
    #[clause("MND.20")]
    pub(crate) fn decide<I, O>(&self, b: Bound<I, O>, party: PartyKey, input: impl FnOnce(&Prefs) -> I) -> O {
        if self.player.household == Some(party) {
            violation!(clause = "OBS.4", "a decision of the player's household taken past the player");
        }
        let (standing, prefs) = self.decider(b, party);
        self.tally(b, standing);
        (b.point.rule)(&input(&prefs))
    }

    /// A decision a household or its person takes: as the player says where the household is the player's, none
    /// where the player keeps it and queued nothing; by the rule for every other household.
    #[clause("MND.20", "OBS.4")]
    pub(crate) fn decide_own<I, O: QueuedPayload>(
        &self,
        b: Bound<I, O>,
        household: PartyKey,
        input: impl FnOnce(&Prefs) -> I,
    ) -> Option<O> {
        match self.say(b.at, household, || self.decider(b, household).1) {
            Some(say) => say.take(b.point, input),
            None => Some(self.decide(b, household, input)),
        }
    }

    /// A decision taken at a party's founding, before the party is begun, by the preferences drawn for it.
    #[clause("MND.20", "MND.16")]
    pub(crate) fn decide_founding<I, O>(&self, b: Bound<I, O>, prefs: &Prefs, input: impl FnOnce(&Prefs) -> I) -> O {
        self.tally(b, Standing::Founding);
        (b.point.rule)(&input(prefs))
    }

    /// A decision taken by a population process for the household at a slot of a kind: as the player says where the
    /// household is the player's; else by the rule at its preferences, counted by its household's standing.
    pub(crate) fn decided_in_process(&self, name: &str, (kind, slot): (u8, Slot)) -> Say {
        let Some(at) = self.decisions.position(name) else {
            violation!(clause = "MND.20", "a process's decision the register does not declare");
        };
        let standing = match self.decisions.taken_in.get(at) {
            Some(TakenIn::Person) => Standing::Person,
            Some(TakenIn::Household) => Standing::Household,
            _ => violation!(clause = "MND.20", "a process's decision taken in an office"),
        };
        if let Some(say) = self.say(at, PartyKey::new(kind, slot), || self.household_prefs(slot)) {
            return say;
        }
        self.decisions.count(at, standing);
        Say::Rule(self.household_prefs(slot))
    }

    fn tally<I, O>(&self, b: Bound<I, O>, standing: Standing) {
        self.decisions.count(b.at, standing);
    }

    /// A decider's stance, reconsidered in a decision, written where its preferences live: its holder's, its
    /// institution's founding preferences, or its household's record.
    #[clause("VAL.7", "MND.20")]
    pub(crate) fn set_stance<I, O>(&mut self, b: Bound<I, O>, party: PartyKey, stance: u16) {
        match self.decider(b, party).0 {
            Standing::Holder => {
                let office = self.decisions.office.get(b.at).and_then(|o| o.get(usize::from(party.kind()))).copied();
                if let Some(Some(office)) = office
                    && let Some((_, p)) = self.decisions.holders.get_mut(&(party, office))
                {
                    p.stance = Missing::Present(stance);
                }
            }
            Standing::Founding => {
                let at = usize::try_from(party.slot().get()).unwrap_or(usize::MAX);
                if let Some(p) = self.decisions.founding.get_mut(usize::from(party.kind())).and_then(|f| f.get_mut(at))
                {
                    p.stance = Missing::Present(stance);
                }
            }
            Standing::Household | Standing::Person => {
                let Some((place, [_, _, h, _])) = self.decisions.household else { return };
                if let Some(w) = self.kinds.get_mut(place).and_then(|k| k.record_mut(party.slot()).get_mut(h)) {
                    *w = MaybeI64::present(i64::from(stance));
                }
            }
            Standing::Player => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use phx_core::decisions::{Prefs, Standing, TakenIn};
    use phx_id::{PartyKey, Slot};
    use phx_num::Missing;

    use super::Decisions;

    /// A decision taken in the second office of kind 1's form, which kind 0's form lacks; kind 1's party at slot 2
    /// founded with a required return.
    fn fixture() -> (Decisions, PartyKey) {
        let firm = PartyKey::new(1, Slot::new(2));
        let founded = Prefs { required_return: Missing::Present(0.1), ..Prefs::NONE };
        let decisions = Decisions {
            names: vec!["FRM.close".to_owned()],
            taken_in: vec![TakenIn::Office("chief_executive".to_owned())],
            office: vec![vec![None, Some(1)]],
            founding: vec![Vec::new(), vec![Prefs::NONE, Prefs::NONE, founded]],
            holders: BTreeMap::new(),
            taken: vec![Standing::ALL.iter().map(|_| phx_exec::Tally::default()).collect()],
            household: None,
        };
        (decisions, firm)
    }

    #[test]
    fn vacant_office_reads_founding() {
        let (d, firm) = fixture();
        let (standing, prefs) = d.in_office(0, firm).expect("the form declares the office");
        assert_eq!(standing, Standing::Founding);
        assert_eq!(prefs.required_return, Missing::Present(0.1), "the institution's founding preferences");
        assert!(d.in_office(0, PartyKey::new(0, Slot::new(2))).is_none(), "a form without the office");
        let (_, unfounded) = d.in_office(0, PartyKey::new(1, Slot::new(7))).expect("the office");
        assert_eq!(unfounded, Prefs::NONE, "no preference drawn is none held");
    }

    #[test]
    fn holder_reads_its_own() {
        let (mut d, firm) = fixture();
        let own = Prefs { required_return: Missing::Present(0.2), ..Prefs::NONE };
        d.holders.insert((firm, 1), (42, own));
        assert_eq!(d.in_office(0, firm), Some((Standing::Holder, own)), "the holder's, never the founding ones");
    }

    #[test]
    fn owner_holds_every_office_until_it_leaves() {
        let (mut d, firm) = fixture();
        let own = Prefs { required_return: Missing::Present(0.2), ..Prefs::NONE };
        d.appoint(firm, 42, own);
        assert_eq!(d.in_office(0, firm), Some((Standing::Holder, own)), "the owner's own");
        d.vacate(firm, Some(7));
        assert_eq!(d.in_office(0, firm), Some((Standing::Holder, own)), "another person's leaving empties nothing");
        d.vacate(firm, Some(42));
        assert_eq!(d.in_office(0, firm).map(|x| x.0), Some(Standing::Founding), "empty, the founding preferences");
        assert_eq!(d.held(), 0, "no office held");
    }
}
