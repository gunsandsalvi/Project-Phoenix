use std::collections::BTreeMap;

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded};
use phx_rand::{Draws, below_u64};

use crate::print::Match;

/// A seller's posted price and what it can serve at it today, its capacity or stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posting {
    pub seller: PartyId,
    pub price: PriceRaw,
    pub capacity: i64,
}

/// A posted-price meeting's outcome: each sale, a buyer buying from a seller at its posted price; each buyer's
/// demand no seller could serve, by its party; and the rounds of re-choice capacity forced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostedDay {
    pub sales: Vec<Match>,
    pub unserved: BTreeMap<u64, i64>,
    pub rounds: u64,
}

/// The least quantity two lots both divide.
fn common_lot(a: i64, b: i64) -> i64 {
    let (mut x, mut y) = (a, b);
    while y != 0 {
        (x, y) = (y, x % y);
    }
    a / x * b
}

/// A posted meeting between firms: each seller's offer stands at its limit, the price it posts; the buyers come in an
/// order drawn by lot, each step of a buyer's, its highest limit first, taking from the cheapest postings at or below
/// its limit, equal prices by lot, in quantities whole in both parties' lots. A
/// buyer no posting serves at its limit goes without; the sellers' prices are their own.
#[clause("MKT.6", "MKT.9", "GDS.7")]
#[must_use]
pub fn between(orders: &[crate::order::Order], lot: &mut Draws) -> PostedDay {
    use crate::order::Side;
    let mut postings: Vec<(usize, Posting)> = Vec::new();
    for (i, o) in orders.iter().enumerate().filter(|(_, o)| o.side == Side::Sell) {
        for s in &o.steps {
            postings.push((i, Posting { seller: o.party, price: s.limit, capacity: s.qty }));
        }
    }
    let mut left: Vec<i64> = postings.iter().map(|(_, p)| p.capacity).collect();
    let mut buyers: Vec<usize> =
        orders.iter().enumerate().filter(|(_, o)| o.side == Side::Buy).map(|(i, _)| i).collect();
    let mut order_of = Vec::with_capacity(buyers.len());
    while !buyers.is_empty() {
        let Ok(pick) = usize::try_from(below_u64(lot, phx_rand::float::len_u64(buyers.len()))) else {
            capacity_exceeded!("buyers", usize::MAX, buyers.len());
        };
        order_of.push(buyers.swap_remove(pick));
    }
    let mut day = PostedDay { sales: Vec::new(), unserved: BTreeMap::new(), rounds: 1 };
    for b in order_of {
        let Some(buyer) = orders.get(b) else { continue };
        let mut steps = buyer.steps.clone();
        steps.sort_by_key(|s| core::cmp::Reverse(s.limit.raw()));
        for step in steps {
            let mut wanted = step.qty;
            let mut passed: Vec<usize> = Vec::new();
            while wanted > 0 {
                let open: Vec<usize> = (0..postings.len())
                    .filter(|p| !passed.contains(p) && left.get(*p).is_some_and(|c| *c > 0))
                    .filter(|p| postings.get(*p).is_some_and(|(_, q)| q.price.raw() <= step.limit.raw()))
                    .collect();
                let Some(low) = open
                    .iter()
                    .filter_map(|p| postings.get(*p))
                    .map(|(_, q)| q.price.raw())
                    .reduce(|a, b| if b < a { b } else { a })
                else {
                    break;
                };
                let cheapest: Vec<usize> =
                    open.into_iter().filter(|p| postings.get(*p).is_some_and(|(_, q)| q.price.raw() == low)).collect();
                let Ok(pick) = usize::try_from(below_u64(lot, phx_rand::float::len_u64(cheapest.len()))) else {
                    capacity_exceeded!("sellers tied", usize::MAX, cheapest.len());
                };
                let Some(&chosen) = cheapest.get(pick) else { break };
                let (Some((seller_order, posting)), Some(cap)) = (postings.get(chosen), left.get_mut(chosen)) else {
                    break;
                };
                let seller_lot = orders.get(*seller_order).map_or(1, |o| o.lot);
                let unit = common_lot(buyer.lot, seller_lot);
                let can = if wanted < *cap { wanted } else { *cap };
                let qty = can - can % unit;
                if qty == 0 {
                    passed.push(chosen);
                    continue;
                }
                *cap -= qty;
                wanted -= qty;
                day.sales.push(Match {
                    buyer: buyer.party,
                    seller: posting.seller,
                    qty,
                    price: posting.price,
                    draws: Missing::Absent,
                });
            }
            if wanted > 0 {
                *day.unserved.entry(buyer.party.get()).or_insert(0) += wanted;
            }
        }
    }
    day
}

#[cfg(test)]
mod tests {
    use phx_id::{MarketId, PartyId};
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::between;

    fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.posted"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    fn order(party: u64, side: crate::order::Side, steps: &[(i64, i64)], lot: i64) -> crate::order::Order {
        let poster = crate::order::Poster {
            party: PartyId::new(party),
            market: MarketId::new(0),
            side,
            timing: crate::order::Timing::AtTheClose,
            day: phx_id::Day::new(1),
            reason: "inputs",
            priority: phx_num::Missing::Absent,
            lot,
        };
        let asked: Vec<crate::order::Asked> = steps
            .iter()
            .map(|(l, q)| crate::order::Asked { limit: phx_num::Missing::Present(PriceRaw::from_raw(*l)), qty: *q })
            .collect();
        crate::order::Order::new(poster, &asked, 1).unwrap()
    }

    #[test]
    fn between_firms_a_buyer_takes_the_cheapest_it_accepts_in_whole_lots() {
        use crate::order::Side;
        let orders = [
            order(1, Side::Sell, &[(5, 30)], 1),
            order(2, Side::Sell, &[(7, 30)], 1),
            order(3, Side::Buy, &[(6, 20)], 10),
            order(4, Side::Buy, &[(8, 40)], 1),
        ];
        let day = between(&orders, &mut lot());
        let bought = |p: u64| -> i64 { day.sales.iter().filter(|s| s.buyer == PartyId::new(p)).map(|s| s.qty).sum() };
        assert!(day.sales.iter().all(|s| s.price.raw() <= if s.buyer == PartyId::new(3) { 6 } else { 8 }));
        assert_eq!(bought(3) % 10, 0, "a lot of ten buys whole lots");
        assert!(bought(3) <= 20 && bought(4) <= 40);
        let sold: i64 = day.sales.iter().map(|s| s.qty).sum();
        assert_eq!(sold, bought(3) + bought(4));
        assert!(day.sales.iter().filter(|s| s.seller == PartyId::new(2)).all(|s| s.buyer == PartyId::new(4)));
    }
}
