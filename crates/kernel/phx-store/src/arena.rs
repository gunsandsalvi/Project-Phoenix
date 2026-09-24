use phx_macros::Pod;
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::consts::{
    ARENA_DEAD_DEN, ARENA_DEAD_NUM, ARENA_GROWTH_DEN, ARENA_GROWTH_NUM, ARENA_ROUND_WORDS, CELL_LIST_SENTINEL,
};
use crate::convert::{to_u32, to_usize};
use crate::region::Region;

/// A list in a chunk's arena, in words: where it starts, how many words it holds, how many it may hold in place.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct ListRef {
    pub off: u32,
    pub len: u32,
    pub cap: u32,
}

impl ListRef {
    pub const EMPTY: ListRef = ListRef { off: 0, len: 0, cap: 0 };
}

/// A cell table's compact list reference; a list longer than 16 bits can count keeps its full reference in the
/// arena's overflow map, marked by the sentinel length.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct CellListRef {
    pub off: u32,
    pub len: u16,
    pub cap: u16,
}

impl CellListRef {
    pub const EMPTY: CellListRef = CellListRef { off: 0, len: 0, cap: 0 };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct Overflow {
    owner: u32,
    list: ListRef,
}

/// The variable-length lists of one chunk's rows, as 8-byte words: a list grows in place while it has room, moves
/// to the end when it does not, and compaction closes the gaps in slot order.
#[derive(Debug)]
pub struct ChunkArena<B: Backing = SystemBacking> {
    words: Region<u64, B>,
    end: u32,
    dead: u32,
    compactions: u64,
    overflow: Region<Overflow, B>,
    n_overflow: usize,
}

/// The lists that live in one arena, in the slot order of the rows that own them.
pub trait ArenaLists {
    fn count(&self) -> usize;
    fn list<B: Backing>(&self, arena: &ChunkArena<B>, i: usize) -> ListRef;
    fn relocate<B: Backing>(&mut self, arena: &mut ChunkArena<B>, i: usize, off: u32);
}

impl ArenaLists for [ListRef] {
    fn count(&self) -> usize {
        self.len()
    }

    fn list<B: Backing>(&self, _: &ChunkArena<B>, i: usize) -> ListRef {
        let Some(list) = self.get(i).copied() else {
            violation!(clause = "SET.12", "a list past the lists given", i = i);
        };
        list
    }

    fn relocate<B: Backing>(&mut self, _: &mut ChunkArena<B>, i: usize, off: u32) {
        if let Some(r) = self.get_mut(i) {
            r.off = off;
        }
    }
}

/// A cell table's list references for one chunk: the reference at position `i` belongs to owner `first_owner + i`.
#[derive(Debug)]
pub struct CellLists<'a> {
    pub first_owner: u32,
    pub refs: &'a mut [CellListRef],
}

impl CellLists<'_> {
    fn owner(&self, i: usize) -> u32 {
        let Some(owner) = self.first_owner.checked_add(to_u32(i)) else {
            capacity_exceeded!("list owners", u32::MAX, u64::from(self.first_owner) + 1);
        };
        owner
    }
}

impl ArenaLists for CellLists<'_> {
    fn count(&self) -> usize {
        self.refs.len()
    }

    fn list<B: Backing>(&self, arena: &ChunkArena<B>, i: usize) -> ListRef {
        let Some(r) = self.refs.get(i).copied() else {
            violation!(clause = "SET.12", "a list past the lists given", i = i);
        };
        arena.resolve(self.owner(i), r)
    }

    fn relocate<B: Backing>(&mut self, arena: &mut ChunkArena<B>, i: usize, off: u32) {
        let owner = self.owner(i);
        if let Some(r) = self.refs.get_mut(i) {
            let list = ListRef { off, ..arena.resolve(owner, *r) };
            arena.store(owner, r, list);
        }
    }
}

