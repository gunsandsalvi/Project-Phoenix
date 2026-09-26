use std::collections::BTreeMap;

use phx_core::GroupDemand;
use phx_id::{MarketId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded};
use phx_rand::{Draws, below_u64};

use crate::print::{Buyer, Match};

/// A seller's posted price and what it can serve at it today, its capacity or stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posting {
    pub seller: PartyId,
    pub price: PriceRaw,
    pub capacity: i64,
}

/// A posted-price meeting's outcome: each sale, a group buying from a seller at its posted price; each group's
/// demand no seller could serve; and the rounds of re-choice capacity forced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostedDay {
    pub sales: Vec<Match>,
    pub unserved: BTreeMap<u64, i64>,
    pub rounds: u64,
}

/// A posted-price meeting: each group, among the postings it sees, goes to the lowest price, equal prices by lot, and
/// buys what its members want at that price as the population answers it; a seller serves whoever comes until its
/// capacity runs out, and a group it could not serve chooses again among the rest, round by round, each round
/// counted, wanting at the new price what its members want there less what they bought. A group that wants nothing
/// more at the prices it sees walks away. The sellers' prices are their own.
#[clause("MKT.6", "MKT.9", "MKT.17")]
pub fn posted(
    market: MarketId,
    postings: &[Posting],
    groups: &[(u64, Vec<usize>)],
    demand: &dyn GroupDemand,
    lot: &mut Draws,
) -> PostedDay {
    let mut left: Vec<i64> = postings.iter().map(|p| p.capacity).collect();
    let mut wanting: Vec<(u64, Vec<usize>, i64, Missing<i64>)> =
        groups.iter().map(|(g, seen)| (*g, seen.clone(), 0, Missing::Absent)).collect();
    let mut day = PostedDay { sales: Vec::new(), unserved: BTreeMap::new(), rounds: 0 };
    while !wanting.is_empty() {
        day.rounds += 1;
        let mut next = Vec::new();
        for (group, seen, bought, still) in wanting {
            let open: Vec<usize> = seen.iter().copied().filter(|i| left.get(*i).is_some_and(|c| *c > 0)).collect();
            let price_of = |i: usize| crate::simplex::at(postings, i).price.raw();
            let Some(low) = open.iter().map(|i| price_of(*i)).reduce(|b, p| if p < b { p } else { b }) else {
                if let Missing::Present(q) = still {
                    *day.unserved.entry(group).or_insert(0) += q;
                }
                continue;
            };
            let cheapest: Vec<usize> = open.iter().copied().filter(|i| price_of(*i) == low).collect();
            let Ok(pick) = usize::try_from(below_u64(lot, phx_rand::float::len_u64(cheapest.len()))) else {
                capacity_exceeded!("sellers tied", usize::MAX, cheapest.len());
            };
            let Some(&chosen) = cheapest.get(pick) else { continue };
            let wants = demand.quantity_at(market, group, low) - bought;
            if wants <= 0 {
                continue;
            }
            let Some(cap) = left.get_mut(chosen) else { continue };
            let served = if wants < *cap { wants } else { *cap };
            *cap -= served;
            if let Some(p) = postings.get(chosen) {
                day.sales.push(Match {
                    buyer: Buyer::Group(group),
                    seller: p.seller,
                    qty: served,
                    price: p.price,
                    draws: Missing::Absent,
                });
            }
            if served < wants {
                let rest: Vec<usize> = seen.into_iter().filter(|i| *i != chosen).collect();
                next.push((group, rest, bought + served, Missing::Present(wants - served)));
            }
        }
        wanting = next;
    }
    day
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
/// its limit, equal prices by lot, in quantities whole in both parties' lots, so each twin's share stays whole. A
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
                    buyer: Buyer::Party(buyer.party),
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
    use phx_core::GroupDemand;
    use phx_id::{MarketId, PartyId};
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Posting, between, posted};
    use crate::print::Buyer;

    struct Linear;

    impl GroupDemand for Linear {
        fn quantity_at(&self, _: MarketId, group: u64, price_raw: i64) -> i64 {
            // Each group wants ten units at no price, one fewer for each unit of price, and its number more.
            i64::try_from(group).unwrap() + 10 - price_raw
        }
    }

    fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.posted"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    #[test]
    fn posted_capacity_rechoice_by_lot() {
        let postings = [
            Posting { seller: PartyId::new(1), price: PriceRaw::from_raw(4), capacity: 5 },
            Posting { seller: PartyId::new(2), price: PriceRaw::from_raw(4), capacity: 5 },
            Posting { seller: PartyId::new(3), price: PriceRaw::from_raw(6), capacity: 20 },
        ];
        // Group 0 wants 6 at 4, group 2 wants 8; both see every seller.
        let groups = [(0, vec![0, 1, 2]), (2, vec![0, 1, 2])];
        let day = posted(MarketId::new(0), &postings, &groups, &Linear, &mut lot());
        let bought = |g: u64| -> i64 { day.sales.iter().filter(|s| s.buyer == Buyer::Group(g)).map(|s| s.qty).sum() };
        // One group takes a cheap seller's five, the other the other's five and, choosing again, wants at 6 one more.
        assert_eq!((bought(0), bought(2)), (5, 6), "a group chooses again at a dearer price, wanting less there");
        let cheap: i64 = day.sales.iter().filter(|s| s.price.raw() == 4).map(|s| s.qty).sum();
        assert_eq!(cheap, 10, "the cheaper sellers sell out first");
        assert!(day.rounds >= 2, "capacity forced a re-choice round");
        assert!(day.unserved.is_empty());
        assert_eq!(
            day,
            posted(MarketId::new(0), &postings, &groups, &Linear, &mut lot()),
            "the same lot, the same day"
        );
        let walk = posted(MarketId::new(0), &postings[..1], &[(0, vec![0])], &Linear, &mut lot());
        assert_eq!(walk.sales[0].qty, 5);
        assert_eq!(walk.unserved.get(&0), Some(&1), "demand left when every seller it sees is out");
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
        let bought = |p: u64| -> i64 {
            day.sales.iter().filter(|s| s.buyer == Buyer::Party(PartyId::new(p))).map(|s| s.qty).sum()
        };
        assert!(
            day.sales.iter().all(|s| s.price.raw() <= if s.buyer == Buyer::Party(PartyId::new(3)) { 6 } else { 8 })
        );
        assert_eq!(bought(3) % 10, 0, "a lot of ten buys whole lots");
        assert!(bought(3) <= 20 && bought(4) <= 40);
        let sold: i64 = day.sales.iter().map(|s| s.qty).sum();
        assert_eq!(sold, bought(3) + bought(4));
        assert!(
            day.sales.iter().filter(|s| s.seller == PartyId::new(2)).all(|s| s.buyer == Buyer::Party(PartyId::new(4)))
        );
    }
}
