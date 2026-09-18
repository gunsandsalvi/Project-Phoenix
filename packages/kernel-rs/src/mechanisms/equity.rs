//! EQUITY: a residual claim, counted in shares, perpetual, and carrying control.
//!
//! @spec 10 A1, A1.a, A1.b, A2, A2.a, A3, A4, A5, A5.a, A5.b, A6, B1 · XI-8 · Law 6, Law 8, Law 9
//!
//! **A1: what is left after every other claim is paid**, ranking below all debt — which is why the
//! estate's waterfall puts it last and why that is what makes it equity at all.
//!
//! **A1.b: its value can be ZERO AND NOT NEGATIVE. Limited liability is a real property**, and it
//! is the one place in this engine where a floor is not Law 6's defect: a holder of a share is not
//! liable past it, so the arithmetic of what it is worth genuinely stops at nothing. The residual
//! below zero does not vanish — **it lands on the creditors**, which is where the estate puts it.
//! A "floor" that dropped the loss instead of moving it would be the defect.
//!
//! **A4: it is PERPETUAL** — no maturity, no redemption. That is why equity is a different
//! instrument from a claim that comes back, and it is why a share has no `matures` field to leave
//! unset (Appendix A: optional-means-unset is how a perpetual becomes a bond nobody dated).

use crate::ids::{CurrencyCode, PartyId};

/// A2.a: **a share count changes only by a NAMED EVENT.** A number that drifted would be a
/// liability nobody issued.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShareEvent {
    Issued,
    /// A buy-back: the company took its own shares in, and they are not outstanding any more.
    BoughtBack,
    /// D3: a split restates the count without changing what anybody owns a share OF.
    Split,
    Cancelled,
}

/// A2, A3, A6: the line itself. Counted in SHARES — a unit that is not money — quoted in the
/// issuer's own currency, and named the way a market would name it (Law 9: the issuer).
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub issuer: PartyId,
    pub ccy: CurrencyCode,
    /// A2: outstanding shares. Changed only by `apply`, never assigned.
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

    /// A2.a: the count moves for a NAMED reason, and by nothing else.
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
            // D3: a split RESTATES. What anybody owns a share of is unchanged, which is why the
            // ratio multiplies the count rather than adding to it.
            ShareEvent::Split => self.outstanding *= shares,
        }
    }
}

/// A1, A1.b: **what the residual is worth to equity.** Zero and not negative — a holder is not
/// liable past its share.
///
/// The loss below zero is RETURNED, not dropped: it lands on the creditors (XI-8), and a function
/// that returned only the floored value would be hiding it. That is the difference between limited
/// liability and a clamp.
pub fn residual(assets: f64, debt: f64) -> (f64, f64) {
    let left = assets - debt;
    if left >= 0.0 {
        (left, 0.0)
    } else {
        (0.0, -left)
    }
}

/// A5: **control rides with it — a vote per share.** A5.a: which makes a majority a thing that can
/// be BOUGHT, and A5.b: control therefore has a value distinct from the cash flows.
///
/// A cell holding shares casts the votes of what it holds, because a weight is a count (XI-15) —
/// there is no per-member fraction of a vote anywhere.
pub fn votes(held: f64) -> f64 {
    held
}

/// A5.a: control is MORE THAN HALF of what exists, read off the outstanding count rather than
/// declared. `down` is the tick: half of an odd count is not a share.
pub fn control_needs(outstanding: f64) -> f64 {
    (outstanding / 2.0).floor() + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_liability_stops_at_nothing_and_the_rest_lands_on_the_creditors() {
        // A1.b: value can be zero and not negative — a real property, not a clamp.
        assert_eq!(residual(1_000.0, 400.0), (600.0, 0.0));
        // And the loss below zero does NOT vanish. It is returned, because it is the creditors'
        // (XI-8). A function that gave back only the floored value would be hiding it, and THAT
        // would be Law 6's defect.
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
        // A5.a: a majority is a thing that can be BOUGHT, so it is read off the register.
        assert_eq!(control_needs(1_000.0), 501.0);
        // Half of an odd count is not a share, so the tick matters.
        assert_eq!(control_needs(999.0), 500.0);
        // XI-15: a cell casts the votes of what it HOLDS — a weight is a count, and there is no
        // per-member fraction of a vote anywhere.
        assert_eq!(votes(4_000.0), 4_000.0);
    }

    #[test]
    #[should_panic(expected = "a line with no shares is not a line")]
    fn a_line_with_no_shares_is_not_a_line() {
        Line::new(PartyId::at(3), CurrencyCode::at(0), 0.0);
    }
}
