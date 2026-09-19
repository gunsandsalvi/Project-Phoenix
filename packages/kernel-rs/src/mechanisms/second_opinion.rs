//! THE SECOND OPINION: a traded price is a market disagreeing with the model, and for it to be one
//! the participants' reservations must be formed from something other than the quantity the price
//! is supposed to reveal.
//!
//! @spec XI-13 · XI-1 · 46 A3 · Law 2, Law 3, Law 4, Law 19 · Appendix B
//!
//! The general form of the failure: any mechanism in which the INPUT to the participants'
//! schedules is derived from the same quantity the clearing is supposed to DISCOVER produces a price
//! that is a fixed point of its own formula. It will look like a market and it will carry no
//! information. Three shapes delete the disagreement quietly, and this module refuses each.
//!
//! A probability computed from the accounts and fed to every seller. Then the credit
//! derivative's spread is a restatement of the accounting model, in the one instrument whose entire
//! purpose is to hold a different view. So there is no function here that turns accounts into a
//! probability anybody trades on: `implied` runs the OTHER way, from the cleared spread, and it is a
//! READ.
//!
//! A book with two participants and both of them hedgers. If every buyer of protection is above
//! an exposure limit and every seller is closing a regulatory gap, the cleared spread is a function
//! of regulatory gaps and never of a view; a period in which neither gap binds does not open the
//! book at all; and the price cannot move because somebody thinks the credit is mispriced.
//! `can_disagree` is the standing question a book must answer yes to.
//!
//! One rating held by nobody. An assessment that is a property of the firm rather than an
//! opinion held by a NAMED assessor means every participant agrees about credit by construction,
//! which removes the dispersion the auction needs to have two sides at all. `Assessments`
//! is keyed by (assessor, subject) and there is no read that takes a subject alone.

use crate::calendar::Day;
use crate::ids::InstrumentId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::standing;
use crate::ids::PartyId;

/// An opinion, held by somebody. Not a property of the firm: the assessor is part of the fact,
/// and two assessors looking at the same borrower are two facts, not one fact written twice.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Assessment {
    pub by: PartyId,
    pub of: PartyId,
    /// This assessor's own probability of default for this borrower, over its own horizon.
    pub probability: f64,
    /// The horizon is part of the number. A probability with no term is not a probability.
    pub year_fraction: f64,
}

/// The opinions in the world. There is no `rating_of(subject)` — asking a borrower for its
/// rating is asking for a fact nobody holds, and answering would make every participant agree by
/// construction.
#[derive(Default)]
pub struct Assessments {
    held: Vec<Assessment>,
}

impl Assessments {
    pub fn new() -> Assessments {
        Assessments { held: Vec::new() }
    }

    /// One writer per (assessor, subject): an assessor revising its view REPLACES its own, and
    /// never anybody else's.
    pub fn formed(&mut self, view: Assessment) {
        assert!(view.year_fraction > 0.0, "XI-13: a probability over no term is not one (Law 8)");
        for held in self.held.iter_mut() {
            if held.by == view.by && held.of == view.of {
                *held = view;
                return;
            }
        }
        self.held.push(view);
    }

    /// What one named assessor thinks. The only read there is.
    pub fn of(&self, by: PartyId, subject: PartyId) -> Option<f64> {
        self.held
            .iter()
            .find(|a| a.by == by && a.of == subject)
            .map(|a| a.probability)
    }

    /// Every opinion on one borrower, each with its holder attached.
    pub fn on(&self, subject: PartyId) -> Vec<Assessment> {
        self.held.iter().filter(|a| a.of == subject).copied().collect()
    }
}

/// The disagreement is load-bearing. It is what gives a market two sides, and a world
/// where every party expected the same thing would trade once and stop. This MEASURES it; nothing
/// reads it and adjusts anybody's view.
///
/// `None` below two opinions: one assessor is not a disagreement, and answering zero would say the
/// world agrees when in fact nobody has asked it.
pub fn dispersion(on: &[Assessment]) -> Option<f64> {
    let held: Vec<f64> = on.iter().map(|a| a.probability).collect();
    crate::num::dispersion(&held)
}

/// What an estate actually realised, carried with the dead party it came from — so a constant
/// cannot be passed where a recovery is wanted without naming a party that died (Appendix B: no
/// fixed recovery rate). The credit content of a credit derivative is exactly this number being an
/// outcome.
#[derive(Clone, Copy, Debug)]
pub struct Recovery {
    pub of: PartyId,
    pub realised: f64,
}

/// The implied probability is a READ from the cleared spread, never an input to it. This
/// is the only direction the arithmetic runs in this module — there is no companion that takes
/// accounts and hands a probability to sellers, which is the shape that makes the derivative's
/// spread a restatement of the accounting model.
pub fn implied(cleared_spread: f64, recovery: Recovery, year_fraction: f64) -> Option<f64> {
    assert!(year_fraction > 0.0, "XI-13: a spread over no term is not a rate (Law 8)");
    let loss_given_default = 1.0 - recovery.realised;
    if loss_given_default <= 0.0 {
        // An estate that paid in full implies nothing about default: there is no loss to divide by,
        // and inventing one would be the numeric default the law refuses.
        return None;
    }
    Some(cleared_spread / loss_given_default)
}

