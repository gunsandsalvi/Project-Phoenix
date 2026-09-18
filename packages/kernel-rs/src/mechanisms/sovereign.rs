//! THE TREASURY RAISES MONEY BEFORE IT SPENDS IT, from a market that must clear.
//!
//! @spec XI-9, Sovereign C3, Central Bank D3, Central Bank E2, Money B3.c, Appendix B, Law 6
//!
//! **The funding constraint is the whole of it.** With an automatic overdraft, causation reverses:
//! the treasury spends into the negative and issues to CLEAR it, so the forward funding plan has
//! nothing to do, the cash buffer has no reason to exist, a failed auction costs nothing and
//! carries no information, **a sovereign cannot fail** — so its paper is risk-free by construction,
//! nothing prices its credit, its rating has no consumer, and the whole assessment system above it
//! is decoration. Everything priced over the sovereign curve assumes a borrower with a funding
//! constraint; a benchmark issued by a borrower that cannot fail is not a benchmark for credit.
//!
//! `money::NoOverdraftForTheTreasury` is the refusal. This is the other half: the programme is
//! sized FORWARD against redemptions and outlays, the buffer is a real holding, **and a shortfall
//! is a real event with real handling** — pay from the buffer, cut or defer an outlay, or come back
//! at a different size. It is never a smaller number quietly substituted (Law 6).

use crate::ids::{CurrencyCode, InstrumentId, PartyId};

/// What the treasury has to find this period, sized FORWARD from what it already owes and what it
/// has already decided to spend. Nothing here is a forecast: both are commitments already made.
#[derive(Clone, Copy, Debug)]
pub struct Programme {
    /// What falls due on paper already issued.
    pub redemptions: f64,
    /// What it has committed to pay out.
    pub outlays: f64,
    /// The buffer it holds, which is a real holding of real money and not a line in a plan.
    pub buffer: f64,
}

impl Programme {
    /// What it must RAISE: what it owes and has committed, less what it is already holding. A
    /// treasury whose buffer covers the period raises nothing, which is an answer.
    pub fn to_raise(&self) -> f64 {
        self.redemptions + self.outlays - self.buffer
    }
}

/// XI-9: what a treasury does when the money is not there. **Each of these is a real act with a
/// consequence**, which is what an overdraft removed by making the shortfall cost nothing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shortfall {
    /// It holds enough. There is nothing to handle.
    None,
    /// Pay it out of the buffer — which is why the buffer exists, and it is smaller afterwards.
    FromTheBuffer { drawn: f64 },
    /// Cut or defer an outlay. Somebody does not get paid this period, and that is an EVENT with a
    /// named counterparty, never a number quietly reduced.
    DeferAnOutlay { deferred: f64 },
    /// Come back at a different size or maturity. The auction is re-run, and a failed one has
    /// cost something — which is what makes its result carry information (Sovereign C3).
    ComeBackToTheMarket { still_short: f64 },
}

/// Sovereign C3, XI-9: what the auction RAISED, which is what cleared and never what was asked for.
/// A market that must clear is one where the answer can be less than the question.
#[derive(Clone, Copy, Debug)]
pub struct Auction {
    pub asked: f64,
    /// What real demand actually took, at a level real demand set.
    pub raised: f64,
}

impl Auction {
    /// XI-9: **A FAILED AUCTION COSTS SOMETHING.** What is still short after it is what the
    /// treasury has to handle, and handling it is the consequence that makes the result
    /// informative rather than decorative.
    pub fn still_short(&self) -> f64 {
        self.asked - self.raised
    }

    pub fn failed(&self) -> bool {
        self.raised < self.asked
    }
}

/// The treasury's own decision when it is short, in the order XI-9 puts them: the buffer is what it
/// is for; an outlay deferred is somebody not paid; and past both it goes back to the market.
///
/// It is a READ of the state, not a policy: which of the three is available is arithmetic about
/// what it holds and what it owes, and the treasury's module decides between those that are.
pub fn handle(short_by: f64, buffer: f64, deferrable: f64) -> Shortfall {
    if short_by <= 0.0 {
        return Shortfall::None;
    }
    if buffer >= short_by {
        return Shortfall::FromTheBuffer { drawn: short_by };
    }
    // Law 6: the buffer is not "as much as it can" — what it does not cover is still short, and
    // the rest of the shortfall is handled by something else rather than clamped away.
    let after_buffer = short_by - buffer;
    if deferrable >= after_buffer {
        return Shortfall::DeferAnOutlay { deferred: after_buffer };
    }
    Shortfall::ComeBackToTheMarket { still_short: after_buffer - deferrable }
}

