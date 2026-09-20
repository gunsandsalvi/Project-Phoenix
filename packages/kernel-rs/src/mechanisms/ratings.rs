//! RATINGS AND ASSESSMENT: an ordinal judgement from OBSERVABLE STATE, published by a named assessor
//! — and never derived from the price.
//!
//! @spec 44 A1 · 44 A2 · 44 A2.a · 44 A3 · 44 A4 · 44 A5 · 44 A5.a · 44 B1 · 44 B2 · 44 B2.a ·
//! @spec 44 B3 · 44 C1 · 44 C1.a · 44 C2 · 44 C3 · 44 C4 · 44 C5 · 44 D1 · 44 D2 · 44 D3 · 44 D4 ·
//! @spec 44 D5 · 44 E1 · 44 E2 · 44 E3 · 44 E4 · XI-2 · XI-13 · Law 2, Law 3, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::booked_equity;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::Grade;


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

/// AN ASSESSOR'S OWN SCALE: where the top of it sits, what one notch of it is worth, and how far it
/// marks down a name it has no record for. The twenty-two band edges fall out of these, so they are
/// not anybody's to declare.
#[derive(Clone, Copy, Debug)]
pub struct Scale {
    /// The strain a name at the top of the scale already carries.
    pub best_carries: f64,
    /// The strain one notch of the scale is worth.
    pub per_notch: f64,
    /// The notches a name with no record is marked down.
    pub without_a_record: f64,
    /// And how long a record has to be before it is one.
    pub record_after: u32,
}

/// The grade this state implies, and no rating changes for no reason — every move traces to a change
/// in state.
pub fn grade_from(s: &State, by: &Scale) -> Grade {
    assert!(by.per_notch > 0.0, "44 A2: a scale whose notches are worth nothing orders nothing");
    // What it owes against what it earns, less the direction it is moving in. The judgement is
    // where the assessor puts the top of its scale and how coarse its notches are, not a table of
    // edges fitted to a target distribution.
    let strain = s.leverage / s.coverage - s.trend;
    // A short record is not strain: the assessor marks the name down notches for it, which is a
    // different thing from pretending its numbers are worse than they are.
    let unproven = match s.age_periods < by.record_after {
        true => by.without_a_record,
        false => 0.0,
    };
    Grade::nearest((strain - by.best_carries) / by.per_notch + unproven)
}

