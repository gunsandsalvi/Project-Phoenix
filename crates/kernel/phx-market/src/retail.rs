//! The retail meeting: each buyer values the sellers in its reach by their posted prices, their distance and its own
//! taste for each that day, and goes to the one it values most; a seller serves those who came in an order drawn by
//! lot until its units run out, and the rest choose again among the sellers with units left, round by round.

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{PriceRaw, capacity_exceeded};
use phx_rand::Draws;
use phx_rand::uniform::below_u64;

use crate::market::MarketDecl;
use crate::print::{Buyer, Match};

/// A retail market kind as its system declares it: its market, an instance per product; the kinds that sell in it;
/// the facts naming what a seller sells and the price it posts; the primitives weighing price and distance and
/// bounding how far a buyer reaches; the stream buyers' tastes are drawn from; and the reason a purchase settles
/// under.
#[clause("SRV.1", "SRV.2", "SRV.9", "MKT.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetailKind {
    pub market: MarketDecl,
    pub sellers: &'static [&'static str],
    pub sells: &'static str,
    pub price: &'static str,
    pub price_weight: &'static str,
    pub distance_weight: &'static str,
    pub reach: &'static str,
    pub tastes: &'static str,
    pub reason: &'static str,
}

/// A seller at the meeting: who, its posted price for the product's least quantity, and the units it can serve today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stall {
    pub seller: PartyId,
    pub price: PriceRaw,
    pub units: i64,
}

/// What a buyer wants, a twin's: units it needs, or money it spends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Units(i64),
    Money(i64),
}

/// A seller in a buyer's reach: its place among the stalls, how far it is in km, and the buyer's taste for it today.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InReach {
    pub stall: usize,
    pub km: f64,
    pub taste: f64,
}

/// A buyer: who, its twins, who all buy alike, what a twin wants, and the sellers in its reach.
#[derive(Clone, Debug, PartialEq)]
pub struct Shopper {
    pub buyer: PartyId,
    pub twins: i64,
    pub want: Want,
    pub reach: Vec<InReach>,
}

/// How a buyer weighs a seller: per unit of the log of its price, and per km of distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    pub price: f64,
    pub distance: f64,
}

/// The meeting's outcome: each sale, a buyer buying whole lots for every twin from a seller at its posted price; what
/// each buyer that found no seller still wanted; the rounds capacity forced; and the sellers every buyer had in reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetailDay {
    pub sales: Vec<Match>,
    pub unserved: Vec<(PartyId, Want)>,
    pub rounds: u64,
    pub in_reach: u64,
}

/// A buyer still choosing: its place, what a twin still wants, and the sellers in its reach it can no longer buy from.
struct Choosing {
    at: usize,
    want: Want,
    bought: bool,
    closed: Vec<bool>,
}

/// Whole lots a twin wants at a price: its units' lots, or the lots its money buys.
fn lots_wanted(want: Want, lot: i64, price: i64) -> i64 {
    match want {
        Want::Units(q) => q / lot,
        Want::Money(m) if price > 0 => m / price,
        Want::Money(_) => 0,
    }
}

/// What a twin still wants after buying `lots` at `price`.
fn less(want: Want, lot: i64, lots: i64, price: i64) -> Want {
    match want {
        Want::Units(q) => Want::Units(q - lots * lot),
        Want::Money(m) => Want::Money(m - lots * price),
    }
}

