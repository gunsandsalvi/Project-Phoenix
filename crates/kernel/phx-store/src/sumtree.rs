//! Sum-trees: a pool of Fenwick trees over integer weights in one column, each addressed by (tree, index), so a weight
//! is changed, a total read and a member drawn in proportion to its weight without rebuilding any table of weights.

use phx_macros::{Pod, opening};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::consts::{SUMTREE_BASE_CAPACITY, SUMTREE_CLASSES, SUMTREE_LEN_BITS, SUMTREE_STEPS};
use crate::convert::{to_u32, to_u64, to_usize};
use crate::stats::StoreStats;

/// A tree's place in the weights: its extent's offset, its length and capacity class in one word (the length in the
/// low bits), and its total.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct TreeHeader {
    offset: u32,
    len_class: u32,
    total: u64,
}

impl TreeHeader {
    fn len(self) -> usize {
        to_usize(self.len_class & ((1 << SUMTREE_LEN_BITS) - 1))
    }

    fn class(self) -> u32 {
        self.len_class >> SUMTREE_LEN_BITS
    }

    fn with(offset: u32, len: usize, class: u32, total: u64) -> TreeHeader {
        let len = to_u32(len);
        if len >> SUMTREE_LEN_BITS != 0 {
            capacity_exceeded!("a sum-tree's members", 1_u64 << SUMTREE_LEN_BITS, len);
        }
        TreeHeader { offset, len_class: len | class << SUMTREE_LEN_BITS, total }
    }
}

/// The entries a class holds: four classes a doubling from the base — 4, 5, 6, 7, 8, 10, 12, 14, 16… — so a tree
/// holds at most a quarter more than its members, and moves a logarithmic number of times as it grows.
fn capacity(class: u32) -> usize {
    let (octave, step) = (class / SUMTREE_STEPS, class % SUMTREE_STEPS);
    (SUMTREE_BASE_CAPACITY + to_usize(step) * (SUMTREE_BASE_CAPACITY / to_usize(SUMTREE_STEPS))) << octave
}

/// A cell's word as an index.
fn word_index(v: u64) -> usize {
    match usize::try_from(v) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("index width", usize::MAX, v),
    }
}

/// The lowest set bit of a Fenwick index: the span its node sums.
fn low(i: usize) -> usize {
    i.isolate_lowest_one()
}

/// Every tree's Fenwick array in one column of weights, each tree's header, and each class's first extent freed by a
/// tree that grew, to be taken again first: a freed extent holds in its first two cells whether another follows it and
/// where, so the free extents cost no memory of their own.
#[derive(Debug)]
pub struct SumTrees<B: Backing = SystemBacking> {
    weights: Column<u64, B>,
    headers: Column<TreeHeader, B>,
    free: [Missing<u32>; SUMTREE_CLASSES],
}

impl Default for SumTrees {
    /// No trees and no room for any: what a load leaves until the owner's rebuild makes them again.
    fn default() -> SumTrees {
        SumTrees::new(&mut AddressSpace::empty(), 0, 0)
    }
}

impl<B: Backing> SumTrees<B> {
    /// Room for at most `max_weights` entries over every tree's extents and `max_trees` trees.
    #[opening]
    pub fn new(space: &mut AddressSpace, max_weights: u32, max_trees: u32) -> SumTrees<B> {
        let rows = crate::consts::SUMTREE_ROWS_PER_CHUNK;
        SumTrees {
            weights: Column::new(space, max_weights, rows),
            headers: Column::new(space, max_trees, rows),
            free: [Missing::Absent; SUMTREE_CLASSES],
        }
    }

    fn header(&self, tree: u32) -> TreeHeader {
        match self.headers.get(phx_id::Slot::new(tree)) {
            Some(h) => h,
            None => violation!(clause = "SET.12", "a sum-tree that was never made", tree = tree),
        }
    }

    /// An extent of a class: the last one a grown tree freed, else a new one at the column's end.
    fn extent(&mut self, class: u32) -> u32 {
        let Some(head) = self.free.get(to_usize(class)).copied() else {
            capacity_exceeded!("sum-tree classes", SUMTREE_CLASSES, to_usize(class) + 1);
        };
        if let Missing::Present(offset) = head {
            let cells = self.cells_mut(offset, capacity(class));
            let next = match cells {
                [0, ..] => Missing::Absent,
                [_, next, ..] => Missing::Present(to_u32(word_index(*next))),
                _ => violation!(clause = "SET.12", "a freed sum-tree extent too short for its chain", offset = offset),
            };
            cells.fill(0);
            if let Some(slot) = self.free.get_mut(to_usize(class)) {
                *slot = next;
            }
            return offset;
        }
        let offset = to_u32(self.weights.len());
        let zeros = self.weights.append_zeroed(capacity(class));
        zeros.fill(0);
        offset
    }

