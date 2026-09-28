//! A few households' persons: set, joined, removed in order, cleared, counted, found by identity, and kept through
//! compaction.
#![cfg(test)]

use phx_id::Slot;
use phx_store::backing::{AddressSpace, SystemBacking};

use super::{Held, Persons};

const CAPACITY: u32 = 64;
const ROWS_PER_CHUNK: u32 = 4;
const PERSONS_AFTER: u64 = 5;
const MOVED: u64 = 100;

fn store(space: &mut AddressSpace) -> Persons<SystemBacking> {
    Persons::new(space, CAPACITY, ROWS_PER_CHUNK)
}

/// Persons whose word and identity are both `n`, so each is seen whole.
fn held(ns: &[u64]) -> Vec<Held> {
    ns.iter().map(|n| Held { word: *n, id: *n }).collect()
}

fn ids(p: &Persons<SystemBacking>, s: u32) -> Vec<u64> {
    p.of(Slot::new(s)).map(|h| h.id).collect()
}

#[test]
fn a_households_persons_are_its_own() {
    let mut space = AddressSpace::empty();
    let mut p = store(&mut space);
    p.set(&mut space, Slot::new(5), &held(&[10, 11]));
    p.set(&mut space, Slot::new(0), &held(&[20]));
    p.push(&mut space, Slot::new(5), Held { word: 12, id: 12 });
    assert_eq!(ids(&p, 5), [10, 11, 12]);
    assert_eq!(ids(&p, 0), [20]);
    assert_eq!(p.count(Slot::new(3)), 0, "a household never formed holds none");
    assert_eq!(p.held(), 4);
    p.remove(&mut space, Slot::new(5), 0);
    assert_eq!(ids(&p, 5), [11, 12], "the rest keep their order");
    assert_eq!(p.place_of(Slot::new(5), 12), Some(1), "a person found by its identity");
    assert_eq!(p.place_of(Slot::new(5), 10), None, "one who left is not found");
    p.clear(&mut space, Slot::new(0));
    assert_eq!(p.held(), 2);
    p.set(&mut space, Slot::new(0), &held(&[30, 31, 32]));
    assert_eq!(ids(&p, 0), [30, 31, 32], "a slot begun again takes its own persons");
    assert_eq!(p.held(), PERSONS_AFTER);
}

#[test]
fn a_persons_word_and_identity_stay_together() {
    let mut space = AddressSpace::empty();
    let mut p = store(&mut space);
    p.set(&mut space, Slot::new(1), &[Held { word: 7, id: 70 }, Held { word: 8, id: 80 }]);
    p.remove(&mut space, Slot::new(1), 0);
    assert_eq!(p.of(Slot::new(1)).collect::<Vec<_>>(), [Held { word: 8, id: 80 }]);
}

#[test]
fn compaction_keeps_every_households_persons() {
    let mut space = AddressSpace::empty();
    let mut p = store(&mut space);
    for s in 0..ROWS_PER_CHUNK {
        p.set(&mut space, Slot::new(s), &held(&[u64::from(s); 3]));
    }
    for s in 0..ROWS_PER_CHUNK {
        p.set(&mut space, Slot::new(s), &held(&[u64::from(s) + MOVED; 5]));
    }
    let _ = p.compact_due(&mut space);
    for s in 0..ROWS_PER_CHUNK {
        assert_eq!(ids(&p, s), [u64::from(s) + MOVED; 5]);
    }
    assert_eq!(p.held(), u64::from(ROWS_PER_CHUNK) * PERSONS_AFTER);
}
