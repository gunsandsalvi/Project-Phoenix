//! The retail meeting: each buyer values the sellers in its reach by their posted prices, their distance and its own
//! taste for each that day, and goes to the one it values most; a seller serves those who came in an order drawn by
//! lot until its units run out, and the rest choose again among the sellers with units left, round by round.

use std::collections::BTreeMap;

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{PriceRaw, capacity_exceeded};
use phx_rand::Draws;
use phx_rand::uniform::below_u64;

use crate::market::MarketDecl;
use crate::print::Match;

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
    /// A seller's units made a day, what it can serve of a product delivered as it is made: its fact, read at its
    /// fixed point.
    pub capacity: phx_core::ItemDecl,
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

/// A seller at the meeting: who, its posted price for the product's least quantity, and the units it can serve today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stall {
    pub seller: PartyId,
    pub price: PriceRaw,
    pub units: i64,
}

/// What a buyer wants: units it needs, or money it spends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Units(i64),
    Money(i64),
}

/// A seller in reach of the place buyers stand at: its place among the stalls and how far it is in km.
pub type Near = (usize, f64);

/// A buyer: who, what it wants, the place among the meeting's reaches its sellers in reach are listed at, and its own
/// stream, which draws its choices.
#[derive(Clone, Debug)]
pub struct Shopper {
    pub buyer: PartyId,
    pub want: Want,
    pub reach: usize,
    pub draws: Draws,
}

/// How a buyer weighs a seller: per unit of the log of its price, and per km of distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    pub price: f64,
    pub distance: f64,
}

/// The meeting's outcome: each sale, a buyer buying whole units from a seller at its posted price; what
/// each buyer that found no seller still wanted; the rounds capacity forced; and the sellers every buyer had in reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetailDay {
    pub sales: Vec<Match>,
    pub unserved: Vec<(PartyId, Want)>,
    pub rounds: u64,
    pub in_reach: u64,
}

/// A buyer still choosing: its place, what it still wants, whether it has bought, and the sellers it was turned
/// away from, by their places in its reach.
struct Choosing {
    at: usize,
    want: Want,
    bought: bool,
    closed: Vec<usize>,
}

/// What `units` of a good cost at `price` a lot of `lot` units, rounded once, as the sale settles.
#[must_use]
pub fn paid(units: i64, price: i64, lot: i64) -> i64 {
    let amount =
        phx_num::round::div_round(i128::from(units) * i128::from(price), i128::from(lot), phx_num::Round::HalfEven);
    let Ok(amount) = i64::try_from(amount) else {
        capacity_exceeded!("a trade's money", i64::MAX, units);
    };
    amount
}

/// Whole units a buyer wants at a price posted for a lot: its units, or the units its money buys.
pub(crate) fn units_wanted(want: Want, lot: i64, price: i64) -> i64 {
    match want {
        Want::Units(q) => q,
        // A word's product divides in a word; only a larger one needs two.
        Want::Money(m) if price > 0 => match m.checked_mul(lot) {
            Some(p) => p / price,
            None => i64::try_from(i128::from(m) * i128::from(lot) / i128::from(price)).unwrap_or(i64::MAX),
        },
        Want::Money(_) => 0,
    }
}

/// What a buyer still wants after buying `units` at `price` a lot.
pub(crate) fn less(want: Want, lot: i64, units: i64, price: i64) -> Want {
    match want {
        Want::Units(q) => Want::Units(q - units),
        Want::Money(m) => Want::Money(m - paid(units, price, lot)),
    }
}

