//! Keys and counters over hand-given names and addresses: a draw depends on its address alone, families never share a
//! key, and a stream keeps no position.
#![cfg(test)]

use phx_num::CapacityExceeded;

use super::{Seed, StreamFamily, Subject, SubjectTag, counter, family_key, stream_key};
use crate::draws::{Draws, SlotOrdinal};

#[test]
fn stream_keys_independent_of_registration() {
    let seed = Seed::new(42);
    let in_order: Vec<_> = ["A", "B", "C"].iter().map(|n| stream_key(seed, n)).collect();
    let reordered: Vec<_> = ["C", "A", "B", "D"].iter().map(|n| stream_key(seed, n)).collect();
    assert_eq!(in_order, vec![reordered[1], reordered[2], reordered[0]]);
    assert_ne!(stream_key(seed, "A"), stream_key(Seed::new(43), "A"));
    assert_ne!(stream_key(seed, "A"), stream_key(seed, "B"));
}

#[test]
fn adding_stream_changes_no_other() {
    // A stream's key is its own name's and the seed's: none of the others declared enters it.
    let seed = Seed::new(7);
    let alone = stream_key(seed, "DEM.mortality");
    for others in [&["DEM.illness"][..], &["DEM.illness", "TEC.discovery", "OBS.tracer"][..]] {
        let keys: Vec<_> = others.iter().chain(&["DEM.mortality"]).map(|n| stream_key(seed, n)).collect();
        assert_eq!(keys.last(), Some(&alone));
    }
}

#[test]
fn families_never_share_a_key() {
    let seed = Seed::new(5);
    let families = [StreamFamily::World, StreamFamily::Observer, StreamFamily::Advice];
    for name in ["DEM.mortality", "OBS.tracer", "a", ""] {
        let keys: Vec<_> = families.iter().map(|f| family_key(seed, *f, name)).collect();
        assert!(keys[0] != keys[1] && keys[1] != keys[2] && keys[0] != keys[2], "{name}");
    }
    // The world's keys are its names' and the seed's alone, as before families.
    assert_eq!(family_key(seed, StreamFamily::World, "DEM.mortality"), stream_key(seed, "DEM.mortality"));
}

#[test]
fn draw_depends_on_address_only() {
    let key = stream_key(Seed::new(9), "DEM.mortality");
    let at = |subject: u64, day: u32, slot: u32| {
        let mut d = Draws::at(key, Subject::new(SubjectTag::Party, subject), day, SlotOrdinal::new(slot));
        d.next_u64()
    };
    // Read in any order, an address gives its draw.
    let forward: Vec<u64> = (0..50).map(|s| at(s, 3, 4)).collect();
    let backward: Vec<u64> = (0..50).rev().map(|s| at(s, 3, 4)).collect();
    assert_eq!(forward, backward.into_iter().rev().collect::<Vec<_>>());
    assert_ne!(at(1, 3, 4), at(2, 3, 4));
    assert_ne!(at(1, 3, 4), at(1, 4, 4));
    assert_ne!(at(1, 3, 4), at(1, 3, 5));
    assert_ne!(
        at(1, 3, 4),
        Draws::at(stream_key(Seed::new(9), "x"), Subject::new(SubjectTag::Party, 1), 3, SlotOrdinal::new(4)).next_u64()
    );
}

#[test]
fn no_stream_state_saved() {
    // A cursor is its address: one opened at a later block gives what one read up to it would, so nothing of a
    // stream's position needs saving.
    let key = stream_key(Seed::new(2), "SRV.taste");
    let subject = Subject::new(SubjectTag::Party, 11);
    let mut read = Draws::new(key, subject, 8, 6);
    for _ in 0..3 * 4 {
        let _ = read.next_u32();
    }
    let mut opened = Draws::from_block(key, subject, 8, 6, 3);
    assert_eq!(
        (0..8).map(|_| read.next_u32()).collect::<Vec<_>>(),
        (0..8).map(|_| opened.next_u32()).collect::<Vec<_>>()
    );
}

#[test]
fn slot_ordinal_width_stops() {
    assert_eq!(SlotOrdinal::new(255).get(), 255);
    let Err(payload) = std::panic::catch_unwind(|| SlotOrdinal::new(256)) else { panic!("256 accepted") };
    let c = payload.downcast_ref::<CapacityExceeded>().expect("a capacity payload");
    assert_eq!((c.declared, c.needed), (256, 257));
}

#[test]
fn subjects_read_back() {
    let s = Subject::new(SubjectTag::Party, 77);
    assert_eq!((Subject::from_raw(s.raw()), s.tag(), s.id()), (Some(s), SubjectTag::Party, 77));
    assert_eq!(Subject::from_raw(u64::MAX), None);
}

#[test]
fn subjects_never_collide() {
    let party = Subject::new(SubjectTag::Party, 7);
    let line = Subject::new(SubjectTag::Line, 7);
    assert_ne!(counter(party, 3, 1, 0), counter(line, 3, 1, 0));
    let caught = std::panic::catch_unwind(|| Subject::new(SubjectTag::World, 1 << 60));
    assert!(caught.is_err());
}

#[test]
fn substeps_never_collide() {
    let s = Subject::new(SubjectTag::Party, 9);
    assert_ne!(counter(s, 3, 1, 0), counter(s, 3, 2, 0));
    assert_ne!(counter(s, 3, 1, 0), counter(s, 4, 1, 0));
}
