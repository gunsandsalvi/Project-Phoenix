use std::collections::BTreeMap;

use phx_core::contribution::apportion;
use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded, violation};
use phx_rand::Draws;

use crate::failure::FailureKind;
use crate::market::{Ration, TieRule};
use crate::order::{Order, Side};
use crate::print::{Buyer, Match};
use crate::simplex::at as entry;

/// What a call reads besides its orders: the operator's tie sequence and rationing, and the market's last print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallRules<'a> {
    pub ties: &'a [TieRule],
    pub ration: Ration,
    pub last: Missing<PriceRaw>,
}

/// What an order was filled, by its index among the call's orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    pub order: usize,
    pub qty: i64,
}

/// A call that formed a price: the price, the quantity traded, each order's fill and the matches that pair the
/// buyers' fills with the sellers'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cleared {
    pub price: PriceRaw,
    pub volume: i64,
    pub fills: Vec<Fill>,
    pub matches: Vec<Match>,
}

/// What a meeting came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Cleared(Cleared),
    Failed(FailureKind),
}

/// One side's steps, sorted by limit, with the quantity at and beyond each.
struct Curve {
    limits: Vec<i64>,
    /// `below[i]` is the quantity of the steps before `i` in limit order.
    below: Vec<i128>,
}

impl Curve {
    fn new(mut steps: Vec<(i64, i128)>) -> Curve {
        steps.sort_unstable();
        let mut below = Vec::with_capacity(steps.len() + 1);
        let mut sum = 0_i128;
        below.push(sum);
        for (_, q) in &steps {
            sum += q;
            below.push(sum);
        }
        Curve { limits: steps.into_iter().map(|(l, _)| l).collect(), below }
    }

    fn total(&self) -> i128 {
        entry(&self.below, self.limits.len())
    }

    /// The quantity of steps whose limit is below `p`, or at most `p` when `inclusive`.
    fn up_to(&self, p: i64, inclusive: bool) -> i128 {
        let n = self.limits.partition_point(|l| if inclusive { *l <= p } else { *l < p });
        entry(&self.below, n)
    }
}

pub(crate) fn lesser(a: i128, b: i128) -> i128 {
    if a < b { a } else { b }
}

/// Supply and demand at a price, and what is strictly better than it on each side.
#[derive(Clone, Copy, Debug)]
struct At {
    supply: i128,
    demand: i128,
    offers_below: i128,
    bids_above: i128,
}

fn at(bids: &Curve, offers: &Curve, p: i64) -> At {
    At {
        supply: offers.up_to(p, true),
        demand: bids.total() - bids.up_to(p, false),
        offers_below: offers.up_to(p, false),
        bids_above: bids.total() - bids.up_to(p, true),
    }
}

/// Whether a price clears: something trades, and the steps strictly better than it on each side fit within what the
/// other side brings at it, so every one of them fills in full.
fn clears(a: At) -> bool {
    lesser(a.supply, a.demand) > 0 && a.offers_below <= a.demand && a.bids_above <= a.supply
}

/// The clearing prices kept by one tie rule: those best by it, or all where it does not apply.
fn keep(rule: TieRule, candidates: Vec<(i64, At)>, last: Missing<PriceRaw>) -> Vec<(i64, At)> {
    let score = |p: i64, a: At| -> Option<i128> {
        match rule {
            TieRule::MaxVolume => Some(-lesser(a.supply, a.demand)),
            TieRule::MinImbalance => Some((a.supply - a.demand).abs()),
            TieRule::NearestLastPrint => match last {
                Missing::Present(l) => Some((i128::from(p) - i128::from(l.raw())).abs()),
                Missing::Absent => None,
            },
            TieRule::LowerPrice => Some(i128::from(p)),
        }
    };
    let scored: Vec<(Option<i128>, (i64, At))> = candidates.into_iter().map(|(p, a)| (score(p, a), (p, a))).collect();
    let Some(best) = scored.iter().filter_map(|(s, _)| *s).reduce(|b, s| if s < b { s } else { b }) else {
        return scored.into_iter().map(|(_, c)| c).collect();
    };
    scored.into_iter().filter(|(s, _)| *s == Some(best)).map(|(_, c)| c).collect()
}

