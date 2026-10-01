//! The directory over hand-begun parties: a reused slot refusing the old generation, an ended party resolving to its
//! day and successor, chains followed, slots resting their day, tombstones merged in order and dropped past the
//! horizon, generations bounded, the runs the same in any order, a mass ending in one merge, the opening at
//! generation zero, and the directory round-tripping.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::{Day, PartyRef, Slot};
use phx_num::Missing;
use phx_store::{AddressSpace, HeapBacking, StoreStats};

use super::{Directory, Resolved};

type Heap = HeapBacking<4096>;

const FIRMS: u8 = 1;
const ESTATES: u8 = 2;

fn directory(horizon: u32) -> Directory<Heap> {
    Directory::new(&mut AddressSpace::empty(), &[64, 64, 64], 16, (Day::new(0), horizon))
}

fn ended(day: u32, successor: Missing<PartyRef>) -> Resolved {
    Resolved::Ended { day: Day::new(day), successor }
}

#[test]
fn opening_begins_at_generation_zero() {
    let mut d = directory(100);
    let a = d.begin(FIRMS);
    let b = d.begin(FIRMS);
    assert_eq!((a.generation(), a.slot(), b.slot()), (0, Slot::new(0), Slot::new(1)));
    assert_eq!((d.live(FIRMS), d.tombstones()), (2, 0), "day zero holds no tombstone");
}

#[test]
fn stale_generation_refused() {
    let mut d = directory(100);
    let old = d.begin(FIRMS);
    d.end(old, Day::new(3), Missing::Absent);
    let _ = d.close_day(Day::new(3));
    let new = d.begin(FIRMS);
    assert_eq!((new.slot(), new.generation()), (old.slot(), 1), "the slot taken again at its next generation");
    assert_eq!(d.resolve(new), Resolved::Live(new.slot()));
    assert_eq!(d.resolve(old), ended(3, Missing::Absent), "the old reference reads its own party, ended");
}

#[test]
fn ended_resolves_to_day_and_successor() {
    let mut d = directory(100);
    let firm = d.begin(FIRMS);
    let estate = d.begin(ESTATES);
    d.end(firm, Day::new(7), Missing::Present(estate));
    assert_eq!(d.resolve(firm), ended(7, Missing::Present(estate)), "ended at once, before the close");
    let _ = d.close_day(Day::new(7));
    assert_eq!(d.resolve(firm), ended(7, Missing::Present(estate)));
    assert!(catch_unwind(AssertUnwindSafe(|| d.end(firm, Day::new(8), Missing::Absent))).is_err(), "ended once");
}

#[test]
fn successor_chain_followed_to_live() {
    let mut d = directory(100);
    let firm = d.begin(FIRMS);
    let estate = d.begin(ESTATES);
    d.end(firm, Day::new(2), Missing::Present(estate));
    let heir = d.begin(FIRMS);
    d.end(estate, Day::new(5), Missing::Present(heir));
    let _ = d.close_day(Day::new(5));
    assert_eq!(d.follow(firm), (heir, Resolved::Live(heir.slot())), "through the estate to its heir");
}

#[test]
fn successor_chain_ends_at_distributed_estate() {
    let mut d = directory(100);
    let firm = d.begin(FIRMS);
    let estate = d.begin(ESTATES);
    d.end(firm, Day::new(2), Missing::Present(estate));
    d.end(estate, Day::new(9), Missing::Absent);
    assert_eq!(d.follow(firm), (estate, ended(9, Missing::Absent)), "an estate distributed to no one ends the chain");
    let heir = d.begin(FIRMS);
    d.set_successor(estate, heir);
    assert_eq!(d.follow(firm).0, heir, "an heir named after the estate ended");
}

#[test]
fn slot_not_reused_within_its_day() {
    let mut d = directory(100);
    let a = d.begin(FIRMS);
    d.end(a, Day::new(1), Missing::Absent);
    let b = d.begin(FIRMS);
    assert_ne!(b.slot(), a.slot(), "the slot rests until the day closes");
    let _ = d.close_day(Day::new(1));
    assert_eq!(d.begin(FIRMS).slot(), a.slot(), "and is the first handed out after");
}

#[test]
fn pruned_past_horizon_reads_ended_beyond_horizon() {
    let mut d = directory(10);
    let early: Vec<PartyRef> = (0..40).map(|_| d.begin(FIRMS)).collect();
    for (i, p) in (1..).zip(&early) {
        d.end(*p, Day::new(i), Missing::Absent);
        let _ = d.close_day(Day::new(i));
    }
    assert!(d.tombstones() < 40, "tombstones flat once the horizon has passed");
    assert_eq!(d.resolve(early[0]), Resolved::EndedBeyondHorizon);
    assert_eq!(d.resolve(early[39]), ended(40, Missing::Absent), "the last within the horizon");
}

