//! The decision core. Every decision the world takes is taken here, each time it is: the core names its decider — an
//! office's holder, or, for an office no one holds, its institution's preferences drawn at its founding; a household;
//! a person — hands the decision's input that decider's preferences and nothing else, calls the decision's rule and
//! counts the decision by its decider.

use phx_core::decisions::{DecisionKinds, DecisionPointDecl, Prefs, QueuedPayload, Say, Standing, TakenIn};
use phx_core::kinds::LegalForm;
use phx_id::{PartyKey, PartyRef, Slot};
use phx_macros::{clause, opening};
use phx_num::{Missing, violation};
use phx_pop::offices::{Holder, OfficeRef};

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

/// Each decision point the day takes, bound once to its place among the declared decisions: those the systems
/// declare, and those the labour market's, the bills' and the benefit's kinds name. Bound once the decisions and those
/// kinds are known, and again at load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Points {
    pub(crate) staff: Option<usize>,
    pub(crate) extract: Option<usize>,
    pub(crate) ship: Option<usize>,
    pub(crate) invest: Option<usize>,
    pub(crate) request: Option<usize>,
    pub(crate) household_stance: Option<usize>,
    pub(crate) standard: Option<usize>,
    pub(crate) decline: Option<usize>,
    pub(crate) quote: Option<usize>,
    pub(crate) choose: Option<usize>,
    pub(crate) consume: Option<usize>,
    pub(crate) produce: Option<usize>,
    pub(crate) inputs: Option<usize>,
    pub(crate) spend: Option<usize>,
    pub(crate) firm_stance: Option<usize>,
    pub(crate) review_price: Option<usize>,
    pub(crate) reprice: Option<usize>,
    pub(crate) attend: Option<usize>,
    pub(crate) offer: Option<usize>,
    pub(crate) post: Option<usize>,
    pub(crate) search: Option<usize>,
    pub(crate) answer: Option<usize>,
    pub(crate) select: Option<usize>,
    pub(crate) accept: Option<usize>,
    pub(crate) size: Option<usize>,
    pub(crate) bid: Option<usize>,
    pub(crate) claim: Option<usize>,
}

/// The markets' and the state's kinds that name decision points of their own.
pub(crate) type PointKinds<'a> = (
    Option<&'a if_labour::kind::LabourKind>,
    Option<&'a if_state::kinds::BillKind>,
    Option<&'static DecisionPointDecl<if_state::kinds::ClaimIn, bool>>,
);

impl Points {
    /// Each point's place among the declared decisions' names; none where the register declares none.
    #[opening]
    #[must_use]
    pub(crate) fn of(names: &[String], (labour, bills, claim): PointKinds<'_>) -> Points {
        let at = |name: &str| names.iter().position(|n| n == name);
        Points {
            staff: at(sys_soc::points::STAFF.name),
            extract: at(sys_gds::points::EXTRACT.name),
            ship: at(sys_frt::points::SHIP.name),
            invest: at(sys_cap::points::INVEST.name),
            request: at(sys_bnk::points::REQUEST.name),
            household_stance: at(sys_hh::points::STANCE.name),
            standard: at(sys_bnk::points::STANDARD.name),
            decline: at(sys_bnk::points::DECLINE.name),
            quote: at(sys_bnk::points::QUOTE.name),
            choose: at(sys_bnk::points::CHOOSE.name),
            consume: at(sys_soc::points::CONSUME.name),
            produce: at(sys_frm::points::PRODUCE.name),
            inputs: at(sys_frm::points::INPUTS.name),
            spend: at(sys_hh::points::SPEND.name),
            firm_stance: at(sys_frm::points::STANCE.name),
            review_price: at(sys_frm::points::REVIEW_PRICE.name),
            reprice: at(sys_frm::points::REPRICE.name),
            attend: at(sys_frm::points::ATTEND.name),
            offer: labour.and_then(|l| at(l.offer.name)),
            post: labour.and_then(|l| at(l.post.name)),
            search: labour.and_then(|l| at(l.search.name)),
            answer: labour.and_then(|l| at(l.answer.name)),
            select: labour.and_then(|l| at(l.select.name)),
            accept: labour.and_then(|l| at(l.accept.name)),
            size: bills.and_then(|b| at(b.size.name)),
            bid: bills.and_then(|b| at(b.bid.name)),
            claim: claim.and_then(|c| at(c.name)),
        }
    }
}

