//! RATINGS AND ASSESSMENT: an ordinal judgement from OBSERVABLE STATE, published by a named assessor
//! — and **never derived from the price**.
//!
//! @spec 44 A1 · 44 A2 · 44 A2.a · 44 A3 · 44 A4 · 44 A5 · 44 A5.a · 44 B1 · 44 B2 · 44 B2.a ·
//! @spec 44 B3 · 44 C1 · 44 C1.a · 44 C2 · 44 C3 · 44 C4 · 44 C5 · 44 D1 · 44 D2 · 44 D3 · 44 D4 ·
//! @spec 44 D5 · 44 E1 · 44 E2 · 44 E3 · 44 E4 · XI-2 · XI-13 · Law 2, Law 3, Law 6, Law 19
//!
//! **A rating is never derived from the price** (A2.a). If it reads the spread it is a restatement of
//! the market, cannot disagree with it, and **deletes both its information content and the feedback in
//! D** — where the loop becomes a tautology. `State` has no price in it, so the forbidden input cannot
//! be supplied.
//!
//! **It is coarse and sticky** (A3): a small change in state does not move it, **which is what makes a
//! move meaningful and what makes it late.** The stickiness is the mechanism, not an approximation of
//! one.
//!
//! **It is an opinion, not a fact, and it can be wrong** (A4) — a rated-safe issuer can fail. **No
//! assessment that is always right** (E3): if a rating never misprices, the forced sales never surprise
//! anyone and A4 is deleted. `was_wrong` is the read that keeps that measurable.
//!
//! **No rating with no consequence** (E1). A published letter that no rule refers to is decoration; the
//! whole system is C, so mandates, capital charges, haircuts and covenant triggers all read the same
//! rating here — and `downgrade` returns every consequence at once, because C1.a's forced sale happens
//! **by every holder bound by it, at the same time.**
//!
//! **A haircut that is one number per instrument type — the same for the best and worst credit — is the
//! one leg of the downgrade loop that is wholly absent** (C3).
//!
//! **It is not the only assessment** (A5.a): holders run their own, and **one universal rating held by
//! nobody means every participant agrees about credit by construction** (XI-13).

use crate::ids::{InstrumentId, PartyId};

/// A1, B3: **an ordinal judgement** — an ordering across issuers, which is what makes it usable in a
/// rule. The ordering is the whole content; the letters are a display.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Grade {
    Highest,
    High,
    Upper,
    Lower,
    Speculative,
    Substantial,
    Defaulted,
}

/// A2: **observable state — leverage, coverage, cash, size, sector, AGE, and the trend in them.**
/// There is no price field, and no spread: A2.a's forbidden input cannot be supplied.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub leverage: f64,
    pub coverage: f64,
    pub cash: f64,
    pub size: f64,
    /// A2: age is state. A young issuer with the same numbers is not the same credit.
    pub age_periods: u32,
    /// A2: and the TREND in them, which is why a deteriorating issuer is rated below a stable one at
    /// the same level.
    pub trend: f64,
}

/// A5: **published by a NAMED assessor, which is a party with its own incentives** — and A5.a: it is
/// not the only assessment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rating {
    pub by: PartyId,
    pub of: PartyId,
    /// B2.a: **an instrument's rating differs from its issuer's**, and both must exist.
    pub instrument: Option<InstrumentId>,
    pub grade: Grade,
    /// B1: the probability of failing to perform, as this assessor sees it.
    pub probability: f64,
    /// B2: and, SEPARATELY, the loss given that failure — which depends on seniority and security.
    pub loss_given_failure: f64,
    pub since_period: u32,
}

