use phx_core::map::KernelMap;
use phx_id::{PartyId, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_store::Backing;

use crate::consts::INDEX_INLINE;
use crate::landing::LandingIndex;
use crate::table::CellTable;

/// The first cells holding one landing key, in order of their permanent identities, kept inline; a key held by more
/// keeps the rest in the index's spill map, so the record carries no pointer and most keys, held by one cell, cost no
/// allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Candidates {
    parties: [u64; INDEX_INLINE],
    slots: [u32; INDEX_INLINE],
    len: u8,
}

impl Candidates {
    fn of(all: &[(PartyId, Slot)]) -> Candidates {
        let mut c = Candidates::default();
        for ((p, s), (party, slot)) in c.parties.iter_mut().zip(c.slots.iter_mut()).zip(all) {
            *p = party.get();
            *s = slot.get();
            c.len += 1;
        }
        c
    }

    fn inline(&self) -> impl Iterator<Item = (PartyId, Slot)> + '_ {
        self.parties.iter().zip(&self.slots).take(usize::from(self.len)).map(|(p, s)| (PartyId::new(*p), Slot::new(*s)))
    }
}

/// A change to the index that 10b's keyed reduction applies: a cell entering a landing key, leaving it, or moving to
/// another slot under it, as renumbering does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexChange {
    Enter(Slot),
    Leave,
    Move(Slot),
}

/// The landing index: from a landing key to the cells holding it, in order of their permanent identities, never
/// their slots, so renumbering cannot move where a part lands. Sharded as a keyed reduction shards, it is rebuilt from
/// the cells at load and never saved.
#[clause("REP.8", "REP.28")]
#[derive(Debug)]
pub struct Index {
    keys: KernelMap<u64, Candidates>,
    spilled: KernelMap<u64, Vec<(PartyId, Slot)>>,
    cells: usize,
}

impl Default for Index {
    fn default() -> Index {
        Index::new()
    }
}

impl Index {
    #[must_use]
    pub fn new() -> Index {
        Index { keys: KernelMap::new(), spilled: KernelMap::new(), cells: 0 }
    }

    /// The index of a table's cells as they stand: every live cell but the individuals, under its landing key.
    #[must_use]
    pub fn rebuild<B: Backing>(table: &CellTable<B>) -> Index {
        let mut index = Index::new();
        let mut changes: Vec<(u64, PartyId, IndexChange)> = table
            .slots()
            .filter(|s| !table.hot(*s).is_individual())
            .map(|s| (table.hot(s).landing_key, table.party(s), IndexChange::Enter(s)))
            .collect();
        changes.sort_unstable_by_key(|(k, p, _)| (*k, *p));
        index.apply(changes);
        index
    }

    /// Changes applied as 10b's keyed reduction applies them: grouped by landing key, each key's in order of
    /// identity, so no outcome depends on the order they came in. One cell changes at most once per key.
    #[clause("REP.8")]
    pub fn apply(&mut self, mut changes: Vec<(u64, PartyId, IndexChange)>) {
        changes.sort_unstable_by_key(|(k, p, _)| (*k, *p));
        if changes.windows(2).any(|w| matches!(w, [a, b] if (a.0, a.1) == (b.0, b.1))) {
            violation!(clause = "REP.8", "a cell changed twice under one landing key in one reduction");
        }
        for group in changes.chunk_by(|a, b| a.0 == b.0) {
            let Some((landing, _, _)) = group.first().copied() else { continue };
            let mut all = self.all(landing);
            for (_, party, change) in group {
                let at = all.binary_search_by_key(party, |(p, _)| *p);
                match (change, at) {
                    (IndexChange::Enter(slot), Err(i)) => {
                        all.insert(i, (*party, *slot));
                        self.cells += 1;
                    }
                    (IndexChange::Leave, Ok(i)) => {
                        let _ = all.remove(i);
                        self.cells -= 1;
                    }
                    (IndexChange::Move(slot), Ok(i)) => {
                        if let Some(entry) = all.get_mut(i) {
                            entry.1 = *slot;
                        }
                    }
                    (IndexChange::Enter(_), Ok(_)) => {
                        violation!(clause = "REP.8", "a cell entered a landing key it holds", party = party.get());
                    }
                    (IndexChange::Leave | IndexChange::Move(_), Err(_)) => {
                        violation!(clause = "REP.8", "a cell left a landing key it does not hold", party = party.get());
                    }
                }
            }
            if all.is_empty() {
                let _ = self.keys.remove(landing);
            } else {
                let _ = self.keys.insert(landing, Candidates::of(&all));
            }
            match all.get(INDEX_INLINE..) {
                Some(rest) if !rest.is_empty() => {
                    let _ = self.spilled.insert(landing, rest.to_vec());
                }
                _ => {
                    let _ = self.spilled.remove(landing);
                }
            }
        }
    }