/// How a kind's institutions hold the preferences they were founded with: drawn for each and kept in its record — its
/// memory, switching and required-return types, with each required-return type's return — or the same for every one;
/// or not declared, for a kind founding none, whose office decisions stop the run.
#[derive(Clone, Debug, PartialEq, phx_macros::Saved)]
pub enum Founding {
    Record(Vec<f64>),
    Shared(Prefs),
    Undeclared,
}

/// The decisions the world takes and who takes them: each decision's name and taker, the place of the office it is
/// taken in among the offices of each party kind's form, how each kind's institutions hold their founding preferences,
/// the age classes' first ages and the day a holder's age is read on, and the decisions taken, by kind and by the
/// decider's standing.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Decisions {
    names: Vec<String>,
    taken_in: Vec<TakenIn>,
    office: Vec<Vec<Option<u16>>>,
    founded: Vec<Founding>,
    windows: Vec<i64>,
    windows_on: Option<phx_id::Date>,
    taken: Vec<Vec<phx_exec::Tally>>,
}

impl Decisions {
    /// The place of the office a decision is taken in among its party's form's offices; none where the form does not
    /// declare it.
    fn office_of(&self, at: usize, kind: u8) -> Option<u16> {
        *self.office.get(at)?.get(usize::from(kind))?
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

    /// A decision counted as taken by a standing of decider.
    pub(crate) fn count(&self, at: usize, standing: Standing) {
        if let Some(c) = self.taken.get(at).and_then(|t| t.get(standing_at(standing))) {
            c.add(1);
        }
    }
}

/// An institution's founding preferences from its record: its memory, switching and required-return types, the last
/// read as its type's return, and the stance its offices decide by. A type the record lacks stops the run.
#[clause("MND.16")]
fn founding_prefs((types, stance): ([Missing<u8>; 3], Missing<u8>), returns: &[f64], party: PartyKey) -> Prefs {
    let [memory, switching, required] = types.map(|t| phx_pop::offices::founding(t, party));
    let Some(required_return) = returns.get(usize::from(required)).copied() else {
        violation!(clause = "MND.16", "a required-return type beyond its types", party = party.word());
    };
    Prefs {
        memory: Missing::Present(u16::from(memory)),
        switching: Missing::Present(u16::from(switching)),
        stance: match stance {
            Missing::Present(s) => Missing::Present(u16::from(s)),
            Missing::Absent => Missing::Absent,
        },
        required_return: Missing::Present(required_return),
        ..Prefs::NONE
    }
}

/// Who decides in an office: the person who owns and manages its institution, bringing the institution's founding
/// preferences at its own age class until persons hold minds of their own; else, while it is empty, the founding
/// preferences. An appointed office waits for the appointments its holder is read through.
#[clause("MND.16", "MND.20")]
fn resolve(
    holder: Holder,
    founding: Prefs,
    window: impl FnOnce(PartyRef) -> Missing<u16>,
    party: PartyKey,
) -> (Standing, Prefs) {
    match holder {
        Holder::Vacant => (Standing::Founding, founding),
        Holder::Owner(p) => (Standing::Holder, Prefs { window: window(p), ..founding }),
        Holder::Appointment(_) => {
            violation!(clause = "PTY.16", "an appointed office read before appointments are", party = party.word())
        }
    }
}

/// The place of a standing among the counts.
fn standing_at(standing: Standing) -> usize {
    Standing::ALL.iter().position(|s| *s == standing).unwrap_or(0)
}

impl Core {
    /// The decisions opened: each kind as the register declares it, its office found among the forms of the core's
    /// kinds, and no kind's founding preferences known until its institutions are founded.
    #[clause("MND.20", "PTY.16")]
    #[opening]
    pub fn open_decisions(&mut self, kinds: &DecisionKinds, forms: &[Option<&LegalForm>]) {
        let office = kinds
            .kinds
            .iter()
            .map(|k| {
                forms
                    .iter()
                    .map(|f| match (&k.taken_in, f) {
                        (TakenIn::Office(o), Some(f)) => f.office(o).and_then(|i| u16::try_from(i).ok()),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        self.decisions = Decisions {
            names: kinds.kinds.iter().map(|k| k.name.clone()).collect(),
            taken_in: kinds.kinds.iter().map(|k| k.taken_in.clone()).collect(),
            office,
            founded: vec![Founding::Undeclared; self.names.len()],
            windows: Vec::new(),
            windows_on: None,
            taken: kinds
                .kinds
                .iter()
                .map(|_| Standing::ALL.iter().map(|_| phx_exec::Tally::default()).collect())
                .collect(),
        };
    }

    /// How a kind's institutions hold their founding preferences, read by each of their offices no one holds.
    #[clause("MND.16")]
    pub(crate) fn found(&mut self, kind: u8, founding: Founding) {
        let Some(f) = self.decisions.founded.get_mut(usize::from(kind)) else {
            violation!(clause = "MND.16", "founding preferences for a kind the core does not keep", kind = kind);
        };
        *f = founding;
    }

    /// The age classes' first ages and the day a holder's age is read on, the last year's close or the opening.
    pub(crate) fn set_windows(&mut self, windows: &[i64], on: phx_id::Date) {
        self.decisions.windows.clear();
        self.decisions.windows.extend_from_slice(windows);
        self.decisions.windows_on = Some(on);
    }

    /// A decision point at the place bound for it; one bound to none, which the register does not declare, stops the
    /// run.
    pub(crate) fn point<I, O>(
        &self,
        at: fn(&Points) -> Option<usize>,
        point: &'static DecisionPointDecl<I, O>,
    ) -> Bound<I, O> {
        match at(&self.declared.points) {
            Some(at) => Bound { at, point },
            None => violation!(clause = "MND.20", "a decision taken that the register does not declare"),
        }
    }

    /// The day's decision points bound among the declared decisions, with the kinds that name some of them.
    #[opening]
    pub(crate) fn bind_points(&mut self, labour: Option<&if_labour::kind::LabourKind>) {
        let kinds = (labour, self.bills.kind.as_ref(), self.state.claim);
        self.declared.points = Points::of(&self.decisions.names, kinds);
    }

    /// A decision point bound to its kind by its name, at the opening; the day reads the bound points.
    #[opening]
    pub(crate) fn bind<I, O>(&self, point: &'static DecisionPointDecl<I, O>) -> Bound<I, O> {
        let Some(at) = self.decisions.names.iter().position(|n| n == point.name) else {
            violation!(clause = "MND.20", "a decision taken that the register does not declare");
        };
        Bound { at, point }
    }

    /// A household's preferences: its memory and switching types, its stance and its age class.
    fn household_prefs(&self, slot: Slot) -> Prefs {
        let Some(v) = self.household_view(slot) else { return Prefs::NONE };
        let present = |x: Option<u16>| x.map_or(Missing::Absent, Missing::Present);
        let types = v.types();
        Prefs {
            memory: present(types.map(|(m, _)| m)),
            switching: present(types.map(|(_, s)| s)),
            stance: present(v.stance()),
            window: match v.window() {
                Missing::Present(c) => Missing::Present(u16::from(c)),
                Missing::Absent => Missing::Absent,
            },
            ..Prefs::NONE
        }
    }

    /// Who takes a decision in an institution's office, read through the first office its record keeps. An
    /// institution founded without its preferences, or without its offices, stops the run.
    #[clause("MND.16", "PTY.16")]
    fn in_office(&self, party: PartyKey, office: u16, returns: &[f64]) -> (Standing, Prefs) {
        let Some(v) = self.firm_of(party) else {
            violation!(clause = "MND.16", "an office decision of an institution with no record", party = party.word());
        };
        let founding = founding_prefs((v.founding(), v.stance()), returns, party);
        let (Missing::Present(first), Some(offices)) = (v.head_office(), self.offices.as_ref()) else {
            violation!(clause = "PTY.16", "an institution whose offices were never opened", party = party.word());
        };
        let o = offices.office(OfficeRef::at(Slot::new(first)), (office, office), party);
        resolve(offices.holder(o, &self.directory), founding, |p| self.holder_window(p), party)
    }

    /// A holder's age class on the day ages are read on; none before they are first read.
    fn holder_window(&self, holder: PartyRef) -> Missing<u16> {
        let Some(on) = self.decisions.windows_on else { return Missing::Absent };
        let Some(view) = self.persons.as_ref().and_then(|ps| ps.view_at(holder.slot())) else {
            violation!(clause = "PTY.17", "an office held by no live person", party = holder.word());
        };
        match u32::try_from(view.word().age_on(on)) {
            Ok(age) => phx_val::types::window_in(&self.decisions.windows, age),
            Err(_) => Missing::Absent,
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
            Some(TakenIn::Office(_)) => {
                let Some(office) = self.decisions.office_of(b.at, party.kind()) else {
                    violation!(
                        clause = "MND.20",
                        "a decision taken in an office its party's form does not declare",
                        kind = party.kind()
                    );
                };
                match self.decisions.founded.get(usize::from(party.kind())) {
                    Some(Founding::Record(returns)) => self.in_office(party, office, returns),
                    Some(Founding::Shared(prefs)) => (Standing::Founding, *prefs),
                    Some(Founding::Undeclared) | None => violation!(
                        clause = "MND.16",
                        "an office decision of a kind founded with no preferences",
                        kind = party.kind()
                    ),
                }
            }
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

    /// A decider's stance, reconsidered in a decision, written where its preferences live: the record of the
    /// institution whose office took it, or its household's store.
    #[clause("VAL.7", "MND.20")]
    pub(crate) fn set_stance<I, O>(&mut self, b: Bound<I, O>, party: PartyKey, stance: u16) {
        match self.decider(b, party).0 {
            Standing::Holder | Standing::Founding => {
                if !matches!(self.decisions.founded.get(usize::from(party.kind())), Some(Founding::Record(_))) {
                    violation!(
                        clause = "VAL.7",
                        "a stance for an institution whose record keeps none",
                        kind = party.kind()
                    );
                }
                if let Some(fs) = self.firms.as_mut() {
                    fs.set_stance(party.slot(), stance);
                }
            }
            Standing::Household | Standing::Person => {
                self.household_write(|hs| hs.set_stance(party.slot(), stance));
            }
            Standing::Player => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::decisions::{Prefs, Standing};
    use phx_id::{Day, PartyKey, Slot};
    use phx_num::Missing;
    use phx_pop::directory::Directory;
    use phx_pop::offices::{OfficeRef, Offices};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{founding_prefs, resolve};

    type Heap = HeapBacking<4096>;

    const FIRMS: u8 = 0;
    const PERSONS: u8 = 1;

    /// A firm's two offices opened empty, its first office, and a person to hold them.
    fn fixture() -> (Directory<Heap>, Offices<Heap>, PartyKey, OfficeRef, phx_id::PartyRef) {
        let mut space = AddressSpace::empty();
        let mut dir = Directory::new(&mut space, &[8, 8], 8, (Day::new(1), 730));
        let firm = dir.begin(FIRMS);
        let person = dir.begin(PERSONS);
        let mut offices = Offices::new(&mut space, PERSONS, 8);
        let firm = PartyKey::new(firm.kind(), firm.slot());
        let Missing::Present(first) = offices.open(firm, &[0, 1]) else { panic!("a company declares offices") };
        (dir, offices, firm, first, person)
    }

    fn founded() -> Prefs {
        founding_prefs(
            ([Missing::Present(1), Missing::Present(0), Missing::Present(2)], Missing::Present(3)),
            &[0.05, 0.1, 0.15],
            PartyKey::new(FIRMS, Slot::new(0)),
        )
    }

    #[test]
    fn founding_read_from_the_record() {
        let p = founded();
        assert_eq!((p.memory, p.switching, p.stance), (Missing::Present(1), Missing::Present(0), Missing::Present(3)));
        assert_eq!(p.required_return, Missing::Present(0.15), "its type's return");
        assert_eq!((p.window, p.management), (Missing::Absent, Missing::Absent));
        let firm = PartyKey::new(FIRMS, Slot::new(0));
        let none = [Missing::Present(1), Missing::Absent, Missing::Present(0)];
        assert!(std::panic::catch_unwind(|| founding_prefs((none, Missing::Absent), &[0.1], firm)).is_err());
        let beyond = [Missing::Present(1), Missing::Present(0), Missing::Present(1)];
        assert!(std::panic::catch_unwind(|| founding_prefs((beyond, Missing::Absent), &[0.1], firm)).is_err());
    }

    #[test]
    fn vacant_office_reads_founding() {
        let (dir, offices, firm, first, _) = fixture();
        let o = offices.office(first, (1, 1), firm);
        let (standing, prefs) = resolve(offices.holder(o, &dir), founded(), |_| Missing::Present(4), firm);
        assert_eq!((standing, prefs), (Standing::Founding, founded()), "the founding preferences, at no age");
    }

    #[test]
    fn owner_holds_until_it_leaves() {
        let (dir, mut offices, firm, first, person) = fixture();
        for at in 0..2 {
            offices.fill_owned(offices.office(first, (at, at), firm), person, Day::new(3));
        }
        let o = offices.office(first, (1, 1), firm);
        let (standing, prefs) = resolve(
            offices.holder(o, &dir),
            founded(),
            |p| {
                assert_eq!(p, person, "the holder's own age read");
                Missing::Present(4)
            },
            firm,
        );
        assert_eq!((standing, prefs.window), (Standing::Holder, Missing::Present(4)));
        assert_eq!(prefs.required_return, founded().required_return, "its institution's founding preferences");
        offices.vacate(o);
        assert_eq!(resolve(offices.holder(o, &dir), founded(), |_| Missing::Absent, firm).0, Standing::Founding);
    }

    /// Every decision the systems declare, as the register lists them, and the labour market's kind.
    fn declared() -> (Vec<String>, if_labour::kind::LabourKind) {
        let labour = sys_lab::LABOUR;
        let mut names: Vec<String> = [
            sys_soc::points::STAFF.name,
            sys_frm::points::PRODUCE.name,
            sys_hh::points::SPEND.name,
            labour.post.name,
            labour.offer.name,
        ]
        .iter()
        .map(|n| (*n).to_owned())
        .collect();
        names.reverse();
        (names, labour)
    }

    #[test]
    fn points_bound_once_by_declaration() {
        let (names, labour) = declared();
        let p = super::Points::of(&names, (Some(&labour), None, None));
        let at = |name: &str| names.iter().position(|n| n == name);
        assert_eq!(p.produce, at(sys_frm::points::PRODUCE.name));
        assert_eq!(p.spend, at(sys_hh::points::SPEND.name));
        assert_eq!(p.post, at(labour.post.name));
        assert_eq!(p.offer, at(labour.offer.name));
        assert_eq!((p.invest, p.size, p.claim), (None, None, None), "points the register lists none of bind none");
    }

    #[test]
    fn points_rebuilt_equal() {
        let (names, labour) = declared();
        let kinds = (Some(&labour), None, None);
        assert_eq!(super::Points::of(&names, kinds), super::Points::of(&names, kinds));
    }

    #[test]
    fn undeclared_point_refused_at_assembly() {
        let p = super::Points::of(&[], (None, None, None));
        assert_eq!(p.produce, None, "bound to none at assembly");
        let bound = |at: Option<usize>| match at {
            Some(at) => at,
            None => phx_num::violation!(clause = "MND.20", "a decision taken that the register does not declare"),
        };
        assert!(std::panic::catch_unwind(|| bound(p.produce)).is_err());
    }
}
