//! The carriage meeting over hand-built networks: a trip's days, a full segment refusing by lot, the bookings loading
//! the segments of their routes, a consignment no route serves failing, and a closure's recomputed route read.
#![cfg(test)]

use phx_geo::transport::{Route, SegmentDecl, SegmentId, SegmentUse, Segments, Transport};
use phx_id::{Day, PartyId};
use phx_num::{Missing, PriceRaw};
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

use super::{Carrier, Consignment, carriage};

const ROAD: u8 = 0;
const TODAY: Day = Day::new(4);

fn draws() -> Draws {
    Draws::new(stream_key(Seed::new(3), "FRT.capacity_lot"), Subject::new(SubjectTag::Market, 1), 0, 0)
}

fn seg(from: u16, to: u16, metres: u32, capacity: u32) -> SegmentDecl {
    SegmentDecl { from, to, mode: ROAD, metres, capacity, condition: 0, unit: Missing::Absent }
}

fn segments(decls: &[SegmentDecl]) -> Segments {
    let mut s = Segments::default();
    for d in decls {
        let _ = s.open(*d);
    }
    s
}

fn consignment(shipper: u64, load: i64, route: Missing<Route<'_>>) -> Consignment<'_> {
    Consignment { shipper: PartyId::new(shipper), need: load * 2, load, route }
}

fn carriers() -> [Carrier; 2] {
    [
        Carrier { carrier: PartyId::new(1), price: PriceRaw::from_raw(9), room: 1_000 },
        Carrier { carrier: PartyId::new(2), price: PriceRaw::from_raw(5), room: 30 },
    ]
}

#[test]
fn arrival_day_from_route_and_speed() {
    assert_eq!(super::transit_days(450_000, 600_000, 0), 1, "part of a day's run is a day");
    assert_eq!(super::transit_days(1_300_000, 600_000, 1), 5, "three days' run and a day loading at each end");
    assert_eq!(super::transit_days(600_000, 600_000, 0), 1);
}

#[test]
fn route_capacity_binds_by_lot() {
    let s = segments(&[seg(0, 1, 10, 1_000), seg(1, 2, 10, 40)]);
    let route = Missing::Present(Route { segments: &[0, 1], metres: 20 });
    let consignments: Vec<Consignment<'_>> = (0..6).map(|i| consignment(10 + i, 10, route)).collect();
    let mut used = SegmentUse::default();
    let day = carriage(&carriers(), &consignments, (&s, &mut used), (&mut draws(), TODAY));
    assert_eq!(day.booked.len(), 4, "the second segment carries four loads a day");
    assert_eq!(day.over_capacity.len(), 2, "the rest are refused, never repriced");
    assert_eq!(day.booked.iter().filter(|(_, k)| *k == 1).count(), 1, "the cheap carrier's room takes one trip");
    let first = Missing::Present(Route { segments: &[0], metres: 10 });
    let mut fresh = SegmentUse::default();
    let none = carriage(&carriers()[1..], &[consignment(99, 100, first)], (&s, &mut fresh), (&mut draws(), TODAY));
    assert_eq!(none.no_room, [0], "no room without a vehicle");
}

#[test]
fn carriage_books_route_loads() {
    // Two meetings of a day share the segments' loads: the second finds what the first booked.
    let s = segments(&[seg(0, 1, 10, 1_000), seg(1, 2, 10, 40)]);
    let route = Missing::Present(Route { segments: &[0, 1], metres: 20 });
    let mut used = SegmentUse::default();
    let first = [consignment(10, 25, route)];
    let _ = carriage(&carriers(), &first, (&s, &mut used), (&mut draws(), TODAY));
    let load = |u: &SegmentUse, i, day| u.load(SegmentId::new(i), day);
    assert_eq!((load(&used, 0, TODAY), load(&used, 1, TODAY)), (25, 25), "the booking loads each leg of its route");
    let second = carriage(&carriers(), &[consignment(11, 20, route)], (&s, &mut used), (&mut draws(), TODAY));
    assert_eq!(second.over_capacity, [0], "the narrow segment carries 40 a day, 25 already booked");
    let tomorrow = carriage(&carriers(), &[consignment(11, 20, route)], (&s, &mut used), (&mut draws(), TODAY.succ()));
    assert_eq!(tomorrow.booked.len(), 1, "a new day's loads start from none");
}

#[test]
fn no_route_consignment_fails() {
    let s = segments(&[seg(0, 1, 10, 1_000)]);
    let mut used = SegmentUse::default();
    let day = carriage(&carriers(), &[consignment(10, 5, Missing::Absent)], (&s, &mut used), (&mut draws(), TODAY));
    assert_eq!((day.no_route, day.booked.len()), (vec![0], 0), "a pair no route joins fails, visibly");
}

#[test]
fn carriage_reads_recomputed_routes() {
    // The direct road closed, the route the network gives runs the long way, and the booking loads it.
    let s = segments(&[seg(0, 1, 10, 1_000), seg(0, 2, 10, 1_000), seg(2, 1, 10, 1_000)]);
    let mut t = Transport::new(s, (&[0, 1, 2], 3), 1, TODAY);
    let _ = t.close(SegmentId::new(0), Day::new(9), TODAY);
    let mut used = SegmentUse::default();
    let consignments = [consignment(10, 7, t.route(ROAD, 0, 1))];
    let day = carriage(&carriers(), &consignments, (&t.segments, &mut used), (&mut draws(), TODAY));
    assert_eq!(day.booked.len(), 1);
    let loads: Vec<u32> = (0..3).map(|i| used.load(SegmentId::new(i), TODAY)).collect();
    assert_eq!(loads, [0, 7, 7], "over the open segments, not the closed one");
}