/// The retail meeting over the day's stalls of one product, traded in lots of `lot` units a twin. Each buyer goes to
/// the open seller in its reach it values most, at `−w.price·ln(price) − w.distance·km + taste`; a seller serves its
/// buyers in an order drawn by lot, each as many whole lots for every twin as it wants and the seller's units allow;
/// a buyer served short, or that its money cannot buy a lot from there, chooses again among the rest. A buyer with no
/// seller left goes without.
#[clause("SRV.4", "SRV.5", "MKT.6", "REP.22", "REP.1")]
#[must_use]
pub fn retail(stalls: &[Stall], shoppers: &[Shopper], lot: i64, w: Weights, draws: &mut Draws) -> RetailDay {
    let mut left: Vec<i64> = stalls.iter().map(|s| s.units).collect();
    let in_reach = shoppers.iter().map(|s| phx_rand::float::len_u64(s.reach.len())).sum();
    let mut day = RetailDay { sales: Vec::new(), unserved: Vec::new(), rounds: 0, in_reach };
    let value = |r: &InReach| -> Option<f64> {
        let p = stalls.get(r.stall)?.price.raw();
        (p > 0).then(|| -w.price * libm::log(phx_rand::float::from_i64(p)) - w.distance * r.km + r.taste)
    };
    let mut choosing: Vec<Choosing> = shoppers
        .iter()
        .enumerate()
        .map(|(at, s)| Choosing { at, want: s.want, bought: false, closed: vec![false; s.reach.len()] })
        .collect();
    while !choosing.is_empty() {
        day.rounds += 1;
        // Each buyer's best open seller, as the stall and its place in the buyer's reach.
        let mut came: Vec<Vec<(usize, usize)>> = vec![Vec::new(); stalls.len()];
        for (c, ch) in choosing.iter_mut().enumerate() {
            let Some(s) = shoppers.get(ch.at) else { continue };
            let mut best: Option<(f64, usize)> = None;
            for (k, r) in s.reach.iter().enumerate() {
                let open = !ch.closed.get(k).copied().unwrap_or(true)
                    && left.get(r.stall).is_some_and(|u| *u >= lot * s.twins);
                let Some(v) = value(r).filter(|_| open) else { continue };
                if best.is_none_or(|(b, _)| v > b) {
                    best = Some((v, k));
                }
            }
            match best.and_then(|(_, k)| s.reach.get(k).map(|r| (r.stall, k))) {
                Some((stall, k)) => {
                    if let Some(list) = came.get_mut(stall) {
                        list.push((c, k));
                    }
                }
                None => day.unserved.push((s.buyer, ch.want)),
            }
        }
        let mut again: Vec<bool> = vec![false; choosing.len()];
        for (stall, mut list) in came.into_iter().enumerate() {
            // Those who came are served in an order drawn by lot.
            for i in (1..list.len()).rev() {
                let Ok(j) = usize::try_from(below_u64(draws, phx_rand::float::len_u64(i + 1))) else {
                    capacity_exceeded!("buyers at a seller", usize::MAX, i);
                };
                list.swap(i, j);
            }
            let (Some(st), Some(units)) = (stalls.get(stall), left.get_mut(stall)) else { continue };
            let price = st.price.raw();
            for (c, k) in list {
                let Some(ch) = choosing.get_mut(c) else { continue };
                let Some(s) = shoppers.get(ch.at) else { continue };
                let wanted = lots_wanted(ch.want, lot, price);
                let fit = *units / (lot * s.twins);
                let lots = if wanted < fit { wanted } else { fit };
                if lots > 0 {
                    *units -= lots * lot * s.twins;
                    day.sales.push(Match {
                        buyer: Buyer::Party(s.buyer),
                        seller: st.seller,
                        qty: lots * lot * s.twins,
                        price: st.price,
                        draws: phx_num::Missing::Absent,
                    });
                    ch.want = less(ch.want, lot, lots, price);
                    ch.bought = true;
                }
                // Served short, it chooses again; one whose money buys no lot here tries another if it has bought
                // nothing, since what is left of a budget after buying is change, not want.
                let short = lots < wanted;
                let priced_out = wanted == 0 && !ch.bought && matches!(ch.want, Want::Money(m) if m > 0);
                if short || priced_out {
                    if let Some(closed) = ch.closed.get_mut(k) {
                        *closed = true;
                    }
                    if let Some(a) = again.get_mut(c) {
                        *a = true;
                    }
                }
            }
        }
        choosing = choosing.into_iter().zip(again).filter(|(_, a)| *a).map(|(c, _)| c).collect();
    }
    day
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, gumbel, stream_key};

    use super::{InReach, Shopper, Stall, Want, Weights, retail};
    use crate::print::Buyer;

    fn draws(n: u64) -> Draws {
        Draws::new(stream_key(Seed::new(7), "SRV.taste"), Subject::new(SubjectTag::Market, n), 0, 0)
    }

    fn stall(seller: u64, price: i64, units: i64) -> Stall {
        Stall { seller: PartyId::new(seller), price: PriceRaw::from_raw(price), units }
    }

    #[test]
    fn logit_shares_from_gumbel_tastes() {
        let stalls = [stall(1, 100, i64::MAX / 4), stall(2, 150, i64::MAX / 4), stall(3, 100, i64::MAX / 4)];
        let km = [0.0, 0.0, 2.0];
        let w = Weights { price: 1.7, distance: 0.3 };
        let n = 40_000_u64;
        let mut taste = draws(1);
        let shoppers: Vec<Shopper> = (0..n)
            .map(|b| Shopper {
                buyer: PartyId::new(b + 10),
                twins: 1,
                want: Want::Units(1),
                reach: (0..stalls.len())
                    .map(|i| InReach { stall: i, km: km[i], taste: gumbel(&mut taste, 0.0, 1.0) })
                    .collect(),
            })
            .collect();
        let day = retail(&stalls, &shoppers, 1, w, &mut draws(2));
        let v: Vec<f64> = (0..3)
            .map(|i| -w.price * phx_rand::float::from_i64(stalls[i].price.raw()).ln() - w.distance * km[i])
            .collect();
        let total: f64 = v.iter().map(|x| x.exp()).sum();
        for (i, s) in stalls.iter().enumerate() {
            let sold = phx_rand::float::from_u64(phx_rand::float::len_u64(
                day.sales.iter().filter(|m| m.seller == s.seller).count(),
            ));
            let expected = v[i].exp() / total;
            assert!(
                (sold / phx_rand::float::from_u64(n) - expected).abs() < 0.01,
                "seller {i}: {} against {expected}",
                sold / phx_rand::float::from_u64(n)
            );
        }
        assert_eq!(day.rounds, 1);
    }

    #[test]
    fn capacity_rechoice_by_lot() {
        let stalls = [stall(1, 10, 10), stall(2, 30, 1_000)];
        let shoppers: Vec<Shopper> = (0..5)
            .map(|b| Shopper {
                buyer: PartyId::new(b + 10),
                twins: 1,
                want: Want::Units(4),
                reach: vec![InReach { stall: 0, km: 0.0, taste: 0.0 }, InReach { stall: 1, km: 0.0, taste: 0.0 }],
            })
            .collect();
        let day = retail(&stalls, &shoppers, 1, Weights { price: 1.0, distance: 0.0 }, &mut draws(3));
        let at = |s: u64| day.sales.iter().filter(|m| m.seller == PartyId::new(s)).map(|m| m.qty).sum::<i64>();
        assert_eq!(at(1), 10, "the cheap seller serves no more than its units");
        assert_eq!(at(2), 10, "the rest choose again");
        assert!(day.rounds >= 2);
        assert!(day.unserved.is_empty());
        for b in 10..15 {
            let got: i64 = day.sales.iter().filter(|m| m.buyer == Buyer::Party(PartyId::new(b))).map(|m| m.qty).sum();
            assert_eq!(got, 4);
        }
        let twins = Shopper { buyer: PartyId::new(99), twins: 3, want: Want::Money(25), ..shoppers[0].clone() };
        let day = retail(&stalls, &[twins], 1, Weights { price: 1.0, distance: 0.0 }, &mut draws(4));
        assert_eq!(day.sales.first().map(|m| m.qty), Some(6), "two lots a twin, whole for every twin");
    }
}
