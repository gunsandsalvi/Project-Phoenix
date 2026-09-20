//! A DEALER DESK: it quotes both sides, it holds inventory, and the quote comes from its own state.
//!
//! @spec 26 A1–A4, B1–B4, C1–C5 · XI-13 · Clearing C3 · Law 3, Law 6, Appendix B

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{book_of, line_of, InstrumentId, MarketId, PartyId};
use crate::module::{Participant, ParticipantView};
use crate::params::Denomination;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};

/// The desk's OWN state, which is where the quote comes from.
#[derive(Clone, Copy, Debug)]
pub struct Desk {
    pub who: PartyId,
    /// What it has bought and not yet sold — and the reverse, so it is signed.
    pub inventory: f64,
    /// What the desk's own money costs it, per period.
    pub carry: f64,
    /// What it charges for immediacy before anything else moves it.
    pub half_spread: f64,
    /// How hard a position pushes the quote.
    pub skew_per_unit: f64,
    pub room: f64,
}

/// The bid–offer is the OUTPUT of C1–C4, and the same posted quote belongs in the book.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Quote {
    pub bid: f64,
    pub offer: f64,
}

impl Quote {
    pub fn spread(&self) -> f64 {
        self.offer - self.bid
    }

    pub fn mid(&self) -> f64 {
        (self.bid + self.offer) / 2.0
    }
}

/// The quote, from the desk's own state and the level it thinks the line is worth.
pub fn quote(desk: &Desk, worth: Option<f64>, risk: f64, adverse: f64) -> Option<Quote> {
    let worth = worth?;
    // A desk with no room is not quoting a smaller size — it is not quoting.
    if desk.inventory.abs() >= desk.room {
        return None;
    }
    assert!(risk >= 0.0 && adverse >= 0.0, "26 C3, C4: a widening of {risk}/{adverse} narrows");
    // Long already bids lower AND offers lower, because it wants less.
    let skewed = worth - desk.inventory * desk.skew_per_unit;
    let half = desk.half_spread + desk.carry + risk + adverse;
    Some(Quote { bid: skewed - half, offer: skewed + half })
}

/// What the spread earned and what the inventory cost.
#[derive(Clone, Copy, Debug)]
pub struct Week {
    pub earned_on_spread: f64,
    /// Signed: the position gained or lost as the mark moved.
    pub on_inventory: f64,
}

impl Week {
    pub fn came_to(&self) -> f64 {
        self.earned_on_spread + self.on_inventory
    }
}

/// What the desk now holds after a fill.
pub fn after(inventory: f64, bought: f64, sold: f64) -> f64 {
    inventory + bought - sold
}

/// Convert a money inventory ceiling into units at the desk's own reservation, then form the quote
/// from its disagreement with the last public print. The disagreement is an observation, not a
/// configured spread.
fn reservation(
    around: f64,
    last_print: Option<f64>,
    held: f64,
    limit_money: f64,
) -> Option<(Quote, f64)> {
    if around <= 0.0 || limit_money <= 0.0 {
        return None;
    }
    let limit_units = limit_money / around;
    if held.abs() >= limit_units {
        return None;
    }
    let width = match last_print {
        Some(price) => (around - price).abs(),
        None => 0.0,
    };
    let skew = width * (held / limit_units);
    Some((Quote { bid: around - width - skew, offer: around + width - skew }, limit_units))
}


/// HOW MANY LINES PRINTED, which is what a desk's own market looks like from outside.
pub struct Lines {
    pub kind: u32,
}

impl Mechanism for Lines {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let n = ctx.prints().that_printed(ctx.instruments().len(), ctx.period()) as f64;
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}


/// A dealer quotes a price at which it will buy and a price at which it will sell, and it is willing
/// to do either.
pub struct Dealers {
    /// What it will carry.
    pub limit: &'static str,
    pub lines: Vec<InstrumentId>,
}

