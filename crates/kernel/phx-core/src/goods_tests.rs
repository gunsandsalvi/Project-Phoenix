//! Goods' arithmetic over a few holders: average cost in and out, what bounds hold, flows applied with nature on one
//! side, the goods' identity, shipments pledged and arriving with their cost, losses cutting pledges, and spoilage.
#![cfg(test)]

use phx_id::{Day, PartyKey, Slot};

use super::{
    Bound, Carriage, Cost, Good, Held, NATURE, Shipment, Shipments, Short, Stocks, UnitIds, breaks, nature_net, spoil,
};
use crate::flows::{Denom, Flow};
use crate::units::Class;

fn firm(i: u32) -> PartyKey {
    PartyKey::new(2, Slot::new(i))
}

fn flow(from: PartyKey, to: PartyKey, unit: u16, amount: i64) -> Flow {
    Flow { payer: from, payee: to, amount, source: 0, denomination: Denom::units(unit), reason: 1, order: 0 }
}

#[test]
fn nature_is_the_kind_no_table_is_of() {
    assert_eq!((NATURE.kind(), NATURE.slot().get()), (phx_id::consts::NATURE_KIND, 0));
}

#[test]
fn goods_are_issued_their_units_once() {
    let mut ids = UnitIds::default();
    let wheat = Good { product: 3, grade: 1, zone: 40 };
    let there = Good { zone: 41, ..wheat };
    let (a, b) = (Held::Good(wheat), Held::Good(there));
    assert_eq!((ids.unit(a), ids.unit(b), ids.unit(a)), (0, 1, 0), "the same grade at two zones is two goods");
    let lorry = Held::Capital(Class { kind: 3, band: 0, condition: 1, zone: 40 });
    assert_eq!((ids.unit(lorry), ids.held(2)), (2, Some(lorry)), "capital and goods share the units");
    assert_eq!((ids.find(b), ids.held(1), ids.find(Held::Good(Good { grade: 2, ..wheat }))), (Some(1), Some(b), None));
}

#[test]
fn average_cost_in_and_out() {
    let mut s = Stocks::default();
    s.receive(firm(1), 0, (100, 1_000), Day::new(10));
    s.receive(firm(1), 0, (100, 3_000), Day::new(20));
    let h = *s.holding(firm(1), 0).unwrap();
    assert_eq!((h.units, h.cost, h.day), (200, 4_000, 15), "the day is the units' mean");
    assert_eq!(s.deliver(firm(1), 0, 50, Bound::Free), Ok(1_000));
    assert_eq!(s.deliver(firm(1), 0, 49, Bound::Free), Ok(980));
    assert_eq!(s.deliver(firm(1), 0, 101, Bound::Free), Ok(2_020), "the last unit takes what cost is left");
    assert_eq!(s.holding(firm(1), 0).map(|h| (h.units, h.cost)), Some((0, 0)));
}

#[test]
fn bounds_hold_what_they_bind() {
    let mut s = Stocks::default();
    s.receive(firm(1), 4, (100, 500), Day::new(1));
    assert_eq!(s.bind(firm(1), 4, 60, (Bound::Free, Bound::Committed)), Ok(()));
    assert_eq!(s.deliver(firm(1), 4, 50, Bound::Free), Err(Short { asked: 50, held: 40 }), "a cover is not free");
    assert_eq!(s.bind(firm(1), 4, 70, (Bound::Committed, Bound::Pledged)), Err(Short { asked: 70, held: 60 }));
    assert_eq!(s.deliver(firm(1), 4, 60, Bound::Committed), Ok(300));
    assert_eq!(s.holding(firm(1), 4).map(|h| (h.units, h.committed, h.free())), Some((40, 0, 40)));
    assert_eq!(s.deliver(firm(9), 4, 1, Bound::Free), Err(Short { asked: 1, held: 0 }), "none held is short");
}

