//! The party directory: every party kind's slots and their generations, and a tombstone for each party that ended
//! within the horizon. A party's lasting identity is its reference — kind, slot and generation — which resolves to
//! its live slot, or through its tombstone to the day it ended and its estate or successor, or, past the horizon, to
//! having ended before it.

use phx_id::{Day, PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, Generations, SlotAlloc, StoreStats, SystemBacking};

use crate::consts::TOMB_KEY_BITS;
use crate::tombs::{Tomb, Tombs};

/// The generations a reference gives a slot.
const GENERATION_BITS: u32 = phx_id::consts::GENERATION_BITS;
const KEY_MASK: u64 = (1 << TOMB_KEY_BITS) - 1;
/// A tombstone with no successor holds this in its successor's key.
const NONE: u64 = KEY_MASK;

/// The parties a kind's table of generations is kept for.
#[derive(Debug)]
pub struct Party;

/// What a reference names now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// The party lives, at its slot.
    Live(Slot),
    /// The party ended on a day, its estate or successor named where it has one.
    Ended { day: Day, successor: Missing<PartyRef> },
    /// The party ended before the horizon, its tombstone no longer kept.
    EndedBeyondHorizon,
}

/// A kind's slots, their generations and how many are live.
#[derive(Debug)]
pub(crate) struct KindTable<B: Backing> {
    pub(crate) slots: SlotAlloc<B>,
    pub(crate) generations: Generations<Party, B>,
    pub(crate) live: u64,
}

/// Every kind's table and the tombstones within the horizon.
#[clause("PTY.1", "PTY.9", "PTY.10", "PTY.13", "SET.13", "REP.13")]
#[derive(Debug)]
pub struct Directory<B: Backing = SystemBacking> {
    pub(crate) kinds: Vec<KindTable<B>>,
    pub(crate) tombs: Tombs,
    pub(crate) first: Day,
    pub(crate) horizon: u32,
}

