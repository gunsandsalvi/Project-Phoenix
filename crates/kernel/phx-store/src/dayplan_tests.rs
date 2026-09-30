//! Buffers whose lives meet never share bytes, and the plan's region is the day's largest live set where the lives
//! allow it.
#![cfg(test)]

use super::{BufDecl, DayPlan, DayRegion, Life, Size};
use crate::backing::{AddressSpace, HeapBacking};

const MB: u64 = 100_000;

/// The day's slots, 1a to 10e, as the stage table runs them.
const SLOTS: [&str; 41] = [
    "1a", "1b", "2a", "2b", "2c", "2d", "2e", "2f", "3a", "3b", "4a", "4b", "4c", "5a", "5b", "5c", "5d", "6a", "6b",
    "6c", "6d", "7a", "7b", "7c", "8a", "8b", "8c", "8d", "8e", "8f", "9a", "9b", "9c", "9d", "10a", "10b", "10c",
    "10d", "10e", "close", "after",
];

fn at(slot: &str) -> u16 {
    u16::try_from(SLOTS.iter().position(|s| *s == slot).unwrap()).unwrap()
}

fn slots() -> u16 {
    u16::try_from(SLOTS.len()).unwrap()
}

fn decl(name: &'static str, tenths: u64, fill: &str, release: &str) -> BufDecl {
    BufDecl { name, size: Size::Bytes(tenths * MB), life: Life { fill: at(fill), release: at(release) } }
}

/// The heaviest day's buffers, in tenths of a megabyte, as the owners' steps state them.
fn heaviest() -> Vec<BufDecl> {
    vec![
        decl("wants", 675, "5b", "6b"),
        decl("between-firm", 396, "5b", "10e"),
        decl("intents", 288, "5b", "5d"),
        decl("write-back", 40, "5b", "5c"),
        decl("spend", 180, "6a", "6b"),
        decl("meeting", 160, "6a", "6a"),
        decl("prints", 80, "6a", "10e"),
        decl("flush 2b", 48, "2b", "2b"),
        decl("flush 5d", 48, "5d", "5d"),
        decl("flush 6b", 48, "6b", "6b"),
        decl("flush 6d", 48, "6d", "7c"),
        BufDecl { name: "dues", size: Size::Rest, life: Life { fill: at("2b"), release: at("2b") } },
        decl("decided", 200, "6d", "7b"),
        decl("short", 40, "7b", "7b"),
        decl("tallies", 110, "1a", "10e"),
    ]
}

fn overlap(a: (u64, u64), b: (u64, u64)) -> bool {
    a.0 < b.0 + b.1 && b.0 < a.0 + a.1
}

#[test]
fn plan_never_shares_overlapping_lives() {
    let plan = DayPlan::plan(&heaviest(), slots()).unwrap();
    for a in plan.placed() {
        for b in plan.placed().iter().filter(|b| b.decl != a.decl && b.life.meets(a.life)) {
            assert!(!overlap((a.offset, a.bytes), (b.offset, b.bytes)), "{} and {} share bytes", a.decl, b.decl);
        }
    }
}

#[test]
fn plan_peak_is_max_live_set() {
    let plan = DayPlan::plan(&heaviest(), slots()).unwrap();
    assert_eq!(plan.peak_live(slots()), 1601 * MB, "the day's maximum is the 6a live set");
    assert_eq!(plan.extent(), plan.peak_live(slots()), "the region is the largest live set, not the sum");
    let dues = plan.placed().iter().find(|p| p.decl == 11).unwrap();
    assert_eq!(dues.bytes, (1601 - 48 - 110) * MB, "the dues' lane is what 2b's other buffers leave");
}

#[test]
fn plan_same_for_any_workers() {
    // The plan reads the declarations alone, so wherever and however often it is made it places every buffer alike.
    let a = DayPlan::plan(&heaviest(), slots()).unwrap();
    let b = DayPlan::plan(&heaviest(), slots()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn read_after_release_refused() {
    let plan = DayPlan::plan(&heaviest(), slots()).unwrap();
    plan.check_read(0, at("6a"));
    if cfg!(debug_assertions) {
        let caught = std::panic::catch_unwind(|| plan.check_read(0, at("7a")));
        assert!(caught.is_err(), "the wants are read after their release at 6b");
    }
}

#[test]
fn plan_is_build_data() {
    let bad = [BufDecl { name: "backwards", size: Size::Bytes(1), life: Life { fill: 3, release: 2 } }];
    assert!(DayPlan::plan(&bad, slots()).is_err(), "a life that ends before it begins");
    let two = [
        BufDecl { name: "a", size: Size::Rest, life: Life { fill: 1, release: 1 } },
        BufDecl { name: "b", size: Size::Rest, life: Life { fill: 2, release: 2 } },
    ];
    assert!(DayPlan::plan(&two, slots()).is_err(), "one buffer at most takes the rest");
    let mut region: DayRegion<HeapBacking<4096>> =
        DayRegion::new(&mut AddressSpace::empty(), DayPlan::plan(&heaviest(), slots()).unwrap());
    region.lane_mut(5, at("6a"), 16).fill(7);
    assert!(region.lane_bytes(5) > 0, "the meeting's lane is committed once written");
}
