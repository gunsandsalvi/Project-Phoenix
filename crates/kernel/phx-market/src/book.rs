use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded};
use phx_rand::{Draws, below_u64};

use crate::call::{CallRules, Outcome, call, lesser};
use crate::order::{Asked, Order, Poster, Side, Timing};
use crate::print::{Buyer, Match};

/// A step resting in the book: its order, poster and side, its limit and what is left of it, and when it arrived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Resting {
    order: usize,
    party: PartyId,
    side: Side,
    limit: i64,
    left: i64,
    arrived: usize,
}

/// A book's day: the trades of its continuous session in arrival order, at the resting price, and its closing call,
/// whose price is the day's close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookDay {
    pub trades: Vec<Match>,
    pub close: Outcome,
    /// The orders the closing call met, in its orders' order: the residue of each continuous order and each order
    /// at the close, by its index among the day's orders.
    pub closing_orders: Vec<usize>,
}

/// The order in which the day's continuous orders reach the book, drawn by lot, since nobody reacts within the day.
#[must_use]
pub fn arrival(orders: &[Order], lot: &mut Draws) -> Vec<usize> {
    let mut seq: Vec<usize> =
        (0..orders.len()).filter(|i| orders.get(*i).is_some_and(|o| o.timing == Timing::Continuous)).collect();
    for i in (1..seq.len()).rev() {
        let n = phx_rand::float::len_u64(i + 1);
        let Ok(j) = usize::try_from(below_u64(lot, n)) else {
            capacity_exceeded!("orders of a book's day", usize::MAX, i);
        };
        seq.swap(i, j);
    }
    seq
}

/// A resting step's place on its side: the better price first, then the earlier arrival.
fn priority(r: &Resting) -> (i64, usize) {
    match r.side {
        Side::Buy => (-r.limit, r.arrived),
        Side::Sell => (r.limit, r.arrived),
    }
}

/// The continuous session over orders in the order they arrive: an incoming step that crosses resting steps of the
/// other side trades with them in price-then-time priority at each resting step's price, and what is left of it
/// rests in its place, found by binary search. A poster's own resting steps are passed over, since a party does not
/// trade with itself.
#[clause("MKT.4", "MKT.11")]
#[must_use]
pub(crate) fn continuous(orders: &[Order], sequence: &[usize]) -> (Vec<Match>, Vec<Resting>) {
    let (mut bids, mut asks): (Vec<Resting>, Vec<Resting>) = (Vec::new(), Vec::new());
    let mut trades = Vec::new();
    let mut arrived = 0;
    for &i in sequence {
        let Some(order) = orders.get(i) else { continue };
        for step in &order.steps {
            let mut left = step.qty;
            let other = match order.side {
                Side::Buy => &mut asks,
                Side::Sell => &mut bids,
            };
            for resting in other.iter_mut() {
                let crosses = match order.side {
                    Side::Buy => resting.limit <= step.limit.raw(),
                    Side::Sell => resting.limit >= step.limit.raw(),
                };
                if left == 0 || !crosses {
                    break;
                }
                if resting.party == order.party {
                    continue;
                }
                let Ok(qty) = i64::try_from(lesser(i128::from(left), i128::from(resting.left))) else {
                    capacity_exceeded!("a book's trade", i64::MAX, 0);
                };
                let (buyer, seller) = match order.side {
                    Side::Buy => (order.party, resting.party),
                    Side::Sell => (resting.party, order.party),
                };
                let price = PriceRaw::from_raw(resting.limit);
                trades.push(Match { buyer: Buyer::Party(buyer), seller, qty, price, draws: Missing::Absent });
                resting.left -= qty;
                left -= qty;
            }
            other.retain(|r| r.left > 0);
            if left > 0 {
                let new =
                    Resting { order: i, party: order.party, side: order.side, limit: step.limit.raw(), left, arrived };
                let own = match order.side {
                    Side::Buy => &mut bids,
                    Side::Sell => &mut asks,
                };
                let at = own.partition_point(|r| priority(r) <= priority(&new));
                own.insert(at, new);
                arrived += 1;
            }
        }
    }
    bids.extend(asks);
    (trades, bids)
}

