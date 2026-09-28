//! Each household's persons on the core, one word each as the household kind packs them, a list per household in its
//! chunk's arena and found by the household's slot, so a person's event reads and writes its own household alone.

use phx_id::Slot;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::arena::{CellListRef, ChunkArena, ListRef};
use phx_store::backing::{AddressSpace, Backing};
use phx_store::consts::ARENA_RESERVED_WORDS;
use phx_store::region::Region;
use phx_store::{Column, SystemBacking};

/// The persons of a kind's parties: each party's list reference by slot, and its chunk's arena.
#[clause("REP.26")]
#[derive(Debug)]
pub struct Persons<B: Backing = SystemBacking> {
    lists: Column<CellListRef, B>,
    arenas: Vec<ChunkArena<B>>,
    rows_per_chunk: u32,
    held: u64,
}

fn index(n: u32) -> usize {
    usize::try_from(n).unwrap_or_else(|_| capacity_exceeded!("index width", usize::MAX, n))
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or_else(|_| capacity_exceeded!("persons held", u64::MAX, n))
}

impl<B: Backing> Persons<B> {
    /// No persons yet, for up to `capacity` parties in chunks of `rows_per_chunk`.
    #[must_use]
    pub fn new(space: &mut AddressSpace, capacity: u32, rows_per_chunk: u32) -> Persons<B> {
        Persons { lists: Column::new(space, capacity, rows_per_chunk), arenas: Vec::new(), rows_per_chunk, held: 0 }
    }

    fn chunk(&self, slot: Slot) -> usize {
        index(slot.get() / self.rows_per_chunk)
    }

    /// The party's list and chunk made ready: a party first seen takes an empty list, and a chunk first reached its
    /// arena.
    fn ready(&mut self, space: &mut AddressSpace, slot: Slot) {
        while self.lists.len() <= index(slot.get()) {
            self.lists.push(CellListRef::EMPTY);
        }
        let chunk = self.chunk(slot);
        while self.arenas.len() <= chunk {
            self.arenas.push(ChunkArena::new(space, ARENA_RESERVED_WORDS));
        }
    }

    fn list(&self, slot: Slot) -> Option<(usize, ListRef)> {
        let r = self.lists.get(slot)?;
        let chunk = self.chunk(slot);
        Some((chunk, self.arenas.get(chunk)?.resolve(slot.get(), r)))
    }

    /// A party's persons, none for a party that holds none.
    #[must_use]
    pub fn of(&self, slot: Slot) -> &[u64] {
        match self.list(slot) {
            Some((chunk, list)) => self.arenas.get(chunk).map_or(&[], |a| a.read(list)),
            None => &[],
        }
    }

    /// Edits a party's list in its arena and writes its reference back.
    fn edit(&mut self, space: &mut AddressSpace, slot: Slot, f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef)) {
        self.ready(space, slot);
        let Some((chunk, mut list)) = self.list(slot) else {
            violation!(clause = "REP.26", "a household's persons with no list", slot = slot.get());
        };
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "REP.26", "a household's persons with no arena", slot = slot.get());
        };
        f(arena, &mut list);
        let mut r = self.lists.get(slot).unwrap_or(CellListRef::EMPTY);
        arena.store(slot.get(), &mut r, list);
        self.lists.set(slot, r);
    }

    /// A party's persons written anew: its household formed, or begun again in a released slot.
    pub fn set(&mut self, space: &mut AddressSpace, slot: Slot, persons: &[u64]) {
        let before = count(self.of(slot).len());
        self.edit(space, slot, |arena, list| {
            arena.clear(list);
            arena.append(list, persons);
        });
        self.held = self.held - before + count(persons.len());
    }

    /// A person joins a party: born into it, or moving in.
    pub fn push(&mut self, space: &mut AddressSpace, slot: Slot, person: u64) {
        self.edit(space, slot, |arena, list| arena.append(list, &[person]));
        self.held += 1;
    }

    /// The person at `at` leaves its party: it died or moved out; those after it move up one place.
    pub fn remove(&mut self, space: &mut AddressSpace, slot: Slot, at: usize) {
        if at >= self.of(slot).len() {
            violation!(clause = "REP.26", "a person removed that its household does not hold", at = at);
        }
        let place = u32::try_from(at).unwrap_or_else(|_| capacity_exceeded!("a household's persons", u32::MAX, at));
        self.edit(space, slot, |arena, list| arena.remove(list, place, 1));
        self.held -= 1;
    }

    /// A party's persons let go, as it ends.
    pub fn clear(&mut self, space: &mut AddressSpace, slot: Slot) {
        let before = count(self.of(slot).len());
        self.edit(space, slot, ChunkArena::clear);
        self.held -= before;
    }

    /// The persons every party holds together.
    #[must_use]
    pub fn held(&self) -> u64 {
        self.held
    }

    /// Closes the gaps of every chunk whose arena's dead words have passed its declared share, each party's list
    /// moved in slot order; returns the chunks compacted.
    pub fn compact_due(&mut self, space: &mut AddressSpace) -> u64 {
        let mut done = 0;
        let per = index(self.rows_per_chunk);
        let mut scratch: Region<u64, B> = Region::reserve(space, index(ARENA_RESERVED_WORDS));
        for chunk in 0..self.arenas.len() {
            if !self.arenas.get(chunk).is_some_and(ChunkArena::needs_compaction) {
                continue;
            }
            let slots: Vec<Slot> = (chunk * per..(chunk + 1) * per)
                .take_while(|s| *s < self.lists.len())
                .filter_map(|s| u32::try_from(s).ok().map(Slot::new))
                .collect();
            let mut refs: Vec<ListRef> = slots.iter().filter_map(|s| self.list(*s).map(|(_, l)| l)).collect();
            let Some(arena) = self.arenas.get_mut(chunk) else { continue };
            arena.compact(refs.as_mut_slice(), &mut scratch);
            for (s, full) in slots.iter().zip(refs) {
                let mut r = self.lists.get(*s).unwrap_or(CellListRef::EMPTY);
                arena.store(s.get(), &mut r, full);
                self.lists.set(*s, r);
            }
            done += 1;
        }
        done
    }
}

#[path = "persons_tests.rs"]
mod tests;
