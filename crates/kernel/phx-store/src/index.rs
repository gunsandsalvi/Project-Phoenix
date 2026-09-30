//! Keyed indexes: who is at a key — owners at a zone, parties by zone and kind, holders by instrument — kept by the
//! events that change a key, read in O(k) for a key's k members, compacted lazily and rebuilt at load, never rebuilt
//! per call or per day.

use phx_macros::{Pod, opening};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::block_list::{BlockBag, BlockList, BlockPool};
use crate::column::Column;
use crate::consts::{BLOCK_ENTRIES, INDEX_BLOCKS_PER_ENTRIES, INDEX_NEW_KEYS, INDEX_NEW_SHARE};
use crate::convert::{to_u32, to_u64, to_usize};
use crate::daybuf::DayBuf;
use crate::index_decl::{IndexDecl, Keys, Mode};
use crate::stats::StoreStats;
use crate::sumtree::SumTrees;

/// A dense key's members: the run of slots the opening placed its members in, key by key, read through the members'
/// own columns like any entry; its lazy list of those come since, in the order they came, duplicates and leavers among
/// them; and how many of all its entries are known dead.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct ListHead {
    bag: BlockBag,
    dead: u32,
    run_first: u32,
    run_len: u32,
}

impl ListHead {
    const EMPTY: ListHead = ListHead { bag: BlockBag::EMPTY, dead: 0, run_first: 0, run_len: 0 };

    /// Every entry the key holds: its run's slots and its list's entries.
    fn entries(self) -> u32 {
        self.run_len + self.bag.len()
    }
}

/// A sparse instance's entries — its keys hold few members each, among a key space of millions — as (key, member)
/// pairs sorted, 8 bytes an entry with no list of its own a key; the pairs come since the last merge sorted apart, in
/// a run a sixteenth of the instance's entries long (and never shorter than a floor), merged in one pass when it fills
/// or at the owner's close; and the leavers counted since the last compaction.
#[derive(Debug)]
struct Pairs<B: Backing> {
    sorted: Column<[u32; 2], B>,
    new: Column<[u32; 2], B>,
    new_room: usize,
    dead: u32,
}

/// An instance: its declaration; a dense instance's list heads and their blocks, and each weighted key's free
/// positions; a sparse instance's pairs; each weighted key's sum-tree; each counted key's count.
#[derive(Debug)]
pub struct Index<B: Backing = SystemBacking> {
    decl: IndexDecl,
    heads: Column<ListHead, B>,
    pairs: Option<Pairs<B>>,
    pool: BlockPool<B>,
    free: Column<BlockList, B>,
    trees: SumTrees<B>,
    counts: Column<i64, B>,
}

/// The blocks a dense instance's entries need: whole blocks at three quarters full, and one more a key.
fn blocks(decl: &IndexDecl) -> u32 {
    let per_block = to_u32(BLOCK_ENTRIES) * INDEX_BLOCKS_PER_ENTRIES.0 / INDEX_BLOCKS_PER_ENTRIES.1;
    let Some(n) = decl.entries.div_ceil(per_block).checked_add(decl.keys.count()) else {
        capacity_exceeded!("an index's blocks", u32::MAX, u64::from(decl.entries) + u64::from(decl.keys.count()));
    };
    n
}

/// The run of pairs at `key`: its start found by binary search, its end by reading on, a key's pairs being few.
fn run(pairs: &[[u32; 2]], key: u32) -> &[[u32; 2]] {
    let start = pairs.partition_point(|p| p[0] < key);
    let rest = pairs.get(start..).unwrap_or(&[]);
    let len = rest.iter().take_while(|p| p[0] == key).count();
    rest.get(..len).unwrap_or(&[])
}