/// A book's day: the continuous orders arrive in an order drawn by lot and trade; then what rests and the orders at
/// the close meet in the closing call, whose price is the day's close.
#[clause("MKT.4", "MKT.12")]
pub fn book_day(orders: &[Order], rules: CallRules<'_>, tick: i64, lot: &mut Draws) -> BookDay {
    let sequence = arrival(orders, lot);
    let (trades, resting) = continuous(orders, &sequence);
    let mut closing: Vec<Order> = Vec::new();
    let mut closing_orders = Vec::new();
    for (i, o) in orders.iter().enumerate() {
        let asked: Vec<Asked> = match o.timing {
            Timing::AtTheClose => {
                o.steps.iter().map(|s| Asked { limit: Missing::Present(s.limit), qty: s.qty }).collect()
            }
            Timing::Continuous => resting
                .iter()
                .filter(|r| r.order == i)
                .map(|r| Asked { limit: Missing::Present(PriceRaw::from_raw(r.limit)), qty: r.left })
                .collect(),
        };
        if asked.is_empty() {
            continue;
        }
        let poster = Poster {
            party: o.party,
            market: o.market,
            side: o.side,
            timing: Timing::AtTheClose,
            day: o.day,
            reason: o.reason,
            priority: o.priority,
        };
        if let Ok(residue) = Order::new(poster, &asked, tick) {
            closing.push(residue);
            closing_orders.push(i);
        }
    }
    let close = call(&closing, rules, lot);
    BookDay { trades, close, closing_orders }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Missing, PriceRaw};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{book_day, continuous};
    use crate::call::{CallRules, Outcome};
    use crate::market::{Ration, TieRule};
    use crate::order::{Asked, Order, Poster, Side, Timing};
    use crate::print::Buyer;

    fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.book"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    fn order(party: u64, side: Side, limit: i64, qty: i64, timing: Timing) -> Order {
        let poster = Poster {
            party: PartyId::new(party),
            market: MarketId::new(0),
            side,
            timing,
            day: Day::new(1),
            reason: "trade",
            priority: Missing::Absent,
        };
        Order::new(poster, &[Asked { limit: Missing::Present(PriceRaw::from_raw(limit)), qty }], 1).unwrap()
    }

    #[test]
    fn book_price_time_priority() {
        let orders = [
            order(1, Side::Sell, 101, 5, Timing::Continuous),
            order(2, Side::Sell, 101, 5, Timing::Continuous),
            order(3, Side::Sell, 100, 3, Timing::Continuous),
            order(4, Side::Buy, 102, 10, Timing::Continuous),
        ];
        let (trades, resting) = continuous(&orders, &[0, 1, 2, 3]);
        let got: Vec<(u64, i64, i64)> = trades.iter().map(|t| (t.seller.get(), t.qty, t.price.raw())).collect();
        assert_eq!(got, vec![(3, 3, 100), (1, 5, 101), (2, 2, 101)], "the best price first, then the earlier arrival");
        assert!(trades.iter().all(|t| t.buyer == Buyer::Party(PartyId::new(4))));
        assert_eq!(resting.len(), 1, "what is left of the later offer rests");
        assert_eq!(resting[0].left, 3);
        let (later, _) = continuous(&orders, &[1, 0, 2, 3]);
        assert_eq!(later[1].seller.get(), 2, "another arrival order, another priority");
    }

    #[test]
    fn book_at_the_close_orders_form_close() {
        let orders = [
            order(1, Side::Sell, 100, 5, Timing::Continuous),
            order(2, Side::Buy, 98, 5, Timing::Continuous),
            order(3, Side::Buy, 99, 4, Timing::AtTheClose),
            order(4, Side::Sell, 99, 2, Timing::AtTheClose),
        ];
        let rules = CallRules {
            ties: &[TieRule::MaxVolume, TieRule::LowerPrice],
            ration: Ration::ProRata,
            last: Missing::Absent,
        };
        let day = book_day(&orders, rules, 1, &mut lot());
        assert!(day.trades.is_empty(), "the continuous orders did not cross");
        let Outcome::Cleared(close) = day.close else { panic!("the close forms a price") };
        assert_eq!((close.price.raw(), close.volume), (99, 2), "the resting residue and the orders at the close");
        assert_eq!(day.closing_orders.len(), 4);
    }
}
