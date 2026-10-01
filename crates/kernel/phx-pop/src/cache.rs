//! The per-day cache: a value that is a pure function of a party's state and the day's prices, read several times a
//! day, computed once a day on its first read and kept in the party's own row beside the day it was computed for. A
//! reader holding the stores shared records what it computed in its chunk's write-back buffer, which the chunk owning
//! the slots applies at its sub-stage's barrier; a reader holding the row mutably writes it in place. A cached value
//! never changes an outcome: it equals the function computed again.

use phx_id::{Day, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{Backing, DayBuf};

use crate::consts::CACHE_VALUE_BYTES as VALUE_BYTES;
use crate::directory::Directory;
use crate::kinds::{AttrW, KindChunk, KindStore, Row, Word};

/// A value computed on a miss, for the chunk owning its party's slot to write at the barrier: 12 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct CacheWrite {
    slot: u32,
    value: [u8; VALUE_BYTES],
}

/// A per-day cache over a kind's rows: the word its value is kept in and the word of the day it was computed for, an
/// offset from the run's first day, both in one group so one gathered row holds them; a stamp of no day reads as
/// never computed.
#[clause("REP.5")]
#[derive(Debug)]
pub struct DayCached<T> {
    value: AttrW<T>,
    stamp: AttrW<u16>,
    first: Day,
}

impl<T> Clone for DayCached<T> {
    fn clone(&self) -> DayCached<T> {
        *self
    }
}

impl<T> Copy for DayCached<T> {}

impl<T: Word> DayCached<T> {
    /// A cache kept in a value word and a stamp word of one group, its stamps counted from the run's first day. Words
    /// of two groups, or a value wider than a write-back carries, stop the run.
    #[must_use]
    pub fn new(value: AttrW<T>, stamp: AttrW<u16>, first: Day) -> DayCached<T> {
        if value.read().place().0 != stamp.read().place().0 {
            violation!(clause = "REP.1", "a cache's value and stamp in two groups");
        }
        if usize::from(T::TY.bytes()) > VALUE_BYTES {
            violation!(clause = "REP.1", "a cached value wider than a write-back carries");
        }
        DayCached { value, stamp, first }
    }

    /// A day's stamp: its offset from the run's first day, the stamp of no day never being one.
    fn stamp_of(&self, today: Day) -> u16 {
        match today.since(self.first).and_then(|d| u16::try_from(d).ok()).filter(|d| *d != u16::MAX) {
            Some(d) => d,
            None => capacity_exceeded!("a cache's days", u16::MAX, today.get()),
        }
    }

    /// The value a row holds for a day; none where it was computed for another day, or never.
    pub fn get(&self, row: &Row<'_>, today: Day) -> Missing<T> {
        match row.get(self.stamp.read()) {
            Missing::Present(s) if s == self.stamp_of(today) => row.get(self.value.read()),
            _ => Missing::Absent,
        }
    }

    /// The value for a day of the party whose row is gathered: the row's where it was computed today, else computed
    /// by `f` on `arg` and recorded in the chunk's write-back buffer for its slot.
    pub fn get_or<A, B: Backing>(
        &self,
        (row, slot): (&Row<'_>, Slot),
        today: Day,
        back: &mut DayBuf<CacheWrite, B>,
        (f, arg): (impl FnOnce(A) -> T, A),
    ) -> T {
        if let Missing::Present(v) = self.get(row, today) {
            return v;
        }
        let v = f(arg);
        let mut value = [0; VALUE_BYTES];
        match value.get_mut(..usize::from(T::TY.bytes())) {
            Some(bytes) => v.write(bytes),
            None => violation!(clause = "REP.1", "a cached value wider than a write-back carries"),
        }
        back.push(CacheWrite { slot: slot.get(), value });
        v
    }

    /// The value for a day of the party at a slot, its row written in place on a miss, by a reader holding the store.
    pub fn get_or_mut<A, B: Backing>(
        &self,
        store: &mut KindStore<B>,
        (slot, today): (Slot, Day),
        (f, arg): (impl FnOnce(A) -> T, A),
    ) -> T {
        let group = self.stamp.read().place().0;
        let Some(row) = store.gather_at(slot, group) else {
            violation!(clause = "REP.1", "a cache read for a slot never begun", slot = slot.get());
        };
        if let Missing::Present(v) = self.get(&row, today) {
            return v;
        }
        let v = f(arg);
        store.set_at(slot, self.value, Missing::Present(v));
        store.set_at(slot, self.stamp, Missing::Present(self.stamp_of(today)));
        v
    }

    /// The day's write-backs of the slots a chunk owns written into its rows; those of other chunks' slots are theirs.
    /// Two of one slot carry one value, the function's on the same state.
    pub fn apply<D: Backing>(&self, chunk: &mut KindChunk<'_, D>, today: Day, writes: &[CacheWrite]) {
        let (first, n) = chunk.slots();
        let stamp = self.stamp_of(today);
        for w in writes.iter().filter(|w| w.slot.checked_sub(first.get()).is_some_and(|at| at < n)) {
            let Some(v) = w.value.get(..usize::from(T::TY.bytes())).and_then(T::read) else {
                violation!(clause = "REP.1", "a write-back's value past its bytes", slot = w.slot);
            };
            let slot = Slot::new(w.slot);
            chunk.set_at(slot, self.value, Missing::Present(v));
            chunk.set_at(slot, self.stamp, Missing::Present(stamp));
        }
    }

    /// The day's write-backs written by a reader holding the whole store, as a chunk of every slot would.
    pub fn apply_all<B: Backing, D: Backing>(
        &self,
        store: &mut KindStore<B>,
        dir: &Directory<D>,
        today: Day,
        writes: &[CacheWrite],
    ) {
        let slots = store.rows();
        if slots == 0 {
            return;
        }
        for mut chunk in store.chunks_mut(dir, slots) {
            self.apply(&mut chunk, today, writes);
        }
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
