//! Routes over hand-built networks: the shortest open path, no path missing, a closure's routes computed again, the
//! same table however built, rebuilt at load alike, and every opening and removal logged.
#![cfg(test)]

use phx_id::Day;
use phx_num::Missing;

use super::{Change, SegmentDecl, SegmentId, Segments, Transport};

const ROAD: u8 = 0;
const SEA: u8 = 1;

fn seg(from: u16, to: u16, mode: u8, metres: u32) -> SegmentDecl {
    SegmentDecl { from, to, mode, metres, capacity: 100, condition: 0, unit: Missing::Absent }
}

/// Four zones: roads 0–1–2 and a longer road 0–2, a sea lane 2–3 to the island.
fn network() -> Transport {
    let mut s = Segments::default();
    for d in [seg(0, 1, ROAD, 10), seg(1, 2, ROAD, 10), seg(0, 2, ROAD, 25), seg(2, 3, SEA, 5)] {
        let _ = s.open(d);
    }
    Transport::new(s, (&[0, 1, 2, 3], 4), 2, Day::new(1))
}

fn metres(t: &Transport, mode: u8, from: u16, to: u16) -> Missing<u64> {
    match t.route(mode, from, to) {
        Missing::Present(r) => Missing::Present(r.metres),
        Missing::Absent => Missing::Absent,
    }
}

#[test]
fn route_is_shortest_open_path() {
    let t = network();
    let Missing::Present(r) = t.route(ROAD, 0, 2) else { panic!("a road joins them") };
    assert_eq!((r.metres, r.segments), (20, [0, 1].as_slice()), "over 1, not the longer road");
    assert_eq!(metres(&t, ROAD, 2, 0), Missing::Present(20), "either way along a segment");
    assert_eq!(metres(&t, SEA, 2, 3), Missing::Present(5));
    let Missing::Present(home) = t.route(ROAD, 1, 1) else { panic!("a place reaches itself") };
    assert_eq!((home.metres, home.segments.len()), (0, 0));
}

#[test]
fn no_route_is_missing() {
    assert_eq!(metres(&network(), ROAD, 0, 3), Missing::Absent, "no road reaches the island");
}

#[test]
fn closure_recomputes_region_routes() {
    let mut t = network();
    t.close(SegmentId::new(1), Day::new(10), Day::new(5));
    assert_eq!(metres(&t, ROAD, 0, 2), Missing::Present(25), "around the closed segment");
    assert_eq!(metres(&t, SEA, 2, 3), Missing::Present(5), "another mode's routes untouched");
    t.reopen(SegmentId::new(1), Day::new(6));
    assert_eq!(metres(&t, ROAD, 0, 2), Missing::Present(20));
    t.close(SegmentId::new(1), Day::new(10), Day::new(5));
    t.remove(SegmentId::new(2), Day::new(5));
    assert_eq!(metres(&t, ROAD, 0, 2), Missing::Absent, "both roads out");
    let _ = t.open(seg(0, 2, ROAD, 40), Day::new(5));
    assert_eq!(metres(&t, ROAD, 0, 2), Missing::Present(40), "a road opened carries at once");
}

#[test]
fn recompute_same_for_any_workers() {
    // The table is computed origin by origin in place order, the same however and however often it is built.
    let mut t = network();
    let first = t.clone();
    t.recompute(ROAD, Day::new(1));
    assert_eq!(t, first);
}

#[test]
fn routes_rebuild_equals() {
    let mut t = network();
    t.close(SegmentId::new(0), Day::new(9), Day::new(3));
    let saved = t.clone();
    t.routes.clear();
    let _ = t.rebuild();
    assert_eq!(t, saved, "the routes rebuilt from the saved segments and day are the ones kept");
}

#[test]
fn change_hook_called_on_open_and_remove() {
    let mut s = Segments::default();
    let a = s.open(seg(0, 1, ROAD, 10));
    let b = s.open(seg(1, 2, ROAD, 10));
    s.remove(a);
    s.close(b, Day::new(4));
    let mut changes = Vec::new();
    s.take_changes(&mut changes);
    assert_eq!(
        changes,
        [(a, Change::Opened), (b, Change::Opened), (a, Change::Removed)],
        "a closure is no change of line"
    );
    s.take_changes(&mut changes);
    assert_eq!(changes.len(), 3, "each change taken once");
}