/// Orders a walk's found members and keeps each once, from `start` of `out`; the members kept. What a walk finds
/// rises through its run and a compacted list, so only the tail after its rising prefix is sorted, then merged into the
/// prefix from the back through the room past `out`'s end: linear in the prefix, which is most of it.
fn unique_from<B: Backing>(out: &mut DayBuf<u32, B>, start: usize) -> usize {
    let total = out.len() - start;
    let prefix = {
        let Some(found) = out.as_slice().get(start..) else {
            violation!(clause = "SET.12", "a walk's output shorter than its start", start = start);
        };
        found.windows(2).take_while(|w| w.first() < w.get(1)).count() + usize::from(total > 0)
    };
    if prefix < total {
        let tail = total - prefix;
        let all = out.as_mut_slice(start + total + tail);
        let (found, spare) = all.split_at_mut(start + total);
        let Some(found) = found.get_mut(start..) else {
            violation!(clause = "SET.12", "a walk's output shorter than its start", start = start);
        };
        let (head, rest) = found.split_at_mut(prefix);
        rest.sort_unstable();
        spare.copy_from_slice(rest);
        let (mut i, mut j) = (prefix, tail);
        for at in (0..total).rev() {
            let from_tail = match (i.checked_sub(1), j.checked_sub(1)) {
                (Some(a), Some(b)) => head.get(a) < spare.get(b),
                (None, Some(_)) => true,
                _ => false,
            };
            let v = if from_tail {
                j -= 1;
                spare.get(j).copied()
            } else {
                i -= 1;
                head.get(i).copied()
            };
            let cell = if at < prefix { head.get_mut(at) } else { rest.get_mut(at - prefix) };
            if let (Some(c), Some(v)) = (cell, v) {
                *c = v;
            }
        }
        let _ = out.as_mut_slice(start + total);
    } else {
        return total;
    }
    let all = out.as_mut_slice(start + total);
    let Some(found) = all.get_mut(start..) else {
        violation!(clause = "SET.12", "a walk's output shorter than its start", start = start);
    };
    let mut unique = 0;
    for i in 0..found.len() {
        if unique == 0 || found.get(i) != found.get(unique - 1) {
            found.swap(unique, i);
            unique += 1;
        }
    }
    let _ = out.as_mut_slice(start + unique);
    unique
}

impl<B: Backing> Index<B> {
    /// An instance's stores, sized from its declared key space and entries, never from its members.
    ///
    /// # Errors
    /// A declaration that lacks what an instance needs.
    #[opening]
    pub fn new(space: &mut AddressSpace, decl: IndexDecl) -> Result<Index<B>, String> {
        decl.check()?;
        let rows = crate::consts::SUMTREE_ROWS_PER_CHUNK;
        let (lists, weighs, counts) = (decl.mode.lists(), decl.mode.weighs(), decl.mode.counts());
        let dense = match decl.keys {
            Keys::Dense(n) => n,
            Keys::Sparse(_) => 0,
        };
        let keys_of = |on: bool| if on { dense } else { 0 };
        let mut heads = Column::new(space, keys_of(lists), rows);
        let mut free = Column::new(space, keys_of(weighs), rows);
        let mut tallies = Column::new(space, keys_of(counts), rows);
        for _ in 0..keys_of(lists) {
            heads.push(ListHead::EMPTY);
        }
        for _ in 0..keys_of(weighs) {
            free.push(BlockList::EMPTY);
        }
        for _ in 0..keys_of(counts) {
            tallies.push(0);
        }
        let share = decl.entries / INDEX_NEW_SHARE;
        let room = if share > INDEX_NEW_KEYS { share } else { INDEX_NEW_KEYS };
        let pairs = match decl.keys {
            Keys::Sparse(_) => Some(Pairs {
                sorted: Column::new(space, decl.entries, rows),
                new: Column::new(space, room, rows),
                new_room: to_usize(room),
                dead: 0,
            }),
            Keys::Dense(_) => None,
        };
        let pool_blocks = if (lists && dense > 0) || weighs { blocks(&decl) } else { 0 };
        // A tree's classes double, so its extents hold at most twice its entries.
        let Some(extents) = (if weighs { decl.entries.checked_mul(2) } else { Some(0) }) else {
            capacity_exceeded!("an index's weights", u32::MAX, u64::from(decl.entries) * 2);
        };
        let mut trees = SumTrees::new(space, extents, keys_of(weighs));
        for _ in 0..keys_of(weighs) {
            let _ = trees.make();
        }
        Ok(Index { decl, heads, pairs, pool: BlockPool::new(space, pool_blocks), free, trees, counts: tallies })
    }

    #[must_use]
    pub fn decl(&self) -> &IndexDecl {
        &self.decl
    }

    /// A key's place in its key space; a key past it stops the run.
    fn place(&self, key: u64) -> u32 {
        match u32::try_from(key) {
            Ok(k) if k < self.decl.keys.count() => k,
            _ => violation!(clause = "SET.12", "a key past its index's key space", key = key),
        }
    }

    fn slot(&self, key: u64) -> phx_id::Slot {
        phx_id::Slot::new(self.place(key))
    }

