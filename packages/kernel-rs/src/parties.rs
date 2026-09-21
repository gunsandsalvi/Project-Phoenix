//! Parties: named individually, or a CELL standing for a population with a weight.

use crate::ids::{PartyId, RegionId};
use std::num::NonZeroU32;

/// The joint state that defines a household cell. These are categorical lattice coordinates, not
/// averages or independently sampled margins: one cell is one occupied joint combination.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct HouseholdKey {
    pub age: u32,
    pub composition: u32,
    pub employment: u32,
    pub income: u32,
    pub tenure: u32,
    pub liquid_wealth: u32,
    pub debt_service: u32,
}

/// Declared coordinates on the household employment axis. Mechanisms use these names when a
/// transition moves members between lattice cells; the kernel never guesses a destination bin.
pub mod household_employment {
    pub const UNEMPLOYED: u32 = 0;
    pub const EMPLOYED: u32 = 1;
    pub const INACTIVE: u32 = 2;
}

/// The joint state that defines a small-business cell. Firm dynamics require age and size beside
/// productivity and financing state; a sector average is not a firm distribution.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SmallBusinessKey {
    pub sector: u32,
    pub age: u32,
    pub size: u32,
    pub productivity: u32,
    pub leverage: u32,
    pub coverage: u32,
    pub credit_access: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum LatticeKey {
    Household(HouseholdKey),
    SmallBusiness(SmallBusinessKey),
    /// Named parties and legacy diagnostic fixtures have identity rather than a population lattice.
    Named(u32),
}

impl From<u32> for LatticeKey {
    fn from(value: u32) -> Self {
        Self::Named(value)
    }
}

/// How many real parties a row IS. A weight is a COUNT, so it is carried by the representation
/// rather than sitting beside it: a named party is one party and a cell of nobody cannot be
/// written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Representation {
    Named,
    Cell(NonZeroU32),
}

/// XI-15, Small-Business Pools E5: the five events that may change a weight, and nothing else may.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeightEvent {
    Entry,
    Death,
    Promotion,
    Split,
    Merge,
}

/// The legal state that receives authority when ordinary party discretion ends.
///
/// This belongs to the party register because the destination remains durable after the mortality
/// mechanism that selected it has finished.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    Estate,
    Heir,
    Resolution,
}

#[derive(Default)]
pub struct Parties {
    kind: Vec<u32>,
    region: Vec<u32>,
    bank: Vec<u32>,
    representation: Vec<Representation>,
    alive: Vec<bool>,
    destination: Vec<Option<Destination>>,
    authority: Vec<Option<u32>>,
    cessation_trigger: Vec<Option<u8>>,
    ceased_at: Vec<Option<u32>>,
    merged_into: Vec<Option<u32>>,
    /// The cell's key on its kind's lattice, as a row in a names table.
    key: Vec<LatticeKey>,
    /// THE PERIOD THIS PARTY ENTERED.
    since: Vec<u32>,
    /// How many of its own observations this party weighs when it forms an outlook. Drawn once on
    /// admission from the run seed, never read from a global behavioural parameter.
    outlook_memory: Vec<f64>,
    draw_state: u64,
    memory_from: f64,
    memory_to: f64,
    /// The period the world is in, told to this store once by the kernel.
    now: u32,
    of_kind: std::collections::HashMap<u32, Vec<u32>>,
    admitted_cell_weight: std::collections::HashMap<(u32, u32), u64>,
}

impl Parties {
    pub fn new() -> Self {
        Self::with_seed_and_memory(0, 2.0, 10.0)
    }

    pub fn with_seed(seed: u64) -> Self {
        Self::with_seed_and_memory(seed, 2.0, 10.0)
    }