    fn cells(&self, offset: u32, n: usize) -> &[u64] {
        let start = to_usize(offset);
        match self.weights.slice().get(start..start + n) {
            Some(cells) => cells,
            None => violation!(clause = "SET.12", "a sum-tree's extent past its weights", offset = offset),
        }
    }

    fn cells_mut(&mut self, offset: u32, n: usize) -> &mut [u64] {
        let start = to_usize(offset);
        match self.weights.slice_mut().get_mut(start..start + n) {
            Some(cells) => cells,
            None => violation!(clause = "SET.12", "a sum-tree's extent past its weights", offset = offset),
        }
    }

    /// A new, empty tree.
    pub fn make(&mut self) -> u32 {
        let offset = self.extent(0);
        let tree = to_u32(self.headers.len());
        self.headers.push(TreeHeader::with(offset, 0, 0, 0));
        tree
    }

    /// Trees made.
    #[must_use]
    pub fn trees(&self) -> u32 {
        to_u32(self.headers.len())
    }

    /// A tree's members, those set to nought among them.
    #[must_use]
    pub fn len(&self, tree: u32) -> usize {
        self.header(tree).len()
    }

    /// A tree's total weight: one read.
    #[must_use]
    pub fn total(&self, tree: u32) -> u64 {
        self.header(tree).total
    }

    /// The weights of a tree's first `n` members.
    fn prefix(cells: &[u64], n: usize) -> u64 {
        let (mut i, mut sum) = (n, 0_u64);
        while i > 0 {
            if let Some(node) = cells.get(i - 1) {
                sum += node;
            }
            i -= low(i);
        }
        sum
    }

    /// The sum of the weights before member `i`.
    #[must_use]
    pub fn prefix_before(&self, tree: u32, i: usize) -> u64 {
        let h = self.header(tree);
        if i > h.len() {
            violation!(clause = "SET.12", "a prefix past a sum-tree's members", tree = tree, i = i);
        }
        Self::prefix(self.cells(h.offset, h.len()), i)
    }

    /// Member `i`'s weight: its node less the nodes its span covers below it, one short walk.
    #[must_use]
    pub fn weight(&self, tree: u32, member: usize) -> u64 {
        let header = self.header(tree);
        let cells = self.cells(header.offset, header.len());
        let Some(&node) = cells.get(member) else {
            violation!(clause = "SET.12", "a sum-tree member past its length", tree = tree, member = member);
        };
        let at = member + 1;
        let (mut weight, mut below, stop) = (node, at - 1, at - low(at));
        while below > stop {
            if let Some(covered) = cells.get(below - 1) {
                weight -= covered;
            }
            below -= low(below);
        }
        weight
    }

    /// A member appended with its weight; a tree that fills moves to the next class, its extent freed.
    pub fn push(&mut self, tree: u32, w: u64) -> usize {
        let mut h = self.header(tree);
        let len = h.len();
        if len == capacity(h.class()) {
            let class = h.class() + 1;
            let offset = self.extent(class);
            let (from, to) = (to_usize(h.offset), to_usize(offset));
            self.weights.slice_mut().copy_within(from..from + len, to);
            let old = h.class();
            let Some(head) = self.free.get(to_usize(old)).copied() else {
                capacity_exceeded!("sum-tree classes", SUMTREE_CLASSES, to_usize(old) + 1);
            };
            let chain = match head {
                Missing::Present(next) => [1, u64::from(next)],
                Missing::Absent => [0, 0],
            };
            if let Some(cells) = self.cells_mut(h.offset, capacity(old)).get_mut(..2) {
                cells.copy_from_slice(&chain);
            }
            if let Some(slot) = self.free.get_mut(to_usize(old)) {
                *slot = Missing::Present(h.offset);
            }
            h = TreeHeader::with(offset, len, class, h.total);
        }
        // A new node sums its own weight and the nodes its span covers below it.
        let n = len + 1;
        let cells = self.cells_mut(h.offset, n);
        let covered = Self::prefix(cells, n - 1) - Self::prefix(cells, n - low(n));
        if let Some(cell) = cells.last_mut() {
            *cell = w + covered;
        }
        let Some(total) = h.total.checked_add(w) else {
            capacity_exceeded!("a sum-tree's total", u64::MAX, i128::from(h.total) + i128::from(w));
        };
        self.headers.set(phx_id::Slot::new(tree), TreeHeader::with(h.offset, n, h.class(), total));
        len
    }