/// Steps of one side at the price shared a quantity: in proportion to what each asked, by largest remainder with
/// ties by lot, or in the order of their posters' declared priority, pro rata within a priority.
pub(crate) fn ration(
    share: i128,
    at_price: &[(usize, i128, Missing<u32>)],
    rule: Ration,
    lot: &mut Draws,
) -> Vec<i128> {
    let whole = |v: i128| -> u64 {
        let Ok(v) = u64::try_from(v) else {
            capacity_exceeded!("a quantity rationed", u64::MAX, 0);
        };
        v
    };
    let pro_rata = |share: i128, steps: &[i128], lot: &mut Draws| -> Vec<i128> {
        let weights: Vec<u64> = steps.iter().map(|q| whole(*q)).collect();
        apportion(whole(share), &weights, lot).into_iter().map(i128::from).collect()
    };
    match rule {
        Ration::ProRata => pro_rata(share, &at_price.iter().map(|s| s.1).collect::<Vec<_>>(), lot),
        Ration::Priority => {
            let mut ranks: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
            for (i, (_, _, p)) in at_price.iter().enumerate() {
                let Missing::Present(rank) = p else {
                    violation!(clause = "MKT.3", "an order with no priority in a market that rations by priority");
                };
                ranks.entry(*rank).or_default().push(i);
            }
            let mut out = vec![0_i128; at_price.len()];
            let mut left = share;
            for members in ranks.values() {
                let asked: Vec<i128> = members.iter().filter_map(|i| at_price.get(*i)).map(|s| s.1).collect();
                let total: i128 = asked.iter().sum();
                let given = if left >= total { asked } else { pro_rata(left, &asked, lot) };
                for (i, g) in members.iter().zip(given) {
                    if let Some(o) = out.get_mut(*i) {
                        *o = g;
                    }
                    left -= g;
                }
                if left == 0 {
                    break;
                }
            }
            out
        }
    }
}

/// One side's fills at the price: steps strictly better fill in full, the steps at the price share the rest.
fn fill_side(orders: &[Order], side: Side, price: i64, volume: i128, rule: Ration, lot: &mut Draws) -> Vec<i128> {
    let better = |limit: i64| match side {
        Side::Buy => limit > price,
        Side::Sell => limit < price,
    };
    let mut fills = vec![0_i128; orders.len()];
    let mut full = 0_i128;
    let mut at_price: Vec<(usize, i128, Missing<u32>)> = Vec::new();
    for (i, o) in orders.iter().enumerate().filter(|(_, o)| o.side == side) {
        for s in &o.steps {
            if better(s.limit.raw()) {
                full += i128::from(s.qty);
                if let Some(f) = fills.get_mut(i) {
                    *f += i128::from(s.qty);
                }
            } else if s.limit.raw() == price {
                at_price.push((i, i128::from(s.qty), o.priority));
            }
        }
    }
    if !at_price.is_empty() {
        for ((i, _, _), g) in at_price.iter().zip(ration(volume - full, &at_price, rule, lot)) {
            if let Some(f) = fills.get_mut(*i) {
                *f += g;
            }
        }
    }
    fills
}

/// The buyers' fills paired with the sellers' in the order of their parties, each pair a match at the price; a
/// party on both sides trades with itself for nothing, so only what it bought or sold beyond the other is matched.
fn pair(orders: &[Order], fills: &[i128], price: PriceRaw) -> Vec<Match> {
    let mut net: BTreeMap<PartyId, i128> = BTreeMap::new();
    for (o, f) in orders.iter().zip(fills) {
        let signed = match o.side {
            Side::Buy => *f,
            Side::Sell => -*f,
        };
        *net.entry(o.party).or_insert(0) += signed;
    }
    pair_net(&net, price)
}

/// Parties' net purchases (positive) and sales (negative) at one price paired in the order of their parties.
pub(crate) fn pair_net(net: &BTreeMap<PartyId, i128>, price: PriceRaw) -> Vec<Match> {
    let mut buyers: Vec<(PartyId, i128)> = net.iter().filter(|(_, q)| **q > 0).map(|(p, q)| (*p, *q)).collect();
    let mut sellers: Vec<(PartyId, i128)> = net.iter().filter(|(_, q)| **q < 0).map(|(p, q)| (*p, -*q)).collect();
    let (mut b, mut s) = (0, 0);
    let mut out = Vec::new();
    while let (Some(buyer), Some(seller)) = (buyers.get_mut(b), sellers.get_mut(s)) {
        let q = lesser(buyer.1, seller.1);
        let Ok(qty) = i64::try_from(q) else {
            capacity_exceeded!("a match's quantity", i64::MAX, 0);
        };
        out.push(Match { buyer: Buyer::Party(buyer.0), seller: seller.0, qty, price, draws: Missing::Absent });
        buyer.1 -= q;
        seller.1 -= q;
        if buyer.1 == 0 {
            b += 1;
        }
        if seller.1 == 0 {
            s += 1;
        }
    }
    out
}