/// The retail meeting over the day's stalls of one product, traded in lots of `lot` units. Each buyer goes to
/// the open seller in its reach it values most, at `−w.price·ln(price) − w.distance·km + taste`, its taste for each a
/// standard Gumbel draw: so it goes to each with the logit's chance, `exp(v) ÷ Σ exp(v)` over the open sellers it has
/// not been turned away from, and that chance is what it draws from, one uniform draw of its own stream a choice,
/// never its tastes, which nothing else reads; a buyer choosing again chooses among the rest by the same chance, as the
/// order its tastes rank the sellers in is. A seller serves its buyers in an order drawn by lot, each as many whole
/// units as it wants and the seller's units allow, at its price for a lot of `lot` units pro rata; a buyer served
/// short, or that its money cannot buy a unit from there, chooses again. A buyer with no seller left goes without.
#[clause("SRV.4", "SRV.5", "MKT.6", "REP.22")]
#[must_use]
pub fn retail(
    stalls: &[Stall],
    shoppers: &mut [Shopper],
    reaches: &[Vec<Near>],
    lot: i64,
    w: Weights,
    draws: &mut Draws,
) -> RetailDay {
    let mut left: Vec<i64> = stalls.iter().map(|s| s.units).collect();
    let reach_of = |s: &Shopper| reaches.get(s.reach).map_or(&[][..], Vec::as_slice);
    let in_reach = shoppers.iter().map(|s| phx_rand::float::len_u64(reach_of(s).len())).sum();
    let mut day = RetailDay { sales: Vec::new(), unserved: Vec::new(), rounds: 0, in_reach };
    let log_price: Vec<Option<f64>> = stalls
        .iter()
        .map(|s| (s.price.raw() > 0).then(|| libm::log(phx_rand::float::from_i64(s.price.raw()))))
        .collect();
    // Each place's weight for each seller in its reach, `exp(v)` over the best's, so the best weighs one and none
    // overflows; a seller with no price weighs nothing.
    let weights: Vec<Vec<f64>> = reaches
        .iter()
        .map(|near| {
            let v: Vec<Option<f64>> = near
                .iter()
                .map(|(stall, km)| log_price.get(*stall).copied().flatten().map(|l| -w.price * l - w.distance * km))
                .collect();
            let best = v.iter().flatten().copied().reduce(|a, b| if b > a { b } else { a });
            v.iter().map(|x| x.zip(best).map_or(0.0, |(x, b)| libm::exp(x - b))).collect()
        })
        .collect();
    let mut choosing: Vec<Choosing> = shoppers
        .iter()
        .enumerate()
        .map(|(at, s)| Choosing { at, want: s.want, bought: false, closed: Vec::new() })
        .collect();
    let mut ranked: BTreeMap<(usize, bool), Ranked> = BTreeMap::new();
    let mut came: Vec<(usize, usize, usize)> = Vec::new();
    while !choosing.is_empty() {
        day.rounds += 1;
        came.clear();
        let round = day.rounds;
        for (c, ch) in choosing.iter_mut().enumerate() {
            let Some(s) = shoppers.get_mut(ch.at) else { continue };
            let near = reaches.get(s.reach).map_or(&[][..], Vec::as_slice);
            let (by_money, brings) = match ch.want {
                Want::Money(m) => (true, m),
                Want::Units(q) => (false, q),
            };
            let r = ranked.entry((s.reach, by_money)).or_insert_with(|| Ranked::new(near, stalls, (by_money, lot)));
            if r.round != round {
                r.open(near, &left, weights.get(s.reach).map_or(&[][..], Vec::as_slice));
                r.round = round;
            }
            // The sellers it can buy a unit at are the first of the order.
            let can = r.bound.partition_point(|b| *b <= brings);
            match choose(r, can, &ch.closed, &mut s.draws) {
                Some(k) => {
                    if let Some((stall, _)) = near.get(k) {
                        came.push((*stall, c, k));
                    }
                }
                // One that has bought and whose money buys a share at no seller still open keeps its change; one with
                // no seller open to it goes without.
                None if ch.bought && open_anywhere(r, &ch.closed) => {}
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
                let wanted = units_wanted(ch.want, lot, price);
                let fit = *units;
                let bought = if wanted < fit { wanted } else { fit };
                if bought > 0 {
                    *units -= bought;
                    day.sales.push(Match {
                        buyer: s.buyer,
                        seller: st.seller,
                        qty: bought,
                        price: st.price,
                        draws: phx_num::Missing::Absent,
                    });
                    ch.want = less(ch.want, lot, bought, price);
                    ch.bought = true;
                }
                // Served short, it chooses again; one whose money buys no unit here tries another if it has bought
                // nothing, since what is left of a budget after buying is change, not want.
                let short = bought < wanted;
                let priced_out = wanted == 0 && !ch.bought && matches!(ch.want, Want::Money(m) if m > 0);
                if short || priced_out {
                    ch.closed.push(k);
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

/// A place's sellers ordered by what a buyer must bring to buy a unit at each — money, a unit's price, for a buyer with
/// a budget; for one that needs units, one unit at any — so what a buyer can buy at is a prefix of the order; with each
/// seller's place in the reach, its place in the order, and the running sums of the open sellers' weights in the order
/// as of the round they were summed in.
struct Ranked {
    places: Vec<usize>,
    at: Vec<usize>,
    bound: Vec<i64>,
    sums: Vec<f64>,
    round: u64,
}

impl Ranked {
    fn new(near: &[Near], stalls: &[Stall], (by_money, lot): (bool, i64)) -> Ranked {
        let mut order: Vec<(i64, usize)> = near
            .iter()
            .enumerate()
            .map(|(k, (stall, _))| {
                let Some(st) = stalls.get(*stall) else { return (i64::MAX, k) };
                // A unit's price rounded up, the least money that buys one there.
                let unit = st.price.raw() / lot + i64::from(st.price.raw() % lot > 0);
                (if by_money { unit } else { 1 }, k)
            })
            .collect();
        order.sort_unstable();
        let places: Vec<usize> = order.iter().map(|(_, k)| *k).collect();
        let mut at = vec![0; near.len()];
        for (i, k) in places.iter().enumerate() {
            if let Some(a) = at.get_mut(*k) {
                *a = i;
            }
        }
        Ranked { bound: order.iter().map(|(b, _)| *b).collect(), places, at, sums: Vec::new(), round: 0 }
    }

    /// The running sums summed again over the sellers with a unit left.
    fn open(&mut self, near: &[Near], left: &[i64], weights: &[f64]) {
        let mut total = 0.0;
        self.sums.clear();
        for k in &self.places {
            let fits = near.get(*k).is_some_and(|(stall, _)| left.get(*stall).is_some_and(|u| *u > 0));
            if fits {
                total += weights.get(*k).copied().unwrap_or(0.0);
            }
            self.sums.push(total);
        }
    }
}

/// Whether any seller of the order is still open to a buyer, whatever it can buy there.
fn open_anywhere(r: &Ranked, closed: &[usize]) -> bool {
    let sum = |i: usize| r.sums.get(i).copied().unwrap_or(0.0);
    (0..r.sums.len()).any(|i| {
        sum(i) - i.checked_sub(1).map_or(0.0, sum) > 0.0 && !closed.contains(r.places.get(i).unwrap_or(&usize::MAX))
    })
}

/// A buyer's choice among the first `can` sellers of an order, as places in its reach: one uniform draw over the running
/// sums of the open sellers' weights, less those of the sellers it was turned away from; none when no weight is left.
fn choose(r: &Ranked, can: usize, closed: &[usize], d: &mut Draws) -> Option<usize> {
    let sum = |i: usize| r.sums.get(i).copied().unwrap_or(0.0);
    let weight = |i: usize| sum(i) - i.checked_sub(1).map_or(0.0, sum);
    let shut_at: Vec<usize> = closed.iter().filter_map(|k| r.at.get(*k).copied()).filter(|i| *i < can).collect();
    // What the sellers it was turned away from weigh up to and including a place in the order.
    let shut = |i: usize| shut_at.iter().filter(|j| **j <= i).map(|j| weight(*j)).sum::<f64>();
    let last = can.checked_sub(1)?;
    let total = sum(last) - shut(last);
    if total <= 0.0 {
        return None;
    }
    let x = phx_rand::uniform::open_unit(d) * total;
    // The first place whose running sum, less what it was turned away from, passes the draw.
    let (mut i, mut hi) = (0, can);
    while i < hi {
        let mid = i + (hi - i) / 2;
        if sum(mid) - shut(mid) <= x {
            i = mid + 1;
        } else {
            hi = mid;
        }
    }
    // A draw at the very top, where rounding leaves no place above it, falls to the last seller still open to it.
    let open = |j: &usize| weight(*j) > 0.0 && !shut_at.contains(j);
    let found = (i..can).find(open).or_else(|| (0..i).rev().find(open))?;
    r.places.get(found).copied()
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Near, Shopper, Stall, Want, Weights, retail};

    fn draws(n: u64) -> Draws {
        Draws::new(stream_key(Seed::new(7), "SRV.taste"), Subject::new(SubjectTag::Market, n), 0, 0)
    }

    fn stall(seller: u64, price: i64, units: i64) -> Stall {
        Stall { seller: PartyId::new(seller), price: PriceRaw::from_raw(price), units }
    }

    fn shopper(b: u64, want: Want) -> Shopper {
        let d = Draws::new(stream_key(Seed::new(7), "SRV.taste"), Subject::new(SubjectTag::Party, b), 0, 0);
        Shopper { buyer: PartyId::new(b), want, reach: 0, draws: d }
    }

    #[test]
    fn logit_shares_of_the_open_sellers() {
        let stalls = [stall(1, 100, i64::MAX / 4), stall(2, 150, i64::MAX / 4), stall(3, 100, i64::MAX / 4)];
        let near: Vec<Near> = vec![(0, 0.0), (1, 0.0), (2, 2.0)];
        let w = Weights { price: 1.7, distance: 0.3 };
        let n = 40_000_u64;
        let mut shoppers: Vec<Shopper> = (0..n).map(|b| shopper(b + 10, Want::Units(1))).collect();
        let day = retail(&stalls, &mut shoppers, std::slice::from_ref(&near), 1, w, &mut draws(2));
        let v: Vec<f64> = near
            .iter()
            .map(|(i, km)| -w.price * phx_rand::float::from_i64(stalls[*i].price.raw()).ln() - w.distance * km)
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
        let near: Vec<Near> = vec![(0, 0.0), (1, 0.0)];
        let w = Weights { price: 1.0, distance: 0.0 };
        let mut shoppers: Vec<Shopper> = (0..5).map(|b| shopper(b + 10, Want::Units(4))).collect();
        let day = retail(&stalls, &mut shoppers, std::slice::from_ref(&near), 1, w, &mut draws(3));
        let at = |s: u64| day.sales.iter().filter(|m| m.seller == PartyId::new(s)).map(|m| m.qty).sum::<i64>();
        assert!(at(1) <= 10, "the cheap seller serves no more than its units");
        assert_eq!(at(1) + at(2), 20, "those served short choose again");
        assert!(day.unserved.is_empty());
        for b in 10..15 {
            let got: i64 = day.sales.iter().filter(|m| m.buyer == PartyId::new(b)).map(|m| m.qty).sum();
            assert_eq!(got, 4);
        }
        let mut budget = [shopper(99, Want::Money(25))];
        let one = [stall(1, 10, 10)];
        let day = retail(&one, &mut budget, &[vec![(0, 0.0)]], 1, w, &mut draws(4));
        assert_eq!(day.sales.first().map(|m| m.qty), Some(2), "the units its money buys, its change kept");
    }

    #[test]
    fn a_turned_away_buyer_goes_without_when_none_is_left() {
        let stalls = [stall(1, 10, 1)];
        let w = Weights { price: 1.0, distance: 0.0 };
        let mut shoppers = [shopper(10, Want::Units(2))];
        let day = retail(&stalls, &mut shoppers, &[vec![(0, 0.0)]], 1, w, &mut draws(8));
        assert_eq!(day.sales.first().map(|m| m.qty), Some(1));
        assert_eq!(day.unserved, vec![(PartyId::new(10), Want::Units(1))]);
    }

    #[test]
    fn a_sale_is_whole_units_at_a_price_for_a_lot() {
        let stalls = [stall(1, 4_990, 1_000_000)];
        let w = Weights { price: 1.0, distance: 0.0 };
        let reach = [vec![(0, 0.0)]];
        let day = retail(&stalls, &mut [shopper(10, Want::Units(7))], &reach, 10_000, w, &mut draws(5));
        let [m] = day.sales.as_slice() else { panic!("one sale") };
        assert_eq!(m.qty, 7, "seven units wanted buy seven, however large the lot a price is posted for");
        let budget = retail(&stalls, &mut [shopper(11, Want::Money(100))], &reach, 10_000, w, &mut draws(6));
        let [b] = budget.sales.as_slice() else { panic!("one sale") };
        assert_eq!(b.qty, 200, "a hundred buys what it covers at 4 990 a lot of 10 000");
        assert_eq!(super::paid(b.qty, 4_990, 10_000), 100, "and costs no more than it holds");
    }
}
