//! Sum-trees against plain prefix sums over hand-built weights.
#![cfg(test)]

use phx_num::Missing;

use super::SumTrees;
use crate::backing::{AddressSpace, HeapBacking};
use crate::roundtrip::roundtrip;

type Heap = HeapBacking<4096>;

fn pool() -> SumTrees<Heap> {
    SumTrees::new(&mut AddressSpace::empty(), 1 << 16, 64)
}

fn weights(n: u64) -> Vec<u64> {
    (0..n).map(|i| (i * 7_919) % 23).collect()
}

/// The least member whose weights through its own exceed `x`, read off plain prefix sums.
fn plain_find(w: &[u64], x: u64) -> Option<usize> {
    let mut sum = 0;
    w.iter().position(|v| {
        sum += v;
        sum > x
    })
}

#[test]
fn prefix_sums_match_plain() {
    let mut t = pool();
    let tree = t.make();
    let w = weights(300);
    for v in &w {
        let _ = t.push(tree, *v);
    }
    for i in 0..=w.len() {
        assert_eq!(t.prefix_before(tree, i), w[..i].iter().sum::<u64>(), "prefix of {i}");
    }
    assert_eq!(t.total(tree), w.iter().sum::<u64>());
}

#[test]
fn find_is_exact_inverse_of_prefix() {
    let mut t = pool();
    let tree = t.make();
    let w = weights(137);
    for v in &w {
        let _ = t.push(tree, *v);
    }
    for x in 0..t.total(tree) {
        assert_eq!(t.find(tree, x), Missing::Present(plain_find(&w, x).unwrap()), "x = {x}");
    }
}

#[test]
fn find_on_zero_total_is_missing() {
    let mut t = pool();
    let empty = t.make();
    assert_eq!(t.find(empty, 0), Missing::Absent);
    let zeros = t.make();
    let _ = t.push(zeros, 5);
    t.set(zeros, 0, 0);
    assert_eq!(t.find(zeros, 0), Missing::Absent, "every member sold out: no draw");
}

#[test]
fn total_overflow_stops() {
    let mut t = pool();
    let tree = t.make();
    let _ = t.push(tree, u64::MAX - 1);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.push(tree, 2)));
    assert!(caught.is_err());
}

#[test]
fn class_growth_keeps_sums() {
    let mut trees = pool();
    let (first, second) = (trees.make(), trees.make());
    let w = weights(70);
    for v in &w {
        let _ = trees.push(first, *v);
        let _ = trees.push(second, v + 1);
    }
    for (i, v) in w.iter().enumerate() {
        assert_eq!(trees.weight(first, i), *v);
        assert_eq!(trees.weight(second, i), v + 1, "trees growing side by side keep their own weights");
    }
    let third = trees.make();
    let _ = trees.push(third, 9);
    assert_eq!(trees.weight(third, 0), 9, "a freed extent taken again starts empty");
}

#[test]
fn removal_keeps_positions() {
    let mut t = pool();
    let tree = t.make();
    let mut w = weights(40);
    for v in &w {
        let _ = t.push(tree, *v);
    }
    for i in [3, 17, 39] {
        t.set(tree, i, 0);
        w[i] = 0;
    }
    t.set(tree, 5, 100);
    w[5] = 100;
    assert_eq!(t.len(tree), 40, "a removal keeps every member's place");
    for x in (0..t.total(tree)).step_by(7) {
        assert_eq!(t.find(tree, x), Missing::Present(plain_find(&w, x).unwrap()));
    }
}

/// An owner's rows and its trees of their weights, rebuilt from the rows after a load.
#[derive(Debug, PartialEq, phx_macros::Saved)]
struct Stalls {
    rows: Vec<(u32, u64)>,
    #[saved(skip, rebuild = Stalls::grow_trees)]
    trees: SumTrees,
}

impl Stalls {
    fn grow_trees(&mut self) -> u64 {
        let keys = self.rows.iter().fold(0, |k, r| if r.0 + 1 > k { r.0 + 1 } else { k });
        let mut trees = SumTrees::new(&mut AddressSpace::empty(), 1 << 16, keys);
        for _ in 0..keys {
            let _ = trees.make();
        }
        for (key, w) in &self.rows {
            let _ = trees.push(*key, *w);
        }
        self.trees = trees;
        crate::convert::to_u64(self.rows.len())
    }
}

#[test]
fn rebuild_equals_incremental() {
    let rows: Vec<(u32, u64)> = (0..500_u32).map(|i| (i % 9, u64::from(i * 31 % 17))).collect();
    let mut kept = Stalls { rows: Vec::new(), trees: SumTrees::new(&mut AddressSpace::empty(), 1 << 16, 9) };
    for _ in 0..9 {
        let _ = kept.trees.make();
    }
    // The day kept the trees as rows came, and set some sold out; the rows hold the weights as they stand.
    for (key, w) in &rows {
        let _ = kept.trees.push(*key, *w);
        kept.rows.push((*key, *w));
    }
    let (back, rebuilt) = roundtrip(&kept).unwrap();
    assert_eq!(rebuilt.stores(), [("Stalls.trees", 500)]);
    assert_eq!(back.trees.total(4), kept.trees.total(4));
}
