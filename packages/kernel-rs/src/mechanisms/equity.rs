//! EQUITY: a residual claim, counted in shares, perpetual, and carrying control.
//!
//! @spec 10 A1, A1.a, A1.b, A2, A2.a, A3, A4, A5, A5.a, A5.b, A6, B1 · XI-8 · Law 6, Law 8, Law 9

use crate::ids::{CurrencyCode, PartyId};

/// A share count changes only by a NAMED EVENT.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShareEvent {
    Issued,
    /// A buy-back: the company took its own shares in, and they are not outstanding any more.
    BoughtBack,
    /// A split restates the count without changing what anybody owns a share OF.
    Split,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub issuer: PartyId,
    pub ccy: CurrencyCode,
    outstanding: f64,
}

impl Line {
    pub fn new(issuer: PartyId, ccy: CurrencyCode, outstanding: f64) -> Self {
        assert!(outstanding > 0.0, "10 A2: a line with no shares is not a line");
        Self { issuer, ccy, outstanding }
    }

    pub fn outstanding(&self) -> f64 {
        self.outstanding
    }

    /// The count moves for a NAMED reason, and by nothing else.
    pub fn apply(&mut self, event: ShareEvent, shares: f64) {
        assert!(shares > 0.0, "10 A2.a: an event over {shares} shares is not an event");
        match event {
            ShareEvent::Issued => self.outstanding += shares,
            ShareEvent::BoughtBack | ShareEvent::Cancelled => {
                assert!(
                    shares <= self.outstanding,
                    "10 A2: {shares} taken in against {} outstanding — a company cannot retire \
                     shares it never issued",
                    self.outstanding
                );
                self.outstanding -= shares;
            }
            // A split RESTATES.
            ShareEvent::Split => self.outstanding *= shares,
        }
    }
}

/// What the residual is worth to equity.
pub fn residual(assets: f64, debt: f64) -> (f64, f64) {
    let left = assets - debt;
    if left >= 0.0 {
        (left, 0.0)
    } else {
        (0.0, -left)
    }
}

/// Control rides with it — a vote per share.
pub fn votes(held: f64) -> f64 {
    held
}

/// Control is MORE THAN HALF of what exists, read off the outstanding count rather than declared.
pub fn control_needs(outstanding: f64) -> f64 {
    (outstanding / 2.0).floor() + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_liability_stops_at_nothing_and_the_rest_lands_on_the_creditors() {
        // Value can be zero and not negative — a real property, not a clamp.
        assert_eq!(residual(1_000.0, 400.0), (600.0, 0.0));
        // And the loss below zero does NOT vanish.
        assert_eq!(residual(400.0, 1_000.0), (0.0, 600.0));
    }

    #[test]
    fn the_share_count_moves_only_for_a_named_reason() {
        let mut l = Line::new(PartyId::at(3), CurrencyCode::at(0), 1_000.0);
        l.apply(ShareEvent::Issued, 200.0);
        assert_eq!(l.outstanding(), 1_200.0);
        l.apply(ShareEvent::BoughtBack, 300.0);
        assert_eq!(l.outstanding(), 900.0);
        // A split RESTATES: what anybody owns a share of is unchanged.
        l.apply(ShareEvent::Split, 2.0);
        assert_eq!(l.outstanding(), 1_800.0);
    }

    #[test]
    #[should_panic(expected = "cannot retire shares it never issued")]
    fn a_company_cannot_take_in_more_than_it_put_out() {
        let mut l = Line::new(PartyId::at(3), CurrencyCode::at(0), 100.0);
        l.apply(ShareEvent::Cancelled, 500.0);
    }

    #[test]
    fn control_is_more_than_half_of_what_exists_and_a_vote_is_a_count() {
        // A majority is a thing that can be BOUGHT, so it is read off the register.
        assert_eq!(control_needs(1_000.0), 501.0);
        // Half of an odd count is not a share, so the tick matters.
        assert_eq!(control_needs(999.0), 500.0);
        // A cell casts the votes of what it HOLDS — a weight is a count, and there is no per-member
        // fraction of a vote anywhere.
        assert_eq!(votes(4_000.0), 4_000.0);
    }

    #[test]
    #[should_panic(expected = "a line with no shares is not a line")]
    fn a_line_with_no_shares_is_not_a_line() {
        Line::new(PartyId::at(3), CurrencyCode::at(0), 0.0);
    }
}
