//! The day's use over a hand-built network: loads the exact sum of the flows over each segment whatever their order,
//! times read at every flow's load, a past day's loads read as none, and a segment over its capacity admitting it by
//! lot, the refused leaving the flow's other legs.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::Day;
use phx_num::Missing;
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

use crate::transport::{PairFlow, Route, SegmentDecl, SegmentId, SegmentRow, SegmentUse, Segments, Transport};

const ROAD: u8 = 0;

fn seg(from: u16, to: u16, metres: u32, capacity: u32) -> SegmentDecl {
    SegmentDecl { from, to, mode: ROAD, metres, capacity, condition: 0, unit: Missing::Absent }
}

/// A line of four zones, 0–1–2–3, the middle segment the narrowest.
fn network(middle: u32) -> Transport {
    let mut s = Segments::default();
    for d in [seg(0, 1, 10, 1_000), seg(1, 2, 10, middle), seg(2, 3, 10, 1_000)] {
        let _ = s.open(d);
    }
    Transport::new(s, (&[0, 1, 2, 3], 4), 1, Day::new(1))
}

fn flow(from: u16, to: u16, count: u32) -> PairFlow {
    PairFlow { mode: ROAD, from, to, count }
}

fn router<'t>(t: &'t Transport) -> impl Fn(&PairFlow) -> Missing<Route<'t>> + 't {
    move |f: &PairFlow| t.route(f.mode, f.from, f.to)
}

fn loads(u: &SegmentUse, day: Day) -> Vec<u32> {
    (0..3).map(|i| u.load(SegmentId::new(i), day)).collect()
}

fn lot(i: u64) -> Draws {
    Draws::new(stream_key(Seed::new(20_261_001), "GEO.lot"), Subject::new(SubjectTag::World, i), 0, 0)
}

/// A leg's time: its length, lengthened by a tenth of its length for each tenth of its capacity loaded.
fn curve(row: SegmentRow, load: u32) -> u64 {
    let (m, c) = (u64::from(row.metres()), u64::from(row.capacity()));
    m + m * u64::from(load) / c
}

#[test]
fn pair_flows_sum_exactly() {
    let t = network(1_000);
    let mut u = SegmentUse::default();
    let day = Day::new(5);
    let flows = [flow(0, 3, 7), flow(1, 2, 11), flow(2, 1, 13), flow(0, 1, 17)];
    let items = u.add_flows(&t.segments, &flows, router(&t), day);
    assert_eq!(items, 3 + 1 + 1 + 1, "one item a leg of each pair's route");
    assert_eq!(loads(&u, day), [7 + 17, 7 + 11 + 13, 7], "each load the sum of the flows crossing it");
    assert_eq!(u.admitted(), [7, 11, 13, 17], "under capacity every flow is admitted whole");
    let twice = catch_unwind(AssertUnwindSafe(|| u.add_flows(&t.segments, &flows, router(&t), day)));
    assert!(twice.is_err(), "a day's flows are added once, all together");
}

#[test]
fn trips_read_all_loads() {
    // The first flow's time is read once the second's is added: it reads the load both put on the middle leg.
    let t = network(1_000);
    let mut u = SegmentUse::default();
    let day = Day::new(5);
    let _ = u.add_flows(&t.segments, &[flow(1, 2, 100), flow(0, 3, 400)], router(&t), day);
    let Missing::Present(r) = t.route(ROAD, 1, 2) else { panic!("joined") };
    assert_eq!(u.time_at(&t.segments, r, day, curve), 10 + 10 * 500 / 1_000, "at 500, not at its own 100");
}

#[test]
fn use_same_for_any_workers() {
    // Loads are integer sums, so the order the flows come in, however their producers were cut, changes nothing.
    let t = network(1_000);
    let flows = [flow(0, 3, 7), flow(1, 2, 11), flow(3, 0, 5), flow(0, 2, 2)];
    let mut reversed = flows;
    reversed.reverse();
    let day = Day::new(5);
    let (mut a, mut b) = (SegmentUse::default(), SegmentUse::default());
    let _ = (a.add_flows(&t.segments, &flows, router(&t), day), b.add_flows(&t.segments, &reversed, router(&t), day));
    assert_eq!(loads(&a, day), loads(&b, day));
}

#[test]
fn stale_stamp_reads_zero() {
    let t = network(1_000);
    let mut u = SegmentUse::default();
    let _ = u.add_flows(&t.segments, &[flow(0, 3, 9)], router(&t), Day::new(5));
    assert_eq!(loads(&u, Day::new(6)), [0, 0, 0], "a day that put nothing on the segments reads none");
    let _ = u.add_flows(&t.segments, &[flow(2, 3, 4)], router(&t), Day::new(6));
    assert_eq!(loads(&u, Day::new(6)), [0, 0, 4], "the day's flows start from none, not from the last day's");
    assert_eq!(u.admit(&t.segments, &[], router(&t), (&mut lot(0), Day::new(7))), 0, "a day with no flows");
}

#[test]
fn over_capacity_admits_by_lot() {
    let t = network(30);
    let flows = [flow(0, 3, 20), flow(1, 2, 15), flow(0, 1, 6)];
    let day = Day::new(5);
    for i in 0..20 {
        let mut u = SegmentUse::default();
        let _ = u.add_flows(&t.segments, &flows, router(&t), day);
        assert_eq!(loads(&u, day), [26, 35, 20]);
        let _ = u.admit(&t.segments, &flows, router(&t), (&mut lot(i), day));
        let a = u.admitted();
        let (Some(&through), Some(&middle), Some(&short)) = (a.first(), a.get(1), a.get(2)) else { panic!("three") };
        assert_eq!(through + middle, 30, "the middle segment admits its capacity across the flows crossing it");
        assert_eq!(short, 6, "a flow off the full segment is admitted whole");
        assert_eq!(loads(&u, day), [through + 6, 30, through], "the refused leave the flow's other legs");
    }
    let admitted = |i| {
        let mut u = SegmentUse::default();
        let _ = u.add_flows(&t.segments, &flows, router(&t), day);
        let _ = u.admit(&t.segments, &flows, router(&t), (&mut lot(i), day));
        u.admitted().to_vec()
    };
    assert_eq!(admitted(3), admitted(3), "the lot is its stream's");
    assert!((0..20).any(|i| admitted(i) != admitted(0)), "and a lot, not a rule");
}

#[test]
fn congestion_time_rises_with_load() {
    let t = network(1_000);
    let Missing::Present(r) = t.route(ROAD, 0, 3) else { panic!("joined") };
    let day = Day::new(5);
    let time = |count| {
        let mut u = SegmentUse::default();
        let _ = u.add_flows(&t.segments, &[flow(0, 3, count)], router(&t), day);
        u.time_at(&t.segments, r, day, curve)
    };
    assert_eq!(time(0), 30, "free flow over three legs");
    assert!(time(200) < time(800), "the more on the legs, the longer");
}