#[test]
fn flows_move_units_and_nature_makes_and_uses_them() {
    let mut s = Stocks::default();
    let day = Day::new(5);
    let open = s.totals();
    let made = flow(NATURE, firm(1), 0, 80);
    let sold = flow(firm(1), firm(2), 0, 30);
    let used = flow(firm(2), NATURE, 0, 10);
    let lent = flow(firm(1), firm(3), 0, 10);
    assert_eq!(s.apply(&made, Bound::Free, Cost::At(800), day), Ok(None));
    assert_eq!(s.apply(&sold, Bound::Free, Cost::At(450), day), Ok(Some(300)), "the payer's units carry their cost");
    assert_eq!(s.apply(&used, Bound::Free, Cost::Carried, day), Ok(Some(150)));
    assert_eq!(s.apply(&lent, Bound::Free, Cost::Carried, day), Ok(Some(100)));
    assert_eq!(s.holding(firm(2), 0).map(|h| (h.units, h.cost)), Some((20, 300)), "a buyer's cost is what it paid");
    assert_eq!(s.holding(firm(3), 0).map(|h| (h.units, h.cost)), Some((10, 100)), "a transfer carries its cost");
    let net = nature_net([&made, &sold, &used, &lent]);
    assert_eq!(net, vec![70]);
    assert!(breaks(&open, &net, &s.totals()).is_empty());
    assert_eq!(breaks(&open, &[71], &s.totals()), vec![(0, 71, 70)], "a unit made from nothing breaks it");
}

#[test]
fn a_shipment_arrives_as_its_good_at_its_cost() {
    let mut s = Stocks::default();
    let mut ships = Shipments::new(Day::new(1), 8);
    s.receive(firm(1), 0, (100, 2_000), Day::new(1));
    let trip = Shipment { owner: firm(1), carrier: firm(7), from: 0, to: 1, arrives: 3, units: 40 };
    let _ = ships.depart(&mut s, trip, (Bound::Free, Day::new(1))).unwrap();
    assert_eq!(
        s.holding(firm(1), 0).map(|h| (h.units, h.pledged, h.free())),
        Some((100, 40, 60)),
        "pledged, still owned"
    );
    let reasons = Carriage { shipped: 5, arrived: 6 };
    let mut out = Vec::new();
    for d in 1..3 {
        ships.arrive(Day::new(d), &mut s, reasons, &mut out);
    }
    assert!(out.is_empty(), "nothing arrives before its day");
    ships.arrive(Day::new(3), &mut s, reasons, &mut out);
    let legs: Vec<(PartyKey, PartyKey, u16, i64, u8)> =
        out.iter().map(|f| (f.payer, f.payee, f.denomination.unit(), f.amount, f.reason)).collect();
    assert_eq!(legs, vec![(firm(1), NATURE, 0, 40, 5), (NATURE, firm(1), 1, 40, 6)], "used up there, made here");
    assert_eq!(s.holding(firm(1), 1).map(|h| (h.units, h.cost, h.day)), Some((40, 800, 1)), "the day in carried");
    assert_eq!(s.holding(firm(1), 0).map(|h| (h.units, h.pledged)), Some((60, 0)));
    assert_eq!(ships.on_the_way(), 0);
}

#[test]
fn a_loss_cuts_the_latest_pledges_first() {
    let mut s = Stocks::default();
    let mut ships = Shipments::new(Day::new(1), 8);
    s.receive(firm(1), 0, (100, 1_000), Day::new(1));
    let trip = |units, arrives| Shipment { owner: firm(1), carrier: firm(7), from: 0, to: 1, arrives, units };
    let _ = ships.depart(&mut s, trip(30, 4), (Bound::Free, Day::new(1))).unwrap();
    let _ = ships.depart(&mut s, trip(50, 5), (Bound::Free, Day::new(1))).unwrap();
    // Ninety units lost leave ten, of eighty pledged: the pledges lose seventy, the latest shipment first.
    let (cost, cut) = s.lose(firm(1), 0, 90);
    assert_eq!((cost, cut), (900, 70));
    ships.shrink(firm(1), 0, cut);
    let left: Vec<i64> = ships.of(firm(1)).map(|(_, x)| x.units).collect();
    assert_eq!(left, vec![10], "the latest is gone, the earlier cut to what is left");
    let mut out = Vec::new();
    for d in 1..=5 {
        ships.arrive(Day::new(d), &mut s, Carriage { shipped: 5, arrived: 6 }, &mut out);
    }
    assert_eq!(s.holding(firm(1), 1).map(|h| h.units), Some(10), "what arrives is what is left");
}