    fn head(&self, key: u64) -> ListHead {
        if !self.decl.mode.lists() {
            violation!(clause = "SET.12", "a lazy list of an index that keeps none", key = key);
        }
        match self.heads.get(self.slot(key)) {
            Some(h) => h,
            None => violation!(clause = "SET.12", "a dense key with no list head", key = key),
        }
    }

    /// A dense key's head, to change in place.
    fn head_mut(&mut self, key: u64) -> &mut ListHead {
        let at = to_usize(self.place(key));
        match self.heads.slice_mut().get_mut(at) {
            Some(h) => h,
            None => violation!(clause = "SET.12", "a dense key with no list head", key = key),
        }
    }

    fn count_by(&mut self, key: u64, delta: i64) {
        if !self.decl.mode.counts() {
            return;
        }
        let at = to_usize(self.place(key));
        match self.counts.slice_mut().get_mut(at) {
            Some(n) => *n += delta,
            None => violation!(clause = "SET.12", "a count of a key the index never counted", key = key),
        }
    }

    /// The pairs come since the last merge merged into the sorted pairs in one pass from the back: the owner's close
    /// calls it, and a full run of new pairs does.
    pub fn settle(&mut self) {
        let Some(pairs) = self.pairs.as_mut() else { return };
        let (old, new) = (pairs.sorted.len(), pairs.new.len());
        if new == 0 {
            return;
        }
        for _ in 0..new {
            pairs.sorted.push([0; 2]);
        }
        let (sorted, fresh) = (pairs.sorted.slice_mut(), pairs.new.slice());
        let (mut i, mut j) = (old, new);
        for at in (0..old + new).rev() {
            let take_new = match (i.checked_sub(1), j.checked_sub(1)) {
                (Some(a), Some(b)) => sorted.get(a) < fresh.get(b),
                (None, Some(_)) => true,
                _ => false,
            };
            let pair = if take_new {
                j -= 1;
                fresh.get(j).copied()
            } else {
                i -= 1;
                sorted.get(i).copied()
            };
            if let (Some(p), Some(cell)) = (pair, sorted.get_mut(at)) {
                *cell = p;
            }
        }
        pairs.new.truncate(0);
    }

    /// A member come to a key: appended to a dense key's list, O(1), or put in a sparse instance's run of new pairs.
    pub fn insert(&mut self, key: u64, member: u32) {
        let place = self.place(key);
        if self.pairs.is_some() {
            if self.pairs.as_ref().is_some_and(|p| p.new.len() == p.new_room) {
                self.settle();
            }
            let Some(pairs) = self.pairs.as_mut() else { return };
            if pairs.sorted.len() + pairs.new.len() >= to_usize(self.decl.entries) {
                capacity_exceeded!("an index's entries", self.decl.entries, pairs.sorted.len() + pairs.new.len() + 1);
            }
            // The applies bring a key range's pairs in order, so a pair mostly sorts last and is appended.
            let pair = [place, member];
            let last = pairs.new.slice().last().copied();
            pairs.new.push(pair);
            if last.is_some_and(|l| l > pair) {
                let at = pairs.new.slice().partition_point(|p| *p < pair);
                if let Some(tail) = pairs.new.slice_mut().get_mut(at..) {
                    tail.rotate_right(1);
                }
            }
        } else {
            if !self.decl.mode.lists() {
                violation!(clause = "SET.12", "a lazy list of an index that keeps none", key = key);
            }
            let mut bag = self.head_mut(key).bag;
            self.pool.push(&mut bag, member);
            self.head_mut(key).bag = bag;
        }
        self.count_by(key, 1);
    }

    /// A member gone from a key: not searched for, only counted dead — in the key's list, or in a sparse instance —
    /// and its entry dropped by the next walk or compaction that finds it no longer there.
    pub fn left(&mut self, key: u64) {
        if let Some(pairs) = self.pairs.as_mut() {
            pairs.dead += 1;
        } else {
            if !self.decl.mode.lists() {
                violation!(clause = "SET.12", "a lazy list of an index that keeps none", key = key);
            }
            self.head_mut(key).dead += 1;
        }
        self.count_by(key, -1);
    }

