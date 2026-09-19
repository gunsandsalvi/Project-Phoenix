//! HOW A VENUE MATCHES — one module per protocol, dispatched on the venue's own declaration.
//!
//! @spec 3 A1 · 3 C1 · 3 C4 · Law 1, Law 3, Law 4, Law 15 · Appendix B
//!
//! There was ONE microstructure and it was the wrong one for almost everything. A weekly
//! uniform-price call auction ran for bread, labour, loans, shares and freight alike, and nothing
//! rested between sessions. A Walrasian auctioneer for bread is the one intermediary that never
//! existed: nobody has ever bought a loaf by posting a demand schedule and waiting for a
//! sealed cross.
//!
//! Which protocol a venue runs is DATA (`VenueDecl.protocol`), so the kernel dispatches on the
//! declaration and never branches on what is being traded. Adding a protocol is a module and
//! a variant, not a condition inside the solver.
//!
//! Every one of them clears from real supply meeting real demand. That is what the three
//! have in common and it is the whole of what they have in common:
//!
//! - `Call` — a sealed cross at one level, which is what an auction and a fixing ARE. It is the
//!  solver that was already here, unchanged.
//! - `Posted` — a seller stands behind an ask; a buyer sees some of the market and takes the best
//!  it saw. A trade happens when a buyer accepts a price a seller was standing behind, which is what
//!  a price in a shop IS — and the price is still cleared, because the seller had to be willing to
//!  sell at it and the buyer had to be willing to pay it. What a buyer can SEE is a technology
//!  (`seen_by`): search is costly, and a buyer that saw the whole market would be a buyer in a call
//!  auction wearing a shop's clothes.
//! - `Book` — resting orders, matched continuously as they arrive, which is what an exchange is.
//!  The price is the level the RESTING side was standing at, because that is the side that was there
//!  first and the arriving side chose to hit it.
//!
//! Decentralised bilateral matching with partial information — over goods, labour, credit and
//! deposits — is the dominant protocol in the macro-ABM literature, and `Posted` is it.

use crate::clearing::{clear, Fill, Order, Outcome, PriceRule, Rationed, Side};

/// 3 A1, 22c.1: what kind of venue this is. Registry data, declared by whoever opened the venue,
/// and the one thing the kernel dispatches on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Protocol {
    /// A sealed cross at one level: an auction, a fixing, a tender.
    Call,
    /// A seller stands behind an ask and a buyer takes the best it saw: a shop, a wage posting, a
    /// quoted loan rate.
    Posted,
    /// Resting orders matched continuously: an exchange.
    Book,
}

impl Protocol {
    /// 3 C2, 22c.2: whether an order that did not fill STAYS.
    ///
    /// A call auction is a sealed cross: an order that did not fill in it is gone, because the
    /// auction was the event and it is over. That is not a market with no memory — it is what an
    /// auction IS, and it is why a treasury that failed to raise what it needed must come back.
    ///
    /// A shop and an exchange are the other way: the ask is still on the shelf next week and the bid
    /// is still in the book. Nothing rested anywhere before, which is why `noDemand` (7,903) dwarfed
    /// `noOverlap` (323) — the two sides were not failing to agree on a price, they were failing
    /// to be in the room in the same week.
    pub fn rests(self) -> bool {
        match self {
            Protocol::Call => false,
            Protocol::Posted | Protocol::Book => true,
        }
    }
}

