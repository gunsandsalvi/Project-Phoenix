//! RATINGS AND ASSESSMENT: an ordinal judgement from OBSERVABLE STATE, published by a named assessor
//! — and never derived from the price.
//!
//! @spec 44 A1 · 44 A2 · 44 A2.a · 44 A3 · 44 A4 · 44 A5 · 44 A5.a · 44 B1 · 44 B2 · 44 B2.a ·
//! @spec 44 B3 · 44 C1 · 44 C1.a · 44 C2 · 44 C3 · 44 C4 · 44 C5 · 44 D1 · 44 D2 · 44 D3 · 44 D4 ·
//! @spec 44 D5 · 44 E1 · 44 E2 · 44 E3 · 44 E4 · XI-2 · XI-13 · Law 2, Law 3, Law 6, Law 19

use crate::stores::Grade;
use crate::ids::{InstrumentId, PartyId};


/// Observable state — leverage, coverage, cash, size, sector, AGE, and the trend in them.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub leverage: f64,
    pub coverage: f64,
    pub cash: f64,
    pub size: f64,
    pub age_periods: u32,
    /// And the TREND in them, which is why a deteriorating issuer is rated below a stable one at the
    /// same level.
    pub trend: f64,
}

/// Published by a NAMED assessor, which is a party with its own incentives — and A5.a: it is not the
/// only assessment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rating {
    pub by: PartyId,
    pub of: PartyId,
    /// An instrument's rating differs from its issuer's, and both must exist.
    pub instrument: Option<InstrumentId>,
    pub grade: Grade,
    /// The probability of failing to perform, as this assessor sees it.
    pub probability: f64,
    /// And, SEPARATELY, the loss given that failure — which depends on seniority and security.
    pub loss_given_failure: f64,
    pub since_period: u32,
}

/// The grade this state implies, and no rating changes for no reason — every move traces to a change
/// in state.
pub fn grade_from(s: &State) -> Grade {
    // The bands are the ordinal judgement itself: a POLICY of the assessor, stated here rather than
    // fitted to a target distribution.
    let strain = s.leverage / s.coverage - s.trend;
    let young = s.age_periods < 8;
    match strain {
        x if x < 0.5 && !young => Grade::Highest,
        x if x < 1.0 => Grade::High,
        x if x < 2.0 => Grade::Upper,
        x if x < 3.5 => Grade::Lower,
        x if x < 5.0 => Grade::Speculative,
        x if x.is_finite() => Grade::Substantial,
        _ => Grade::Defaulted,
    }
}

/// It is sticky — a move happens only when the state has moved far enough to cross a band, and that
/// is what makes a move meaningful and what makes it LATE.
pub fn reassess(held: &Rating, now: &State, period: u32) -> Option<Rating> {
    let grade = grade_from(now);
    if grade == held.grade {
        return None;
    }
    Some(Rating { grade, since_period: period, ..*held })
}

/// A mandate restricts what a fund, insurer or pension may hold, so a downgrade past a boundary is a
/// forced sale by every holder bound by it, AT THE SAME TIME — a real, dated, mechanical flow.
#[derive(Clone, Copy, Debug)]
pub struct Mandate {
    pub holder: PartyId,
    pub lowest_allowed: Grade,
    pub holds: f64,
}

/// Everything the downgrade sets off, at once — because a rating with no consequence is decoration
/// and the whole system is C.
#[derive(Clone, Debug, PartialEq)]
pub struct Downgraded {
    /// Every bound holder, and what each must sell.
    pub forced_sales: Vec<(PartyId, f64)>,
    /// A downgrade consumes a bank's capital without the bank doing anything.
    pub extra_capital: f64,
    /// And reduces how much can be borrowed against the asset — the leg of the loop that a per-type
    /// haircut deletes.
    pub lost_borrowing: f64,
    /// Covenants, triggers, the right to demand more collateral.
    pub triggers_fired: usize,
}

