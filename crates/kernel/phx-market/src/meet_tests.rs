//! The posted-price meeting's arithmetic over a few stalls and buyers: the logit's shares, capacity and choosing
//! again, whole units at a lot's price, money buyers among what they can pay, and the same result for any workers and
//! any order of the buyers.
#![cfg(test)]

use phx_core::flows::Denom;
use phx_id::{PartyKey, Slot};
use phx_rand::float::from_i64;
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

use super::{Buyer, Meeting, Place, Sale, Stall, Tastes, meet};
use crate::retail::{Want, Weights};

const W: Weights = Weights { price: 2.0, distance: 0.05 };

fn seller(i: u32) -> PartyKey {
    PartyKey::new(2, Slot::new(i))
}

fn buyer(i: u32) -> PartyKey {
    PartyKey::new(1, Slot::new(i))
}

fn tastes() -> Tastes {
    Tastes { key: stream_key(Seed::new(1), "SRV.taste"), day: 9, substep: 6 }
}

fn lots(s: PartyKey, round: u32) -> Draws {
    Draws::new(
        stream_key(Seed::new(1), "SRV.capacity_lot"),
        Subject::new(SubjectTag::Party, u64::from(s.word())),
        9,
        u8::try_from(round).unwrap(),
    )
}

fn run(stalls: &[Stall], near: &[(u32, f64)], wants: &[Want], lot: i64, workers: Option<usize>) -> Meeting {
    let places = [Place { near: near.to_vec() }];
    let buyers: Vec<Buyer> = (0_u32..)
        .zip(wants)
        .map(|(i, w)| Buyer { party: buyer(i), subject: u64::from(i) + 1, want: *w, place: 0 })
        .collect();
    let pool = workers.map(|n| phx_exec::Pool::new(&phx_exec::PoolSpec::unpinned(n)).unwrap());
    let mut m = Meeting::default();
    meet(&mut m, pool.as_ref(), (stalls, &places, &buyers), (lot, W), tastes(), &lots);
    m
}

fn sales(m: &Meeting) -> Vec<Sale> {
    m.sales().copied().collect()
}

fn outcome(m: &Meeting) -> (Vec<Sale>, Vec<(PartyKey, Want)>, u64) {
    (sales(m), m.unserved.clone(), m.rounds)
}

fn units_of(m: &Meeting, s: PartyKey) -> i64 {
    m.sales().filter(|x| x.seller == s).map(|x| x.units).sum()
}

#[test]
fn logit_shares_of_the_open_sellers() {
    let stalls: Vec<Stall> =
        (0..3).map(|i| Stall { seller: seller(i), price: 100 + 40 * i64::from(i), units: 1_000_000 }).collect();
    let near = [(0, 3.0), (1, 1.0), (2, 0.0)];
    let n = 40_000;
    let m = run(&stalls, &near, &vec![Want::Units(1); n], 1, None);
    let v: Vec<f64> = near
        .iter()
        .map(|(s, km)| -W.price * from_i64(stalls[usize::try_from(*s).unwrap()].price).ln() - W.distance * km)
        .collect();
    let total: f64 = v.iter().map(|x| x.exp()).sum();
    for (i, s) in stalls.iter().enumerate() {
        let share = from_i64(units_of(&m, s.seller)) / from_i64(i64::try_from(n).unwrap());
        assert!((share - v[i].exp() / total).abs() < 0.01, "seller {i}: {share} against {}", v[i].exp() / total);
    }
    assert_eq!(m.rounds, 1, "no seller ran out, so no one chose again");
}

#[test]
fn capacity_is_never_exceeded_and_the_short_choose_again() {
    let stalls =
        [Stall { seller: seller(0), price: 10, units: 10 }, Stall { seller: seller(1), price: 12, units: 100 }];
    let m = run(&stalls, &[(0, 0.0), (1, 0.0)], &[Want::Units(4); 20], 1, None);
    assert!(units_of(&m, seller(0)) <= 10);
    assert_eq!(units_of(&m, seller(0)) + units_of(&m, seller(1)), 80, "every buyer gets its four");
    assert!(m.unserved.is_empty());
    assert!(m.rounds >= 2, "those the cheap seller served short went on to the other");
}

#[test]
fn a_buyer_turned_away_goes_without_when_none_is_left() {
    let m = run(
        &[Stall { seller: seller(0), price: 10, units: 5 }],
        &[(0, 0.0)],
        &[Want::Units(3), Want::Units(3)],
        1,
        None,
    );
    let mut got: Vec<i64> = m.sales().map(|s| s.units).collect();
    got.sort_unstable();
    assert_eq!(got, vec![2, 3], "one served in full, the other short");
    assert_eq!(m.unserved.len(), 1);
    assert_eq!(m.unserved[0].1, Want::Units(1), "it goes without what it still wants");
}