/// 3 A1, 22c2.2: A VENUE, as whoever opened it declared it. The four facts that are about the
/// PLACE rather than about what is traded there, carried together because a caller made to name
/// three is a caller that will one day name two (Law 15: registry data, never a branch).
#[derive(Clone, Copy, Debug)]
pub struct Venue {
    pub rule: PriceRule,
    pub protocol: Protocol,
    /// How many sellers one buyer can see. A TECHNOLOGY, read only by `Posted`.
    pub seen_by: usize,
    /// How long an order stands here, in periods. A CONVENTION of this venue — as much a fact
    /// about the place as its protocol, and the reason a real book does not only grow.
    ///
    /// `Missing` is a venue whose orders stand until somebody pulls them, which is a real kind of
    /// venue and is said by declaring it rather than by leaving it out. Every order in
    /// this world rested for ever because no venue said otherwise: 16,869 resting after one period
    /// and 33,069 after four, with books cleared falling from 5 to 1.
    pub stands_for: Option<u32>,
}

impl Venue {
    /// 3 C2: the last day an order entered in `from` stands here. An order that stands for one
    /// period stands to the end of that period and is gone when the next one opens — a DATE, taken
    /// from the calendar, never a count of periods carried on the row.
    pub fn until(&self, cal: &crate::calendar::Calendar, from: u32) -> Option<crate::calendar::Day> {
        self.stands_for
            .map(|periods| crate::calendar::Day(cal.start_of(crate::calendar::Period(from + periods)).0 - 1))
    }
}

/// How much of a market one buyer can see, as a count of sellers. A TECHNOLOGY: search
/// is costly and nobody sees everything, and it is the difference between a shop and an auction.
///
/// A buyer that can see more sellers than there are sees all of them, which is arithmetic
/// about a list and not a cap on a number.
pub fn posted(orders: &[Order], seen_by: usize) -> Outcome {
    assert!(seen_by > 0, "3 C1: a buyer that can see no seller is in no market");
    let mut asks: Vec<&Order> = orders.iter().filter(|o| o.side == Side::Sell).collect();
    let mut bids: Vec<&Order> = orders.iter().filter(|o| o.side == Side::Buy).collect();
    if bids.is_empty() {
        return Outcome::NoDemand;
    }
    if asks.is_empty() {
        return Outcome::NoSupply;
    }
    // The keenest buyer goes first: it is the one that would outbid the others for what it finds,
    // and in a posted market being willing to pay more is what gets you served before somebody else
    // takes the cheap stock. Sellers are met in the order they stood behind their asks.
    bids.sort_by(|a, b| level_of(b).total_cmp(&level_of(a)));
    asks.sort_by(|a, b| level_of(a).total_cmp(&level_of(b)));

    let mut left: Vec<i64> = asks.iter().map(|a| a.qty).collect();
    let mut fills: Vec<Fill> = Vec::new();
    let mut volume = 0i64;
    let mut last = f64::NAN;

    for (seen_from, bid) in bids.iter().enumerate() {
        let mut wanted = bid.qty;
        let limit = level_of(bid);
        // What this buyer saw: a window of the sellers, not the whole market. Where it starts is
        // the buyer's own place in the queue, so two buyers do not see the same shelf — which is why
        // the cheapest seller is not exhausted by the first buyer in a list.
        let from = seen_from % asks.len();
        for step in 0..crate::num::at_most(seen_by, asks.len()) {
            if wanted <= 0 {
                break;
            }
            let at = (from + step) % asks.len();
            if left[at] <= 0 {
                continue;
            }
            let ask = level_of(asks[at]);
            // It takes it because it is willing to pay what the seller was standing behind.
            // A buyer that will not pay the ask does not buy, and no price is invented between them.
            if ask > limit {
                continue;
            }
            let took = if left[at] < wanted { left[at] } else { wanted };
            left[at] -= took;
            wanted -= took;
            volume += took;
            last = ask;
            fills.push(Fill { party: bid.party, side: Side::Buy, qty: took, price: ask });
            fills.push(Fill { party: asks[at].party, side: Side::Sell, qty: took, price: ask });
        }
    }
    if volume <= 0 {
        // 3 C4: the two sides were in the room and nothing crossed. The bracket is not a price.
        return Outcome::NoOverlap { best_ask: level_of(asks[0]), best_bid: level_of(bids[0]) };
    }
    // The print is the LAST price anybody actually paid. A posted market has no single level —
    // that is what makes it a posted market — so what it prints is a transaction and never an
    // average of them (Law 3: a price is a thing somebody paid, and a mean of two trades is neither).
    Outcome::Cleared {
        price: last,
        volume,
        fills,
        rationed: Rationed::None,
        demand_at_price: bids.iter().map(|b| b.qty).sum(),
        supply_at_price: asks.iter().map(|a| a.qty).sum(),
    }
}