impl Participant for Dealers {
    fn party_kind(&self) -> u32 {
        kinds::DEALER
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let line = line_of(m);
        let held = view.quantity(line);
        let limit_money = view.params().amount(self.limit, Denomination::Money);
        let Some(around) = view.price_outlook(line_of(m)) else {
            return Vec::new();
        };
        // The declared limit is money. `reservation` converts it at this desk's own outlook before
        // comparing it with inventory in units. Its spread is observed disagreement, not a declared
        // market outcome. No disagreement means no asserted width.
        let last_print = match view.confidence(crate::stores::about::price_of(line)) {
            Some(width) => Some(around - width),
            None => view.print(line).map(|print| print.price),
        };
        let Some((quote, limit)) = reservation(around, last_print, held, limit_money) else {
            return Vec::new();
        };
        // In whole pieces, and an order for none of them is not an order — a desk one half-piece
        // from its limit has room for nothing.
        let (bidding, offering) = view.resting(m);
        let room = whole_pieces(limit - held) - bidding;
        let long = whole_pieces(held) - offering;
        let mut out = Vec::new();
        if view.own_cash() > 0.0 && quote.bid > 0.0 && room > 0 {
            out.push(Order { party: view.self_id(), side: Side::Buy, price: Some(quote.bid), qty: room });
        }
        if long > 0 {
            out.push(Order { party: view.self_id(), side: Side::Sell, price: Some(quote.offer), qty: long });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desk(inventory: f64, room: f64) -> Desk {
        Desk {
            who: PartyId::at(2),
            inventory,
            carry: 0.02,
            half_spread: 0.10,
            skew_per_unit: 0.001,
            room,
            }
    }

    #[test]
    fn inventory_skews_the_quote_and_that_is_how_a_book_mean_reverts() {
        let flat = quote(&desk(0.0, 1_000.0), Some(20.0), 0.0, 0.0).unwrap();
        let long = quote(&desk(500.0, 1_000.0), Some(20.0), 0.0, 0.0).unwrap();
        let short = quote(&desk(-500.0, 1_000.0), Some(20.0), 0.0, 0.0).unwrap();
        // Long already bids lower AND offers lower, because it wants less.
        assert!(long.bid < flat.bid && long.offer < flat.offer);
        assert!(short.bid > flat.bid && short.offer > flat.offer);
        // Nothing told it to revert — the skew follows the position, and there is no target
        // inventory anywhere in this module.
        assert!((long.spread() - flat.spread()).abs() <= crate::num::dust(4, &[long.spread(), flat.spread()]), "the skew moves both sides together");
    }

    #[test]
    fn risk_and_adverse_selection_widen_it_rather_than_moving_it() {
        let calm = quote(&desk(0.0, 1_000.0), Some(20.0), 0.0, 0.0).unwrap();
        let hard = quote(&desk(0.0, 1_000.0), Some(20.0), 0.05, 0.03).unwrap();
        // Wider, and around the same level — a desk facing somebody who knows more charges for it;
        // it does not change its mind about what the line is worth.
        assert!(hard.spread() > calm.spread());
        assert!((hard.mid() - calm.mid()).abs() <= crate::num::dust(4, &[hard.mid(), calm.mid()]));
    }

    #[test]
    fn a_desk_with_no_room_does_not_quote_a_smaller_size_it_does_not_quote() {
        // A dealer without a limit is a synthetic counterparty wearing a dealer's name, and B4
        // forbids quoting because the mechanism needs somebody to.
        assert!(quote(&desk(1_000.0, 1_000.0), Some(20.0), 0.0, 0.0).is_none());
        // And a desk with no view has nothing to quote around.
        assert!(quote(&desk(0.0, 1_000.0), None, 0.0, 0.0).is_none());
    }

    #[test]
    fn the_spread_and_the_inventory_are_reported_apart() {
        // A desk that netted them could not tell a good week of trading from a lucky position.
        let lucky = Week { earned_on_spread: 10.0, on_inventory: 400.0 };
        let skilled = Week { earned_on_spread: 410.0, on_inventory: 0.0 };
        assert_eq!(lucky.came_to(), skilled.came_to());
        assert_ne!(lucky.earned_on_spread, skilled.earned_on_spread);
    }

    #[test]
    fn inventory_is_signed_because_a_desk_can_be_short() {
        assert_eq!(after(100.0, 0.0, 250.0), -150.0);
        assert_eq!(after(-150.0, 200.0, 0.0), 50.0);
    }

    #[test]
    fn a_money_limit_is_converted_to_units_and_the_width_is_observed() {
        let (flat, limit) = reservation(10.0, Some(8.0), 0.0, 100.0).unwrap();
        let (long, _) = reservation(10.0, Some(8.0), 2.0, 100.0).unwrap();
        assert_eq!(limit, 10.0);
        assert_eq!(flat, Quote { bid: 8.0, offer: 12.0 });
        assert_eq!(long, Quote { bid: 7.6, offer: 11.6 });
        assert!(reservation(10.0, Some(8.0), 10.0, 100.0).is_none());
    }
}
