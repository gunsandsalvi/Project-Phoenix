//! Keyed indexes over hand-built member columns: a member is read at a key exactly when its own column puts it there,
//! once, in slot order, whatever came and went before.
#![cfg(test)]

use phx_num::Missing;

use super::Index;
use crate::backing::{AddressSpace, HeapBacking};
use crate::daybuf::DayBuf;
use crate::index_decl::{IndexDecl, Keys, Mode};
use crate::roundtrip::roundtrip;

type Heap = HeapBacking<4096>;

fn decl(mode: Mode) -> IndexDecl {
    IndexDecl {
        name: "owners by zone",
        member_table: "owners",
        key: "zone",
        predicate: Missing::Present("owns units"),
        mode,
        keys: Keys::Dense(8),
        entries: 4_096,
        clause: "SET.12",
    }
}

fn index(mode: Mode) -> Index<Heap> {
    Index::new(&mut AddressSpace::empty(), decl(mode)).unwrap()
}

fn out() -> DayBuf<u32, Heap> {
    DayBuf::new(&mut AddressSpace::empty(), "test.walk", 4_096)
}

/// Each member slot's key now, none for a slot no member holds.
struct Members(Vec<Missing<u64>>);

impl Members {
    fn at(&self, key: u64) -> impl Fn(u32) -> bool + '_ {
        move |m| self.0.get(usize::try_from(m).unwrap()) == Some(&Missing::Present(key))
    }
}

fn walk(ix: &Index<Heap>, members: &Members, key: u64) -> (Vec<u32>, u32) {
    let mut o = out();
    let dropped = ix.members(key, members.at(key), &mut o);
    (o.as_slice().to_vec(), dropped)
}

#[test]
fn insert_then_walk() {
    let mut ix = index(Mode::Lazy);
    let members = Members(vec![Missing::Present(2), Missing::Present(5), Missing::Present(2)]);
    for (m, key) in [(0, 2), (1, 5), (2, 2)] {
        ix.insert(key, m);
    }
    assert_eq!(walk(&ix, &members, 2), (vec![0, 2], 0));
    assert_eq!(walk(&ix, &members, 5), (vec![1], 0));
    assert_eq!(walk(&ix, &members, 3), (vec![], 0), "an empty key reads nothing");
}

#[test]
fn left_counts_dead() {
    let mut ix = index(Mode::Lazy);
    for m in 0..4 {
        ix.insert(1, m);
    }
    ix.left(1);
    assert_eq!((ix.entries(1), ix.dead(1)), (4, 1), "a leaver is counted, not searched for");
}

#[test]
fn ended_member_not_read() {
    let mut ix = index(Mode::Lazy);
    let mut members = Members(vec![Missing::Present(1); 3]);
    for m in 0..3 {
        ix.insert(1, m);
    }
    members.0[1] = Missing::Absent;
    ix.left(1);
    assert_eq!(walk(&ix, &members, 1), (vec![0, 2], 1), "the ended member's entry is dropped by the walk");
}

#[test]
fn reused_slot_read_once() {
    let mut ix = index(Mode::Lazy);
    let members = Members(vec![Missing::Present(4)]);
    ix.insert(4, 0);
    ix.left(4);
    ix.insert(4, 0);
    assert_eq!(walk(&ix, &members, 4), (vec![0], 1), "the slot's new occupant at the same key, once");
}

#[test]
fn reused_slot_other_key_not_read() {
    let mut ix = index(Mode::Lazy);
    let members = Members(vec![Missing::Present(6)]);
    ix.insert(4, 0);
    ix.left(4);
    ix.insert(6, 0);
    assert_eq!(walk(&ix, &members, 4), (vec![], 1), "its new occupant is at another key");
    assert_eq!(walk(&ix, &members, 6), (vec![0], 0));
}

#[test]
fn walk_order_is_slot_order() {
    let mut ix = index(Mode::Lazy);
    let members = Members(vec![Missing::Present(0); 10]);
    for m in [7, 2, 9, 0, 5, 2] {
        ix.insert(0, m);
    }
    assert_eq!(walk(&ix, &members, 0).0, vec![0, 2, 5, 7, 9], "never the order they came in");
}

