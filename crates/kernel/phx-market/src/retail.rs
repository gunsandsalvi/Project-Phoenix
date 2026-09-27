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
    /// A seller's units made a day, what it can serve of a product delivered as it is made.
    pub capacity: &'static str,
    /// The way a seller makes its product by.
    pub way: &'static str,
    /// A seller's units a day its plant allows, where its plant limits it.
    pub plant: &'static str,
    pub price_weight: &'static str,
    pub distance_weight: &'static str,
    pub reach: &'static str,
    pub tastes: &'static str,
    pub reason: &'static str,
}

/// A seller at the meeting: who, its posted price for the product's least quantity, the units it can serve today, and
/// its twins, who all sell alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stall {
    pub seller: PartyId,
    pub price: PriceRaw,
    pub units: i64,
    pub twins: i64,
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

/// A buyer still choosing: its place, what a twin still wants, whether it has bought, and, once it chooses again, the
/// sellers in its reach it has not closed, best first, with how many it has passed.
struct Choosing {
    at: usize,
    want: Want,
    bought: bool,
    ranked: Vec<(f64, usize)>,
    passed: usize,
}

/// Whole lots a twin wants at a price: its units' lots, or the lots its money buys.
fn lots_wanted(want: Want, lot: i64, price: i64) -> i64 {
    match want {
        Want::Units(q) => q / lot,
        Want::Money(m) if price > 0 => m / price,
        Want::Money(_) => 0,
    }
}