/// A2, A3, E2: the grade this state implies, and **no rating changes for no reason** — every move
/// traces to a change in state. **Coarse**, so a small change does not move it.
pub fn grade_from(s: &State) -> Grade {
    // The bands are the ordinal judgement itself: a POLICY of the assessor (Law 2), stated here
    // rather than fitted to a target distribution (E4).
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

/// A3: **it is sticky** — a move happens only when the state has moved far enough to cross a band, and
/// that is what makes a move meaningful and what makes it LATE. `None` where the state has not moved
/// the grade, which is most periods.
pub fn reassess(held: &Rating, now: &State, period: u32) -> Option<Rating> {
    let grade = grade_from(now);
    if grade == held.grade {
        return None;
    }
    Some(Rating { grade, since_period: period, ..*held })
}

/// C1, C1.a: **a mandate restricts what a fund, insurer or pension may hold**, so a downgrade past a
/// boundary is **a forced sale by every holder bound by it, AT THE SAME TIME** — a real, dated,
/// mechanical flow (XI-2).
#[derive(Clone, Copy, Debug)]
pub struct Mandate {
    pub holder: PartyId,
    pub lowest_allowed: Grade,
    pub holds: f64,
}

/// C2, C3, C4: everything the downgrade sets off, **at once** — because a rating with no consequence is
/// decoration (E1) and the whole system is C.
#[derive(Clone, Debug, PartialEq)]
pub struct Downgraded {
    /// C1.a: every bound holder, and what each must sell. All on the same date.
    pub forced_sales: Vec<(PartyId, f64)>,
    /// C2: **a downgrade consumes a bank's capital without the bank doing anything.**
    pub extra_capital: f64,
    /// C3: and reduces how much can be borrowed against the asset — the leg of the loop that a
    /// per-type haircut deletes.
    pub lost_borrowing: f64,
    /// C4: covenants, triggers, the right to demand more collateral.
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

/// C3: **a per-ISSUER risk measure closes the loop, with or without a rating table.** The haircut reads
/// this issuer's own grade, so the best and worst credit of the same instrument type are not the same
/// number.
pub fn haircut(g: Grade, by_tenor: f64) -> f64 {
    let by_credit = match g {
        Grade::Highest => 1.01,
        Grade::High => 1.02,
        Grade::Upper => 1.05,
        Grade::Lower => 1.10,
        Grade::Speculative => 1.25,
        Grade::Substantial => 1.60,
        Grade::Defaulted => 4.0,
    };
    by_tenor * by_credit
}

/// D1, D2, D3: **a downgrade causes selling, capital pressure and funding loss; those raise the
/// issuer's cost of funds; which WORSENS the state** — and can cause a further downgrade. D4: the loop
/// must be emergent and **traceable step by step**, which is what this returns: the state after the
/// cost of funds it caused.
pub fn worsened_by(s: &State, cost_of_funds_rose_by: f64) -> State {
    State {
        // Dearer funding eats coverage, which is the observable the next assessment reads. Nothing
        // here reads a price: the rise arrived as a fact about what the issuer now pays.
        coverage: s.coverage - cost_of_funds_rose_by,
        trend: s.trend - cost_of_funds_rose_by,
        ..*s
    }
}

/// D5: **it works the other way too — improvement widens the buyer base and cheapens funding.** How
/// many mandates can hold this credit at all, which is the buyer base as a read.
pub fn buyer_base(g: Grade, mandates: &[Mandate]) -> usize {
    mandates.iter().filter(|m| g <= m.lowest_allowed).count()
}

/// A4, E3: **no assessment that is always right.** A rated-safe issuer can fail, and if a rating never
/// misprices, C1's forced sales never surprise anyone. This is the read that keeps it measurable.
impl Grade {
    /// 22i.2: the grade as a term, so a house can STAND behind it (`standing::GRADE`). A grade is
    /// ordinal, so the number is its rank on the scale and nothing else — never a score, never
    /// something to average (E4: the distribution is a read of the states, not a target).
    pub fn rank(self) -> f64 {
        match self {
            Grade::Highest => 0.0,
            Grade::High => 1.0,
            Grade::Upper => 2.0,
            Grade::Lower => 3.0,
            Grade::Speculative => 4.0,
            Grade::Substantial => 5.0,
            Grade::Defaulted => 6.0,
        }
    }

    /// And back, reading a term a house is standing behind. Anything off the scale is not a grade.
    pub fn at_rank(rank: f64) -> Option<Grade> {
        Some(match rank as i64 {
            0 => Grade::Highest,
            1 => Grade::High,
            2 => Grade::Upper,
            3 => Grade::Lower,
            4 => Grade::Speculative,
            5 => Grade::Substantial,
            6 => Grade::Defaulted,
            _ => return None,
        })
    }
}

pub fn was_wrong(r: &Rating, actually_failed: bool) -> bool {
    actually_failed && r.grade <= Grade::Upper
}

/// E4: **the distribution of ratings across issuers is a READ of their states, never a target
/// distribution the issuers were fitted to.** Count by grade, from the states themselves.
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
        // A2, A2.a: if it read the spread it would be a restatement of the market and could not
        // disagree with it. `State` has no price field, so the input cannot be supplied.
        let strong = state(1.0, 4.0, 0.1);
        let weak = state(6.0, 1.5, -0.2);
        assert!(grade_from(&strong) < grade_from(&weak));
    }

    #[test]
    fn age_and_trend_are_state_so_two_issuers_with_the_same_numbers_are_not_the_same_credit() {
        // A2: age is state, and the TREND in the observables is too.
        let established = state(1.0, 4.0, 0.1);
        let young = State { age_periods: 2, ..established };
        assert!(grade_from(&young) > grade_from(&established));
        let deteriorating = state(1.0, 4.0, -1.5);
        assert!(grade_from(&deteriorating) > grade_from(&established));
    }

    #[test]
    fn it_is_sticky_so_a_small_change_does_not_move_it_and_a_move_is_late() {
        // A3, E2: no rating changes for no reason, and the stickiness is what makes a move meaningful.
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
        // C1, C1.a, E1: a real, dated, mechanical flow — and a rating no rule refers to is decoration.
        let mandates = [
            Mandate { holder: party(30), lowest_allowed: Grade::Upper, holds: 5_000.0 },
            Mandate { holder: party(31), lowest_allowed: Grade::Lower, holds: 2_000.0 },
            Mandate { holder: party(32), lowest_allowed: Grade::Substantial, holds: 800.0 },
        ];
        let d = downgrade(Grade::Speculative, &mandates, 10_000.0, 0.04, 0.15, 6_000.0, &[Grade::Lower]);
        assert_eq!(d.forced_sales.len(), 2);
        assert_eq!(d.forced_sales[0], (party(30), 5_000.0));
        assert_eq!(d.forced_sales[1], (party(31), 2_000.0));
        // C2: the bank's capital is consumed without the bank doing anything.
        assert_eq!(d.extra_capital, 400.0);
        // C3: and what can be borrowed against it falls.
        assert_eq!(d.lost_borrowing, 900.0);
        // C4: and a covenant fires.
        assert_eq!(d.triggers_fired, 1);
    }

    #[test]
    fn the_haircut_reads_the_issuers_own_grade_and_not_the_instrument_type() {
        // C3: a haircut that is one number per type — the same for the best and worst credit — is the
        // one leg of the downgrade loop that is wholly absent.
        assert!(haircut(Grade::Speculative, 1.0) > haircut(Grade::Highest, 1.0));
    }

    #[test]
    fn the_loop_is_traceable_step_by_step() {
        // D1, D2, D3, D4: the downgrade raises the cost of funds, which worsens the observable state,
        // which can cause a further downgrade. It is emergent, and nothing here reads a price — which
        // is exactly what a rating read off the spread could never produce.
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
        // D5: it works the other way too.
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
        // A4, E3: no assessment that is always right. If a rating never misprices, C1's forced sales
        // never surprise anyone.
        assert!(was_wrong(&rated(Grade::Highest), true));
        assert!(!was_wrong(&rated(Grade::Highest), false));
        assert!(!was_wrong(&rated(Grade::Substantial), true));
    }

    #[test]
    fn an_instruments_rating_differs_from_its_issuers_and_both_exist() {
        // B2, B2.a: the probability of failing to perform, and SEPARATELY the loss given that failure,
        // which depends on seniority and security.
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
        // E4: never a target distribution the issuers were fitted to. Change the states and the
        // distribution changes; there is nothing to fit to.
        let states = [state(1.0, 4.0, 0.1), state(3.0, 2.0, 0.0), state(9.0, 1.0, -1.0)];
        let d = distribution(&states);
        let counted: usize = d.iter().map(|(_, n)| n).sum();
        assert_eq!(counted, 3);
        assert!(d.iter().any(|(g, n)| *g == Grade::Highest && *n == 1));
    }

    #[test]
    fn a_rating_is_an_opinion_held_by_a_named_assessor_and_is_not_the_only_one() {
        // A5, A5.a, XI-13: one universal rating held by nobody means every participant agrees about
        // credit by construction.
        let one = rated(Grade::Upper);
        let another = Rating { by: party(81), grade: Grade::Lower, ..one };
        assert_ne!(one.by, another.by);
        assert_ne!(one.grade, another.grade);
    }
}
