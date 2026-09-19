//! Parties: named individually, or a CELL standing for a population with a weight.
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

/// XI-15, Small-Business Pools E5: the five events that may change a weight, and nothing else may.
///
/// The fifth is SPLIT, and it was written down as `Crossing`. A crossing is the READ XI-1 names
/// — *population-level default must be a read of cell-level crossings* — and `loss::Crossing` is
/// that read, a borrower passing a threshold on a date. It changes no weight. What XI-15 and E5 both
/// name as the fifth event is the SPLIT: an event applying to SOME members makes them a new cell
/// with the same state, exactly, because identical members divide without remainder. Written as a
/// crossing it read as a kernel bookkeeping step, which is why nothing ever called it.
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
    /// How many real parties this row IS. One for a named party; a count for a cell.
    weight: Vec<u32>,
    alive: Vec<bool>,
    /// The cell's key on its kind's lattice, as a row in a names table. `NONE` for a named party.
    key: Vec<u32>,
    /// THE PERIOD THIS PARTY ENTERED. A party that cannot say how old it is cannot
    /// be graded (§21 A4 reads age), cannot have a fiscal year (§48 A3 places one from when the
    /// company started) and cannot be told from one that has been trading for twenty years. It was
    /// missing entirely: `add` took a key and no `since`.
    since: Vec<u32>,
    /// The period the world is in, told to this store once by the kernel. A party does not choose
    /// when it was born, so `add` stamps rather than asking.
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

    /// The period the world has reached, so a party added in it is stamped with it. The
    /// kernel says this once as a period opens; nothing else has business telling this store when
    /// it is.
    pub fn opened(&mut self, period: u32) {
        self.now = period;
    }

    /// The period it entered. The world opened at period 0, so a party the assembly
    /// admitted before the first step entered at 0 — which is a fact about it and not a default.
    #[inline]
    pub fn since(&self, p: PartyId) -> u32 {
        self.since[p.0 as usize]
    }

    /// How many periods it has been going, which is one of the things a grade reads. A party
    /// cannot be older than the world, so this is arithmetic and never a bound.
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

    /// A weight is a COUNT. A named party is one party; a cell is however many it stands for.
    #[inline]
    pub fn weight(&self, p: PartyId) -> u32 {
        self.weight[p.row()]
    }

    /// Whether this party is one party or a CELL standing for many. A reader rather than a
    /// branch: the kernel asks so it can write a party down, never so it can behave
    /// differently towards one.
    #[inline]
    pub fn representation_of(&self, p: PartyId) -> Representation {
        self.representation[p.row()]
    }

    /// A cell's identity is a KEY on its kind's declared lattice, and the key is part of what
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

    /// A weight changes ONLY by one of the five events, and the event is named at the call.
    /// A caller with no event to name has no business changing a weight.
    pub fn reweigh(&mut self, p: PartyId, to: u32, by: WeightEvent) {
        assert!(
            self.representation[p.row()] == Representation::Cell,
            "XI-15: a named party's weight is one and does not change ({by:?})"
        );
        assert!(to > 0, "XI-15: a cell of nobody is not a cell — it dies ({by:?})");
        self.weight[p.row()] = to;
    }

    /// An event that applies to SOME members splits the cell. The affected members become
    /// a new cell with the same kind, the same region, the same bank and the same key — the same
    /// state, because a cell is homogeneous and identical members divide without remainder. What the
    /// two cells HOLD is settled over the wire by the caller, exactly: the kernel divides the count
    /// and settlement divides the holdings.
    ///
    /// Without this, a partial event has only two answers and both are wrong: move the whole cell,
    /// which quantises the world to the weight, or carry a headcount inside the cell and let its
    /// members differ, which is an average one level down.
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
        // A split is not a birth. The members were already here; the row they are counted
        // in is new and they are not, so the child is as old as the parent (§21 A4 reads age).
        self.since[child.row()] = self.since[p.row()];
        self.reweigh(p, had - taking, WeightEvent::Split);
        child
    }

    /// Nothing is immortal, and a death has a destination. What it held is the
    /// estate's; this only records that the party has ceased.
    pub fn cease(&mut self, p: PartyId) {
        self.alive[p.row()] = false;
    }

    /// What ONE MEMBER of a cell holds, as a READ over the total. There is no stored
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
    fn an_event_that_applies_to_some_members_splits_the_cell() {
        // The affected members become a new cell with the same kind, region, bank and key —
        // the same state — and the two counts add back to the one they came from. A population is a
        // read of the cells, so a split changes how many cells there are and not how many people.
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
        // Taking everybody leaves a cell of nobody, which is a DEATH and not a split. The
        // two are different events with different consequences, and a split that could mean either
        // is a weight change nobody can read.
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