pub fn downgrade(
    to: Grade,
    mandates: &[Mandate],
    bank_holding: f64,
    capital_per_grade: f64,
    haircut_per_grade: f64,
    borrowed_against: f64,
    covenants_at: &[Grade],
) -> Downgraded {
    let forced_sales = mandates
        .iter()
        .filter(|m| to > m.lowest_allowed)
        .map(|m| (m.holder, m.holds))
        .collect();
    Downgraded {
        forced_sales,
        extra_capital: bank_holding * capital_per_grade,
        lost_borrowing: borrowed_against * haircut_per_grade,
        triggers_fired: covenants_at.iter().filter(|g| to > **g).count(),
    }
}


/// A downgrade causes selling, capital pressure and funding loss; those raise the issuer's cost of
/// funds; which WORSENS the state — and can cause a further downgrade.
pub fn worsened_by(s: &State, cost_of_funds_rose_by: f64) -> State {
    State {
        // Dearer funding eats coverage, which is the observable the next assessment reads.
        coverage: s.coverage - cost_of_funds_rose_by,
        trend: s.trend - cost_of_funds_rose_by,
        ..*s
    }
}

/// It works the other way too — improvement widens the buyer base and cheapens funding.
pub fn buyer_base(g: Grade, mandates: &[Mandate]) -> usize {
    mandates.iter().filter(|m| g <= m.lowest_allowed).count()
}

/// No assessment that is always right.

pub fn was_wrong(r: &Rating, actually_failed: bool) -> bool {
    actually_failed && r.grade <= Grade::Upper
}