fn span(list: ListRef, len: u32) -> std::ops::Range<usize> {
    to_usize(list.off)..to_usize(list.off) + to_usize(len)
}

/// Words a list needing `needed` words is given when it moves: 5/4 of them, rounded up to whole records.
fn grown(needed: u32) -> u32 {
    let wanted = (u64::from(needed) * ARENA_GROWTH_NUM).div_ceil(ARENA_GROWTH_DEN);
    let round = u64::from(ARENA_ROUND_WORDS);
    let Ok(cap) = u32::try_from(wanted.div_ceil(round) * round) else {
        capacity_exceeded!("list words", u32::MAX, wanted);
    };
    cap
}

impl<B: Backing> ChunkArena<B> {
    pub fn new(space: &mut AddressSpace, reserved_words: u32) -> ChunkArena<B> {
        ChunkArena {
            words: Region::reserve(space, to_usize(reserved_words)),
            end: 0,
            dead: 0,
            compactions: 0,
            overflow: Region::reserve(space, to_usize(reserved_words / ARENA_ROUND_WORDS)),
            n_overflow: 0,
        }
    }

    /// Words in use, dead ones included.
    #[must_use]
    pub fn used_words(&self) -> u32 {
        self.end
    }

    #[must_use]
    pub fn dead_words(&self) -> u32 {
        self.dead
    }

    #[must_use]
    pub fn compactions(&self) -> u64 {
        self.compactions
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.words.bytes_committed() + self.overflow.bytes_committed()
    }

    fn check(&self, list: ListRef) {
        let fits = list.len <= list.cap && list.off.checked_add(list.cap).is_some_and(|e| e <= self.end);
        if !fits {
            violation!(clause = "SET.12", "a list reference outside its arena", off = list.off, cap = list.cap);
        }
    }

    #[must_use]
    pub fn read(&self, list: ListRef) -> &[u64] {
        self.check(list);
        let Some(words) = self.words.slice(to_usize(self.end)).get(span(list, list.len)) else {
            violation!(clause = "SET.12", "a list reference outside its arena", off = list.off, len = list.len);
        };
        words
    }

    pub fn read_mut(&mut self, list: ListRef) -> &mut [u64] {
        self.check(list);
        let Some(words) = self.words.slice_mut(to_usize(self.end)).get_mut(span(list, list.len)) else {
            violation!(clause = "SET.12", "a list reference outside its arena", off = list.off, len = list.len);
        };
        words
    }

    /// Takes `cap` words at the arena's end.
    fn take(&mut self, cap: u32) -> u32 {
        let off = self.end;
        let Some(end) = self.end.checked_add(cap) else {
            capacity_exceeded!("arena words", u32::MAX, u64::from(self.end) + u64::from(cap));
        };
        self.words.ensure(to_usize(end));
        self.end = end;
        off
    }

    /// Adds words at a list's end: in place while the list has room, else at the arena's end with room to grow.
    pub fn append(&mut self, list: &mut ListRef, words: &[u64]) {
        self.check(*list);
        let Some(needed) = list.len.checked_add(to_u32(words.len())) else {
            capacity_exceeded!("list words", u32::MAX, u64::from(list.len) + u64::from(to_u32(words.len())));
        };
        if needed > list.cap {
            let cap = grown(needed);
            if list.off + list.cap == self.end {
                // The last list grows where it lies.
                self.take(cap - list.cap);
            } else {
                let off = self.take(cap);
                let all = self.words.slice_mut(to_usize(self.end));
                all.copy_within(span(*list, list.len), to_usize(off));
                self.dead += list.cap;
                list.off = off;
            }
            list.cap = cap;
        }
        let at = to_usize(list.off + list.len);
        if let Some(dst) = self.words.slice_mut(to_usize(self.end)).get_mut(at..at + words.len()) {
            dst.copy_from_slice(words);
        }
        list.len = needed;
    }