#[test]
fn a_sale_is_whole_units_at_a_price_for_a_lot() {
    let m =
        run(&[Stall { seller: seller(0), price: 4_990, units: 1_000 }], &[(0, 0.0)], &[Want::Money(100)], 10_000, None);
    assert_eq!(sales(&m), vec![Sale { buyer: buyer(0), seller: seller(0), units: 200, paid: 100 }]);
    assert!(m.unserved.is_empty(), "a buyer with money that bought keeps its change");
}

#[test]
fn money_buys_only_where_it_can_pay_a_unit() {
    let stalls = [
        Stall { seller: seller(0), price: 10, units: 10_000 },
        Stall { seller: seller(1), price: 1_000, units: 1_000 },
    ];
    let m = run(&stalls, &[(0, 50.0), (1, 0.0)], &vec![Want::Money(50); 500], 1, None);
    assert_eq!(units_of(&m, seller(1)), 0, "none can buy a unit at 1 000");
    assert_eq!(units_of(&m, seller(0)), 500 * 5);
    let m = run(&stalls, &[(0, 0.0), (1, 0.0)], &[Want::Money(5)], 1, None);
    assert_eq!(
        (sales(&m).len(), m.unserved.len()),
        (0, 1),
        "priced out of every seller before buying, it goes without"
    );
}

#[test]
fn the_same_for_any_workers() {
    let stalls: Vec<Stall> = (0..7)
        .map(|i| Stall { seller: seller(i), price: 50 + 13 * i64::from(i), units: 3 + 5 * i64::from(i) })
        .collect();
    let near: Vec<(u32, f64)> = (0..7).map(|i| (i, f64::from(i))).collect();
    let wants: Vec<Want> =
        (0..9_000).map(|i| if i % 3 == 0 { Want::Money(200 + i) } else { Want::Units(1 + i % 4) }).collect();
    let one = run(&stalls, &near, &wants, 1, None);
    assert_eq!(outcome(&one), outcome(&run(&stalls, &near, &wants, 1, Some(1))));
    assert_eq!(outcome(&one), outcome(&run(&stalls, &near, &wants, 1, Some(4))));
    for s in &stalls {
        assert!(units_of(&one, s.seller) <= s.units, "no seller serves beyond its units");
    }
}

#[test]
fn a_sale_makes_its_flows() {
    let sale = Sale { buyer: buyer(1), seller: seller(2), units: 3, paid: 30 };
    let mut out = Vec::new();
    sale.flows((Denom::money(0), 2, 0), Some((Denom::units(7), 3, 0)), 11, &mut out);
    let got: Vec<(PartyKey, PartyKey, i64)> = out.iter().map(|f| (f.payer, f.payee, f.amount)).collect();
    assert_eq!(got, vec![(buyer(1), seller(2), 30), (seller(2), buyer(1), 3)], "money one way, the goods the other");
}

#[test]
fn rationing_by_lot_order_free() {
    let stalls: Vec<Stall> = (0..5)
        .map(|i| Stall { seller: seller(i), price: 40 + 9 * i64::from(i), units: 20 + 7 * i64::from(i) })
        .collect();
    let near: Vec<(u32, f64)> = (0..5).map(|i| (i, f64::from(i))).collect();
    let places = [Place { near }];
    let mut buyers: Vec<Buyer> = (0..600_u32)
        .map(|i| Buyer {
            party: buyer(i),
            subject: u64::from(i) + 1,
            want: if i % 4 == 0 { Want::Money(150) } else { Want::Units(1 + i64::from(i % 3)) },
            place: 0,
        })
        .collect();
    let sorted = |buyers: &[Buyer]| {
        let mut m = Meeting::default();
        meet(&mut m, None, (&stalls, &places, buyers), (1, W), tastes(), &lots);
        let mut s = sales(&m);
        s.sort_by_key(|x| (x.buyer.word(), x.seller.word(), x.units));
        let mut u = m.unserved.clone();
        u.sort_by_key(|(p, _)| p.word());
        (s, u)
    };
    let forward = sorted(&buyers);
    assert!(!forward.1.is_empty(), "the stalls ran out, so rationing drew");
    buyers.reverse();
    assert_eq!(forward, sorted(&buyers), "who is served does not depend on the order buyers are listed in");
}