impl<B: Backing> Directory<B> {
    /// No party yet, each kind with room for its declared slots, the run beginning on its first day and tombstones
    /// kept the horizon's days.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(
        space: &mut AddressSpace,
        capacities: &[u32],
        rows_per_chunk: u32,
        (first, horizon): (Day, u32),
    ) -> Directory<B> {
        if capacities.len() > usize::from(phx_id::consts::NATURE_KIND) {
            violation!(clause = "Law 5", "a table of nature's kind, which holds no party", kinds = capacities.len());
        }
        let kinds = capacities
            .iter()
            .map(|max| KindTable {
                slots: SlotAlloc::new(space, *max),
                generations: Generations::new(space, *max, rows_per_chunk, GENERATION_BITS),
                live: 0,
            })
            .collect();
        Directory { kinds, tombs: Tombs::default(), first, horizon }
    }

    fn table(&self, kind: u8) -> &KindTable<B> {
        match self.kinds.get(usize::from(kind)) {
            Some(t) => t,
            None => violation!(clause = "PTY.1", "a party of a kind the directory does not hold", kind = kind),
        }
    }

    fn table_mut(&mut self, kind: u8) -> &mut KindTable<B> {
        match self.kinds.get_mut(usize::from(kind)) {
            Some(t) => t,
            None => violation!(clause = "PTY.1", "a party of a kind the directory does not hold", kind = kind),
        }
    }

    /// A party begun: a slot of its kind, the one that has rested longest, at its next generation; never one released
    /// the same day.
    #[clause("PTY.1", "PTY.13", "SET.7")]
    pub fn begin(&mut self, kind: u8) -> PartyRef {
        let t = self.table_mut(kind);
        let r = t.generations.alloc(&mut t.slots);
        t.live += 1;
        PartyRef::new(kind, r.generation(), r.slot())
    }

    /// A party ended on a day, naming its estate or successor where it has one: it reads as ended at once, its
    /// tombstone joins the day's, and its slot rests until the day closes. A party not live stops the run.
    #[clause("PTY.9", "PTY.10", "SET.7")]
    pub fn end(&mut self, party: PartyRef, day: Day, successor: Missing<PartyRef>) {
        let offset = self.offset(day);
        let t = self.table_mut(party.kind());
        let slot = party.slot();
        if !t.slots.is_live(slot) || t.generations.get(slot) != Some(party.generation()) {
            violation!(clause = "PTY.9", "a party ended that is not live", party = party.word());
        }
        t.slots.release(slot);
        t.live -= 1;
        let [high, low] = offset.to_be_bytes();
        let next = match successor {
            Missing::Present(s) => s.packed(),
            Missing::Absent => NONE,
        };
        self.tombs.push(Tomb {
            ended: u64::from(high) << TOMB_KEY_BITS | party.packed(),
            successor: u64::from(low) << TOMB_KEY_BITS | next,
        });
    }

    /// The days from the run's first to a day, in the two bytes a tombstone holds; past them stops the run.
    fn offset(&self, day: Day) -> u16 {
        let Some(days) = day.since(self.first) else {
            violation!(clause = "TIME.10", "a party ended before the run began", day = day.get());
        };
        match u16::try_from(days) {
            Ok(d) => d,
            Err(_) => capacity_exceeded!("a run's days a tombstone counts", u16::MAX, days),
        }
    }

    /// The day's endings so far made searchable: called as a phase of endings ends, so the day's reads of the parties
    /// ended in it are a search, not a read through them.
    pub fn sort_endings(&mut self) {
        self.tombs.sort_today();
    }

    /// An ended party's successor named afterwards: an estate's own ending names its heir. A party with no tombstone
    /// within the horizon stops the run.
    #[clause("PTY.10")]
    pub fn set_successor(&mut self, ended: PartyRef, successor: PartyRef) {
        let Some(t) = self.tombs.find_mut(ended.packed()) else {
            violation!(clause = "PTY.10", "a successor named for a party with no tombstone", party = ended.word());
        };
        t.successor = (t.successor & !KEY_MASK) | successor.packed();
    }

    /// What a reference names now: its live slot while its generation is the slot's and the slot is live; else its
    /// tombstone's day and successor; else, its tombstone dropped, ended before the horizon. A generation above the
    /// slot's own was never issued and stops the run.
    #[clause("PTY.13", "SET.13")]
    #[must_use]
    pub fn resolve(&self, r: PartyRef) -> Resolved {
        let t = self.table(r.kind());
        let slot = r.slot();
        let generation = t.generations.get(slot);
        // The live case reads the slot's generation and its live bit, nothing else.
        if generation == Some(r.generation()) && phx_store::table::live_at(t.slots.live_words(), slot) {
            return Resolved::Live(slot);
        }
        self.resolve_ended(r, generation)
    }

    /// An ended reference's tombstone, or none past the horizon; a generation the slot has not reached stops the run.
    fn resolve_ended(&self, r: PartyRef, generation: Option<u32>) -> Resolved {
        if generation.is_none_or(|g| g < r.generation()) {
            violation!(clause = "PTY.13", "a reference to a generation never issued", party = r.word());
        }
        match self.tombs.find(r.packed()) {
            Some(tomb) => {
                let next = tomb.successor & KEY_MASK;
                let successor =
                    if next == NONE { Missing::Absent } else { Missing::Present(PartyRef::from_packed(next)) };
                Resolved::Ended { day: Day::unpacked(self.first, tomb.offset()), successor }
            }
            None => Resolved::EndedBeyondHorizon,
        }
    }

    /// A reference followed through its successors to a live party, or to the last that ended with none, or past the
    /// horizon: what its claims and contracts now face.
    #[clause("PTY.10")]
    pub fn follow(&self, r: PartyRef) -> (PartyRef, Resolved) {
        let mut at = r;
        // A successor begins after the party it succeeds, so a chain is never longer than the tombstones kept.
        for _ in 0..=self.tombstones() {
            match self.resolve(at) {
                Resolved::Ended { successor: Missing::Present(next), .. } => at = next,
                done => return (at, done),
            }
        }
        violation!(clause = "PTY.10", "a chain of successors that returns on itself", party = r.word())
    }

    /// The day's close: its endings sorted into the recent run, every kind's rested slots handed out again from the
    /// next day, and, once the recent run passes a sixteenth of the main one, one sequential merge that drops every
    /// tombstone ended before the horizon. The tombstones the merge read.
    #[clause("SET.13", "SET.7")]
    pub fn close_day(&mut self, today: Day) -> u64 {
        for t in &mut self.kinds {
            t.slots.close_day();
        }
        // Before the run has lived a horizon, nothing has ended past it.
        let (first, oldest) = (self.first, today.get().checked_sub(self.horizon));
        let read = self.tombs.close(|t| oldest.is_none_or(|h| Day::unpacked(first, t.offset()).get() >= h));
        match u64::try_from(read) {
            Ok(n) => n,
            Err(_) => capacity_exceeded!("tombstones", u64::MAX, read),
        }
    }

    /// A kind's live slots, in order.
    pub fn live_slots(&self, kind: u8) -> impl Iterator<Item = Slot> + '_ {
        self.table(kind).slots.live_slots()
    }

    /// A kind's slots ever handed out: every live party's slot lies below, so a column of the kind is this long.
    #[must_use]
    pub fn high_water(&self, kind: u8) -> u32 {
        self.table(kind).slots.high_water()
    }

    #[must_use]
    pub fn is_live(&self, kind: u8, slot: Slot) -> bool {
        self.table(kind).slots.is_live(slot)
    }

    /// The live party at a slot of a kind, or none.
    #[must_use]
    pub fn at(&self, kind: u8, slot: Slot) -> Option<PartyRef> {
        let t = self.table(kind);
        if !t.slots.is_live(slot) {
            return None;
        }
        t.generations.get(slot).map(|g| PartyRef::new(kind, g, slot))
    }

    /// The party a slot of a kind holds, live or ended this day: its slot is not handed out again before the day
    /// closes. None for a slot never handed out.
    #[must_use]
    pub fn reference(&self, kind: u8, slot: Slot) -> Option<PartyRef> {
        let t = self.table(kind);
        (slot.get() < t.slots.high_water())
            .then(|| t.generations.get(slot).map(|g| PartyRef::new(kind, g, slot)))
            .flatten()
    }

    /// The kinds the directory holds.
    #[must_use]
    pub fn kinds(&self) -> usize {
        self.kinds.len()
    }

    /// Each kind's number, in order.
    pub fn kind_numbers(&self) -> impl Iterator<Item = u8> + use<B> {
        let kinds = u8::try_from(self.kinds.len())
            .unwrap_or_else(|_| capacity_exceeded!("kinds of party", u8::MAX, self.kinds.len()));
        0..kinds
    }

    /// The parties of a kind live now.
    #[must_use]
    pub fn live(&self, kind: u8) -> u64 {
        self.table(kind).live
    }

    /// The tombstones kept.
    #[must_use]
    pub fn tombstones(&self) -> usize {
        self.tombs.len()
    }
}

impl<B: Backing> StoreStats for Directory<B> {
    fn rows_live(&self) -> u64 {
        self.kinds.iter().map(|t| t.live).sum()
    }

    fn rows_ever(&self) -> u64 {
        self.kinds.iter().map(|t| u64::from(t.slots.high_water())).sum()
    }

    fn bytes(&self) -> u64 {
        let tables: usize =
            self.kinds.iter().map(|t| t.slots.bytes_committed() + t.generations.bytes_committed()).sum();
        u64::try_from(tables + self.tombs.bytes()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