    /// Removes `n` words at `at`, shifting the rest down, so a sorted list stays sorted.
    pub fn remove(&mut self, list: &mut ListRef, at: u32, n: u32) {
        self.check(*list);
        let Some(end) = at.checked_add(n).filter(|e| *e <= list.len) else {
            violation!(clause = "SET.12", "a removal past a list's end", at = at, n = n, len = list.len);
        };
        let tail = span(*list, list.len);
        self.words
            .slice_mut(to_usize(self.end))
            .copy_within(tail.start + to_usize(end)..tail.end, tail.start + to_usize(at));
        list.len -= n;
    }

    /// Frees a list whose row has ended; its words are dead until the next compaction.
    pub fn clear(&mut self, list: &mut ListRef) {
        self.check(*list);
        self.dead += list.cap;
        *list = ListRef::EMPTY;
    }

    #[must_use]
    pub fn needs_compaction(&self) -> bool {
        u64::from(self.dead) * ARENA_DEAD_DEN > u64::from(self.end) * ARENA_DEAD_NUM
    }

    /// Copies every live list, in the order given, into the scratch and back to the arena's start, rewrites each
    /// reference, and returns the pages above the new end.
    pub fn compact<L: ArenaLists + ?Sized>(&mut self, lists: &mut L, scratch: &mut Region<u64, B>) {
        let mut at = 0_u32;
        for i in 0..lists.count() {
            let list = lists.list(self, i);
            scratch.ensure(to_usize(at + list.cap));
            let src = self.read(list);
            if let Some(dst) = scratch.slice_mut(to_usize(at + list.len)).get_mut(to_usize(at)..) {
                dst.copy_from_slice(src);
            }
            lists.relocate(self, i, at);
            at += list.cap;
        }
        if at != self.end - self.dead {
            violation!(
                clause = "SET.12",
                "a compaction that does not account for every live word",
                live = self.end - self.dead,
                copied = at
            );
        }
        let live = to_usize(at);
        self.words.slice_mut(live).copy_from_slice(scratch.slice(live));
        self.end = at;
        self.dead = 0;
        self.compactions += 1;
        self.words.release_above(live);
        scratch.release_above(0);
    }

    fn overflow_find(&self, owner: u32) -> Result<usize, usize> {
        self.overflow.slice(self.n_overflow).binary_search_by_key(&owner, |o| o.owner)
    }

    /// A cell list's full reference.
    pub fn resolve(&self, owner: u32, r: CellListRef) -> ListRef {
        if r.len != CELL_LIST_SENTINEL {
            return ListRef { off: r.off, len: u32::from(r.len), cap: u32::from(r.cap) };
        }
        let found = self.overflow_find(owner).ok().and_then(|i| self.overflow.slice(self.n_overflow).get(i));
        let Some(entry) = found else {
            violation!(clause = "SET.12", "a long cell list with no overflow entry", owner = owner);
        };
        entry.list
    }

    /// Writes a cell list's reference back: inline while it fits 16 bits, in the overflow map when it does not.
    pub fn store(&mut self, owner: u32, r: &mut CellListRef, list: ListRef) {
        let inline = (u16::try_from(list.len).ok().filter(|l| *l != CELL_LIST_SENTINEL), u16::try_from(list.cap).ok());
        match (inline, self.overflow_find(owner)) {
            ((Some(len), Some(cap)), found) => {
                if let Ok(i) = found {
                    let n = self.n_overflow;
                    self.overflow.slice_mut(n).copy_within(i + 1..n, i);
                    self.n_overflow = n - 1;
                }
                *r = CellListRef { off: list.off, len, cap };
            }
            (_, Ok(i)) => {
                if let Some(entry) = self.overflow.slice_mut(self.n_overflow).get_mut(i) {
                    entry.list = list;
                }
                *r = CellListRef { off: list.off, len: CELL_LIST_SENTINEL, cap: 0 };
            }
            (_, Err(i)) => {
                let n = self.n_overflow + 1;
                self.overflow.ensure(n);
                let entries = self.overflow.slice_mut(n);
                entries.copy_within(i..n - 1, i + 1);
                if let Some(entry) = entries.get_mut(i) {
                    *entry = Overflow { owner, list };
                }
                self.n_overflow = n;
                *r = CellListRef { off: list.off, len: CELL_LIST_SENTINEL, cap: 0 };
            }
        }
    }
}

