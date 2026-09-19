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

#[derive(Default)]
pub struct Parties {
    kind: Vec<u32>,
    region: Vec<u32>,
    bank: Vec<u32>,
    representation: Vec<Representation>,
    alive: Vec<bool>,
    /// The cell's key on its kind's lattice, as a row in a names table.
    key: Vec<u32>,
    /// THE PERIOD THIS PARTY ENTERED.
    since: Vec<u32>,
    /// The period the world is in, told to this store once by the kernel.
    now: u32,
    of_kind: std::collections::HashMap<u32, Vec<u32>>,
}

impl Parties {
    pub fn new() -> Self {
        Self::default()
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
        self.key.push(key);
        self.since.push(self.now);
        self.of_kind.entry(kind).or_default().push(row);
        PartyId(row)
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

    /// Nothing is immortal, and a death has a destination.
    pub fn cease(&mut self, p: PartyId) {
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