    /// Every cell holding a landing key, in order of identity.
    fn all(&self, landing: u64) -> Vec<(PartyId, Slot)> {
        let Some(c) = self.keys.get(landing) else { return Vec::new() };
        let mut out: Vec<(PartyId, Slot)> = c.inline().collect();
        if let Some(rest) = self.spilled.get(landing) {
            out.extend(rest.iter().copied());
        }
        out
    }

    /// Cells indexed.
    #[must_use]
    pub fn cells(&self) -> usize {
        self.cells
    }

    /// Landing keys held.
    #[must_use]
    pub fn keys(&self) -> usize {
        self.keys.len()
    }

    /// Every landing key with its cells, in key order, to compare an index with a rebuilt one.
    #[must_use]
    pub fn entries(&self) -> Vec<(u64, Vec<(PartyId, Slot)>)> {
        self.keys.sorted().into_iter().map(|(k, _)| (k, self.all(k))).collect()
    }

    /// The bytes the index's entries take: each key with its inline record, and each spilled key with its cells.
    #[must_use]
    pub fn bytes(&self) -> usize {
        let spilled: usize = self.spilled.sorted().iter().map(|(_, rest)| rest.len()).sum();
        self.keys.len() * (size_of::<u64>() + size_of::<Candidates>())
            + self.spilled.len() * (size_of::<u64>() + size_of::<Vec<(PartyId, Slot)>>())
            + spilled * size_of::<(PartyId, Slot)>()
    }
}

impl LandingIndex for Index {
    fn candidates(&self, landing: u64) -> Vec<(PartyId, Slot)> {
        self.all(landing)
    }

    fn insert(&mut self, landing: u64, party: PartyId, slot: Slot) {
        self.apply(vec![(landing, party, IndexChange::Enter(slot))]);
    }

    fn remove(&mut self, landing: u64, party: PartyId) {
        self.apply(vec![(landing, party, IndexChange::Leave)]);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyId, Slot};

    use super::{Index, IndexChange};
    use crate::landing::LandingIndex;

    fn cell(p: u64, s: u32) -> (PartyId, Slot) {
        (PartyId::new(p), Slot::new(s))
    }

    #[test]
    fn index_insert_remove_lookup() {
        let mut ix = Index::new();
        for (p, s) in [(9, 1), (3, 2), (7, 3), (1, 4), (5, 5), (8, 6)] {
            ix.insert(42, PartyId::new(p), Slot::new(s));
        }
        ix.insert(17, PartyId::new(2), Slot::new(7));
        let want = [cell(1, 4), cell(3, 2), cell(5, 5), cell(7, 3), cell(8, 6), cell(9, 1)];
        assert_eq!(ix.candidates(42), want, "in order of identity, past the inline four");
        assert_eq!((ix.cells(), ix.keys()), (7, 2));
        ix.remove(42, PartyId::new(3));
        ix.apply(vec![
            (42, PartyId::new(9), IndexChange::Move(Slot::new(11))),
            (17, PartyId::new(2), IndexChange::Leave),
        ]);
        assert_eq!(ix.candidates(42), [cell(1, 4), cell(5, 5), cell(7, 3), cell(8, 6), cell(9, 11)]);
        assert!(ix.candidates(17).is_empty(), "a key no cell holds any more is gone");
        assert_eq!((ix.cells(), ix.keys()), (5, 1));
        assert!(ix.candidates(99).is_empty());
    }

    #[test]
    fn index_changes_in_any_order_give_one_index() {
        let changes: Vec<(u64, PartyId, IndexChange)> = (0..40_u64)
            .map(|i| (i % 7, PartyId::new(i * 13 % 41 + 1), IndexChange::Enter(Slot::new(u32::try_from(i).unwrap()))))
            .collect();
        let mut forward = Index::new();
        forward.apply(changes.clone());
        let mut backward = Index::new();
        backward.apply(changes.into_iter().rev().collect());
        assert_eq!(forward.entries(), backward.entries());
    }

    #[test]
    fn index_refuses_what_it_does_not_hold() {
        let caught = std::panic::catch_unwind(|| {
            let mut ix = Index::new();
            ix.remove(5, PartyId::new(1));
        });
        assert!(caught.is_err(), "a cell leaving a key it does not hold stops the run");
        let caught = std::panic::catch_unwind(|| {
            let mut ix = Index::new();
            ix.insert(5, PartyId::new(1), Slot::new(0));
            ix.insert(5, PartyId::new(1), Slot::new(1));
        });
        assert!(caught.is_err(), "a cell entering a key twice stops the run");
    }
}