/// Each order's fill in whole lots of its own, the two sides kept equal: a fill is first cut to its whole lots, then
/// the side that gives more loses lots until it gives what the other takes, from the orders at the price before those
/// better than it, the smallest lots first, the last posted first; so an agent trades whole shares for each twin.
fn whole_lots(orders: &[Order], mut fills: Vec<i128>, price: i64) -> Vec<i128> {
    for (o, f) in orders.iter().zip(fills.iter_mut()) {
        *f -= *f % i128::from(o.lot);
    }
    let mut cutting: Vec<(usize, &Order)> = orders.iter().enumerate().collect();
    cutting.sort_by_key(|(i, o)| (!o.steps.iter().any(|s| s.limit.raw() == price), o.lot, core::cmp::Reverse(*i)));
    loop {
        let side_total =
            |side: Side| -> i128 { orders.iter().zip(&fills).filter(|(o, _)| o.side == side).map(|(_, f)| f).sum() };
        let (bought, sold) = (side_total(Side::Buy), side_total(Side::Sell));
        if bought == sold {
            return fills;
        }
        let (larger, excess) = if bought > sold { (Side::Buy, bought - sold) } else { (Side::Sell, sold - bought) };
        let Some((c, o)) =
            cutting.iter().find(|(i, o)| o.side == larger && fills.get(*i).is_some_and(|f| *f > 0)).copied()
        else {
            violation!(clause = "MKT.3", "a call's sides unequal with nothing left to cut");
        };
        let lot = i128::from(o.lot);
        if let Some(f) = fills.get_mut(c) {
            let lots = lesser(*f / lot, (excess + lot - 1) / lot);
            *f -= lots * lot;
        }
    }
}

