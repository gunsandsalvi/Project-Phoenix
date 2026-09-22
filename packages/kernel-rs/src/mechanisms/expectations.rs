//! WHAT A PARTY EXPECTS, formed from its own history and nobody else's.
//!
//! @spec 46 A1, A2, A2.a, A2.b, A3, A4, A5, B1, B1.a, B1.b, B2, B2.a, B3, B4, B5 · XI-16 · Law 2, Law 8, Law 17

/// An expectation carries its unit and its payment frequency.
use crate::ids::PartyId;
use crate::instruments::Class;
use crate::journal::Value;
use crate::ledger::Leg;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::about;

// §46 RUNS HERE.

/// EVERY DECIDING PARTY FORMS ITS OWN OUTLOOK FROM ITS OWN HISTORY.
pub struct Forming {
    pub firm_result: u32,
    pub at_cash: u32,
}

impl Mechanism for Forming {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut observations: Vec<(PartyId, u32, f64)> = Vec::new();
        let prior = ctx.week().saturating_sub(1);
        for &row in ctx.journal().of_kind(self.firm_result) {
            if ctx.journal().period_of(row) != prior {
                continue;
            }
            if let (Some(&who), Some(Value::Num(cash))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, self.at_cash),
            ) {
                let firm = PartyId::at(who);
                if ctx.parties().alive(firm) {
                    observations.push((firm, about::WHAT_IT_KEEPS_EARNING, cash));
                }
            }
        }
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Each observation keeps its own unit and subject. A price of wheat, a share and a bond
            // are three outlooks, never three operands of a mean.
            for row in ctx.register().of_holder(who) {
                let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                if let Some(print) = ctx.print_here(line, ctx.parties().region_of(who)) {
                    let subject = about::price_of(line);
                    observations.push((who, subject, print.price));
                }
            }
            // The contractual rate on this party's issued credit is its observable cost of credit.
            let coupons: Vec<f64> = ctx
                .instruments()
                .of_issuer(who)
                .iter()
                .map(|line| crate::ids::InstrumentId::at(*line))
                .filter(|line| ctx.instruments().class_of(*line) == Class::Claim)
                .filter_map(|line| ctx.instruments().coupon_of(line))
                .collect();
            if !coupons.is_empty() {
                observations.push((
                    who,
                    about::WHAT_CREDIT_COSTS,
                    coupons.iter().sum::<f64>() / coupons.len() as f64,
                ));
            }

            // Repayment is the realised share of this borrower's matured obligations, not a global
            // default-rate input. A party with no matured obligations has no observation.
            let matured: Vec<crate::stores::DueId> = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|row| crate::stores::DueId(*row))
                .filter(|due| ctx.schedules().due(*due) < ctx.today())
                .collect();
            let due: f64 = matured.iter().map(|row| ctx.schedules().amount(*row)).sum();
            if due > 0.0 {
                let paid: f64 = matured
                    .iter()
                    .map(|row| ctx.schedules().recovered(*row))
                    .sum();
                observations.push((who, about::WHETHER_IT_IS_PAID_BACK, paid / due));
                let failed: Vec<crate::stores::DueId> = matured
                    .iter()
                    .copied()
                    .filter(|row| {
                        matches!(
                            ctx.schedules().state(*row),
                            crate::stores::DueState::Failed { .. }
                        )
                    })
                    .collect();
                let failed_due: f64 = failed.iter().map(|row| ctx.schedules().amount(*row)).sum();
                let loss_given_failure = if failed_due > 0.0 {
                    Some(
                        failed
                            .iter()
                            .map(|row| {
                                ctx.schedules().amount(*row) - ctx.schedules().recovered(*row)
                            })
                            .sum::<f64>()
                            / failed_due,
                    )
                } else {
                    None
                };
                for assessor in ctx.parties().of_kind(crate::assembly::kinds::ASSESSOR) {
                    let house = PartyId::at(*assessor);
                    observations.push((house, about::repayment_of(who), paid / due));
                    if let Some(loss) = loss_given_failure {
                        observations.push((house, about::loss_given_failure_of(who), loss));
                    }
                }
            }
        }

        // 37 B1, §46: and how much it expects to sell, which is a different fact from the price and
        // is the first reason the production decision has.
        let mut delivered: Vec<(PartyId, f64)> = Vec::new();
        for n in ctx.wire().in_period(ctx.week()) {
            for leg in ctx.wire().legs_of(n) {
                if let Leg::Asset { from, qty, .. } = *leg {
                    match delivered.iter_mut().find(|(who, _)| *who == from) {
                        Some((_, units)) => *units += qty.get(),
                        None => delivered.push((from, qty.get())),
                    }
                }
            }
        }
        for (who, units) in delivered {
            if !ctx.parties().alive(who) {
                continue;
            }
            observations.push((who, about::HOW_MUCH_IT_SELLS, units));
        }

        for (who, subject, observed) in observations {
            ctx.observe(who, subject, observed);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::stores::{corrected, moved_without_a_surprise, ForecastError};

    fn error(expected: f64, observed: f64) -> ForecastError {
        ForecastError {
            party: crate::ids::PartyId(1),
            about: crate::stores::about::WHAT_IT_KEEPS_EARNING,
            week: 1,
            expected,
            observed,
        }
    }

    #[test]
    fn different_memories_make_different_outlooks_and_that_is_the_point() {
        // A world in which every party expected the same thing would trade once and stop.
        let (mut patient, mut jumpy) = (100.0, 100.0);
        for seen in [100.0, 140.0] {
            patient = corrected(patient, seen, 10.0);
            jumpy = corrected(jumpy, seen, 2.0);
        }
        assert!(jumpy > patient, "the shorter memory moved further");
        // Both lag the turn, and the longer memory lags more.
        assert!(patient < 140.0 && jumpy < 140.0);
    }

    #[test]
    fn a_surprise_is_observed_less_expected_and_it_is_what_moves_the_outlook() {
        let s = error(50.0, 90.0);
        assert_eq!(s.size(), 40.0);
        assert_ne!(corrected(s.expected, s.observed, 2.0), s.expected);
    }

    #[test]
    fn the_falsification_test_is_a_move_with_nothing_recorded_behind_it() {
        assert!(moved_without_a_surprise(Some(50.0), Some(70.0), 0));
        assert!(!moved_without_a_surprise(Some(50.0), Some(70.0), 1));
        assert!(!moved_without_a_surprise(Some(50.0), Some(50.0), 0));
        // A party that has observed nothing has NO expectation, so nothing moved.
        assert!(!moved_without_a_surprise(None, Some(70.0), 0));
    }

    #[test]
    fn confidence_is_a_read_of_its_own_surprises_and_never_an_input() {
        let width = |past: &[ForecastError]| {
            let sizes: Vec<f64> = past.iter().map(|s| s.size().abs()).collect();
            crate::num::mean(&sizes)
        };
        // A party with no surprises yet has no width to read: absence, not certainty.
        assert!(width(&[]).is_none());
        let steady = [error(100.0, 100.0), error(100.0, 100.0)];
        let battered = [error(100.0, 180.0), error(100.0, 20.0)];
        assert_eq!(width(&steady), Some(0.0), "nothing ever surprised it");
        assert!(width(&battered) > width(&steady));
    }

    #[test]
    #[should_panic(expected = "is not a memory")]
    fn memory_is_the_one_preference_and_a_memory_of_nothing_is_not_one() {
        corrected(100.0, 120.0, 0.0);
    }
}