/// Why a participant is in this book at all. XI-13 needs at least one of them to be here because of
/// what it THINKS.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// It is above an exposure limit, or closing a regulatory gap. A book of only these clears at a
    /// function of regulatory gaps and never of a view.
    Hedge,
    /// It thinks the credit is mispriced. This is the participant the book must have.
    View,
    /// A dealer quoting both sides, willing to do either (Dealer Desks A2).
    Dealer,
}

#[derive(Clone, Copy, Debug)]
pub struct Participant {
    pub who: PartyId,
    pub reason: Reason,
    /// Dealer Desks A2: a dealer that will only buy is not two-sided, and a book that has only such
    /// a dealer in it cannot be made a market in.
    pub two_sided: bool,
}

/// Every derivative book needs a participant whose reason is a view, and a two-sided dealer
/// posting into it. A book that answers false clears at a price that cannot move because
/// somebody thinks the credit is mispriced — which is what the instrument is for.
pub fn can_disagree(book: &[Participant]) -> bool {
    let a_view = book.iter().any(|p| p.reason == Reason::View);
    let a_dealer = book.iter().any(|p| p.reason == Reason::Dealer && p.two_sided);
    a_view && a_dealer
}

// XI-13 RUNS HERE. `SecondOpinion` was in `running.rs`, apart from `dispersion` and
// `can_disagree`, which are in this file and which it did not call.

/// EVERY LENDER FORMS ITS OWN VIEW OF EVERY BORROWER IT HOLDS.
///
/// The `second_opinion` row counted how many lines printed. So this world had ONE opinion of every
/// borrower — whatever the ratings row said — and XI-13's whole point is that it must not: if the
/// loss is an arithmetic function of the borrower's accounts and every participant's reservation is
/// built from that function, the market cannot disagree with the accounting model and its price
/// carries no information.
///
/// The view is formed from what THIS lender has seen, which is why two lenders disagree: a
/// lender's experience of a borrower is the dues on ITS OWN paper that went past their day, and two
/// lenders holding different paper of the same borrower have seen different things. There is no
/// `rating_of(subject)` here — asking a borrower for its probability is asking for a fact nobody
/// holds, and answering would make every participant agree by construction.
///
/// The horizon is part of the number. A probability with no term is not a probability, so
/// the term is stood behind beside it.
pub struct SecondOpinion {
    pub kind: u32,
    pub days_per_period: i64,
}

impl Mechanism for SecondOpinion {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let today = Day(i64::from(ctx.period()) * self.days_per_period);

