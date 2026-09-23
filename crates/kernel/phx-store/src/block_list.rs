use phx_macros::Pod;
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::consts::{BLOCK_ENTRIES, BLOCK_HALF, BLOCK_TREE_DEPTH};
use crate::convert::{to_u32, to_usize};
use crate::region::Region;

const NONE: u32 = u32::MAX;

/// Up to 16 entries of a list in order, and the next block of the same list.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
struct Leaf {
    keys: [u32; BLOCK_ENTRIES],
    next: u32,
    len: u32,
}

/// Up to 16 subtrees and the least entry of each, so a search reads one block per level.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
struct Node {
    keys: [u32; BLOCK_ENTRIES],
    kids: [u32; BLOCK_ENTRIES],
    len: u32,
}

/// A sorted set of 32-bit entries spanning chunks — a line's holders, an instrument's holders — held as a tree of
/// 16-entry blocks from a shared pool; `root` is a leaf block when `height` is zero.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct BlockList {
    root: u32,
    height: u32,
    len: u32,
}

impl BlockList {
    pub const EMPTY: BlockList = BlockList { root: NONE, height: 0, len: 0 };

    #[must_use]
    pub fn len(self) -> u32 {
        self.len
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Entries in the order they were added, duplicates kept — a day's bucket of an agenda — as a chain of 16-entry
/// blocks from the same pool as the block lists, emptied whole.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct BlockBag {
    head: u32,
    tail: u32,
    len: u32,
}

impl BlockBag {
    pub const EMPTY: BlockBag = BlockBag { head: NONE, tail: NONE, len: 0 };