#[test]
fn a_cover_beyond_what_was_lost_fails_its_sale() {
    let mut s = Stocks::default();
    s.receive(firm(1), 0, (10, 100), Day::new(1));
    s.bind(firm(1), 0, 8, (Bound::Free, Bound::Committed)).unwrap();
    assert_eq!(s.lose(firm(1), 0, 5), (50, 0), "no pledge to cut");
    assert_eq!(s.deliver(firm(1), 0, 8, Bound::Committed), Err(Short { asked: 8, held: 5 }));
}

#[test]
fn spoilage_takes_what_the_rate_takes_over_the_units_days() {
    let mut s = Stocks::default();
    s.receive(firm(1), 0, (1_000, 5_000), Day::new(0));
    s.receive(firm(1), 1, (1_000, 5_000), Day::new(0));
    let mut out = Vec::new();
    let rate = |u: u16| (u == 0).then_some(0.1);
    spoil(&s, firm(1), rate, (365, Day::new(400), 365), 9, &mut out);
    let got: Vec<(PartyKey, u16, i64)> = out.iter().map(|f| (f.payee, f.denomination.unit(), f.amount)).collect();
    assert_eq!(got, vec![(NATURE, 0, 95)], "1 000 × (1 − e^−0.1) = 95.16; the good that keeps loses nothing");
}

#[test]
fn a_party_ends_holding_nothing() {
    let mut s = Stocks::default();
    s.receive(firm(1), 0, (5, 50), Day::new(1));
    assert!(
        std::panic::catch_unwind(move || {
            let mut s = s;
            s.end(firm(1));
        })
        .is_err()
    );
    let mut s = Stocks::default();
    s.receive(firm(1), 0, (5, 50), Day::new(1));
    s.deliver(firm(1), 0, 5, Bound::Free).unwrap();
    s.end(firm(1));
    assert_eq!(s.holdings(firm(1)).count(), 0);
    s.receive(firm(2), 3, (1, 1), Day::new(2));
    assert_eq!(s.all().count(), 1, "the ended party's row is another's");
}

#[test]
fn a_successor_takes_every_holding_with_what_binds_it() {
    let mut s = Stocks::default();
    let (ended, estate, carrier) = (firm(1), firm(2), firm(3));
    s.receive(ended, 0, (10, 400), Day::new(4));
    s.receive(ended, 1, (3, 90), Day::new(6));
    assert!(s.bind(ended, 0, 2, (Bound::Free, Bound::Committed)).is_ok());
    let mut ships = Shipments::new(Day::new(0), 16);
    let trip = Shipment { owner: ended, carrier, from: 0, to: 1, arrives: 9, units: 5 };
    assert!(ships.depart(&mut s, trip, (Bound::Free, Day::new(7))).is_ok());
    s.succeed(ended, estate);
    ships.pass(ended, estate);
    let h = *s.holding(estate, 0).unwrap();
    assert_eq!((h.units, h.cost, h.committed, h.pledged, h.day), (10, 400, 2, 5, 4), "whole, bound and aged as held");
    assert_eq!(s.holding(estate, 1).map(|h| (h.units, h.cost)), Some((3, 90)));
    assert!(s.holdings(ended).all(|h| h.units == 0 && h.committed == 0 && h.pledged == 0), "the ended holds nothing");
    s.end(ended);
    assert_eq!((ships.of(ended).count(), ships.of(estate).count()), (0, 1));
    let mut out = Vec::new();
    for d in 0..=9 {
        ships.arrive(Day::new(d), &mut s, Carriage { shipped: 1, arrived: 2 }, &mut out);
    }
    assert_eq!(s.holding(estate, 1).map(|h| h.units), Some(8), "the goods on their way arrive at the successor");
}