/// It is sticky — a move happens only when the state has moved far enough to cross a notch, and that
/// is what makes a move meaningful and what makes it LATE.
pub fn reassess(held: &Rating, now: &State, by: &Scale, period: u32) -> Option<Rating> {
    let grade = grade_from(now, by);
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
///
pub fn was_wrong(r: &Rating, actually_failed: bool) -> bool {
    actually_failed && r.grade.investment_grade()
}

/// The distribution of ratings across issuers is a READ of their states, never a target distribution
/// the issuers were fitted to.
pub fn distribution(states: &[State], by: &Scale) -> Vec<(Grade, usize)> {
    (0..Grade::NOTCHES)
        .filter_map(|notch| Grade::at_rank(notch as f64))
        .map(|g| (g, states.iter().filter(|s| grade_from(s, by) == g).count()))
        .collect()
}


/// EVERY HOUSE GRADES EVERY NAME IT CAN READ, AND THEY DISAGREE.
pub struct Grading {
    /// The event kind a rating action is published under.
    pub kind: u32,
    /// §48's published accounts, which is what the coverage and the trend are read from.
    pub accounts: u32,
    pub at_income: u32,
    /// The house's own scale: where its top sits, what a notch of it is worth, and what it makes of
    /// a name it has no record for.
    pub best_carries: &'static str,
    pub per_notch: &'static str,
    pub without_a_record: &'static str,
    pub record_after: &'static str,
}

impl Mechanism for Grading {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let houses: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::ASSESSOR)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if houses.is_empty() {
            return;
        }
        // What each name last published, and what it published before that — the trend.
        let mut last: std::collections::HashMap<u32, (f64, Option<f64>)> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.accounts) {
            if let (Some(&who), Some(Value::Num(income))) =
                (ctx.journal().subjects_of(row).first(), ctx.journal().says(row, self.at_income))
            {
                let was = last.get(&who).map(|(now, _)| *now);
                last.insert(who, (income, was));
            }
        }

        let scale = Scale {
            best_carries: ctx.params().ratio(self.best_carries),
            per_notch: ctx.params().ratio(self.per_notch),
            without_a_record: ctx.params().count(self.without_a_record),
            record_after: ctx.params().periods(self.record_after) as u32,
        };
        let mut actions: Vec<(PartyId, PartyId, crate::stores::Grade)> = Vec::new();
        for (&who, &(income, before)) in &last {
            let of = PartyId(who);
            if !ctx.parties().alive(of) {
                continue;
            }
            // Leverage is what it owes against what it holds.
            let owes: f64 = ctx
                .instruments()
                .of_issuer(of)
                .iter()
                .map(|i| ctx.schedules().outstanding(InstrumentId::at(*i)))
                .sum();
            let Some(holds) = booked_equity(of, ctx.register(), ctx.instruments(), ctx.prints(), ctx.claims(), ctx.period()) else { continue };
            let state = State {
                leverage: owes / holds,
                // Coverage is what it earns against what it owes.
                coverage: if owes > 0.0 { income / owes } else { f64::INFINITY },
                cash: ctx.register().quantity(
                    ctx.register().row(of, match crate::ledger::account_of(ctx.parties(), ctx.instruments(), of) {
                        Some(cash) => cash,
                        None => continue,
                    }),
                ),
                size: holds,
                age_periods: ctx.parties().age(of, ctx.period()),
                // And the TREND — this year's published income against last year's.
                trend: match before {
                    Some(was) if was != 0.0 => (income - was) / was.abs(),
                    _ => 0.0,
                },
            };
            let grade = grade_from(&state, &scale);
            for &by in &houses {
                actions.push((by, of, grade));
            }
        }

        for (by, of, grade) in actions {
            // It is STICKY.
            let held = ctx
                .standing()
                .of_party_about(by, of, crate::stores::standing::GRADE)
                .map(|s| ctx.standing().terms(s)[0]);
            if matches!(held, Some(rank) if rank == grade.rank()) {
                continue;
            }
            // The probability of failing and, separately, loss given failure are the house's own
            // adaptive reads of realised borrower performance. Until both histories exist they are
            // absent, never authoritative zeroes hidden behind a grade.
            let probability = ctx
                .outlooks()
                .of(by, crate::stores::about::repayment_of(of))
                .map(|repaid| 1.0 - repaid);
            let loss_given_failure = ctx
                .outlooks()
                .of(by, crate::stores::about::loss_given_failure_of(of));
            let mut terms = vec![grade.rank()];
            if let (Some(probability), Some(loss)) = (probability, loss_given_failure) {
                terms.extend([probability, loss]);
            }
            ctx.now_stands(
                crate::stores::standing::GRADE,
                by,
                of,
                terms,
            );
            ctx.say(self.kind, &[by.0, of.0], &[(0, Value::Num(grade.rank()))], true);
        }
    }
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

    fn scale() -> Scale {
        Scale { best_carries: 0.25, per_notch: 0.25, without_a_record: 3.0, record_after: 8 }
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
        assert!(grade_from(&strong, &scale()) < grade_from(&weak, &scale()));
    }

    #[test]
    fn the_scale_is_the_markets_own_and_a_name_off_the_end_of_it_is_still_on_the_scale() {
        // Twenty-two rungs, named as a market names them, and the position IS the rank both ways.
        for notch in 0..Grade::NOTCHES {
            let g = Grade::at_rank(notch as f64).unwrap();
            assert_eq!(g.rank(), notch as f64);
        }
        assert_eq!(Grade::BEST.shown(), "AAA");
        assert_eq!(Grade::WORST.shown(), "D");
        assert_eq!(Grade::LOWEST_INVESTMENT_GRADE.shown(), "BBB-");
        assert!(Grade::BBBminus.investment_grade());
        assert!(!Grade::BBplus.investment_grade());
        // A credit better than the best grade is still the best grade, and one worse than the worst
        // is still the worst: there are no further rungs to name it with.
        assert_eq!(Grade::nearest(-4.0), Grade::BEST);
        assert_eq!(Grade::nearest(1_000.0), Grade::WORST);
        assert_eq!(Grade::nearest(f64::INFINITY), Grade::WORST);
    }

    #[test]
    fn age_and_trend_are_state_so_two_issuers_with_the_same_numbers_are_not_the_same_credit() {
        // Age is state, and the TREND in the observables is too.
        let established = state(1.0, 4.0, 0.1);
        let young = State { age_periods: 2, ..established };
        assert!(grade_from(&young, &scale()) > grade_from(&established, &scale()));
        let deteriorating = state(1.0, 4.0, -1.5);
        assert!(grade_from(&deteriorating, &scale()) > grade_from(&established, &scale()));
    }

    #[test]
    fn it_is_sticky_so_a_small_change_does_not_move_it_and_a_move_is_late() {
        // No rating changes for no reason, and the stickiness is what makes a move meaningful.
        let held = rated(Grade::Aplus);
        let barely = state(3.0, 2.0, 0.1);
        assert_eq!(grade_from(&barely, &scale()), Grade::Aplus);
        assert!(reassess(&held, &barely, &scale(), 10).is_none());
        let much_worse = state(9.0, 2.0, -0.5);
        let moved = reassess(&held, &much_worse, &scale(), 10).unwrap();
        assert!(moved.grade > Grade::Aplus);
        assert_eq!(moved.since_period, 10);
    }

    #[test]
    fn a_downgrade_forces_every_bound_holder_to_sell_on_the_same_date() {
        // A real, dated, mechanical flow — and a rating no rule refers to is decoration.
        // The boundary a mandate is written against is BBB-, which a seven-label scale cannot say.
        let mandates = [
            Mandate { holder: party(30), lowest_allowed: Grade::LOWEST_INVESTMENT_GRADE, holds: 5_000.0 },
            Mandate { holder: party(31), lowest_allowed: Grade::BBminus, holds: 2_000.0 },
            Mandate { holder: party(32), lowest_allowed: Grade::CCC, holds: 800.0 },
        ];
        let d = downgrade(Grade::Bminus, &mandates, 10_000.0, 0.04, 0.15, 6_000.0, &[Grade::BBminus]);
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
        let first = grade_from(&before, &scale());
        let after = worsened_by(&before, 2.0);
        let second = grade_from(&after, &scale());
        assert!(second > first);
        // And a second round is worse again: the loop runs.
        let third = grade_from(&worsened_by(&after, 1.0), &scale());
        assert!(third >= second);
    }

    #[test]
    fn improvement_widens_the_buyer_base() {
        // It works the other way too.
        let mandates = [
            Mandate { holder: party(30), lowest_allowed: Grade::LOWEST_INVESTMENT_GRADE, holds: 5_000.0 },
            Mandate { holder: party(31), lowest_allowed: Grade::BBminus, holds: 2_000.0 },
            Mandate { holder: party(32), lowest_allowed: Grade::CCC, holds: 800.0 },
        ];
        assert_eq!(buyer_base(Grade::Bminus, &mandates), 1);
        assert_eq!(buyer_base(Grade::AA, &mandates), 3);
    }

    #[test]
    fn a_rated_safe_issuer_can_fail() {
        // No assessment that is always right. Being wrong is failing while rated somewhere a
        // mandate would have let its holder buy.
        assert!(was_wrong(&rated(Grade::BEST), true));
        assert!(!was_wrong(&rated(Grade::BEST), false));
        assert!(was_wrong(&rated(Grade::LOWEST_INVESTMENT_GRADE), true));
        assert!(!was_wrong(&rated(Grade::CCC), true));
    }

    #[test]
    fn an_instruments_rating_differs_from_its_issuers_and_both_exist() {
        // The probability of failing to perform, and SEPARATELY the loss given that failure, which
        // depends on seniority and security.
        let issuer = rated(Grade::A);
        let subordinated = Rating {
            instrument: Some(InstrumentId::at(5)),
            grade: Grade::BBB,
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
        let d = distribution(&states, &scale());
        let counted: usize = d.iter().map(|(_, n)| n).sum();
        assert_eq!(counted, 3);
        assert_eq!(d.len(), Grade::NOTCHES);
        assert!(d.iter().any(|(g, n)| *g == Grade::BEST && *n == 1));
    }

    #[test]
    fn a_rating_is_an_opinion_held_by_a_named_assessor_and_is_not_the_only_one() {
        // One universal rating held by nobody means every participant agrees about credit by
        // construction.
        let one = rated(Grade::A);
        let another = Rating { by: party(81), grade: Grade::BBB, ..one };
        assert_ne!(one.by, another.by);
        assert_ne!(one.grade, another.grade);
    }
}