    #[must_use]
    pub fn len(self) -> u32 {
        self.len
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

fn get(values: &[u32], i: usize) -> u32 {
    let Some(v) = values.get(i).copied() else {
        violation!(clause = "SET.12", "a block entry past its block", i = i);
    };
    v
}

fn set(values: &mut [u32], i: usize, v: u32) {
    let Some(cell) = values.get_mut(i) else {
        violation!(clause = "SET.12", "a block entry past its block", i = i);
    };
    *cell = v;
}

/// The first `n` values, which a block always holds.
fn prefix(values: &[u32], n: usize) -> &[u32] {
    let Some(p) = values.get(..n) else {
        violation!(clause = "SET.12", "a block holding more entries than it has room for", n = n);
    };
    p
}

/// Writes `from` at the start of `to`.
fn fill(to: &mut [u32], from: &[u32]) {
    let Some(dst) = to.get_mut(..from.len()) else {
        violation!(clause = "SET.12", "a block holding more entries than it has room for", n = from.len());
    };
    dst.copy_from_slice(from);
}

/// Writes `from` into `to` from position `at`.
fn fill_at(to: &mut [u32], at: usize, from: &[u32]) {
    let Some(dst) = to.get_mut(at..at + from.len()) else {
        violation!(clause = "SET.12", "a block holding more entries than it has room for", n = at + from.len());
    };
    dst.copy_from_slice(from);
}

/// Opens a gap at `at` in the first `len` values and writes `v` there.
fn insert_at(values: &mut [u32], len: usize, at: usize, v: u32) {
    values.copy_within(at..len, at + 1);
    set(values, at, v);
}

/// Closes the gap at `at` in the first `len` values.
fn remove_at(values: &mut [u32], len: usize, at: usize) {
    values.copy_within(at + 1..len, at);
}

/// The subtree that holds `key`: the last whose least entry is at most `key`, or the first.
fn child_index(node: &Node, key: u32) -> usize {
    node.keys.get(1..to_usize(node.len)).map_or(0, |rest| rest.partition_point(|k| *k <= key))
}

/// The blocks every block list of one kind draws from, with their free lists.
#[derive(Debug)]
pub struct BlockPool<B: Backing = SystemBacking> {
    leaves: Region<Leaf, B>,
    n_leaves: u32,
    free_leaves: u32,
    nodes: Region<Node, B>,
    n_nodes: u32,
    free_nodes: u32,
    max_blocks: u32,
}

impl<B: Backing> BlockPool<B> {
    pub fn new(space: &mut AddressSpace, max_blocks: u32) -> BlockPool<B> {
        BlockPool {
            leaves: Region::reserve(space, to_usize(max_blocks)),
            n_leaves: 0,
            free_leaves: NONE,
            nodes: Region::reserve(space, to_usize(max_blocks)),
            n_nodes: 0,
            free_nodes: NONE,
            max_blocks,
        }
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.leaves.bytes_committed() + self.nodes.bytes_committed()
    }

    fn leaf(&self, id: u32) -> Leaf {
        let Some(leaf) = self.leaves.slice(to_usize(self.n_leaves)).get(to_usize(id)).copied() else {
            violation!(clause = "SET.12", "a block list naming a block not in its pool", block = id);
        };
        leaf
    }

    fn put_leaf(&mut self, id: u32, leaf: Leaf) {
        let Some(cell) = self.leaves.slice_mut(to_usize(self.n_leaves)).get_mut(to_usize(id)) else {
            violation!(clause = "SET.12", "a block list naming a block not in its pool", block = id);
        };
        *cell = leaf;
    }

    fn node(&self, id: u32) -> Node {
        let Some(node) = self.nodes.slice(to_usize(self.n_nodes)).get(to_usize(id)).copied() else {
            violation!(clause = "SET.12", "a block list naming a node not in its pool", node = id);
        };
        node
    }

    fn put_node(&mut self, id: u32, node: Node) {
        let Some(cell) = self.nodes.slice_mut(to_usize(self.n_nodes)).get_mut(to_usize(id)) else {
            violation!(clause = "SET.12", "a block list naming a node not in its pool", node = id);
        };
        *cell = node;
    }

    fn new_leaf(&mut self, leaf: Leaf) -> u32 {
        let id = if self.free_leaves == NONE {
            if self.n_leaves == self.max_blocks {
                capacity_exceeded!("block pool leaves", self.max_blocks, u64::from(self.max_blocks) + 1);
            }
            self.n_leaves += 1;
            self.leaves.ensure(to_usize(self.n_leaves));
            self.n_leaves - 1
        } else {
            let id = self.free_leaves;
            self.free_leaves = self.leaf(id).next;
            id
        };
        self.put_leaf(id, leaf);
        id
    }

    fn drop_leaf(&mut self, id: u32) {
        self.put_leaf(id, Leaf { keys: [0; BLOCK_ENTRIES], next: self.free_leaves, len: 0 });
        self.free_leaves = id;
    }

    fn new_node(&mut self, node: Node) -> u32 {
        let id = if self.free_nodes == NONE {
            if self.n_nodes == self.max_blocks {
                capacity_exceeded!("block pool nodes", self.max_blocks, u64::from(self.max_blocks) + 1);
            }
            self.n_nodes += 1;
            self.nodes.ensure(to_usize(self.n_nodes));
            self.n_nodes - 1
        } else {
            let id = self.free_nodes;
            self.free_nodes = get(&self.node(id).kids, 0);
            id
        };
        self.put_node(id, node);
        id
    }

    fn drop_node(&mut self, id: u32) {
        let mut kids = [0; BLOCK_ENTRIES];
        set(&mut kids, 0, self.free_nodes);
        self.put_node(id, Node { keys: [0; BLOCK_ENTRIES], kids, len: 0 });
        self.free_nodes = id;
    }

    fn min_of(&self, id: u32, height: u32) -> u32 {
        if height == 0 { get(&self.leaf(id).keys, 0) } else { get(&self.node(id).keys, 0) }
    }

    /// The path from the root to the leaf that holds or would hold `key`: each node and the child taken.
    fn descend(&self, list: BlockList, key: u32) -> ([(u32, usize); BLOCK_TREE_DEPTH], usize, u32) {
        let mut path = [(NONE, 0); BLOCK_TREE_DEPTH];
        let depth = to_usize(list.height);
        if depth > BLOCK_TREE_DEPTH {
            capacity_exceeded!("block tree depth", BLOCK_TREE_DEPTH, depth);
        }
        let mut id = list.root;
        for step in path.iter_mut().take(depth) {
            let node = self.node(id);
            let idx = child_index(&node, key);
            *step = (id, idx);
            id = get(&node.kids, idx);
        }
        (path, depth, id)
    }

    /// Adds an entry the list does not hold, keeping order; splits a full block in two halves.
    pub fn insert_sorted(&mut self, list: &mut BlockList, key: u32) {
        if list.root == NONE {
            let mut keys = [0; BLOCK_ENTRIES];
            set(&mut keys, 0, key);
            list.root = self.new_leaf(Leaf { keys, next: NONE, len: 1 });
            list.height = 0;
            list.len = 1;
            return;
        }
        let (path, depth, leaf_id) = self.descend(*list, key);
        let mut leaf = self.leaf(leaf_id);
        let len = to_usize(leaf.len);
        let Err(pos) = prefix(&leaf.keys, len).binary_search(&key) else {
            violation!(clause = "SET.12", "an entry added twice to a sorted block list", key = key);
        };
        let mut carry = if len < BLOCK_ENTRIES {
            insert_at(&mut leaf.keys, len, pos, key);
            leaf.len += 1;
            self.put_leaf(leaf_id, leaf);
            None
        } else {
            let mut all = [0; BLOCK_ENTRIES + 1];
            fill(&mut all, &leaf.keys);
            insert_at(&mut all, BLOCK_ENTRIES, pos, key);
            let (low, high) = all.split_at(BLOCK_HALF);
            let mut right = Leaf { keys: [0; BLOCK_ENTRIES], next: leaf.next, len: to_u32(high.len()) };
            fill(&mut right.keys, high);
            let right_id = self.new_leaf(right);
            let mut left = Leaf { keys: [0; BLOCK_ENTRIES], next: right_id, len: to_u32(low.len()) };
            fill(&mut left.keys, low);
            self.put_leaf(leaf_id, left);
            Some((get(high, 0), right_id))
        };
        list.len += 1;
        let mut new_min = (pos == 0).then_some(key);
        for &(node_id, idx) in path.iter().take(depth).rev() {
            if carry.is_none() && new_min.is_none() {
                break;
            }
            let mut node = self.node(node_id);
            if let Some(m) = new_min {
                set(&mut node.keys, idx, m);
                new_min = new_min.filter(|_| idx == 0);
            }
            carry = match carry {
                None => {
                    self.put_node(node_id, node);
                    None
                }
                Some((k, kid)) => self.insert_child(node_id, node, idx + 1, k, kid),
            };
        }
        if let Some((k, kid)) = carry {
            let mut keys = [0; BLOCK_ENTRIES];
            let mut kids = [0; BLOCK_ENTRIES];
            set(&mut keys, 0, self.min_of(list.root, list.height));
            set(&mut kids, 0, list.root);
            set(&mut keys, 1, k);
            set(&mut kids, 1, kid);
            list.root = self.new_node(Node { keys, kids, len: 2 });
            list.height += 1;
        }
    }

    /// Adds a subtree to a node at `at`, splitting the node when full; returns the new right half to add above.
    fn insert_child(&mut self, id: u32, mut node: Node, at: usize, key: u32, kid: u32) -> Option<(u32, u32)> {
        let len = to_usize(node.len);
        if len < BLOCK_ENTRIES {
            insert_at(&mut node.keys, len, at, key);
            insert_at(&mut node.kids, len, at, kid);
            node.len += 1;
            self.put_node(id, node);
            return None;
        }
        let (mut keys, mut kids) = ([0; BLOCK_ENTRIES + 1], [0; BLOCK_ENTRIES + 1]);
        fill(&mut keys, &node.keys);
        fill(&mut kids, &node.kids);
        insert_at(&mut keys, BLOCK_ENTRIES, at, key);
        insert_at(&mut kids, BLOCK_ENTRIES, at, kid);
        let (low_keys, high_keys) = keys.split_at(BLOCK_HALF);
        let (low_kids, high_kids) = kids.split_at(BLOCK_HALF);
        let mut left = Node { keys: [0; BLOCK_ENTRIES], kids: [0; BLOCK_ENTRIES], len: to_u32(low_keys.len()) };
        let mut right = Node { keys: [0; BLOCK_ENTRIES], kids: [0; BLOCK_ENTRIES], len: to_u32(high_keys.len()) };
        fill(&mut left.keys, low_keys);
        fill(&mut left.kids, low_kids);
        fill(&mut right.keys, high_keys);
        fill(&mut right.kids, high_kids);
        self.put_node(id, left);
        let right_id = self.new_node(right);
        Some((get(high_keys, 0), right_id))
    }

    /// Removes an entry the list holds, keeping order; a block below half full borrows from or merges with its
    /// neighbour.
    pub fn remove_sorted(&mut self, list: &mut BlockList, key: u32) {
        let (path, depth, leaf_id) = if list.root == NONE {
            violation!(clause = "SET.12", "an entry removed from an empty block list", key = key);
        } else {
            self.descend(*list, key)
        };
        let mut leaf = self.leaf(leaf_id);
        let len = to_usize(leaf.len);
        let Ok(pos) = prefix(&leaf.keys, len).binary_search(&key) else {
            violation!(clause = "SET.12", "an entry removed that a block list does not hold", key = key);
        };
        remove_at(&mut leaf.keys, len, pos);
        leaf.len -= 1;
        list.len -= 1;
        if depth == 0 {
            if leaf.len == 0 {
                self.drop_leaf(leaf_id);
                *list = BlockList::EMPTY;
            } else {
                self.put_leaf(leaf_id, leaf);
            }
            return;
        }
        self.put_leaf(leaf_id, leaf);
        let mut new_min = (pos == 0).then(|| get(&leaf.keys, 0));
        let mut under = to_usize(leaf.len) < BLOCK_HALF;
        for (level, &(node_id, idx)) in path.iter().enumerate().take(depth).rev() {
            if !under && new_min.is_none() {
                break;
            }
            let mut node = self.node(node_id);
            if let Some(m) = new_min {
                set(&mut node.keys, idx, m);
                new_min = new_min.filter(|_| idx == 0);
            }
            if under {
                let (left, right) = if idx + 1 < to_usize(node.len) { (idx, idx + 1) } else { (idx - 1, idx) };
                if level + 1 == depth {
                    self.rebalance_leaves(&mut node, left, right);
                } else {
                    self.rebalance_nodes(&mut node, left, right);
                }
            }
            under = to_usize(node.len) < BLOCK_HALF;
            self.put_node(node_id, node);
        }
        while list.height > 0 && self.node(list.root).len == 1 {
            let old = list.root;
            list.root = get(&self.node(old).kids, 0);
            list.height -= 1;
            self.drop_node(old);
        }
    }

    fn rebalance_leaves(&mut self, parent: &mut Node, left: usize, right: usize) {
        let (lid, rid) = (get(&parent.kids, left), get(&parent.kids, right));
        let (mut low, mut high) = (self.leaf(lid), self.leaf(rid));
        let (n_low, n_high) = (to_usize(low.len), to_usize(high.len));
        if n_low + n_high <= BLOCK_ENTRIES {
            fill_at(&mut low.keys, n_low, prefix(&high.keys, n_high));
            low.len += high.len;
            low.next = high.next;
            self.put_leaf(lid, low);
            self.drop_leaf(rid);
            let count = to_usize(parent.len);
            remove_at(&mut parent.keys, count, right);
            remove_at(&mut parent.kids, count, right);
            parent.len -= 1;
            return;
        }
        if n_low < n_high {
            set(&mut low.keys, n_low, get(&high.keys, 0));
            remove_at(&mut high.keys, n_high, 0);
            low.len += 1;
            high.len -= 1;
        } else {
            insert_at(&mut high.keys, n_high, 0, get(&low.keys, n_low - 1));
            low.len -= 1;
            high.len += 1;
        }
        set(&mut parent.keys, right, get(&high.keys, 0));
        self.put_leaf(lid, low);
        self.put_leaf(rid, high);
    }

    fn rebalance_nodes(&mut self, parent: &mut Node, left: usize, right: usize) {
        let (lid, rid) = (get(&parent.kids, left), get(&parent.kids, right));
        let (mut low, mut high) = (self.node(lid), self.node(rid));
        let (n_low, n_high) = (to_usize(low.len), to_usize(high.len));
        if n_low + n_high <= BLOCK_ENTRIES {
            fill_at(&mut low.keys, n_low, prefix(&high.keys, n_high));
            fill_at(&mut low.kids, n_low, prefix(&high.kids, n_high));
            low.len += high.len;
            self.put_node(lid, low);
            self.drop_node(rid);
            let count = to_usize(parent.len);
            remove_at(&mut parent.keys, count, right);
            remove_at(&mut parent.kids, count, right);
            parent.len -= 1;
            return;
        }
        if n_low < n_high {
            set(&mut low.keys, n_low, get(&high.keys, 0));
            set(&mut low.kids, n_low, get(&high.kids, 0));
            remove_at(&mut high.keys, n_high, 0);
            remove_at(&mut high.kids, n_high, 0);
            low.len += 1;
            high.len -= 1;
        } else {
            insert_at(&mut high.keys, n_high, 0, get(&low.keys, n_low - 1));
            insert_at(&mut high.kids, n_high, 0, get(&low.kids, n_low - 1));
            low.len -= 1;
            high.len += 1;
        }
        set(&mut parent.keys, right, get(&high.keys, 0));
        self.put_node(lid, low);
        self.put_node(rid, high);
    }

    /// Adds an entry after the bag's last; a bag holds duplicates.
    pub fn push(&mut self, bag: &mut BlockBag, key: u32) {
        if bag.tail != NONE {
            let mut leaf = self.leaf(bag.tail);
            let len = to_usize(leaf.len);
            if len < BLOCK_ENTRIES {
                set(&mut leaf.keys, len, key);
                leaf.len += 1;
                self.put_leaf(bag.tail, leaf);
                bag.len += 1;
                return;
            }
        }
        let mut keys = [0; BLOCK_ENTRIES];
        set(&mut keys, 0, key);
        let id = self.new_leaf(Leaf { keys, next: NONE, len: 1 });
        if bag.tail == NONE {
            bag.head = id;
        } else {
            let mut tail = self.leaf(bag.tail);
            tail.next = id;
            self.put_leaf(bag.tail, tail);
        }
        bag.tail = id;
        bag.len += 1;
    }

    /// Appends the bag's entries to `out` in the order they were pushed, returns its blocks to the pool and leaves it
    /// empty.
    pub fn drain(&mut self, bag: &mut BlockBag, out: &mut Vec<u32>) {
        let mut id = bag.head;
        while id != NONE {
            let leaf = self.leaf(id);
            out.extend_from_slice(prefix(&leaf.keys, to_usize(leaf.len)));
            self.drop_leaf(id);
            id = leaf.next;
        }
        *bag = BlockBag::EMPTY;
    }

    #[must_use]
    pub fn contains(&self, list: BlockList, key: u32) -> bool {
        if list.root == NONE {
            return false;
        }
        let (_, _, leaf_id) = self.descend(list, key);
        let leaf = self.leaf(leaf_id);
        prefix(&leaf.keys, to_usize(leaf.len)).binary_search(&key).is_ok()
    }

    /// The entries in ascending order.
    pub fn iter(&self, list: BlockList) -> impl Iterator<Item = u32> + '_ {
        let mut first = list.root;
        if first != NONE {
            for _ in 0..list.height {
                first = get(&self.node(first).kids, 0);
            }
        }
        std::iter::successors(Some(first).filter(|f| *f != NONE), |id| Some(self.leaf(*id).next).filter(|n| *n != NONE))
            .flat_map(|id| {
                let leaf = self.leaf(id);
                leaf.keys.into_iter().take(to_usize(leaf.len))
            })
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{BlockBag, BlockList, BlockPool, Leaf, NONE};
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::consts::{BLOCK_ENTRIES, BLOCK_HALF};

    type Heap = HeapBacking<4096>;

    /// Every block but the root at least half full, entries ascending across blocks, and each node's keys the least
    /// entry of its subtree; returns the subtree's least entry and entry count.
    fn check(pool: &BlockPool<Heap>, id: u32, height: u32, root: bool) -> (u32, usize) {
        if height == 0 {
            let leaf: Leaf = pool.leaf(id);
            let len = usize::try_from(leaf.len).unwrap();
            assert!(root || len >= BLOCK_HALF, "leaf {id} holds {len}");
            assert!(leaf.keys[..len].windows(2).all(|w| w[0] < w[1]));
            return (leaf.keys[0], len);
        }
        let node = pool.node(id);
        let len = usize::try_from(node.len).unwrap();
        assert!(len <= BLOCK_ENTRIES && (if root { len >= 2 } else { len >= BLOCK_HALF }));
        let mut total = 0;
        for i in 0..len {
            let (min, n) = check(pool, node.kids[i], height - 1, false);
            assert_eq!(node.keys[i], min);
            total += n;
        }
        (node.keys[0], total)
    }

    #[test]
    fn block_list_sorted() {
        let rounds = if cfg!(miri) { 2_000 } else { 200_000 };
        let mut space = AddressSpace::empty();
        let mut pool: BlockPool<Heap> = BlockPool::new(&mut space, 1 << 16);
        let mut lists = [BlockList::EMPTY; 3];
        let mut model: [Vec<u32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut d = Draws::new(stream_key(Seed::new(5), "blocks"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut tallest = 0;
        for step in 0..rounds {
            let which = usize::try_from(below_u64(&mut d, 3)).unwrap();
            // Grow for the first half, then shrink, so trees gain and lose levels.
            let grow = if step < rounds / 2 { 3 } else { 1 };
            let key = u32::try_from(below_u64(&mut d, 1 << 20)).unwrap();
            let (list, m) = (&mut lists[which], &mut model[which]);
            match m.binary_search(&key) {
                Ok(at) if below_u64(&mut d, 4) >= grow => {
                    pool.remove_sorted(list, key);
                    m.remove(at);
                }
                Err(at) if below_u64(&mut d, 4) < grow => {
                    pool.insert_sorted(list, key);
                    m.insert(at, key);
                }
                Ok(_) | Err(_) if !m.is_empty() => {
                    let at = usize::try_from(below_u64(&mut d, u64::try_from(m.len()).unwrap())).unwrap();
                    pool.remove_sorted(list, m.remove(at));
                }
                _ => {}
            }
            tallest = lists.iter().map(|l| l.height).fold(tallest, |t, h| if h > t { h } else { t });
            if step % 997 == 0 || step + 1 == rounds {
                for (l, m) in lists.iter().zip(&model) {
                    assert_eq!(pool.iter(*l).collect::<Vec<_>>(), *m);
                    assert_eq!(usize::try_from(l.len()).unwrap(), m.len());
                    if l.root != NONE {
                        assert_eq!(check(&pool, l.root, l.height, true).1, m.len());
                    }
                }
            }
        }
        assert!(tallest >= 2, "trees gained levels and lost them again");
        for (l, m) in lists.iter_mut().zip(&model) {
            for k in m {
                assert!(pool.contains(*l, *k));
                pool.remove_sorted(l, *k);
            }
            assert_eq!(*l, BlockList::EMPTY);
        }
    }

    #[test]
    fn deep_lists_split_and_collapse() {
        let n = if cfg!(miri) { 3_000 } else { 100_000 };
        let mut space = AddressSpace::empty();
        let mut pool: BlockPool<Heap> = BlockPool::new(&mut space, 1 << 16);
        let mut list = BlockList::EMPTY;
        for k in (0..n).rev() {
            pool.insert_sorted(&mut list, k * 2);
        }
        assert!(list.height >= 2);
        assert_eq!(check(&pool, list.root, list.height, true).1, usize::try_from(n).unwrap());
        assert!(!pool.contains(list, 3) && pool.contains(list, 4));
        for k in 0..n {
            pool.remove_sorted(&mut list, k * 2);
        }
        assert_eq!(list, BlockList::EMPTY);
    }

    #[test]
    fn bags_keep_order_and_reuse_blocks() {
        let n = if cfg!(miri) { 500 } else { 5_000 };
        let mut space = AddressSpace::empty();
        let mut pool: BlockPool<Heap> = BlockPool::new(&mut space, 1 << 12);
        let mut bags = [BlockBag::EMPTY; 2];
        for k in 0..n {
            pool.push(&mut bags[usize::from(k % 3 == 0)], k % 7);
        }
        let leaves = pool.n_leaves;
        let mut out = Vec::new();
        pool.drain(&mut bags[1], &mut out);
        assert_eq!(out, (0..n).filter(|k| k % 3 == 0).map(|k| k % 7).collect::<Vec<_>>());
        assert_eq!(bags[1], BlockBag::EMPTY);
        for k in 0..n / 3 {
            pool.push(&mut bags[1], k);
        }
        assert_eq!(pool.n_leaves, leaves, "the drained blocks were reused");
        assert_eq!(bags[0].len() + bags[1].len(), n - n.div_ceil(3) + n / 3);
    }
}