/// Moves a list from one chunk's arena to another's, keeping its capacity, as renumbering does when a row changes
/// chunk.
pub fn move_list<B: Backing>(from: &mut ChunkArena<B>, to: &mut ChunkArena<B>, list: &mut ListRef) {
    let src = from.read(*list);
    let off = to.take(list.cap);
    to.read_mut(ListRef { off, ..*list }).copy_from_slice(src);
    from.dead += list.cap;
    list.off = off;
}

impl<B: Backing> crate::save::Saved for ChunkArena<B> {
    fn save(&self, w: &mut crate::save::Writer<'_>) {
        self.dead.save(w);
        self.compactions.save(w);
        self.words.save_prefix(to_usize(self.end), w, crate::Transform::Plain);
        self.overflow.save_prefix(self.n_overflow, w, crate::Transform::Plain);
    }

    fn load(r: &mut crate::save::Reader<'_>) -> Result<ChunkArena<B>, crate::save::LoadError> {
        let (dead, compactions) = (u32::load(r)?, u64::load(r)?);
        let (words, end) = Region::load_prefix(r, crate::Transform::Plain)?;
        let (overflow, n_overflow) = Region::load_prefix(r, crate::Transform::Plain)?;
        let end = crate::save::narrow::<u32>(end, "an arena's words")?;
        if dead > end {
            return Err(crate::save::LoadError::Invalid("an arena with more dead words than words".to_owned()));
        }
        Ok(ChunkArena { words, end, dead, compactions, overflow, n_overflow })
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{CellListRef, CellLists, ChunkArena, ListRef, move_list};
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::region::Region;

    type Heap = HeapBacking<4096>;

    fn arena(space: &mut AddressSpace) -> ChunkArena<Heap> {
        ChunkArena::new(space, 1 << 16)
    }

    #[test]
    fn arena_append_relocates_and_counts_dead() {
        let mut space = AddressSpace::empty();
        let mut a = arena(&mut space);
        let (mut x, mut y) = (ListRef::EMPTY, ListRef::EMPTY);
        a.append(&mut x, &[1, 2, 3]);
        assert_eq!(x, ListRef { off: 0, len: 3, cap: 4 });
        a.append(&mut y, &[9]);
        a.append(&mut x, &[4]);
        assert_eq!((x.off, a.dead_words()), (0, 0), "a list with room grows in place");
        a.append(&mut x, &[5]);
        assert_eq!((x, a.dead_words()), (ListRef { off: 8, len: 5, cap: 8 }, 4));
        a.append(&mut x, &[6, 7, 8, 9]);
        assert_eq!((x.off, x.cap, a.dead_words()), (8, 12, 4), "the last list grows where it lies");
        assert_eq!(a.read(x), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(a.read(y), &[9]);
    }

    #[test]
    fn arena_sorted_remove_keeps_order() {
        let mut space = AddressSpace::empty();
        let mut a = arena(&mut space);
        let mut x = ListRef::EMPTY;
        a.append(&mut x, &[1, 3, 5, 7, 9, 11]);
        a.remove(&mut x, 1, 2);
        assert_eq!(a.read(x), &[1, 7, 9, 11]);
        a.remove(&mut x, 3, 1);
        assert_eq!(a.read(x), &[1, 7, 9]);
    }

    /// Random appends, removals and clears over many lists, checked against a plain model after compaction.
    #[test]
    fn arena_compaction_preserves_lists_any_physical_order() {
        let rounds = if cfg!(miri) { 300 } else { 20_000 };
        let lists = 24_usize;
        let mut space = AddressSpace::empty();
        let mut a = arena(&mut space);
        let mut scratch: Region<u64, Heap> = Region::reserve(&mut space, 1 << 16);
        let mut refs = vec![ListRef::EMPTY; lists];
        let mut model: Vec<Vec<u64>> = vec![Vec::new(); lists];
        let mut d = Draws::new(stream_key(Seed::new(11), "arena"), Subject::new(SubjectTag::World, 0), 0, 0);
        for step in 0..rounds {
            let i = usize::try_from(below_u64(&mut d, u64::try_from(lists).unwrap())).unwrap();
            match below_u64(&mut d, 10) {
                0 => {
                    a.clear(&mut refs[i]);
                    model[i].clear();
                }
                1 | 2 if !model[i].is_empty() => {
                    let at = below_u64(&mut d, u64::try_from(model[i].len()).unwrap());
                    a.remove(&mut refs[i], u32::try_from(at).unwrap(), 1);
                    model[i].remove(usize::try_from(at).unwrap());
                }
                _ => {
                    let n = below_u64(&mut d, 6);
                    let words: Vec<u64> = (0..n).map(|k| step * 8 + k).collect();
                    a.append(&mut refs[i], &words);
                    model[i].extend(&words);
                }
            }
            if a.needs_compaction() {
                a.compact(refs.as_mut_slice(), &mut scratch);
                assert_eq!(a.dead_words(), 0);
                let mut at = 0;
                for r in &refs {
                    assert_eq!(r.off, at, "lists lie in slot order after compaction");
                    at += r.cap;
                }
            }
            for (r, m) in refs.iter().zip(&model) {
                assert_eq!(a.read(*r), m.as_slice());
            }
        }
        assert!(a.compactions() > 0);
    }

    #[test]
    fn cell_lists_overflow_and_compact() {
        let mut space = AddressSpace::empty();
        let mut a: ChunkArena<Heap> = ChunkArena::new(&mut space, 1 << 18);
        let mut scratch: Region<u64, Heap> = Region::reserve(&mut space, 1 << 18);
        let mut cells = [CellListRef::EMPTY; 3];
        let long: Vec<u64> = (0..70_000).collect();
        for (owner, (r, n)) in (40_u32..).zip(cells.iter_mut().zip([5_usize, 70_000, 2])) {
            let mut list = a.resolve(owner, *r);
            a.append(&mut list, &long[..n]);
            a.store(owner, r, list);
        }
        assert_eq!(cells[1].len, u16::MAX);
        let mut junk = ListRef::EMPTY;
        a.append(&mut junk, &vec![0; 100_000]);
        a.clear(&mut junk);
        a.compact(&mut CellLists { first_owner: 40, refs: &mut cells }, &mut scratch);
        assert_eq!(a.read(a.resolve(41, cells[1])).len(), 70_000);
        assert_eq!(a.read(a.resolve(42, cells[2])), &[0, 1]);
        let mut list = a.resolve(41, cells[1]);
        a.remove(&mut list, 0, 69_990);
        a.store(41, &mut cells[1], list);
        assert_eq!(cells[1].len, u16::MAX, "a capacity past 16 bits keeps the reference in the overflow map");
        assert_eq!(
            a.read(a.resolve(41, cells[1])),
            &[69_990, 69_991, 69_992, 69_993, 69_994, 69_995, 69_996, 69_997, 69_998, 69_999]
        );
    }

    #[test]
    fn move_list_carries_words_and_capacity() {
        let mut space = AddressSpace::empty();
        let (mut from, mut to) = (arena(&mut space), arena(&mut space));
        let mut x = ListRef::EMPTY;
        from.append(&mut x, &[3, 1, 4]);
        let mut y = ListRef::EMPTY;
        to.append(&mut y, &[2]);
        move_list(&mut from, &mut to, &mut x);
        assert_eq!((to.read(x), x.cap, from.dead_words()), (&[3, 1, 4][..], 4, 4));
    }
}
