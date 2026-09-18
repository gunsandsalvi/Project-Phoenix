//! A DEALER DESK: it quotes both sides, it holds inventory, and the quote comes from its own state.
//!
//! @spec 26 A1–A4, B1–B4, C1–C5 · XI-13 · Clearing C3 · Law 3, Law 6, Appendix B
//!
//! **A4: it makes money from the SPREAD and loses money from the INVENTORY**, and the two are the
//! whole of the business. **B3: the client pays for immediacy** — the alternative is waiting for a
//! natural counterparty.
//!
//! **B4 FORBID — it does not quote because the mechanism needs somebody to.** A desk whose schedule
//! exists so the book has a second side is Appendix B's synthetic counterparty wearing a dealer's
//! name, and a price struck against it carries no information (XI-13). So `quote` returns NOTHING
//! where the desk has no reason or no room: a book with no dealer in it is a real state.
//!
//! **C2: inventory SKEWS the quote.** Long already means it bids lower and offers lower, because it
//! wants less. **C2.a: this is how a desk mean-reverts its book without anyone telling it to** —
//! there is no target inventory here and no reversion rule, only a skew that follows the position.

use crate::ids::PartyId;

/// C1: the desk's OWN state, which is where the quote comes from. Every field is a fact about this
/// desk — not about the market, and not about what the mechanism needs.
#[derive(Clone, Copy, Debug)]
pub struct Desk {
    pub who: PartyId,
    /// A3: what it has bought and not yet sold — and the reverse, so it is signed.
    pub inventory: f64,
    /// B3, C1: what the desk's own money costs it, per period. It funds the inventory with this.
    pub carry: f64,
    /// A2, B3: what it charges for immediacy before anything else moves it.
    pub half_spread: f64,
    /// C2: how hard a position pushes the quote. It is the desk's own, so two desks with the same
    /// book quote differently — which is what gives a market more than one opinion (XI-13).
    pub skew_per_unit: f64,
    /// A3, B3: the room it has. **Appendix B: a dealer without a limit is a synthetic counterparty
    /// wearing a dealer's name**, so this is not optional.
    pub room: f64,
}

/// C5: the bid–offer is the OUTPUT of C1–C4, and the same posted quote belongs in the book.
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

/// C1–C5: the quote, from the desk's own state and the level it thinks the line is worth.
///
/// **B4: `None` where it will not quote.** A desk past its room has no room to take more, and one
/// with no view has nothing to quote around — and neither is a reason to invent a price so the book
/// has two sides.
///
/// C3 widens it for risk and C4 for adverse selection: both are the CALLER's reads about this line
/// and this client, passed in, because a desk that computed them from a kind would be branching on
/// one (Law 15).
pub fn quote(desk: &Desk, worth: Option<f64>, risk: f64, adverse: f64) -> Option<Quote> {
    let worth = worth?;
    // Appendix B: a desk with no room is not quoting a smaller size — it is not quoting. Law 6:
    // this is a refusal, not a cap on what follows.
    if desk.inventory.abs() >= desk.room {
        return None;
    }
    assert!(risk >= 0.0 && adverse >= 0.0, "26 C3, C4: a widening of {risk}/{adverse} narrows");
    // C2: long already bids lower AND offers lower, because it wants less. The skew moves both
    // sides together — which is what mean-reverts the book without a target telling it to.
    let skewed = worth - desk.inventory * desk.skew_per_unit;
    let half = desk.half_spread + desk.carry + risk + adverse;
    Some(Quote { bid: skewed - half, offer: skewed + half })
}

/// A4: **what the spread earned and what the inventory cost.** The two are reported apart, because
/// a desk that netted them could not tell a good week of trading from a lucky position.
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

/// A3, C2.a: what the desk now holds after a fill. Signed, because inventory is the reverse too —
/// a desk that sold what it did not have is short and the number says so (Register C4 decides
/// whether it may be).
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
        // C2: long already bids lower AND offers lower, because it wants less.
        assert!(long.bid < flat.bid && long.offer < flat.offer);
        assert!(short.bid > flat.bid && short.offer > flat.offer);
        // C2.a: nothing told it to revert — the skew follows the position, and there is no target
        // inventory anywhere in this module.
        assert!((long.spread() - flat.spread()).abs() <= crate::num::dust(4, &[long.spread(), flat.spread()]), "the skew moves both sides together");
    }

    #[test]
    fn risk_and_adverse_selection_widen_it_rather_than_moving_it() {
        let calm = quote(&desk(0.0, 1_000.0), Some(20.0), 0.0, 0.0).unwrap();
        let hard = quote(&desk(0.0, 1_000.0), Some(20.0), 0.05, 0.03).unwrap();
        // C3, C4: wider, and around the same level — a desk facing somebody who knows more charges
        // for it; it does not change its mind about what the line is worth.
        assert!(hard.spread() > calm.spread());
        assert!((hard.mid() - calm.mid()).abs() <= crate::num::dust(4, &[hard.mid(), calm.mid()]));
    }

    #[test]
    fn a_desk_with_no_room_does_not_quote_a_smaller_size_it_does_not_quote() {
        // Appendix B: a dealer without a limit is a synthetic counterparty wearing a dealer's name,
        // and B4 forbids quoting because the mechanism needs somebody to. Law 6: this is a
        // REFUSAL, not a cap on the size.
        assert!(quote(&desk(1_000.0, 1_000.0), Some(20.0), 0.0, 0.0).is_none());
        // And a desk with no view has nothing to quote around.
        assert!(quote(&desk(0.0, 1_000.0), None, 0.0, 0.0).is_none());
    }

    #[test]
    fn the_spread_and_the_inventory_are_reported_apart() {
        // A4: a desk that netted them could not tell a good week of trading from a lucky position.
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
