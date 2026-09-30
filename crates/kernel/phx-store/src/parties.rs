//! A kind's parties: the slots its table hands out, each with its party's permanent identity and the generation that
//! tells a live party's reference from a stale one once its slot holds another.

use phx_id::{PartyId, PartyRef, Slot};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::stats::StoreStats;
use crate::table::SlotAlloc;

/// The parties of one kind: slots allocated lowest first and released at the day's close, each slot's party's
/// identity, and each slot's generation, raised each time the slot takes a new party. Which event begins or ends a
/// party, and where an ended party's holdings go, is the world's; this is where the party is held.
#[clause("PTY.1", "REP.13")]
#[derive(Debug, phx_macros::Saved)]
pub struct Parties<B: Backing = SystemBacking> {
    kind: u8,
    slots: SlotAlloc<B>,
    generations: Column<u32, B>,
    ids: Column<PartyId, B>,
}

impl<B: Backing> Parties<B> {
    /// A kind's table of at most `max` parties, chunked by `rows_per_chunk`.
    pub fn new(space: &mut AddressSpace, kind: u8, max: u32, rows_per_chunk: u32) -> Parties<B> {
        if kind == phx_id::consts::NATURE_KIND {
            violation!(clause = "Law 5", "a table of nature's kind, which holds no party", kind = kind);
        }
        Parties {
            kind,
            slots: SlotAlloc::new(space, max),
            generations: Column::new(space, max, rows_per_chunk),
            ids: Column::new(space, max, rows_per_chunk),
        }
    }

    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// A new party of identity `id`: the lowest free slot, at its next generation.
    pub fn begin(&mut self, id: PartyId) -> PartyRef {
        let slot = self.slots.alloc();
        let generation = self.generations.get(slot).map_or(0, |g| g + 1);
        self.generations.put(slot, generation);
        self.ids.put(slot, id);
        PartyRef::new(self.kind, generation, slot)
    }

    /// The identity of the party at a slot, live or ended this day.
    #[must_use]
    pub fn id(&self, slot: Slot) -> Option<PartyId> {
        self.ids.get(slot)
    }

    /// Ends a live party; its slot is handed out again after the day closes.
    pub fn end(&mut self, party: PartyRef) {
        let Some(slot) = self.resolve(party) else {
            violation!(clause = "PTY.9", "a party ended that is not live", party = party.word());
        };
        self.slots.release(slot);
    }

    /// The slot of a live party, or none for a reference of another kind, an ended party or a slot since reused.
    #[must_use]
    pub fn resolve(&self, party: PartyRef) -> Option<Slot> {
        let slot = party.slot();
        (party.kind() == self.kind
            && self.slots.is_live(slot)
            && self.generations.get(slot) == Some(party.generation()))
        .then_some(slot)
    }

    /// The live party at a slot, or none.
    #[must_use]
    pub fn at(&self, slot: Slot) -> Option<PartyRef> {
        if !self.slots.is_live(slot) {
            return None;
        }
        self.generations.get(slot).map(|g| PartyRef::new(self.kind, g, slot))
    }

    /// Slots ever handed out: every live party's slot lies below, so a column of the kind is this long.
    #[must_use]
    pub fn high_water(&self) -> u32 {
        self.slots.high_water()
    }

    pub fn live_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.slots.live_slots()
    }

    /// Returns the day's ended slots to use.
    pub fn close_day(&mut self) {
        self.slots.close_day();
    }
}

impl<B: Backing> StoreStats for Parties<B> {
    fn rows_live(&self) -> u64 {
        self.slots.live_count()
    }

    fn rows_ever(&self) -> u64 {
        u64::from(self.slots.high_water())
    }

    fn bytes(&self) -> u64 {
        let bytes = self.slots.bytes_committed() + self.generations.bytes_committed() + self.ids.bytes_committed();
        u64::try_from(bytes).unwrap_or_else(|_| capacity_exceeded!("a store's bytes", u64::MAX, bytes))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyId, Slot};

    use super::Parties;
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::stats::StoreStats;

    #[test]
    fn slots_reuse_with_new_generation() {
        let mut space = AddressSpace::empty();
        let mut p: Parties<HeapBacking> = Parties::new(&mut space, 3, 16, 4);
        let a = p.begin(PartyId::new(7));
        let b = p.begin(PartyId::new(8));
        assert_eq!((a.kind(), a.slot(), b.slot()), (3, Slot::new(0), Slot::new(1)));
        p.end(a);
        assert_eq!(p.resolve(a), None, "an ended party no longer resolves");
        p.close_day();
        let c = p.begin(PartyId::new(9));
        assert_eq!(c.slot(), a.slot(), "the lowest free slot is reused after the close");
        assert_eq!(c.generation(), a.generation() + 1);
        assert_eq!(p.resolve(a), None, "a stale reference is refused once the slot holds another");
        assert_eq!(p.resolve(c), Some(a.slot()));
        assert_eq!(p.at(Slot::new(1)), Some(b));
        assert_eq!(p.id(c.slot()), Some(PartyId::new(9)), "the slot holds its new party's identity");
        p.end(b);
        assert_eq!((p.rows_live(), p.rows_ever()), (1, 2), "one live of the two slots ever handed out");
        assert!(p.bytes() > 0);
    }
}
