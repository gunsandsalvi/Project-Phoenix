//! A few households' persons: set, joined, removed in order, cleared, counted, and kept through compaction.
#![cfg(test)]

use phx_id::Slot;
use phx_store::backing::{AddressSpace, SystemBacking};

use super::Persons;

const CAPACITY: u32 = 64;
const ROWS_PER_CHUNK: u32 = 4;

fn store(space: &mut AddressSpace) -> Persons<SystemBacking> {
    Persons::new(space, CAPACITY, ROWS_PER_CHUNK)
}

#[test]
fn a_households_persons_are_its_own() {
    let mut space = AddressSpace::empty();
    let mut p = store(&mut space);
    p.set(&mut space, Slot::new(5), &[10, 11]);
    p.set(&mut space, Slot::new(0), &[20]);
    p.push(&mut space, Slot::new(5), 12);
    assert_eq!(p.of(Slot::new(5)), &[10, 11, 12]);
    assert_eq!(p.of(Slot::new(0)), &[20]);
    assert!(p.of(Slot::new(3)).is_empty(), "a household never formed holds none");
    assert_eq!(p.held(), 4);
    p.remove(&mut space, Slot::new(5), 0);
    assert_eq!(p.of(Slot::new(5)), &[11, 12], "the rest keep their order");
    p.clear(&mut space, Slot::new(0));
    assert_eq!(p.held(), 2);
    p.set(&mut space, Slot::new(0), &[30, 31, 32]);
    assert_eq!(p.of(Slot::new(0)), &[30, 31, 32], "a slot begun again takes its own persons");
    assert_eq!(p.held(), 5);
}

#[test]
fn compaction_keeps_every_households_persons() {
    let mut space = AddressSpace::empty();
    let mut p = store(&mut space);
    for s in 0..ROWS_PER_CHUNK {
        p.set(&mut space, Slot::new(s), &[u64::from(s); 3]);
    }
    for s in 0..ROWS_PER_CHUNK {
        p.set(&mut space, Slot::new(s), &[u64::from(s) + 100; 5]);
    }
    let _ = p.compact_due(&mut space);
    for s in 0..ROWS_PER_CHUNK {
        assert_eq!(p.of(Slot::new(s)), &[u64::from(s) + 100; 5]);
    }
    assert_eq!(p.held(), u64::from(ROWS_PER_CHUNK) * 5);
}