    pub fn with_seed_and_memory(seed: u64, memory_from: f64, memory_to: f64) -> Self {
        assert!(memory_from >= 1.0 && memory_to > memory_from);
        Self { draw_state: seed, memory_from, memory_to, ..Self::default() }
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    /// The period the world has reached, so a party added in it is stamped with it.
    pub fn opened(&mut self, period: u32) {
        self.now = period;
    }

    /// The period it entered.
    #[inline]
    pub fn since(&self, p: PartyId) -> u32 {
        self.since[p.0 as usize]
    }

    /// How many periods it has been going, which is one of the things a grade reads.
    #[inline]
    pub fn age(&self, p: PartyId, now: u32) -> u32 {
        now - self.since(p)
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    pub fn add<K: Into<LatticeKey>>(
        &mut self,
        kind: u32,
        region: RegionId,
        bank: PartyId,
        representation: Representation,
        key: K,
    ) -> PartyId {
        let key = key.into();
        assert!(
            matches!(
                (&representation, &key),
                (Representation::Named, LatticeKey::Named(_))
                    | (Representation::Cell(_), LatticeKey::Household(_))
                    | (Representation::Cell(_), LatticeKey::SmallBusiness(_))
            ),
            "XI-15: a population cell needs a declared household or small-business lattice key"
        );
        if matches!(representation, Representation::Cell(_)) {
            let duplicate = self.key.iter().enumerate().any(|(row, existing)| {
                self.alive[row]
                    && matches!(self.representation[row], Representation::Cell(_))
                    && self.kind[row] == kind
                    && self.region[row] == region.0
                    && self.bank[row] == bank.0
                    && existing == &key
            });
            assert!(!duplicate, "XI-15: a live population lattice coordinate is unique");
        }
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.region.push(region.0);
        self.bank.push(bank.0);
        self.representation.push(representation);
        self.alive.push(true);
        self.destination.push(None);
        self.authority.push(None);
        self.cessation_trigger.push(None);
        self.ceased_at.push(None);
        self.merged_into.push(None);
        self.key.push(key);
        self.since.push(self.now);
        self.draw_state = self.draw_state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut draw = self.draw_state;
        draw = (draw ^ (draw >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        draw = (draw ^ (draw >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        draw ^= draw >> 31;
        let unit = ((draw >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64));
        self.outlook_memory.push(self.memory_from + unit * (self.memory_to - self.memory_from));
        self.of_kind.entry(kind).or_default().push(row);
        if let Representation::Cell(weight) = representation {
            *self.admitted_cell_weight.entry((kind, region.0)).or_default() += u64::from(weight.get());
        }
        PartyId(row)
    }

    #[inline]
    pub fn outlook_memory(&self, p: PartyId) -> f64 {
        self.outlook_memory[p.row()]
    }

    #[inline]
    pub fn kind_of(&self, p: PartyId) -> u32 {
        self.kind[p.row()]
    }

    #[inline]
    pub fn region_of(&self, p: PartyId) -> RegionId {
        RegionId(self.region[p.row()])
    }

    #[inline]
    pub fn bank_of(&self, p: PartyId) -> PartyId {
        PartyId(self.bank[p.row()])
    }

    #[inline]
    pub fn alive(&self, p: PartyId) -> bool {
        self.alive[p.row()]
    }

    #[inline]
    pub fn weight(&self, p: PartyId) -> u32 {
        match self.representation[p.row()] {
            Representation::Named => 1,
            Representation::Cell(of) => of.get(),
        }
    }

    /// Whether this party is one party or a CELL standing for many.
    #[inline]
    pub fn representation_of(&self, p: PartyId) -> Representation {
        self.representation[p.row()]
    }

    /// A cell's identity is a KEY on its kind's declared lattice, and the key is part of what the
    /// party IS — so a snapshot that dropped it would open a world of different cells.
    #[inline]
    pub fn key_of(&self, p: PartyId) -> &LatticeKey {
        &self.key[p.row()]
    }

    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.of_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// A weight changes ONLY by one of the five events, and the event is named at the call.
    pub fn reweigh(&mut self, p: PartyId, to: NonZeroU32, by: WeightEvent) {
        assert!(
            matches!(self.representation[p.row()], Representation::Cell(_)),
            "XI-15: a named party's weight is one and does not change ({by:?})"
        );
        self.representation[p.row()] = Representation::Cell(to);
    }

    /// An event that applies to SOME members splits the cell.
    pub fn split(&mut self, p: PartyId, taking: NonZeroU32, destination: LatticeKey) -> PartyId {
        let Representation::Cell(of) = self.representation[p.row()] else {
            panic!("XI-15: a named party is one party and has no part to split off");
        };
        let had = of.get();
        let Some(left) = had.checked_sub(taking.get()).and_then(NonZeroU32::new) else {
            panic!("XI-15: {taking} of a cell of {had} is not a part of it");
        };
        let child = self.add(
            self.kind[p.row()],
            RegionId(self.region[p.row()]),
            PartyId(self.bank[p.row()]),
            Representation::Cell(taking),
            destination,
        );
        *self.admitted_cell_weight.get_mut(&(self.kind[p.row()], self.region[p.row()])).expect("a split parent was admitted") -= u64::from(taking.get());
        // A split is not a birth.
        self.since[child.row()] = self.since[p.row()];
        // Nor is it a new behavioural draw. Both rows are partitions of the same admitted cell;
        // only observations after the split may make their outlook histories diverge.
        self.outlook_memory[child.row()] = self.outlook_memory[p.row()];
        self.reweigh(p, left, WeightEvent::Split);
        child
    }

    /// Merge two live cells after a transition has made them one lattice population again. The
    /// consumed row remains a tombstone so journal references stay valid.
    pub fn merge(&mut self, into: PartyId, from: PartyId, destination: LatticeKey) {
        assert_ne!(into, from, "XI-15: a cell cannot merge into itself");
        assert!(self.alive(into) && self.alive(from), "XI-15: only live cells can merge");
        assert_eq!(self.kind[into.row()], self.kind[from.row()], "XI-15: merged cells need one kind");
        assert_eq!(self.region[into.row()], self.region[from.row()], "XI-15: merged cells need one region");
        assert_eq!(self.bank[into.row()], self.bank[from.row()], "XI-15: merged cells need one bank");
        let Representation::Cell(into_weight) = self.representation[into.row()] else { panic!("XI-15: a named party cannot receive a cell merge") };
        let Representation::Cell(from_weight) = self.representation[from.row()] else { panic!("XI-15: a named party cannot be merged as a cell") };
        assert!(matches!((&self.key[into.row()], &destination), (LatticeKey::Household(_), LatticeKey::Household(_)) | (LatticeKey::SmallBusiness(_), LatticeKey::SmallBusiness(_))), "XI-15: a merge destination must stay on the population's lattice");
        let duplicate = self.key.iter().enumerate().any(|(row, existing)| row != into.row() && row != from.row() && self.alive[row] && matches!(self.representation[row], Representation::Cell(_)) && self.kind[row] == self.kind[into.row()] && self.region[row] == self.region[into.row()] && self.bank[row] == self.bank[into.row()] && existing == &destination);
        assert!(!duplicate, "XI-15: a live population lattice coordinate is unique");
        let combined = into_weight.get().checked_add(from_weight.get()).and_then(NonZeroU32::new).expect("XI-15: merged cell weight overflowed");
        self.key[into.row()] = destination;
        self.reweigh(into, combined, WeightEvent::Merge);
        self.alive[from.row()] = false;
        self.merged_into[from.row()] = Some(into.0);
    }

    pub fn merged_into(&self, p: PartyId) -> Option<PartyId> {
        self.merged_into[p.row()].map(PartyId::at)
    }

    pub fn weight_conservation_gaps(&self) -> Vec<((u32, RegionId), i64)> {
        let mut effective: std::collections::HashMap<(u32, u32), u64> = std::collections::HashMap::new();
        for row in 0..self.len() {
            if self.merged_into[row].is_some() { continue; }
            if let Representation::Cell(weight) = self.representation[row] {
                *effective.entry((self.kind[row], self.region[row])).or_default() += u64::from(weight.get());
            }
        }
        self.admitted_cell_weight.iter().filter_map(|(&(kind, region), &admitted)| {
            let standing = match effective.get(&(kind, region)) {
                Some(weight) => *weight,
                None => 0,
            };
            (standing != admitted).then_some(((kind, RegionId(region)), i64::try_from(standing).expect("cell weight fits i64") - i64::try_from(admitted).expect("cell weight fits i64")))
        }).collect()
    }

    /// Designate the legal destination before ordinary discretion is disabled.
    pub fn open_destination(&mut self, p: PartyId, to: Destination, period: u32) {
        self.open_destination_for(p, to, p, period, None);
    }

    pub fn open_destination_for(
        &mut self,
        p: PartyId,
        to: Destination,
        authority: PartyId,
        period: u32,
        trigger: Option<u8>,
    ) {
        assert!(self.alive(p), "a ceased party cannot open another destination");
        assert!(authority.some(), "a destination needs a named legal authority");
        assert!(self.destination[p.row()].is_none(), "a party has exactly one destination");
        self.destination[p.row()] = Some(to);
        self.authority[p.row()] = Some(authority.0);
        self.cessation_trigger[p.row()] = trigger;
        self.ceased_at[p.row()] = Some(period);
    }

    #[inline]
    pub fn destination_of(&self, p: PartyId) -> Option<Destination> {
        self.destination[p.row()]
    }

    #[inline]
    pub fn ceased_at(&self, p: PartyId) -> Option<u32> {
        self.ceased_at[p.row()]
    }

    pub fn authority_of(&self, p: PartyId) -> Option<PartyId> {
        self.authority[p.row()].map(PartyId::at)
    }

    pub fn cessation_trigger(&self, p: PartyId) -> Option<u8> {
        self.cessation_trigger[p.row()]
    }

    /// Nothing is immortal, and a death cannot occur before its destination exists.
    pub fn cease(&mut self, p: PartyId) {
        assert!(self.destination_of(p).is_some(), "cessation requires an open destination");
        self.alive[p.row()] = false;
    }

    /// What ONE MEMBER of a cell holds, as a READ over the total.
    #[inline]
    pub fn per_member(&self, p: PartyId, total: f64) -> f64 {
        total / f64::from(self.weight(p))
    }
}

// Three of the five tests here asserted refusals the TYPE now makes unconstructible: a cell of
// nobody (`NonZeroU32`), a named party with a weight other than one (`Named` carries none), and a
// split that takes nobody. What is left runtime is what relates an argument to what the store
// already holds — reweighing a named party, and taking more of a cell than it has — and each
// panics at the site.
//
// `per_member` is `total / weight`, and `weight` is a read of the representation.

#[cfg(test)]
mod tests {
    use super::*;

    fn admit_two(seed: u64) -> (f64, f64) {
        let mut parties = Parties::with_seed(seed);
        let one = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);
        let two = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 2);
        (parties.outlook_memory(one), parties.outlook_memory(two))
    }

    #[test]
    fn outlook_memory_is_drawn_once_per_party_and_reproduced_by_the_run_seed() {
        let first = admit_two(17);
        assert_eq!(first, admit_two(17));
        assert_ne!(first, admit_two(18));
        assert_ne!(first.0, first.1);
        assert!(first.0 >= 2.0 && first.0 < 10.0);
        assert!(first.1 >= 2.0 && first.1 < 10.0);
    }

    #[test]
    fn a_split_preserves_entry_and_memory_and_conserves_the_cell_weight() {
        let mut parties = Parties::with_seed(17);
        parties.opened(9);
        let parent = parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            LatticeKey::Household(HouseholdKey {
                age: 4,
                composition: 1,
                employment: 2,
                income: 5,
                tenure: 1,
                liquid_wealth: 3,
                debt_service: 2,
            }),
        );
        let memory = parties.outlook_memory(parent);
        parties.opened(20);
        let destination = LatticeKey::Household(HouseholdKey {
            employment: household_employment::EMPLOYED,
            ..match parties.key_of(parent).clone() { LatticeKey::Household(key) => key, _ => unreachable!() }
        });
        let child = parties.split(parent, NonZeroU32::new(3).unwrap(), destination.clone());

        assert_eq!(parties.since(parent), 9);
        assert_eq!(parties.since(child), 9);
        assert_eq!(parties.outlook_memory(child), memory);
        assert_eq!(parties.weight(parent) + parties.weight(child), 10);
        assert_eq!(parties.key_of(child), &destination);
        let original = parties.key_of(parent).clone();
        parties.merge(parent, child, original);
        assert_eq!(parties.weight(parent), 10);
        assert_eq!(parties.merged_into(child), Some(parent));
        assert!(!parties.alive(child));
    }

    #[test]
    #[should_panic(expected = "population cell needs a declared household or small-business lattice key")]
    fn a_scalar_label_is_not_a_population_lattice() {
        let mut parties = Parties::with_seed(17);
        parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            4,
        );
    }

    #[test]
    #[should_panic(expected = "a live population lattice coordinate is unique")]
    fn a_second_live_cell_cannot_occupy_the_same_complete_lattice_identity() {
        let mut parties = Parties::with_seed(0);
        let key = LatticeKey::Household(HouseholdKey { age: 4, composition: 1, employment: household_employment::UNEMPLOYED, income: 5, tenure: 1, liquid_wealth: 3, debt_service: 2 });
        parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Cell(NonZeroU32::new(10).unwrap()), key.clone());
        parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Cell(NonZeroU32::new(3).unwrap()), key);
    }