/// The greatest count dividing both.
fn common(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The lots a buyer's twin buys at a stall in steps of: as many as make the sale a whole share for every twin of the
/// seller as well, since each of the seller's twins sells its own share.
fn step(buyer: i64, seller: i64) -> i64 {
    seller / common(buyer, seller)
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
    let log_price: Vec<Option<f64>> = stalls
        .iter()
        .map(|s| (s.price.raw() > 0).then(|| libm::log(phx_rand::float::from_i64(s.price.raw()))))
        .collect();
    let value = |r: &InReach| -> Option<f64> {
        log_price.get(r.stall).copied().flatten().map(|l| -w.price * l - w.distance * r.km + r.taste)
    };
    let mut choosing: Vec<Choosing> = shoppers
        .iter()
        .enumerate()
        .map(|(at, s)| Choosing { at, want: s.want, bought: false, ranked: Vec::new(), passed: 0 })
        .collect();
    let mut first = true;
    let mut came: Vec<(usize, usize, usize)> = Vec::new();
    while !choosing.is_empty() {
        day.rounds += 1;
        // Each buyer's best open seller, as the stall and its place in the buyer's reach. A seller once closed to a
        // buyer, or with too few units left for it, stays so, since units only fall: a buyer choosing again reads
        // its ranked sellers on from where it stopped.
        came.clear();
        for (c, ch) in choosing.iter_mut().enumerate() {
            let Some(s) = shoppers.get(ch.at) else { continue };
            let open = |k: usize| {
                s.reach.get(k).is_some_and(|r| {
                    stalls
                        .get(r.stall)
                        .zip(left.get(r.stall))
                        .is_some_and(|(st, u)| *u >= lot * s.twins * step(s.twins, st.twins))
                })
            };
            let best = if first {
                let mut best: Option<(f64, usize)> = None;
                for (k, r) in s.reach.iter().enumerate() {
                    let Some(v) = value(r).filter(|_| open(k)) else { continue };
                    if best.is_none_or(|(b, _)| v > b) {
                        best = Some((v, k));
                    }
                }
                best.map(|(_, k)| k)
            } else {
                while ch.ranked.get(ch.passed).is_some_and(|(_, k)| !open(*k)) {
                    ch.passed += 1;
                }
                ch.ranked.get(ch.passed).map(|(_, k)| *k)
            };
            match best.and_then(|k| s.reach.get(k).map(|r| (r.stall, k))) {
                Some((stall, k)) => came.push((stall, c, k)),
                None => day.unserved.push((s.buyer, ch.want)),
            }
        }
        // Stable, so those at a seller stay in the buyers' order before the lot is drawn.
        came.sort_by_key(|(stall, _, _)| *stall);
        let mut again: Vec<bool> = vec![false; choosing.len()];
        for group in came.chunk_by_mut(|a, b| a.0 == b.0) {
            let stall = group.first().map_or(usize::MAX, |g| g.0);
            // Those who came are served in an order drawn by lot.
            for i in (1..group.len()).rev() {
                let Ok(j) = usize::try_from(below_u64(draws, phx_rand::float::len_u64(i + 1))) else {
                    capacity_exceeded!("buyers at a seller", usize::MAX, i);
                };
                group.swap(i, j);
            }
            let (Some(st), Some(units)) = (stalls.get(stall), left.get_mut(stall)) else { continue };
            let price = st.price.raw();
            for &(_, c, k) in group.iter() {
                let Some(ch) = choosing.get_mut(c) else { continue };
                let Some(s) = shoppers.get(ch.at) else { continue };
                let g = step(s.twins, st.twins);
                let asked = lots_wanted(ch.want, lot, price);
                let wanted = asked / g * g;
                let fit = *units / (lot * s.twins) / g * g;
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
                // One that wants less than the seller's twins can each sell a whole share of looks elsewhere too.
                let short = lots < wanted || (asked > 0 && wanted == 0);
                let priced_out = asked == 0 && !ch.bought && matches!(ch.want, Want::Money(m) if m > 0);
                if short || priced_out {
                    close(ch, s, k, &value, first);
                    if let Some(a) = again.get_mut(c) {
                        *a = true;
                    }
                }
            }
        }
        choosing = choosing.into_iter().zip(again).filter(|(_, a)| *a).map(|(c, _)| c).collect();
        first = false;
    }
    day
}

/// Closes the seller at `k` in a buyer's reach: on its first choosing again the buyer ranks the rest best first, ties
/// to the earlier in its reach, as a scan of its reach would choose; after that the closed one is the one it had
/// reached.
fn close(ch: &mut Choosing, s: &Shopper, k: usize, value: &impl Fn(&InReach) -> Option<f64>, first: bool) {
    if first {
        ch.ranked =
            s.reach.iter().enumerate().filter(|(i, _)| *i != k).filter_map(|(i, r)| value(r).map(|v| (v, i))).collect();
        ch.ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        ch.passed = 0;
    } else {
        ch.passed += 1;
    }
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
        Stall { seller: PartyId::new(seller), price: PriceRaw::from_raw(price), units, twins: 1 }
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

    #[test]
    fn a_sale_is_whole_for_every_twin_of_both_sides() {
        let stalls = [Stall { twins: 3, ..stall(1, 10, 1_000) }];
        let buyer = |b: u64, twins: i64, want: i64| Shopper {
            buyer: PartyId::new(b),
            twins,
            want: Want::Units(want),
            reach: vec![InReach { stall: 0, km: 0.0, taste: 0.0 }],
        };
        let one = retail(&stalls, &[buyer(10, 1, 7)], 1, Weights { price: 1.0, distance: 0.0 }, &mut draws(5));
        let [m] = one.sales.as_slice() else { panic!("one sale") };
        assert_eq!(m.qty, 6, "a lone buyer takes lots in threes, one for each of the seller's twins");
        let small = retail(&stalls, &[buyer(11, 1, 2)], 1, Weights { price: 1.0, distance: 0.0 }, &mut draws(6));
        assert!(small.sales.is_empty(), "less than a share for each of the seller's twins buys nothing there");
        let twins = retail(&stalls, &[buyer(12, 2, 2)], 1, Weights { price: 1.0, distance: 0.0 }, &mut draws(7));
        assert!(twins.sales.is_empty(), "two twins wanting two lots each cannot split them among three");
    }
}