#[test]
fn compaction_bounds_dead() {
    let mut ix = index(Mode::Lazy);
    let mut members = Members(vec![Missing::Present(3); 100]);
    for m in 0..100 {
        ix.insert(3, m);
    }
    for m in 0..60 {
        members.0[m] = Missing::Absent;
        ix.left(3);
    }
    assert!(ix.needs_compaction(3), "60 dead past 40 live");
    let mut scratch = out();
    ix.compact(3, members.at(3), &mut scratch);
    assert_eq!((ix.entries(3), ix.dead(3)), (40, 0));
    assert!(!ix.needs_compaction(3));
    assert_eq!(walk(&ix, &members, 3).0, (60..100).collect::<Vec<u32>>());
}

#[test]
fn empty_key_draws_missing() {
    let mut ix = index(Mode::Weighted);
    assert_eq!(ix.draw(1, 0), Missing::Absent);
    let pos = ix.insert_weighted(1, 5);
    ix.remove(1, pos);
    assert_eq!(ix.draw(1, 0), Missing::Absent, "a key of total nought draws none");
}

#[test]
fn weighted_update_and_draw() {
    let mut ix = index(Mode::Weighted);
    let (a, b, c) = (ix.insert_weighted(2, 3), ix.insert_weighted(2, 5), ix.insert_weighted(2, 2));
    assert_eq!((a, b, c, ix.total(2)), (0, 1, 2, 10));
    assert_eq!(ix.draw(2, 3), Missing::Present(1));
    ix.update(2, b, 0);
    assert_eq!(ix.draw(2, 3), Missing::Present(2));
    ix.remove(2, a);
    let d = ix.insert_weighted(2, 7);
    assert_eq!(d, 0, "a removed member's position is taken again first");
    assert_eq!(ix.total(2), 9);
}

#[test]
fn count_mode_matches_recount() {
    let mut ix = index(Mode::LazyCounted);
    let mut members = Members(Vec::new());
    for m in 0..30_u32 {
        let key = u64::from(m % 4);
        members.0.push(Missing::Present(key));
        ix.insert(key, m);
    }
    for m in [3_usize, 7, 11] {
        members.0[m] = Missing::Absent;
        ix.left(3);
    }
    for key in 0..4 {
        let recount = members.0.iter().filter(|k| **k == Missing::Present(key)).count();
        assert_eq!(ix.count(key), i64::try_from(recount).unwrap(), "key {key}");
    }
    let mut alone = index(Mode::Count);
    alone.add(5, 3);
    alone.add(5, -1);
    assert_eq!(alone.count(5), 2);
}

/// An owner's members and its index of them by key, rebuilt from the members at load in slot order.
#[derive(Debug, phx_macros::Saved)]
struct Owners {
    keys: Vec<Missing<u64>>,
    #[saved(skip, rebuild = Owners::index_members)]
    by_key: Option<Index<Heap>>,
}

impl Owners {
    fn index_members(&mut self) -> u64 {
        let mut ix = index(Mode::Lazy);
        for (m, key) in (0_u32..).zip(&self.keys) {
            if let Missing::Present(key) = key {
                ix.insert(*key, m);
            }
        }
        self.by_key = Some(ix);
        crate::convert::to_u64(self.keys.len())
    }

    fn walks(&self) -> Vec<Vec<u32>> {
        let members = Members(self.keys.clone());
        (0..8).map(|k| self.by_key.as_ref().map_or_else(Vec::new, |ix| walk(ix, &members, k).0)).collect()
    }
}

impl PartialEq for Owners {
    /// Two owners are equal when their members sit at the same keys and every walk reads the same members.
    fn eq(&self, other: &Owners) -> bool {
        self.keys == other.keys && self.walks() == other.walks()
    }
}

#[test]
fn rebuild_equals_incremental() {
    let mut kept = Owners { keys: Vec::new(), by_key: Some(index(Mode::Lazy)) };
    for m in 0..200_u32 {
        let key = u64::from(m * 7 % 8);
        kept.keys.push(Missing::Present(key));
        kept.by_key.as_mut().unwrap().insert(key, m);
    }
    // The day moved some members and ended others; the index kept only the events.
    for m in (0..200_usize).step_by(9) {
        let old = kept.keys[m];
        let Missing::Present(from) = old else { continue };
        let ix = kept.by_key.as_mut().unwrap();
        ix.left(from);
        if m % 2 == 0 {
            kept.keys[m] = Missing::Absent;
        } else {
            let to = (from + 1) % 8;
            kept.keys[m] = Missing::Present(to);
            ix.insert(to, u32::try_from(m).unwrap());
        }
    }
    let (_, rebuilt) = roundtrip(&kept).unwrap();
    assert_eq!(rebuilt.stores(), [("Owners.by_key", 200)]);
}