    #[test]
    fn the_store_boundary_detects_cell_weight_drift() {
        let mut parties = Parties::with_seed(0);
        let cell = parties.add(1, RegionId::at(2), PartyId::NONE, Representation::Cell(NonZeroU32::new(10).unwrap()), LatticeKey::Household(HouseholdKey { age: 4, composition: 1, employment: household_employment::UNEMPLOYED, income: 5, tenure: 1, liquid_wealth: 3, debt_service: 2 }));
        assert!(parties.weight_conservation_gaps().is_empty());
        parties.reweigh(cell, NonZeroU32::new(9).unwrap(), WeightEvent::Death);
        assert_eq!(parties.weight_conservation_gaps(), vec![((1, RegionId::at(2)), -1)]);
    }

    #[test]
    fn cessation_opens_one_durable_destination_before_disabling_discretion() {
        let mut parties = Parties::with_seed(0);
        let party = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);

        parties.open_destination(party, Destination::Estate, 12);
        assert!(parties.alive(party));
        assert_eq!(parties.destination_of(party), Some(Destination::Estate));
        assert_eq!(parties.ceased_at(party), Some(12));
        assert_eq!(parties.authority_of(party), Some(party));

        parties.cease(party);
        assert!(!parties.alive(party));
        assert_eq!(parties.destination_of(party), Some(Destination::Estate));
    }

    #[test]
    #[should_panic(expected = "cessation requires an open destination")]
    fn cessation_without_a_destination_is_refused() {
        let mut parties = Parties::with_seed(0);
        let party = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);
        parties.cease(party);
    }
}