    /// The key's members, appended to `out` in slot order, each once: an entry is kept exactly when `valid` finds its
    /// member at the key by its own columns now, so a leaver, a reused slot's other occupant and a repeat are dropped.
    /// Returns the entries dropped.
    pub fn members(&self, key: u64, valid: impl Fn(u32) -> bool, out: &mut DayBuf<u32, B>) -> u32 {
        let start = out.len();
        // The output is sized once for every entry the key holds and written in place, then cut to those kept.
        let (read, kept) = if let Some(pairs) = self.pairs.as_ref() {
            let place = self.place(key);
            let (old, new) = (run(pairs.sorted.slice(), place), run(pairs.new.slice(), place));
            let read = old.len() + new.len();
            let Some(dst) = out.as_mut_slice(start + read).get_mut(start..) else {
                violation!(clause = "SET.12", "a walk's output shorter than its start", start = start);
            };
            let mut kept = 0;
            for p in old.iter().chain(new) {
                if valid(p[1])
                    && let Some(cell) = dst.get_mut(kept)
                {
                    *cell = p[1];
                    kept += 1;
                }
            }
            (to_u32(read), kept)
        } else {
            let head = self.head(key);
            let Some(dst) = out.as_mut_slice(start + to_usize(head.entries())).get_mut(start..) else {
                violation!(clause = "SET.12", "a walk's output shorter than its start", start = start);
            };
            let mut kept = 0;
            let mut keep = |member: u32| {
                if valid(member)
                    && let Some(cell) = dst.get_mut(kept)
                {
                    *cell = member;
                    kept += 1;
                }
            };
            for member in head.run_first..head.run_first + head.run_len {
                keep(member);
            }
            self.pool.for_each_in_bag(head.bag, &mut keep);
            (head.entries(), kept)
        };
        let _ = out.as_mut_slice(start + kept);
        read - to_u32(unique_from(out, start))
    }

    /// Entries a key holds, its dead among them.
    #[must_use]
    pub fn entries(&self, key: u64) -> u32 {
        match self.pairs.as_ref() {
            Some(pairs) => {
                let place = self.place(key);
                to_u32(run(pairs.sorted.slice(), place).len() + run(pairs.new.slice(), place).len())
            }
            None => self.head(key).entries(),
        }
    }

    /// The run of slots the opening placed a dense key's members in, key by key: `len` slots from `first`, held as two
    /// words. A key takes one run, before any member comes to it.
    pub fn place_run(&mut self, key: u64, first: u32, len: u32) {
        let mut head = self.head(key);
        if head.entries() > 0 {
            violation!(clause = "SET.12", "a run placed at a key that holds members", key = key);
        }
        (head.run_first, head.run_len) = (first, len);
        self.heads.set(self.slot(key), head);
        self.count_by(key, i64::from(len));
    }

    /// Entries a dense key's list knows dead.
    #[must_use]
    pub fn dead(&self, key: u64) -> u32 {
        self.head(key).dead
    }

    /// The dead entries over the entries: every key's in a dense instance, the pairs' in a sparse one.
    #[must_use]
    pub fn dead_and_entries(&self) -> (u64, u64) {
        match self.pairs.as_ref() {
            Some(p) => (u64::from(p.dead), to_u64(p.sorted.len() + p.new.len())),
            None => {
                self.heads.slice().iter().fold((0, 0), |(d, e), h| (d + u64::from(h.dead), e + u64::from(h.entries())))
            }
        }
    }

    /// Whether a dense key's dead entries have passed its live ones, so its owner compacts it at its next apply.
    #[must_use]
    pub fn needs_compaction(&self, key: u64) -> bool {
        let head = self.head(key);
        head.dead > head.entries() - head.dead
    }

    /// A dense key's members rewritten as a walk finds them, in slot order, each once, its dead count cleared of all
    /// but its run's: its run kept while at least half its slots are still its members, the rest in its list; else
    /// every member in its list. `scratch` is the owner's day buffer.
    pub fn compact(&mut self, key: u64, valid: impl Fn(u32) -> bool, scratch: &mut DayBuf<u32, B>) {
        let mut head = self.head(key);
        scratch.clear();
        let _ = self.members(key, valid, scratch);
        let (first, end) = (head.run_first, head.run_first + head.run_len);
        let in_run = to_u32(scratch.as_slice().iter().filter(|m| (first..end).contains(*m)).count());
        let keep_run = head.run_len > 0 && in_run * 2 >= head.run_len;
        self.pool.empty_bag(&mut head.bag);
        for member in scratch.as_slice().iter().filter(|m| !keep_run || !(first..end).contains(*m)) {
            self.pool.push(&mut head.bag, *member);
        }
        if keep_run {
            head.dead = head.run_len - in_run;
        } else {
            (head.run_first, head.run_len, head.dead) = (0, 0, 0);
        }
        self.heads.set(self.slot(key), head);
    }

