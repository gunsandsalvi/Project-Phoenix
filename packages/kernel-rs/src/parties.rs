//! Parties: named individually, or a CELL standing for a population with a weight.

use crate::ids::{PartyId, RegionId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Representation {
    Named,
    Cell,
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
    /// How many real parties this row IS.
    weight: Vec<u32>,
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
        self.weight[p.row()]
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
    pub fn reweigh(&mut self, p: PartyId, to: u32, by: WeightEvent) {
        assert!(
            self.representation[p.row()] == Representation::Cell,
            "XI-15: a named party's weight is one and does not change ({by:?})"
        );
        assert!(to > 0, "XI-15: a cell of nobody is not a cell — it dies ({by:?})");
        self.weight[p.row()] = to;
    }

    /// An event that applies to SOME members splits the cell.
    pub fn split(&mut self, p: PartyId, taking: u32) -> PartyId {
        assert!(
            self.representation[p.row()] == Representation::Cell,
            "XI-15: a named party is one party and has no part to split off"
        );
        let had = self.weight[p.row()];
        assert!(
            taking > 0 && taking < had,
            "XI-15: {taking} of a cell of {had} is not a part of it"
        );
        let child = self.add(
            self.kind[p.row()],
            RegionId(self.region[p.row()]),
            PartyId(self.bank[p.row()]),
            Representation::Cell,
            taking,
            self.key[p.row()],
        );
        // A split is not a birth.
        self.since[child.row()] = self.since[p.row()];
        self.reweigh(p, had - taking, WeightEvent::Split);
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
    fn an_event_that_applies_to_some_members_splits_the_cell() {
        // The affected members become a new cell with the same kind, region, bank and key — the same
        // state — and the two counts add back to the one they came from.
        let mut ps = Parties::new();
        let cb = ps.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        let cell = ps.add(1, RegionId::at(2), cb, Representation::Cell, 1_800, 7);
        let part = ps.split(cell, 500);
        assert_eq!(ps.weight(cell), 1_300);
        assert_eq!(ps.weight(part), 500);
        assert_eq!(ps.weight(cell) + ps.weight(part), 1_800);
        assert_eq!(ps.key_of(part), ps.key_of(cell));
        assert_eq!(ps.kind_of(part), ps.kind_of(cell));
        assert_eq!(ps.region_of(part), ps.region_of(cell));
        assert_eq!(ps.bank_of(part), ps.bank_of(cell));
        assert_eq!(ps.representation_of(part), Representation::Cell);
    }

    #[test]
    #[should_panic(expected = "is not a part of it")]
    fn all_of_a_cell_is_not_a_part_of_it() {
        // Taking everybody leaves a cell of nobody, which is a DEATH and not a split.
        let mut ps = Parties::new();
        let cb = ps.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        let cell = ps.add(1, RegionId::at(0), cb, Representation::Cell, 40, 7);
        ps.split(cell, 40);
    }

    #[test]
    #[should_panic(expected = "a cell of nobody")]
    fn a_cell_of_nobody_is_not_a_cell() {
        let mut ps = Parties::new();
        ps.add(1, RegionId::at(0), PartyId::at(0), Representation::Cell, 0, 7);
    }
}
