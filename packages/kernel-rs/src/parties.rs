//! Parties: named individually, or a CELL standing for a population with a weight (XI-15).
//!
//! A cell's holdings are TOTALS and `per_member` is a read. A weight is a COUNT and never a share,
//! it changes only by the five events XI-15 allows, and there is at most one live cell per key on
//! its kind's declared lattice. No mean: a decision taken at an average is a decision nobody took.

use crate::ids::{PartyId, RegionId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Representation {
    Named,
    Cell,
}

/// XI-15: the five events that may change a weight, and nothing else may.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeightEvent {
    Entry,
    Death,
    Promotion,
    Merge,
    Crossing,
}

#[derive(Default)]
pub struct Parties {
    kind: Vec<u32>,
    region: Vec<u32>,
    bank: Vec<u32>,
    representation: Vec<Representation>,
    /// How many real parties this row IS. One for a named party; a count for a cell.
    weight: Vec<u32>,
    alive: Vec<bool>,
    /// The cell's key on its kind's lattice, as a row in a names table. `NONE` for a named party.
    key: Vec<u32>,
    of_kind: std::collections::HashMap<u32, Vec<u32>>,
}

impl Parties {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.kind.len()
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
        weight: u32,
        key: u32,
    ) -> PartyId {
        assert!(
            representation == Representation::Named || weight > 0,
            "XI-15: a cell of nobody is not a cell"
        );
        assert!(
            representation == Representation::Cell || weight == 1,
            "XI-15: a named party is one party"
        );
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.region.push(region.0);
        self.bank.push(bank.0);
        self.representation.push(representation);
        self.weight.push(weight);
        self.alive.push(true);
        self.key.push(key);
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

    /// XI-15: a weight is a COUNT. A named party is one party; a cell is however many it stands for.
    #[inline]
    pub fn weight(&self, p: PartyId) -> u32 {
        self.weight[p.row()]
    }

    /// XI-15: whether this party is one party or a CELL standing for many. A reader rather than a
    /// branch: the kernel asks so it can write a party down (22b.7), never so it can behave
    /// differently towards one.
    #[inline]
    pub fn representation_of(&self, p: PartyId) -> Representation {
        self.representation[p.row()]
    }

    /// XI-15: a cell's identity is a KEY on its kind's declared lattice, and the key is part of what
    /// the party IS — so a snapshot that dropped it would open a world of different cells.
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

    /// XI-15: a weight changes ONLY by one of the five events, and the event is named at the call.
    /// A caller with no event to name has no business changing a weight.
    pub fn reweigh(&mut self, p: PartyId, to: u32, by: WeightEvent) {
        assert!(
            self.representation[p.row()] == Representation::Cell,
            "XI-15: a named party's weight is one and does not change ({by:?})"
        );
        assert!(to > 0, "XI-15: a cell of nobody is not a cell — it dies ({by:?})");
        self.weight[p.row()] = to;
    }

    /// Money E4, XI-3: nothing is immortal, and a death has a destination. What it held is the
    /// estate's; this only records that the party has ceased.
    pub fn cease(&mut self, p: PartyId) {
        self.alive[p.row()] = false;
    }

    /// XI-15: what ONE MEMBER of a cell holds, as a READ over the total. There is no stored
    /// per-member number anywhere, which is what stops a cell from having two books.
    #[inline]
    pub fn per_member(&self, p: PartyId, total: f64) -> f64 {
        total / f64::from(self.weight(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weight_is_a_count_and_per_member_is_a_read() {
        let mut ps = Parties::new();
        let cb = ps.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        let cell = ps.add(1, RegionId::at(0), cb, Representation::Cell, 4_000, 7);
        assert_eq!(ps.weight(cell), 4_000);
        assert_eq!(ps.per_member(cell, 8_000.0), 2.0);
        assert_eq!(ps.weight(cb), 1);
        assert_eq!(ps.of_kind(1), &[cell.0]);
    }

    #[test]
    #[should_panic(expected = "a named party's weight is one")]
    fn a_named_party_has_no_weight_to_change() {
        let mut ps = Parties::new();
        let p = ps.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        ps.reweigh(p, 9, WeightEvent::Entry);
    }

    #[test]
    #[should_panic(expected = "a cell of nobody")]
    fn a_cell_of_nobody_is_not_a_cell() {
        let mut ps = Parties::new();
        ps.add(1, RegionId::at(0), PartyId::at(0), Representation::Cell, 0, 7);
    }
}