    /// Whether a sparse instance's leavers have passed its live pairs, so its owner compacts it at the close.
    #[must_use]
    pub fn pairs_need_compaction(&self) -> bool {
        self.pairs.as_ref().is_some_and(|p| {
            let len = to_u32(p.sorted.len() + p.new.len());
            p.dead > len - p.dead
        })
    }

    /// A sparse instance's pairs merged and rewritten as those whose member `valid` finds at the pair's key now, each
    /// once, its dead count cleared: one pass over the pairs.
    pub fn compact_pairs(&mut self, valid: impl Fn(u64, u32) -> bool) {
        self.settle();
        let Some(pairs) = self.pairs.as_mut() else { return };
        let all = pairs.sorted.slice_mut();
        let mut kept = 0;
        for i in 0..all.len() {
            let Some(p) = all.get(i).copied() else { continue };
            let repeat = kept > 0 && all.get(kept - 1) == Some(&p);
            if !repeat && valid(u64::from(p[0]), p[1]) {
                all.swap(kept, i);
                kept += 1;
            }
        }
        pairs.sorted.truncate(kept);
        pairs.dead = 0;
    }

    /// A weighted member come to a key at a weight: the key's lowest free position, else a new one; its position is
    /// the member's own to keep.
    pub fn insert_weighted(&mut self, key: u64, weight: u64) -> usize {
        let (slot, tree) = (self.slot(key), self.place(key));
        let Some(mut free) = self.free.get(slot) else {
            violation!(clause = "SET.12", "a weighted insert into an index that weighs nothing", key = key);
        };
        let lowest = self.pool.iter(free).next();
        let position = match lowest {
            Some(pos) => {
                self.pool.remove_sorted(&mut free, pos);
                self.free.set(slot, free);
                let pos = to_usize(pos);
                self.trees.set(tree, pos, weight);
                pos
            }
            None => self.trees.push(tree, weight),
        };
        self.count_by(key, 1);
        position
    }

    /// A weighted member's weight changed in place.
    pub fn update(&mut self, key: u64, position: usize, weight: u64) {
        let tree = self.place(key);
        self.trees.set(tree, position, weight);
    }

    /// A weighted member gone: its weight nought and its position returned to the key's free positions.
    pub fn remove(&mut self, key: u64, position: usize) {
        let (slot, tree) = (self.slot(key), self.place(key));
        self.trees.set(tree, position, 0);
        let Some(mut free) = self.free.get(slot) else {
            violation!(clause = "SET.12", "a weighted removal from an index that weighs nothing", key = key);
        };
        self.pool.insert_sorted(&mut free, to_u32(position));
        self.free.set(slot, free);
        self.count_by(key, -1);
    }

    /// A weighted key's total.
    #[must_use]
    pub fn total(&self, key: u64) -> u64 {
        self.trees.total(self.place(key))
    }

    /// The member drawn at `x` below the key's total, in proportion to weight; a key of total nought draws none.
    pub fn draw(&self, key: u64, x: u64) -> Missing<usize> {
        self.trees.find(self.place(key), x)
    }

    /// A counted key's members.
    #[must_use]
    pub fn count(&self, key: u64) -> i64 {
        match self.counts.get(self.slot(key)) {
            Some(n) => n,
            None => violation!(clause = "SET.12", "a count read from an index that counts nothing", key = key),
        }
    }

    /// A count kept alone moved by an event.
    pub fn add(&mut self, key: u64, delta: i64) {
        if !matches!(self.decl.mode, Mode::Count) {
            violation!(clause = "SET.12", "a count moved by hand in an index that keeps lists", key = key);
        }
        self.count_by(key, delta);
    }

    /// The bytes every part holds committed.
    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.heads.bytes_committed()
            + self.pool.bytes_committed()
            + self.free.bytes_committed()
            + self.trees.bytes_committed()
            + self.counts.bytes_committed()
            + match self.pairs.as_ref() {
                Some(p) => p.sorted.bytes_committed() + p.new.bytes_committed(),
                None => 0,
            }
    }
}

impl<B: Backing> StoreStats for Index<B> {
    fn rows_live(&self) -> u64 {
        let (dead, entries) = self.dead_and_entries();
        entries - dead + self.trees.rows_live()
    }

    fn rows_ever(&self) -> u64 {
        self.dead_and_entries().1 + self.trees.rows_ever()
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