    /// Member `i`'s weight moved by `delta`; a weight below nought or a total past `u64::MAX` stops the run.
    pub fn update(&mut self, tree: u32, i: usize, delta: i64) {
        let h = self.header(tree);
        let len = h.len();
        if i >= len {
            violation!(clause = "SET.12", "a sum-tree member past its length", tree = tree, i = i);
        }
        let step = |v: u64| match (delta >= 0, v.checked_add_signed(delta)) {
            (_, Some(n)) => n,
            (true, None) => capacity_exceeded!("a sum-tree's total", u64::MAX, i128::from(v) + i128::from(delta)),
            (false, None) => violation!(clause = "SET.12", "a sum-tree weight below nought", tree = tree, i = i),
        };
        let cells = self.cells_mut(h.offset, len);
        let mut j = i + 1;
        while j <= len {
            if let Some(cell) = cells.get_mut(j - 1) {
                *cell = step(*cell);
            }
            j += low(j);
        }
        self.headers.set(phx_id::Slot::new(tree), TreeHeader { total: step(h.total), ..h });
    }

    /// Member `i`'s weight set; a removal is a weight of nought, the member keeping its place.
    pub fn set(&mut self, tree: u32, i: usize, w: u64) {
        let now = self.weight(tree, i);
        let (Ok(to), Ok(from)) = (i64::try_from(w), i64::try_from(now)) else {
            capacity_exceeded!("a sum-tree weight", i64::MAX, if w > now { w } else { now });
        };
        if to != from {
            self.update(tree, i, to - from);
        }
    }

    /// The least member whose weights up to and including its own exceed `x`: with `x` drawn below the total, a member
    /// drawn in proportion to its weight. A tree whose total is nought draws none; `x` at or past the total stops the
    /// run.
    pub fn find(&self, tree: u32, x: u64) -> Missing<usize> {
        let h = self.header(tree);
        if h.total == 0 {
            return Missing::Absent;
        }
        if x >= h.total {
            violation!(clause = "SET.12", "a sum-tree drawn at or past its total", tree = tree, x = x);
        }
        let (len, cells) = (h.len(), self.cells(h.offset, h.len()));
        let (mut pos, mut rest) = (0, x);
        let mut step = if len == 0 { 0 } else { 1 << (usize::BITS - 1 - len.leading_zeros()) };
        while step > 0 {
            if pos + step <= len
                && let Some(node) = cells.get(pos + step - 1)
                && *node <= rest
            {
                rest -= node;
                pos += step;
            }
            step >>= 1;
        }
        Missing::Present(pos)
    }

    /// The bytes the weights' extents hold committed, freed extents with them.
    #[must_use]
    pub fn weight_bytes(&self) -> usize {
        self.weights.bytes_committed()
    }

    /// The bytes the weights and the headers hold committed.
    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.weights.bytes_committed() + self.headers.bytes_committed()
    }
}

impl<B: Backing> PartialEq for SumTrees<B> {
    /// Two pools are equal when their trees hold the same members at the same weights, wherever their extents lie.
    fn eq(&self, other: &SumTrees<B>) -> bool {
        self.trees() == other.trees()
            && (0..self.trees()).all(|t| {
                let (a, b) = (self.header(t), other.header(t));
                a.len() == b.len()
                    && a.total == b.total
                    && self.cells(a.offset, a.len()) == other.cells(b.offset, b.len())
            })
    }
}

impl<B: Backing> StoreStats for SumTrees<B> {
    fn rows_live(&self) -> u64 {
        (0..self.trees()).map(|t| to_u64(self.header(t).len())).sum()
    }

    fn rows_ever(&self) -> u64 {
        to_u64(self.weights.len())
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

#[cfg(test)]
#[path = "sumtree_tests.rs"]
mod tests;
