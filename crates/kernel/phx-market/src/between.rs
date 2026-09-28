//! The posted meeting between firms on the core. Each seller's offer stands at the price it posts, sorted once into
//! price levels; the buyers come in an order drawn by lot, each step of a buyer's, its highest limit first, taking from
//! the cheapest level at or below its limit, the offers of a level in an order drawn by lot, in quantities whole in
//! both parties' lots. What no offer serves at a buyer's limit it goes without; the sellers' prices are their own.
//! Every sale becomes flows as the posted-price meeting's do.

use phx_id::PartyKey;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;
use phx_rand::float::{index, len_u64};
use phx_rand::uniform::below_u64;

use crate::meet::Sale;
use crate::retail::paid;

/// A seller's offer: its price for a lot of `price_lot` units, the units it offers and the lot it trades in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offer {
    pub seller: PartyKey,
    pub price: i64,
    pub units: i64,
    pub lot: i64,
}

/// A step of a buyer's bid: the most it pays for a lot of `price_lot` units, the units it wants at that, and the lot
/// it trades in. A buyer's steps are listed together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bid {
    pub buyer: PartyKey,
    pub limit: i64,
    pub units: i64,
    pub lot: i64,
}

/// The meeting's outcome: its sales in the order made, and what each buyer's steps went without.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Between {
    pub sales: Vec<Sale>,
    pub unserved: Vec<(PartyKey, i64)>,
}

/// The least quantity two lots both divide.
fn common_lot(a: i64, b: i64) -> i64 {
    let (mut x, mut y) = (a, b);
    while y != 0 {
        (x, y) = (y, x % y);
    }
    a / x * b
}

/// The meeting over a day's offers and bids between firms, every price for a lot of `price_lot` units; `lot` draws the
/// buyers' order and each level's order for each step.
#[clause("MKT.6", "MKT.9", "GDS.7")]
pub fn between(offers: &[Offer], bids: &[Bid], price_lot: i64, lot: &mut Draws) -> Between {
    if offers.iter().any(|o| o.lot <= 0 || o.units < 0) || bids.iter().any(|b| b.lot <= 0 || b.units < 0) {
        violation!(clause = "MKT.9", "an order with no lot or fewer than no units");
    }
    // The offers by price, each level a run of one price whose live offers lead it.
    let mut order: Vec<u32> = (0..u32::try_from(offers.len()).unwrap_or(u32::MAX)).collect();
    order.sort_by_key(|i| offers.get(to_usize(*i)).map(|o| (o.price, *i)));
    let mut levels: Vec<(i64, usize, usize)> = Vec::new();
    for (k, i) in order.iter().enumerate() {
        let Some(o) = offers.get(to_usize(*i)) else { continue };
        match levels.last_mut() {
            Some((p, _, end)) if *p == o.price => *end = k + 1,
            _ => levels.push((o.price, k, k + 1)),
        }
    }
    let mut left: Vec<i64> = offers.iter().map(|o| o.units).collect();
    // The buyers, each a run of steps, in an order drawn by lot.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (k, b) in bids.iter().enumerate() {
        match runs.last_mut() {
            Some((from, to)) if bids.get(*from).is_some_and(|x| x.buyer == b.buyer) => *to = k + 1,
            _ => runs.push((k, k + 1)),
        }
    }
    for k in 0..runs.len() {
        let j = k + index(below_u64(lot, len_u64(runs.len() - k)));
        runs.swap(k, j);
    }
    let mut out = Between::default();
    // The cheapest level with an offer left; a buyer's steps, kept from one buyer to the next.
    let mut low = 0;
    let mut steps: Vec<Bid> = Vec::new();
    for (from, to) in runs {
        steps.clear();
        steps.extend_from_slice(bids.get(from..to).unwrap_or(&[]));
        steps.sort_by_key(|s| core::cmp::Reverse(s.limit));
        let mut without = 0;
        for step in steps.iter().copied() {
            let mut wanted = step.units;
            for level in levels.iter_mut().skip(low) {
                let (price, start, live_end) = *level;
                if wanted == 0 || price > step.limit {
                    break;
                }
                let live = live_end - start;
                let mut k = 0;
                while k < live && wanted > 0 {
                    let j = k + index(below_u64(lot, len_u64(live - k)));
                    order.swap(start + k, start + j);
                    let at = order.get(start + k).map_or(usize::MAX, |i| to_usize(*i));
                    let (Some(o), Some(cap)) = (offers.get(at), left.get_mut(at)) else { break };
                    let unit = common_lot(step.lot, o.lot);
                    let can = if wanted < *cap { wanted } else { *cap };
                    let qty = can - can % unit;
                    if qty > 0 {
                        *cap -= qty;
                        wanted -= qty;
                        out.sales.push(Sale {
                            buyer: step.buyer,
                            seller: o.seller,
                            units: qty,
                            paid: paid(qty, price, price_lot),
                        });
                    }
                    k += 1;
                }
                // The offers sold out leave the level's live run.
                let mut end = live_end;
                let mut i = start;
                while i < end {
                    if order.get(i).and_then(|x| left.get(to_usize(*x))).is_some_and(|c| *c == 0) {
                        end -= 1;
                        order.swap(i, end);
                    } else {
                        i += 1;
                    }
                }
                level.2 = end;
            }
            without += wanted;
            while levels.get(low).is_some_and(|(_, start, end)| end == start) {
                low += 1;
            }
        }
        if without > 0 {
            out.unserved.push((
                bids.get(from).map_or_else(|| violation!(clause = "MKT.6", "a buyer with no steps"), |b| b.buyer),
                without,
            ));
        }
    }
    out
}

