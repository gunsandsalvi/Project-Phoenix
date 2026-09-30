//! Each household's persons on the core, each its word as the household kind packs it and its identity, a list per
//! household in its chunk's arena and found by the household's slot, so a person's event reads and writes its own
//! household alone.

use phx_core::capacity::PERSON_ARENA_WORDS;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::arena::{CellListRef, ChunkArena, ListRef};
use phx_store::backing::{AddressSpace, Backing};
use phx_store::region::Region;
use phx_store::{Column, SystemBacking};

/// A person as its household holds it: its word, as its kind packs it, and its identity, its own from its birth to its
/// death whichever household it lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub word: u64,
    pub id: u64,
}

/// The words a person takes in its household's list.
const WORDS: usize = 2;

/// The persons of a kind's parties: each party's list reference by slot, and its chunk's arena.
#[clause("REP.26", "REP.16")]
#[derive(Debug, phx_macros::Saved)]
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
            self.arenas.push(ChunkArena::new(space, PERSON_ARENA_WORDS));
        }
    }

    fn list(&self, slot: Slot) -> Option<(usize, ListRef)> {
        let r = self.lists.get(slot)?;
        let chunk = self.chunk(slot);
        Some((chunk, self.arenas.get(chunk)?.resolve(slot.get(), r)))
    }

    fn raw(&self, slot: Slot) -> &[u64] {
        match self.list(slot) {
            Some((chunk, list)) => self.arenas.get(chunk).map_or(&[], |a| a.read(list)),
            None => &[],
        }
    }

    /// A party's persons in their places, none for a party that holds none.
    pub fn of(&self, slot: Slot) -> impl ExactSizeIterator<Item = Held> + '_ {
        self.raw(slot).as_chunks::<WORDS>().0.iter().map(|[word, id]| Held { word: *word, id: *id })
    }

    /// The persons a party holds.
    #[must_use]
    pub fn count(&self, slot: Slot) -> usize {
        self.raw(slot).len() / WORDS
    }

    /// A person's place in its household, by its identity; none where the household does not hold it.
    #[must_use]
    pub fn place_of(&self, slot: Slot, id: u64) -> Option<usize> {
        self.of(slot).position(|p| p.id == id)
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
    pub fn set(&mut self, space: &mut AddressSpace, slot: Slot, persons: &[Held]) {
        let before = count(self.count(slot));
        let words: Vec<u64> = persons.iter().flat_map(|p| [p.word, p.id]).collect();
        self.edit(space, slot, |arena, list| {
            arena.clear(list);
            arena.append(list, &words);
        });
        self.held = self.held - before + count(persons.len());
    }

    /// A person joins a party: born into it, or moving in.
    pub fn push(&mut self, space: &mut AddressSpace, slot: Slot, person: Held) {
        self.edit(space, slot, |arena, list| arena.append(list, &[person.word, person.id]));
        self.held += 1;
    }

    /// The person at `at` leaves its party: it died or moved out; those after it move up one place.
    pub fn remove(&mut self, space: &mut AddressSpace, slot: Slot, at: usize) {
        if at >= self.count(slot) {
            violation!(clause = "REP.26", "a person removed that its household does not hold", at = at);
        }
        let first =
            u32::try_from(at * WORDS).unwrap_or_else(|_| capacity_exceeded!("a household's persons", u32::MAX, at));
        let words = u32::try_from(WORDS).unwrap_or_else(|_| capacity_exceeded!("a person's words", u32::MAX, WORDS));
        self.edit(space, slot, |arena, list| arena.remove(list, first, words));
        self.held -= 1;
    }

    /// The word of the person at `at` written anew, its identity kept: a change of its attributes.
    pub fn set_word(&mut self, space: &mut AddressSpace, slot: Slot, at: usize, word: u64) {
        if at >= self.count(slot) {
            violation!(clause = "REP.26", "a person rewritten that its household does not hold", at = at);
        }
        self.edit(space, slot, |arena, list| {
            if let Some(w) = arena.read_mut(*list).get_mut(at * WORDS) {
                *w = word;
            }
        });
    }

    /// A party's persons let go, as it ends.
    pub fn clear(&mut self, space: &mut AddressSpace, slot: Slot) {
        let before = count(self.count(slot));
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
        let mut scratch: Region<u64, B> = Region::reserve(space, index(PERSON_ARENA_WORDS));
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