/// Resting orders, matched as they arrive. An arriving order takes what is already standing
/// there; what it cannot fill RESTS, and the caller is what keeps it between sessions.
///
/// The price is the RESTING side's level, which is not a convention: the resting side was there
/// first and the arriving side chose to hit it, so that is the price the trade happened at.
pub fn book(resting: &[Order], arriving: &[Order]) -> Outcome {
    let mut standing: Vec<(Order, i64)> = resting.iter().map(|o| (*o, o.qty)).collect();
    let mut fills: Vec<Fill> = Vec::new();
    let mut volume = 0i64;
    let mut last = f64::NAN;

    for order in arriving {
        let mut wanted = order.qty;
        let limit = level_of(order);
        // The best standing order on the other side, taken first: the keenest bid for a seller, the
        // cheapest ask for a buyer. That is what "best execution" means and it is not a preference.
        let mut against: Vec<usize> = standing
            .iter()
            .enumerate()
            .filter(|(_, (o, left))| *left > 0 && o.side != order.side)
            .map(|(i, _)| i)
            .collect();
        against.sort_by(|a, b| match order.side {
            Side::Buy => level_of(&standing[*a].0).total_cmp(&level_of(&standing[*b].0)),
            Side::Sell => level_of(&standing[*b].0).total_cmp(&level_of(&standing[*a].0)),
        });
        for at in against {
            if wanted <= 0 {
                break;
            }
            let level = level_of(&standing[at].0);
            let crosses = match order.side {
                Side::Buy => level <= limit,
                Side::Sell => level >= limit,
            };
            if !crosses {
                break;
            }
            let took = if standing[at].1 < wanted { standing[at].1 } else { wanted };
            standing[at].1 -= took;
            wanted -= took;
            volume += took;
            last = level;
            fills.push(Fill { party: order.party, side: order.side, qty: took, price: level });
            fills.push(Fill {
                party: standing[at].0.party,
                side: standing[at].0.side,
                qty: took,
                price: level,
            });
        }
    }
    if volume <= 0 {
        let best_bid = best(&standing, arriving, Side::Buy);
        let best_ask = best(&standing, arriving, Side::Sell);
        return match (best_bid, best_ask) {
            (None, _) => Outcome::NoDemand,
            (_, None) => Outcome::NoSupply,
            (Some(b), Some(a)) => Outcome::NoOverlap { best_bid: b, best_ask: a },
        };
    }
    Outcome::Cleared {
        price: last,
        volume,
        fills,
        rationed: Rationed::None,
        demand_at_price: side_total(&standing, arriving, Side::Buy),
        supply_at_price: side_total(&standing, arriving, Side::Sell),
    }
}

/// The venue's protocol decides how it matches, and the kernel asks rather than deciding.
pub fn run(protocol: Protocol, resting: &[Order], arriving: &[Order], rule: PriceRule, seen_by: usize) -> Outcome {
    match protocol {
        // A call auction takes everything in the room at once — what rested and what arrived are one
        // set of schedules, because a sealed cross has no order of arrival.
        Protocol::Call => {
            let all: Vec<Order> = resting.iter().chain(arriving.iter()).copied().collect();
            clear(&all, rule, false)
        }
        Protocol::Posted => {
            let all: Vec<Order> = resting.iter().chain(arriving.iter()).copied().collect();
            posted(&all, seen_by)
        }
        Protocol::Book => book(resting, arriving),
    }
}