/// XI-9: **AND A SOVEREIGN CAN FAIL.** A payment it owed and did not make, on a date, to a named
/// holder — which is what gives its rating its first real consumer.
#[derive(Clone, Copy, Debug)]
pub struct Missed {
    pub issuer: PartyId,
    pub instrument: InstrumentId,
    pub owed: f64,
    pub paid: f64,
    pub ccy: CurrencyCode,
    pub period: u32,
}

impl Missed {
    /// A missed payment is a FACT about two numbers, not a judgement: what fell due and what
    /// arrived. There is no grace here and no tolerance — Law 7's dust is the arithmetic of the
    /// sum, and a payment short by more than its own dust was not made.
    pub fn is_default(&self, dust: f64) -> bool {
        self.owed - self.paid > dust
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_programme_is_sized_forward_and_the_buffer_is_part_of_it() {
        let p = Programme { redemptions: 400.0, outlays: 250.0, buffer: 100.0 };
        assert_eq!(p.to_raise(), 550.0);
        // A treasury whose buffer covers the period raises nothing, which is an answer and not a
        // special case.
        let flush = Programme { redemptions: 10.0, outlays: 5.0, buffer: 90.0 };
        assert!(flush.to_raise() < 0.0, "it needs nothing and is holding more than it owes");
    }

    #[test]
    fn a_failed_auction_leaves_a_real_shortfall_and_therefore_carries_information() {
        let a = Auction { asked: 1_000.0, raised: 600.0 };
        assert!(a.failed());
        assert_eq!(a.still_short(), 400.0);
        // With an automatic overdraft this number would be zero and the auction would say nothing.
        let full = Auction { asked: 1_000.0, raised: 1_000.0 };
        assert!(!full.failed());
        assert_eq!(full.still_short(), 0.0);
    }

    #[test]
    fn the_buffer_is_what_it_is_for_and_what_it_does_not_cover_is_still_short() {
        // It covers: this is the reason to hold one.
        assert_eq!(handle(300.0, 500.0, 0.0), Shortfall::FromTheBuffer { drawn: 300.0 });
        // It does not cover: Law 6 — the rest is NOT clamped away, it goes to the next handling.
        assert_eq!(handle(800.0, 500.0, 400.0), Shortfall::DeferAnOutlay { deferred: 300.0 });
        // And past both, it goes back to the market with what is still short.
        assert_eq!(
            handle(2_000.0, 500.0, 400.0),
            Shortfall::ComeBackToTheMarket { still_short: 1_100.0 }
        );
        // Nothing short is nothing to handle.
        assert_eq!(handle(0.0, 500.0, 400.0), Shortfall::None);
    }

    #[test]
    fn a_sovereign_can_fail_and_it_is_two_numbers_rather_than_a_judgement() {
        let coupon = Missed {
            issuer: PartyId::at(2),
            instrument: InstrumentId::at(7),
            owed: 1_000.0,
            paid: 999.0,
            ccy: CurrencyCode::at(0),
            period: 12,
        };
        // Law 7: dust is the arithmetic of the sum, never a grace period somebody chose.
        let dust = 3.0 * f64::EPSILON * (coupon.owed + coupon.paid);
        assert!(coupon.is_default(dust), "a pound short is short");
        let met = Missed { paid: 1_000.0, ..coupon };
        assert!(!met.is_default(dust));
    }

    #[test]
    fn a_shortfall_handled_is_never_a_number_quietly_reduced() {
        // The three handlings account for the WHOLE shortfall between them: what the buffer draws,
        // what is deferred and what goes back to the market sum to what was short. A world where
        // they did not would be one where the missing money simply stopped existing.
        let (short, buffer, deferrable) = (2_000.0, 500.0, 400.0);
        match handle(short, buffer, deferrable) {
            Shortfall::ComeBackToTheMarket { still_short } => {
                assert_eq!(buffer + deferrable + still_short, short);
            }
            other => panic!("{other:?}"),
        }
    }
}