/// The call auction: one price where posted supply meets posted demand on the tick grid, chosen among the prices
/// that clear by the operator's tie sequence; steps better than the price fill in full and those at it are rationed
/// by the declared rule. No overlap, no bid or no offer is a failure; the market adds nothing to clear.
#[clause("MKT.3", "MKT.10", "MKT.13", "MKT.17", "MKT.18", "MKT.21")]
pub fn call(orders: &[Order], rules: CallRules<'_>, lot: &mut Draws) -> Outcome {
    let steps = |side: Side| -> Vec<(i64, i128)> {
        orders
            .iter()
            .filter(|o| o.side == side)
            .flat_map(|o| o.steps.iter().map(|s| (s.limit.raw(), i128::from(s.qty))))
            .collect()
    };
    let (bids, offers) = (Curve::new(steps(Side::Buy)), Curve::new(steps(Side::Sell)));
    if bids.limits.is_empty() {
        return Outcome::Failed(FailureKind::NoBid);
    }
    if offers.limits.is_empty() {
        return Outcome::Failed(FailureKind::NoSeller);
    }
    let mut prices: Vec<i64> = bids.limits.iter().chain(offers.limits.iter()).copied().collect();
    if let Missing::Present(l) = rules.last {
        prices.push(l.raw());
    }
    prices.sort_unstable();
    prices.dedup();
    let mut candidates: Vec<(i64, At)> =
        prices.into_iter().map(|p| (p, at(&bids, &offers, p))).filter(|(_, a)| clears(*a)).collect();
    if candidates.is_empty() {
        return Outcome::Failed(FailureKind::NoOverlap);
    }
    for rule in rules.ties {
        candidates = keep(*rule, candidates, rules.last);
    }
    let [(price, chosen)] = candidates.as_slice() else {
        violation!(clause = "MKT.21", "a call's tie sequence left more than one price", left = candidates.len());
    };
    let volume = lesser(chosen.supply, chosen.demand);
    let bought = fill_side(orders, Side::Buy, *price, volume, rules.ration, lot);
    let sold = fill_side(orders, Side::Sell, *price, volume, rules.ration, lot);
    let per_order = whole_lots(orders, bought.iter().zip(&sold).map(|(b, s)| b + s).collect(), *price);
    let volume: i128 = orders.iter().zip(&per_order).filter(|(o, _)| o.side == Side::Buy).map(|(_, f)| f).sum();
    if volume == 0 {
        return Outcome::Failed(FailureKind::NoOverlap);
    }
    let price = PriceRaw::from_raw(*price);
    let matches = pair(orders, &per_order, price);
    let fills = per_order
        .iter()
        .enumerate()
        .filter(|(_, q)| **q > 0)
        .map(|(order, q)| {
            let Ok(qty) = i64::try_from(*q) else {
                capacity_exceeded!("an order's fill", i64::MAX, 0);
            };
            Fill { order, qty }
        })
        .collect();
    let Ok(volume) = i64::try_from(volume) else {
        capacity_exceeded!("a call's volume", i64::MAX, 0);
    };
    Outcome::Cleared(Cleared { price, volume, fills, matches })
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Missing, PriceRaw};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{CallRules, Cleared, Outcome, call};
    use crate::failure::FailureKind;
    use crate::market::{Ration, TieRule};
    use crate::order::{Asked, Order, Poster, Side, Timing};

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.session"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    const TIES: &[TieRule] =
        &[TieRule::MaxVolume, TieRule::MinImbalance, TieRule::NearestLastPrint, TieRule::LowerPrice];

    fn order(party: u64, side: Side, steps: &[(i64, i64)]) -> Order {
        order_ranked(party, side, steps, Missing::Absent)
    }

    fn order_ranked(party: u64, side: Side, steps: &[(i64, i64)], priority: Missing<u32>) -> Order {
        let poster = Poster {
            party: PartyId::new(party),
            market: MarketId::new(0),
            side,
            timing: Timing::AtTheClose,
            day: Day::new(1),
            reason: "trade",
            priority,
            lot: 1,
        };
        let asked: Vec<Asked> =
            steps.iter().map(|(l, q)| Asked { limit: Missing::Present(PriceRaw::from_raw(*l)), qty: *q }).collect();
        Order::new(poster, &asked, 1).unwrap()
    }

    fn rules(last: Option<i64>) -> CallRules<'static> {
        let last = last.map_or(Missing::Absent, |l| Missing::Present(PriceRaw::from_raw(l)));
        CallRules { ties: TIES, ration: Ration::ProRata, last }
    }

    fn cleared(o: Outcome) -> Cleared {
        match o {
            Outcome::Cleared(c) => c,
            Outcome::Failed(f) => panic!("failed: {f:?}"),
        }
    }

    fn filled(c: &Cleared, order: usize) -> i64 {
        c.fills.iter().filter(|f| f.order == order).map(|f| f.qty).sum()
    }

    #[test]
    fn an_agent_fills_whole_lots_and_the_sides_stay_equal() {
        let mut agent = order(1, Side::Buy, &[(10, 30)]);
        agent.lot = 10;
        let orders = vec![agent, order(2, Side::Buy, &[(10, 5)]), order(3, Side::Sell, &[(9, 23)])];
        let Outcome::Cleared(c) = call(&orders, rules(None), &mut draws()) else { panic!("the call clears") };
        let fill = |o: usize| c.fills.iter().find(|f| f.order == o).map_or(0, |f| f.qty);
        assert_eq!(fill(0) % 10, 0, "the agent's fill is whole lots");
        assert_eq!(fill(0) + fill(1), fill(2));
        assert_eq!(c.volume, fill(2));
        assert_eq!(c.matches.iter().map(|m| m.qty).sum::<i64>(), c.volume);
    }

    #[test]
    fn call_auction_clears_known_book() {
        // Demand 30 at 102, 50 at 101, 70 at 100; supply 20 at 99, 45 at 100, 60 at 101, 90 at 102. At 101, 50 are
        // wanted and 60 offered: 50 trade, the most any price gives (100 gives 45, 102 gives 30).
        let orders = [
            order(1, Side::Buy, &[(102, 30)]),
            order(2, Side::Buy, &[(101, 20)]),
            order(3, Side::Buy, &[(100, 20)]),
            order(4, Side::Sell, &[(99, 20)]),
            order(5, Side::Sell, &[(100, 25)]),
            order(6, Side::Sell, &[(101, 15)]),
            order(7, Side::Sell, &[(102, 30)]),
        ];
        let c = cleared(call(&orders, rules(None), &mut draws()));
        assert_eq!((c.price.raw(), c.volume), (101, 50));
        let bought: i64 = c.matches.iter().map(|m| m.qty).sum();
        assert_eq!(bought, 50, "every unit bought is a unit sold, match by match");
        assert!(c.matches.iter().all(|m| m.price.raw() == 101 && m.buyer != crate::print::Buyer::Party(m.seller)));
        assert_eq!(
            (filled(&c, 2), filled(&c, 5)),
            (0, 5),
            "the bid below the price trades nothing; the offer at it is rationed"
        );
    }

    #[test]
    fn call_better_limits_fill_in_full() {
        let orders = [
            order(1, Side::Buy, &[(110, 10), (100, 10)]),
            order(2, Side::Buy, &[(100, 10)]),
            order(3, Side::Sell, &[(90, 12)]),
        ];
        let c = cleared(call(&orders, rules(None), &mut draws()));
        assert_eq!(c.price.raw(), 100, "the one price that clears");
        assert_eq!(filled(&c, 2), 12, "the offer below the price fills in full");
        assert_eq!(filled(&c, 0) + filled(&c, 1), 12);
        assert!(filled(&c, 0) >= 10, "the step above the price fills in full before any at it");
    }

    #[test]
    fn call_tie_rules_follow_declared_sequence() {
        // 10 bid at 105 and 10 offered at 95: every price from 95 to 105 trades 10 with no imbalance.
        let orders = [order(1, Side::Buy, &[(105, 10)]), order(2, Side::Sell, &[(95, 10)])];
        let lower = cleared(call(&orders, rules(None), &mut draws()));
        assert_eq!(lower.price.raw(), 95, "with no last print, nearest-last is skipped and the lower price chooses");
        let nearest = cleared(call(&orders, rules(Some(101)), &mut draws()));
        assert_eq!(nearest.price.raw(), 101, "the last print lies in the range and is chosen");
        let beyond = cleared(call(&orders, rules(Some(120)), &mut draws()));
        assert_eq!(beyond.price.raw(), 105, "outside the range, the nearest clearing price");
        let only_lower = CallRules { ties: &[TieRule::LowerPrice], ..rules(Some(101)) };
        assert_eq!(cleared(call(&orders, only_lower, &mut draws())).price.raw(), 95, "the declared sequence alone");
    }

    #[test]
    fn call_rationing_largest_remainder_and_lots() {
        // 10 offered at 100 against three bids of 5 at 100: 10 shared 3.33 each, the one unit left by lot.
        let bids = |p| {
            [
                order(p, Side::Buy, &[(100, 5)]),
                order(p + 1, Side::Buy, &[(100, 5)]),
                order(p + 2, Side::Buy, &[(100, 5)]),
            ]
        };
        let [a, b, c] = bids(1);
        let orders = [a, b, c, order(9, Side::Sell, &[(100, 10)])];
        let cl = cleared(call(&orders, rules(None), &mut draws()));
        let got: Vec<i64> = (0..3).map(|i| filled(&cl, i)).collect();
        assert_eq!(got.iter().sum::<i64>(), 10);
        assert!(got.iter().all(|q| *q == 3 || *q == 4), "each 3, one 4 by lot: {got:?}");
        let again = cleared(call(&orders, rules(None), &mut draws()));
        assert_eq!(cl, again, "the same lot gives the same fills");
        // By priority: the first rank fills before the second.
        let ranked = [
            order_ranked(1, Side::Buy, &[(100, 5)], Missing::Present(2)),
            order_ranked(2, Side::Buy, &[(100, 8)], Missing::Present(1)),
            order_ranked(3, Side::Sell, &[(100, 10)], Missing::Present(1)),
        ];
        let by_rank = CallRules { ration: crate::market::Ration::Priority, ..rules(None) };
        let pr = cleared(call(&ranked, by_rank, &mut draws()));
        assert_eq!((filled(&pr, 0), filled(&pr, 1)), (2, 8));
    }

    #[test]
    fn no_overlap_records_failure() {
        let apart = [order(1, Side::Buy, &[(90, 10)]), order(2, Side::Sell, &[(100, 10)])];
        assert_eq!(call(&apart, rules(None), &mut draws()), Outcome::Failed(FailureKind::NoOverlap));
        assert_eq!(call(&apart[1..], rules(None), &mut draws()), Outcome::Failed(FailureKind::NoBid));
        assert_eq!(call(&apart[..1], rules(None), &mut draws()), Outcome::Failed(FailureKind::NoSeller));
    }
}