#[test]
fn sparse_keys_found_across_merges() {
    let sparse = IndexDecl { keys: Keys::Sparse(8_800_000), entries: 20_000, ..decl(Mode::Lazy) };
    let mut ix: Index<Heap> = Index::new(&mut AddressSpace::empty(), sparse).unwrap();
    // Keys among millions, more new pairs than a run holds, so some merge before the close and some after.
    let key = |i: u32| u64::from(i) * 7_919 % 8_800_000;
    let mut members = Members(Vec::new());
    for i in 0..10_000_u32 {
        members.0.push(Missing::Present(key(i)));
        ix.insert(key(i), i);
    }
    for i in (0..10_000_u32).step_by(997) {
        assert_eq!(walk(&ix, &members, key(i)).0, vec![i], "key of member {i} before the close");
    }
    ix.settle();
    for i in (0..10_000_u32).step_by(631) {
        assert_eq!(walk(&ix, &members, key(i)).0, vec![i], "key of member {i} after the close");
    }
    assert_eq!(walk(&ix, &members, 3).0, Vec::<u32>::new(), "a key holding no pair reads nothing");
    let sparse_weights = IndexDecl { keys: Keys::Sparse(10), ..decl(Mode::Weighted) };
    assert!(Index::<Heap>::new(&mut AddressSpace::empty(), sparse_weights).is_err(), "weights need dense keys");
}

#[test]
fn sparse_pairs_compacted_at_the_close() {
    let sparse = IndexDecl { keys: Keys::Sparse(1_000), entries: 100, ..decl(Mode::Lazy) };
    let mut ix: Index<Heap> = Index::new(&mut AddressSpace::empty(), sparse).unwrap();
    let mut members = Members(Vec::new());
    for m in 0..20_u32 {
        members.0.push(Missing::Present(u64::from(m * 37)));
        ix.insert(u64::from(m * 37), m);
    }
    for m in 0..12_usize {
        let Missing::Present(from) = members.0[m] else { continue };
        ix.left(from);
        members.0[m] = Missing::Absent;
    }
    assert!(ix.pairs_need_compaction(), "12 leavers past 8 live");
    ix.compact_pairs(|key, m| members.at(key)(m));
    assert_eq!(ix.dead_and_entries(), (0, 8));
    assert_eq!(walk(&ix, &members, 19 * 37).0, vec![19]);
}

#[test]
fn opening_run_read_through_columns() {
    let mut ix = index(Mode::LazyCounted);
    // The opening placed key 2's members in slots 10 to 19, and key 3's in 20 to 24.
    let mut members = Members((0..30).map(|s| Missing::Present(if s < 20 { 2 } else { 3 })).collect());
    for s in 0..10 {
        members.0[s] = Missing::Present(1);
    }
    ix.place_run(2, 10, 10);
    ix.place_run(3, 20, 5);
    assert_eq!(walk(&ix, &members, 2).0, (10..20).collect::<Vec<u32>>());
    assert_eq!(ix.count(2), 10);
    // Slot 12 ended and its slot came back at key 3; slot 15 moved to key 3; slot 4 came to key 2.
    members.0[12] = Missing::Present(3);
    members.0[15] = Missing::Present(3);
    members.0[4] = Missing::Present(2);
    ix.left(2);
    ix.left(2);
    ix.insert(3, 12);
    ix.insert(3, 15);
    ix.insert(2, 4);
    let (key2, dropped) = walk(&ix, &members, 2);
    assert_eq!(key2, [4, 10, 11, 13, 14, 16, 17, 18, 19], "the run's leavers dropped, the newcomer read");
    assert_eq!(dropped, 2);
    assert_eq!(walk(&ix, &members, 3).0, [12, 15, 20, 21, 22, 23, 24], "the reused and moved slots read once");
    let mut scratch = out();
    ix.compact(2, members.at(2), &mut scratch);
    assert_eq!((ix.entries(2), ix.dead(2)), (11, 2), "the run kept, its two holes dead, the newcomer listed");
    assert_eq!(walk(&ix, &members, 2).0, key2);
}
