//! Parties: named individually, or a CELL standing for a population with a weight.

use crate::ids::{PartyId, RegionId};
use std::num::NonZeroU32;

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
    ceased_at: Vec<Option<u32>>,
    /// The cell's key on its kind's lattice, as a row in a names table.
    key: Vec<u32>,
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

    pub fn add(
        &mut self,
        kind: u32,
        region: RegionId,
        bank: PartyId,
        representation: Representation,
        key: u32,
    ) -> PartyId {
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.region.push(region.0);
        self.bank.push(bank.0);
        self.representation.push(representation);
        self.alive.push(true);
        self.destination.push(None);
        self.ceased_at.push(None);
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
    pub fn key_of(&self, p: PartyId) -> u32 {
        self.key[p.row()]
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
    pub fn split(&mut self, p: PartyId, taking: NonZeroU32) -> PartyId {
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
            self.key[p.row()],
        );
        // A split is not a birth.
        self.since[child.row()] = self.since[p.row()];
        self.reweigh(p, left, WeightEvent::Split);
        child
    }

    /// Designate the legal destination before ordinary discretion is disabled.
    pub fn open_destination(&mut self, p: PartyId, to: Destination, period: u32) {
        assert!(self.alive(p), "a ceased party cannot open another destination");
        assert!(self.destination[p.row()].is_none(), "a party has exactly one destination");
        self.destination[p.row()] = Some(to);
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
    fn cessation_opens_one_durable_destination_before_disabling_discretion() {
        let mut parties = Parties::with_seed(0);
        let party = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);

        parties.open_destination(party, Destination::Estate, 12);
        assert!(parties.alive(party));
        assert_eq!(parties.destination_of(party), Some(Destination::Estate));
        assert_eq!(parties.ceased_at(party), Some(12));

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