fn to_usize(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyKey, Slot};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Bid, Offer, between};

    fn party(i: u32) -> PartyKey {
        PartyKey::new(2, Slot::new(i))
    }

    fn lot(day: u32) -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.posted"), Subject::new(SubjectTag::Market, 0), day, 0)
    }

    #[test]
    fn a_buyer_takes_the_cheapest_it_accepts_in_whole_lots() {
        let offers = [
            Offer { seller: party(1), price: 5, units: 30, lot: 1 },
            Offer { seller: party(2), price: 7, units: 30, lot: 1 },
        ];
        let bids = [
            Bid { buyer: party(3), limit: 6, units: 20, lot: 10 },
            Bid { buyer: party(4), limit: 8, units: 40, lot: 1 },
        ];
        for day in 0..20 {
            let m = between(&offers, &bids, 1, &mut lot(day));
            let bought = |p: u32| -> i64 { m.sales.iter().filter(|s| s.buyer == party(p)).map(|s| s.units).sum() };
            assert!(m.sales.iter().all(|s| s.paid == s.units * if s.seller == party(1) { 5 } else { 7 }));
            assert_eq!(bought(3) % 10, 0, "a lot of ten buys whole lots");
            assert!(bought(3) <= 20 && bought(4) <= 40);
            assert!(m.sales.iter().filter(|s| s.seller == party(2)).all(|s| s.buyer == party(4)), "none above a limit");
            let sold: i64 = m.sales.iter().map(|s| s.units).sum();
            assert_eq!(sold, bought(3) + bought(4));
            assert!(sold <= 60 && bought(3) + bought(4) + m.unserved.iter().map(|u| u.1).sum::<i64>() == 60);
        }
    }

    #[test]
    fn equal_prices_are_met_by_lot() {
        let offers: Vec<Offer> = (0..4).map(|i| Offer { seller: party(i), price: 10, units: 1_000, lot: 1 }).collect();
        let bids: Vec<Bid> = (0..4_000).map(|i| Bid { buyer: party(100 + i), limit: 10, units: 1, lot: 1 }).collect();
        let m = between(&offers, &bids, 1, &mut lot(3));
        for s in 0..4 {
            let n = m.sales.iter().filter(|x| x.seller == party(s)).count();
            assert_eq!(n, 1_000, "each sells all it offers, whichever buyers came first");
        }
        let few: Vec<Bid> = bids.iter().take(400).copied().collect();
        let m = between(&offers, &few, 1, &mut lot(3));
        for s in 0..4 {
            let n = m.sales.iter().filter(|x| x.seller == party(s)).count();
            assert!((60..140).contains(&n), "seller {s} met {n} of 400 buyers, about a quarter by lot");
        }
    }

    #[test]
    fn a_buyer_goes_without_what_no_offer_serves() {
        let offers = [Offer { seller: party(1), price: 9, units: 5, lot: 1 }];
        let bids =
            [Bid { buyer: party(2), limit: 10, units: 8, lot: 1 }, Bid { buyer: party(2), limit: 8, units: 3, lot: 1 }];
        let m = between(&offers, &bids, 1, &mut lot(1));
        assert_eq!(m.sales.iter().map(|s| s.units).sum::<i64>(), 5);
        assert_eq!(m.unserved, vec![(party(2), 6)], "three short at ten, three at eight none sells at");
    }
}