#[test]
fn recent_merge_keeps_order_and_prunes() {
    let mut d = directory(1_000);
    let parties: Vec<PartyRef> = (0..60).map(|_| d.begin(FIRMS)).collect();
    for (i, p) in (0..).zip(parties.iter().rev()) {
        d.end(*p, Day::new(1 + i / 10), Missing::Absent);
        if i % 10 == 9 {
            let _ = d.close_day(Day::new(1 + i / 10));
        }
    }
    assert!(d.tombs.main.windows(2).all(|w| w[0].key() < w[1].key()), "the main run in reference order");
    assert!(d.tombs.recent.windows(2).all(|w| w[0].key() < w[1].key()));
    for (i, p) in (0..).zip(parties.iter().rev()) {
        assert_eq!(d.resolve(*p), ended(1 + i / 10, Missing::Absent));
    }
}

#[test]
fn generation_overflow_stops() {
    let mut d = directory(100);
    assert!(
        catch_unwind(AssertUnwindSafe(|| d.end(
            PartyRef::new(FIRMS, 0, Slot::new(0)),
            Day::new(70_000),
            Missing::Absent
        )))
        .is_err()
    );
    let p = d.begin(FIRMS);
    let late = catch_unwind(AssertUnwindSafe(|| d.end(p, Day::new(65_536), Missing::Absent)));
    assert!(late.is_err(), "a run past 65 535 days");
    assert!(catch_unwind(|| PartyRef::new(FIRMS, 1 << 24, Slot::new(0))).is_err(), "the 2^24th generation");
}

#[test]
fn unissued_generation_stops() {
    let mut d = directory(100);
    let p = d.begin(FIRMS);
    let ahead = PartyRef::new(FIRMS, p.generation() + 1, p.slot());
    assert!(catch_unwind(AssertUnwindSafe(|| d.resolve(ahead))).is_err(), "a generation not yet issued");
    let never = PartyRef::new(FIRMS, 0, Slot::new(9));
    assert!(catch_unwind(AssertUnwindSafe(|| d.resolve(never))).is_err(), "a slot never handed out");
}

#[test]
fn endings_in_any_chunk_order_same_runs() {
    // The day's endings are sorted by reference at the close, so the order the chunks wrote them in changes nothing.
    let (mut a, mut b) = (directory(100), directory(100));
    let pa: Vec<PartyRef> = (0..20).map(|_| a.begin(FIRMS)).collect();
    let pb: Vec<PartyRef> = (0..20).map(|_| b.begin(FIRMS)).collect();
    for p in &pa {
        a.end(*p, Day::new(4), Missing::Absent);
    }
    for p in pb.iter().rev() {
        b.end(*p, Day::new(4), Missing::Absent);
    }
    let _ = (a.close_day(Day::new(4)), b.close_day(Day::new(4)));
    assert_eq!((&a.tombs.main, &a.tombs.recent), (&b.tombs.main, &b.tombs.recent));
}

#[test]
fn mass_endings_one_merge() {
    let mut d: Directory<Heap> = Directory::new(&mut AddressSpace::empty(), &[2_000, 2_000], 64, (Day::new(0), 100));
    let parties: Vec<PartyRef> = (0..2_000).map(|_| d.begin(FIRMS)).collect();
    for p in &parties {
        d.end(*p, Day::new(1), Missing::Absent);
    }
    let read = d.close_day(Day::new(1));
    assert_eq!(
        (read, d.tombs.main.len(), d.tombs.recent.len()),
        (2_000, 2_000, 0),
        "the day's endings joined in one merge"
    );
    assert_eq!(d.live(FIRMS), 0);
    assert!(parties.iter().all(|p| d.resolve(*p) == ended(1, Missing::Absent)), "each found through the fence");
}

#[test]
fn save_round_trip_resolves_alike() {
    let mut dir = directory(100);
    let (first, second, estate) = (dir.begin(FIRMS), dir.begin(FIRMS), dir.begin(ESTATES));
    dir.end(first, Day::new(2), Missing::Present(estate));
    let _ = dir.close_day(Day::new(2));
    dir.end(second, Day::new(3), Missing::Absent);
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&dir, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let back: Directory<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    for party in [first, second, estate] {
        assert_eq!(back.resolve(party), dir.resolve(party));
    }
    assert_eq!((back.rows_live(), back.tombstones()), (dir.rows_live(), dir.tombstones()));
}