/// The distribution of ratings across issuers is a READ of their states, never a target distribution
/// the issuers were fitted to.
pub fn distribution(states: &[State]) -> Vec<(Grade, usize)> {
    let grades = [
        Grade::Highest,
        Grade::High,
        Grade::Upper,
        Grade::Lower,
        Grade::Speculative,
        Grade::Substantial,
        Grade::Defaulted,
    ];
    grades
        .iter()
        .map(|g| (*g, states.iter().filter(|s| grade_from(s) == *g).count()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn state(leverage: f64, coverage: f64, trend: f64) -> State {
        State { leverage, coverage, cash: 500.0, size: 10_000.0, age_periods: 40, trend }
    }

    fn rated(grade: Grade) -> Rating {
        Rating {
            by: party(80),
            of: party(9),
            instrument: None,
            grade,
            probability: 0.01,
            loss_given_failure: 0.6,
            since_period: 1,
        }
    }

    #[test]
    fn a_rating_reads_observable_state_and_there_is_no_price_to_read() {
        // If it read the spread it would be a restatement of the market and could not disagree with
        // it.
        let strong = state(1.0, 4.0, 0.1);
        let weak = state(6.0, 1.5, -0.2);
        assert!(grade_from(&strong) < grade_from(&weak));
    }

    #[test]
    fn age_and_trend_are_state_so_two_issuers_with_the_same_numbers_are_not_the_same_credit() {
        // Age is state, and the TREND in the observables is too.
        let established = state(1.0, 4.0, 0.1);
        let young = State { age_periods: 2, ..established };
        assert!(grade_from(&young) > grade_from(&established));
        let deteriorating = state(1.0, 4.0, -1.5);
        assert!(grade_from(&deteriorating) > grade_from(&established));
    }

    #[test]
    fn it_is_sticky_so_a_small_change_does_not_move_it_and_a_move_is_late() {
        // No rating changes for no reason, and the stickiness is what makes a move meaningful.
        let held = rated(Grade::Upper);
        let barely = state(3.0, 2.0, 0.1);
        assert_eq!(grade_from(&barely), Grade::Upper);
        assert!(reassess(&held, &barely, 10).is_none());
        let much_worse = state(9.0, 2.0, -0.5);
        let moved = reassess(&held, &much_worse, 10).unwrap();
        assert!(moved.grade > Grade::Upper);
        assert_eq!(moved.since_period, 10);
    }

    #[test]
    fn a_downgrade_forces_every_bound_holder_to_sell_on_the_same_date() {
        // A real, dated, mechanical flow — and a rating no rule refers to is decoration.
        let mandates = [
            Mandate { holder: party(30), lowest_allowed: Grade::Upper, holds: 5_000.0 },
            Mandate { holder: party(31), lowest_allowed: Grade::Lower, holds: 2_000.0 },
            Mandate { holder: party(32), lowest_allowed: Grade::Substantial, holds: 800.0 },
        ];
        let d = downgrade(Grade::Speculative, &mandates, 10_000.0, 0.04, 0.15, 6_000.0, &[Grade::Lower]);
        assert_eq!(d.forced_sales.len(), 2);
        assert_eq!(d.forced_sales[0], (party(30), 5_000.0));
        assert_eq!(d.forced_sales[1], (party(31), 2_000.0));
        // The bank's capital is consumed without the bank doing anything.
        assert_eq!(d.extra_capital, 400.0);
        // And what can be borrowed against it falls.
        assert_eq!(d.lost_borrowing, 900.0);
        // And a covenant fires.
        assert_eq!(d.triggers_fired, 1);
    }

    #[test]
    fn the_loop_is_traceable_step_by_step() {
        // The downgrade raises the cost of funds, which worsens the observable state, which can
        // cause a further downgrade.
        let before = state(3.0, 4.0, 0.0);
        let first = grade_from(&before);
        let after = worsened_by(&before, 2.0);
        let second = grade_from(&after);
        assert!(second > first);
        // And a second round is worse again: the loop runs.
        let third = grade_from(&worsened_by(&after, 1.0));
        assert!(third >= second);
    }

    #[test]
    fn improvement_widens_the_buyer_base() {
        // It works the other way too.
        let mandates = [
            Mandate { holder: party(30), lowest_allowed: Grade::Upper, holds: 5_000.0 },
            Mandate { holder: party(31), lowest_allowed: Grade::Lower, holds: 2_000.0 },
            Mandate { holder: party(32), lowest_allowed: Grade::Substantial, holds: 800.0 },
        ];
        assert_eq!(buyer_base(Grade::Speculative, &mandates), 1);
        assert_eq!(buyer_base(Grade::High, &mandates), 3);
    }

    #[test]
    fn a_rated_safe_issuer_can_fail() {
        // No assessment that is always right.
        assert!(was_wrong(&rated(Grade::Highest), true));
        assert!(!was_wrong(&rated(Grade::Highest), false));
        assert!(!was_wrong(&rated(Grade::Substantial), true));
    }

    #[test]
    fn an_instruments_rating_differs_from_its_issuers_and_both_exist() {
        // The probability of failing to perform, and SEPARATELY the loss given that failure, which
        // depends on seniority and security.
        let issuer = rated(Grade::Upper);
        let subordinated = Rating {
            instrument: Some(InstrumentId::at(5)),
            grade: Grade::Lower,
            loss_given_failure: 0.9,
            ..issuer
        };
        assert!(subordinated.grade > issuer.grade);
        assert_eq!(subordinated.probability, issuer.probability);
        assert!(subordinated.loss_given_failure > issuer.loss_given_failure);
    }

    #[test]
    fn the_distribution_is_a_read_of_the_issuers_states() {
        // Never a target distribution the issuers were fitted to.
        let states = [state(1.0, 4.0, 0.1), state(3.0, 2.0, 0.0), state(9.0, 1.0, -1.0)];
        let d = distribution(&states);
        let counted: usize = d.iter().map(|(_, n)| n).sum();
        assert_eq!(counted, 3);
        assert!(d.iter().any(|(g, n)| *g == Grade::Highest && *n == 1));
    }

    #[test]
    fn a_rating_is_an_opinion_held_by_a_named_assessor_and_is_not_the_only_one() {
        // One universal rating held by nobody means every participant agrees about credit by
        // construction.
        let one = rated(Grade::Upper);
        let another = Rating { by: party(81), grade: Grade::Lower, ..one };
        assert_ne!(one.by, another.by);
        assert_ne!(one.grade, another.grade);
    }
}