        // What this lender has SEEN of this borrower: the dues on the paper it holds, and how many
        // of them went past their day. Both are reads of the schedules.
        let mut seen: std::collections::HashMap<(u32, u32), (f64, f64)> = std::collections::HashMap::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            let borrower = ctx.instruments().issuer_of(line);
            let dues = ctx.schedules().of_instrument(line);
            if dues.is_empty() {
                continue;
            }
            let mut owed = 0.0;
            let mut late = 0.0;
            for &d in dues {
                let d = crate::stores::DueId(d);
                if ctx.schedules().due(d) > today {
                    continue;
                }
                owed += 1.0;
                if !ctx.schedules().paid(d) {
                    late += 1.0;
                }
            }
            if owed <= 0.0 {
                continue;
            }
            // And it is seen by whoever HOLDS the paper, and by nobody else.
            for &row in ctx.register().of_instrument(line) {
                let holder = ctx.register().holder_of(crate::ids::HoldingId(row)).0;
                if holder == borrower.0 || ctx.register().quantity(crate::ids::HoldingId(row)) <= 0.0 {
                    continue;
                }
                let e = seen.entry((holder, borrower.0)).or_insert((0.0, 0.0));
                e.0 += owed;
                e.1 += late;
            }
        }

        let mut formed: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (&(lender, borrower), &(owed, late)) in &seen {
            let lender = PartyId(lender);
            let borrower = PartyId(borrower);
            if !ctx.parties().alive(lender) || !ctx.parties().alive(borrower) {
                continue;
            }
            // Its own probability, over its own experience. Nothing is drawn and no model is
            // consulted: this is what happened to THIS lender.
            formed.push((lender, borrower, late / owed));
        }

        for (lender, borrower, probability) in formed {
            // A view a lender does not hold is one it cannot be shown to have been wrong
            // about, so it stands behind it — and a revision REPLACES its own and nobody else's.
            ctx.now_stands(standing::OWN_VIEW, lender, borrower, vec![probability, 1.0]);
            ctx.say(self.kind, &[lender.0, borrower.0], &[(0, Value::Num(probability))], false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn view(by: u32, of: u32, probability: f64) -> Assessment {
        Assessment { by: party(by), of: party(of), probability, year_fraction: 1.0 }
    }

    #[test]
    fn a_rating_is_an_opinion_held_by_a_named_assessor_and_not_a_property_of_the_firm() {
        // One rating held by nobody means every participant agrees about credit by
        // construction, which removes the dispersion the auction needs to have two sides at all.
        // Two assessors, one borrower, two different numbers — and both are facts.
        let mut a = Assessments::new();
        a.formed(view(1, 9, 0.02));
        a.formed(view(2, 9, 0.07));
        assert_eq!(a.of(party(1), party(9)), Some(0.02));
        assert_eq!(a.of(party(2), party(9)), Some(0.07));
        assert_eq!(a.on(party(9)).len(), 2);
    }

    #[test]
    fn an_assessor_revising_its_view_replaces_its_own_and_nobody_elses() {
        // One writer per fact, and the fact is (assessor, subject).
        let mut a = Assessments::new();
        a.formed(view(1, 9, 0.02));
        a.formed(view(2, 9, 0.07));
        a.formed(view(1, 9, 0.05));
        assert_eq!(a.on(party(9)).len(), 2);
        assert_eq!(a.of(party(1), party(9)), Some(0.05));
        assert_eq!(a.of(party(2), party(9)), Some(0.07));
    }

    #[test]
    fn a_borrower_has_no_rating_of_its_own_to_be_asked_for() {
        // The absence is the mechanism: there is no read that takes a subject alone, so no caller
        // can accidentally price off a figure nobody holds.
        let a = Assessments::new();
        assert!(a.of(party(1), party(9)).is_none());
    }

    #[test]
    fn the_disagreement_is_measured_and_one_opinion_is_not_a_disagreement() {
        // A world where every party expected the same thing would trade once and stop.
        let alone = [view(1, 9, 0.02)];
        assert!(dispersion(&alone).is_none());
        let apart = [view(1, 9, 0.02), view(2, 9, 0.10), view(3, 9, 0.06)];
        let together = [view(1, 9, 0.059), view(2, 9, 0.060), view(3, 9, 0.061)];
        assert!(dispersion(&apart).unwrap() > dispersion(&together).unwrap());
    }

    #[test]
    fn the_implied_probability_is_read_from_the_spread_and_never_fed_to_the_sellers() {
        // The arithmetic runs one way. A wider cleared spread implies a higher probability,
        // and nothing in this module runs the other direction.
        let r = Recovery { of: party(9), realised: 0.4 };
        let tight = implied(0.012, r, 1.0).unwrap();
        let wide = implied(0.030, r, 1.0).unwrap();
        assert!(wide > tight);
        let dust = 4.0 * f64::EPSILON * (0.030 + 0.6);
        assert!((wide - 0.05).abs() <= dust);
    }

    #[test]
    fn the_recovery_is_what_an_estate_realised_and_carries_the_party_it_came_from() {
        // No fixed recovery rate. A constant cannot be passed here without naming a
        // party that died, and the credit content of a credit derivative IS this number being an
        // outcome. An estate that paid in full implies nothing about default rather than zero.
        let paid_in_full = Recovery { of: party(9), realised: 1.0 };
        assert!(implied(0.012, paid_in_full, 1.0).is_none());
    }

    #[test]
    fn a_book_of_two_hedgers_cannot_disagree_with_the_model() {
        // If every buyer of protection is above an exposure limit and every seller is
        // closing a regulatory gap, the cleared spread is a function of regulatory gaps and never
        // of a view — and a period in which neither gap binds does not open the book at all.
        let hedgers = [
            Participant { who: party(1), reason: Reason::Hedge, two_sided: false },
            Participant { who: party(2), reason: Reason::Hedge, two_sided: false },
        ];
        assert!(!can_disagree(&hedgers));
    }

    #[test]
    fn a_book_needs_a_view_and_a_two_sided_dealer() {
        // Both, and the test says so by removing each in turn.
        let view_only = [
            Participant { who: party(1), reason: Reason::Hedge, two_sided: false },
            Participant { who: party(3), reason: Reason::View, two_sided: false },
        ];
        assert!(!can_disagree(&view_only));

        let one_way_dealer = [
            Participant { who: party(3), reason: Reason::View, two_sided: false },
            Participant { who: party(4), reason: Reason::Dealer, two_sided: false },
        ];
        assert!(!can_disagree(&one_way_dealer));

        let whole = [
            Participant { who: party(1), reason: Reason::Hedge, two_sided: false },
            Participant { who: party(3), reason: Reason::View, two_sided: false },
            Participant { who: party(4), reason: Reason::Dealer, two_sided: true },
        ];
        assert!(can_disagree(&whole));
    }

    #[test]
    #[should_panic(expected = "is not one")]
    fn a_probability_over_no_term_is_not_a_probability() {
        // The horizon is part of the number.
        let mut a = Assessments::new();
        a.formed(Assessment { year_fraction: 0.0, ..view(1, 9, 0.02) });
    }
}
