//! A DEALER DESK: it quotes both sides, it holds inventory, and the quote comes from its own state.
//!
//! @spec 26 A1–A4, B1–B4, C1–C5 · XI-13 · Clearing C3 · Law 3, Law 6, Appendix B

use crate::ids::PartyId;

/// The desk's OWN state, which is where the quote comes from. Every field is a fact about this desk
/// — not about the market, and not about what the mechanism needs.
#[derive(Clone, Copy, Debug)]
pub struct Desk {
    pub who: PartyId,
    /// What it has bought and not yet sold — and the reverse, so it is signed.
    pub inventory: f64,
    /// What the desk's own money costs it, per period. It funds the inventory with this.
    pub carry: f64,
    /// What it charges for immediacy before anything else moves it.
    pub half_spread: f64,
    /// How hard a position pushes the quote. It is the desk's own, so two desks with the same book
    /// quote differently — which is what gives a market more than one opinion.
    pub skew_per_unit: f64,
    /// The room it has. Appendix B: a dealer without a limit is a synthetic counterparty wearing a
    /// dealer's name, so this is not optional.
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
    // A desk with no room is not quoting a smaller size — it is not quoting. Law 6: this is a
    // refusal, not a cap on what follows.
    if desk.inventory.abs() >= desk.room {
        return None;
    }
    assert!(risk >= 0.0 && adverse >= 0.0, "26 C3, C4: a widening of {risk}/{adverse} narrows");
    // Long already bids lower AND offers lower, because it wants less. The skew moves both sides
    // together — which is what mean-reverts the book without a target telling it to.
    let skewed = worth - desk.inventory * desk.skew_per_unit;
    let half = desk.half_spread + desk.carry + risk + adverse;
    Some(Quote { bid: skewed - half, offer: skewed + half })
}

/// What the spread earned and what the inventory cost. The two are reported apart, because a desk
/// that netted them could not tell a good week of trading from a lucky position.
#[derive(Clone, Copy, Debug)]
pub struct Week {
    pub earned_on_spread: f64,
    /// Signed: the position gained or lost as the mark moved. XI-13: the desk puts its own capital
    /// behind what it thinks a line is worth and takes the loss when it is wrong.
    pub on_inventory: f64,
}

impl Week {
    pub fn came_to(&self) -> f64 {
        self.earned_on_spread + self.on_inventory
    }
}

/// What the desk now holds after a fill. Signed, because inventory is the reverse too — a desk that
/// sold what it did not have is short and the number says so (Register C4 decides whether it may
/// be).
pub fn after(inventory: f64, bought: f64, sold: f64) -> f64 {
    inventory + bought - sold
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
        // forbids quoting because the mechanism needs somebody to. Law 6: this is a REFUSAL, not a
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
}
