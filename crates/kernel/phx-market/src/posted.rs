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

#[cfg(test)]
mod tests {
    use phx_core::GroupDemand;
    use phx_id::{MarketId, PartyId};
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Posting, posted};
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
}