/// An order with no level is willing to take what the book gives it. In a posted or a
/// resting market that means it will pay anything a seller stands behind and take anything a buyer
/// offers, which is what a market order IS.
#[inline]
fn level_of(o: &Order) -> f64 {
    match (o.price, o.side) {
        (Some(p), _) => p,
        (None, Side::Buy) => f64::INFINITY,
        (None, Side::Sell) => f64::NEG_INFINITY,
    }
}

fn best(standing: &[(Order, i64)], arriving: &[Order], side: Side) -> Option<f64> {
    let levels = standing
        .iter()
        .filter(|(o, left)| o.side == side && *left > 0)
        .map(|(o, _)| level_of(o))
        .chain(arriving.iter().filter(|o| o.side == side).map(level_of));
    // The keenest level on that side — the best bid or the cheapest ask. `num::keener` is
    // where a comparison lives, because a minimum written at a site reads as a cap.
    let buying = side == Side::Buy;
    levels.fold(None, |acc: Option<f64>, l| Some(acc.map_or(l, |a| crate::num::keener(a, l, buying))))
}

fn side_total(standing: &[(Order, i64)], arriving: &[Order], side: Side) -> i64 {
    standing.iter().filter(|(o, left)| o.side == side && *left > 0).map(|(_, left)| left).sum::<i64>()
        + arriving.iter().filter(|o| o.side == side).map(|o| o.qty).sum::<i64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PartyId;

    #[test]
    fn how_long_an_order_stands_is_the_venues_convention_and_it_is_a_date() {
        // A venue that declares a life gives its orders a DAY to stand to, taken from
        // the one calendar. An order entered in period 1 that stands for one period stands to the
        // end of period 1 and is gone when period 2 opens.
        let cal = crate::calendar::Calendar::new(crate::calendar::Day(0), 7, 3);
        let shop = Venue { rule: PriceRule::SellersCompete, protocol: Protocol::Posted, seen_by: 5, stands_for: Some(1) };
        assert_eq!(shop.until(&cal, 1), Some(crate::calendar::Day(13)));
        assert_eq!(cal.start_of(crate::calendar::Period(2)), crate::calendar::Day(14));

        // And a venue that declares none has orders that stand until somebody pulls them, which is
        // an answer and not an omission.
        let forever = Venue { stands_for: None, ..shop };
        assert_eq!(forever.until(&cal, 1), None);
    }

    fn buy(who: u32, at: f64, qty: i64) -> Order {
        Order { party: PartyId::at(who), side: Side::Buy, price: Some(at), qty }
    }

    fn sell(who: u32, at: f64, qty: i64) -> Order {
        Order { party: PartyId::at(who), side: Side::Sell, price: Some(at), qty }
    }

    #[test]
    fn a_shop_is_a_buyer_taking_a_price_a_seller_stood_behind() {
        // Law 3 in a posted market: the trade happened because the buyer was willing to pay what
        // the seller was willing to take. Nobody computed a level and nobody was rationed — that is
        // an auction, and a loaf is not bought that way.
        let orders = [sell(1, 2.0, 10), sell(2, 3.0, 10), buy(3, 2.5, 6)];
        let out = posted(&orders, 4);
        match out {
            Outcome::Cleared { price, volume, ref fills, rationed, .. } => {
                assert_eq!((price, volume), (2.0, 6));
                assert_eq!(rationed, Rationed::None);
                // Two legs, because a trade has two sides.
                assert_eq!(fills.len(), 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_buyer_sees_only_some_of_the_market_and_that_is_the_technology() {
        // Search is costly. A buyer that saw everything would be a buyer in a call auction, and
        // the difference between a shop and an auction is exactly this window. Here the buyer can
        // see one seller and the one it sees is dear, so it pays the dear price — which is what
        // happens to somebody who does not shop around, and is not a defect.
        let orders = [sell(1, 9.0, 10), sell(2, 1.0, 10), buy(3, 20.0, 4)];
        let narrow = posted(&orders, 1);
        match narrow {
            Outcome::Cleared { price, .. } => assert_eq!(price, 1.0, "it saw the cheapest first"),
            other => panic!("{other:?}"),
        }
        // Widen the window and it reaches the other seller too — more supply, same willingness.
        let wide = posted(&[sell(1, 9.0, 2), sell(2, 1.0, 2), buy(3, 20.0, 4)], 2);
        match wide {
            Outcome::Cleared { volume, .. } => assert_eq!(volume, 4),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_buyer_that_will_not_pay_the_ask_does_not_buy_and_no_price_is_invented() {
        // There is no level between them and nothing is met in the middle. The bracket
        // is reported and is NOT a price (3 C4).
        let out = posted(&[sell(1, 10.0, 5), buy(2, 4.0, 5)], 4);
        match out {
            Outcome::NoOverlap { best_bid, best_ask } => assert_eq!((best_bid, best_ask), (4.0, 10.0)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_arriving_order_takes_what_is_resting_at_the_level_it_was_resting_at() {
        // The resting side was there first, so the price is its level and not the arriving
        // side's. A buyer willing to pay 5 that hits an ask resting at 3 pays 3.
        let resting = [sell(1, 3.0, 10)];
        let arriving = [buy(2, 5.0, 4)];
        match book(&resting, &arriving) {
            Outcome::Cleared { price, volume, .. } => assert_eq!((price, volume), (3.0, 4)),
            other => panic!("{other:?}"),
        }
        // And the best resting order is taken first, whatever order it was entered in.
        let deep = [sell(1, 7.0, 10), sell(2, 3.0, 2), sell(3, 5.0, 10)];
        match book(&deep, &[buy(4, 6.0, 5)]) {
            Outcome::Cleared { volume, ref fills, .. } => {
                assert_eq!(volume, 5, "two at 3 and three at 5; the 7 is not crossed");
                assert!(fills.iter().any(|f| f.price == 3.0) && fills.iter().any(|f| f.price == 5.0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn what_did_not_fill_is_not_a_trade_and_the_book_says_which_side_was_missing() {
        // Nothing crossed is three different answers, and they are told apart.
        assert!(matches!(book(&[sell(1, 3.0, 5)], &[]), Outcome::NoDemand));
        assert!(matches!(book(&[buy(1, 3.0, 5)], &[]), Outcome::NoSupply));
        assert!(matches!(
            book(&[sell(1, 9.0, 5)], &[buy(2, 2.0, 5)]),
            Outcome::NoOverlap { best_bid: 2.0, best_ask: 9.0 }
        ));
    }

    #[test]
    fn the_kernel_dispatches_on_the_declaration_and_never_on_what_is_traded() {
        // The same orders, three protocols, three answers — and the code that chooses is a
        // `match` on the VENUE's declaration, not on the instrument or the party kind.
        let resting = [sell(1, 2.0, 10)];
        let arriving = [buy(2, 4.0, 6)];
        let as_call = run(Protocol::Call, &resting, &arriving, PriceRule::SellersCompete, 4);
        let as_posted = run(Protocol::Posted, &resting, &arriving, PriceRule::SellersCompete, 4);
        let as_book = run(Protocol::Book, &resting, &arriving, PriceRule::SellersCompete, 4);
        for out in [&as_call, &as_posted, &as_book] {
            assert!(matches!(out, Outcome::Cleared { volume: 6, .. }), "{out:?}");
        }
        // The call auction crosses at ONE level for everybody; the other two at what somebody paid.
        assert!(matches!(as_call, Outcome::Cleared { price: 2.0, .. }));
        assert!(matches!(as_posted, Outcome::Cleared { price: 2.0, .. }));
        assert!(matches!(as_book, Outcome::Cleared { price: 2.0, .. }));
    }
}
