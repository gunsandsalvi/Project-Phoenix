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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum About {
    /// What it will be paid, per week.
    IncomePerPeriod,
    /// What a named line will fetch, per piece.
    PricePerPiece(u32),
    /// What it will pay to borrow, per annum.
    RatePerAnnum,
    /// What it will sell, per week.
    DemandPerPeriod,
}

/// Observed minus expected, per party, per variable, per week.
#[derive(Clone, Copy, Debug)]
pub struct Surprise {
    pub about: About,
    pub week: u32,
    pub expected: f64,
    pub observed: f64,
}

impl Surprise {
    pub fn size(&self) -> f64 {
        self.observed - self.expected
    }
}

/// One party's outlook on one subject.
pub struct Outlook {
    about: About,
    /// How many of its OWN weeks it weighs.
    memory: f64,
    expects: Option<f64>,
    /// Its own surprises, in order.
    surprises: Vec<Surprise>,
    /// The last week it observed, so an outlook cannot be fed the week it is used in.
    through: Option<u32>,
}

impl Outlook {
    /// Memory is drawn once at entry and is the only preference here.
    pub fn new(about: About, memory: f64) -> Self {
        assert!(
            memory >= 1.0 && memory.is_finite(),
            "§46 B1.a: a memory of {memory} weeks is not a memory"
        );
        Self {
            about,
            memory,
            expects: None,
            surprises: Vec::new(),
            through: None,
        }
    }

    pub fn about(&self) -> About {
        self.about
    }

    /// What the decider sees.
    pub fn expects(&self) -> Option<f64> {
        self.expects
    }

    /// Adaptively, from its own history — the last outlook corrected towards what actually happened,
    /// at the party's OWN speed.
    pub fn observe(&mut self, observed: f64, at: u32, acting_in: u32) {
        assert!(
            at < acting_in,
            "§46 B4: an outlook acting in week {acting_in} was handed week {at}'s own result"
        );
        if let Some(last) = self.through {
            assert!(at >= last, "Law 10: an observation arrives out of order");
        }
        self.through = Some(at);
        match self.expects {
            None => {
                // Formed from what THIS party observed, and from nothing it did not.
                self.expects = Some(observed);
            }
            Some(expected) => {
                self.surprises.push(Surprise {
                    about: self.about,
                    week: at,
                    expected,
                    observed,
                });
                // Corrected towards what happened, at its own speed.
                self.expects = Some(expected + (observed - expected) / self.memory);
            }
        }
    }

    pub fn surprises(&self) -> &[Surprise] {
        &self.surprises
    }

    /// CONFIDENCE IS A READ — how wide its own recent surprises have been.
    pub fn confidence(&self) -> Option<f64> {
        let recent = self.memory as usize;
        let from = self.surprises.len().saturating_sub(recent);
        let seen = &self.surprises[from..];
        if seen.is_empty() {
            return None;
        }
        let mut width = 0.0;
        for s in seen {
            width += s.size().abs();
        }
        Some(width / seen.len() as f64)
    }

    /// The falsification test.
    pub fn moved_without_a_surprise(&self, was: Option<f64>) -> bool {
        match (was, self.expects) {
            (Some(before), Some(now)) => before != now && self.surprises.is_empty(),
            _ => false,
        }
    }
}

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
                if let Some(print) = ctx.prints().latest(line, ctx.week()) {
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
    use super::*;

    #[test]
    fn an_outlook_is_formed_from_what_this_party_saw_and_from_nothing_it_did_not() {
        let mut o = Outlook::new(About::IncomePerPeriod, 4.0);
        // A party that has observed nothing has NO expectation — not a zero.
        assert!(o.expects().is_none());
        // The first thing it sees IS its outlook: there is nothing to correct from, and inventing a
        // prior would be a number nobody could derive.
        o.observe(100.0, 1, 2);
        assert_eq!(o.expects(), Some(100.0));
        assert!(
            o.surprises().is_empty(),
            "the first observation surprised nobody"
        );
    }

    #[test]
    fn different_histories_make_different_outlooks_and_that_is_the_point() {
        // A world in which every party expected the same thing would trade once and stop.
        let mut patient = Outlook::new(About::PricePerPiece(7), 10.0);
        let mut jumpy = Outlook::new(About::PricePerPiece(7), 2.0);
        for (n, seen) in [100.0, 100.0, 140.0].iter().enumerate() {
            let at = n as u32 + 1;
            patient.observe(*seen, at, at + 1);
            jumpy.observe(*seen, at, at + 1);
        }
        let (p, j) = (patient.expects().unwrap(), jumpy.expects().unwrap());
        // Same history, different memories, DIFFERENT outlooks — which is the two sides of a book.
        assert!(j > p, "the shorter memory moved further: {j} against {p}");
        // Both lag the turn, and the longer memory lags more.
        assert!(
            p < 140.0 && j < 140.0,
            "neither saw the turn in the week it happened"
        );
    }

    #[test]
    #[should_panic(expected = "was handed week")]
    fn an_outlook_cannot_read_the_period_it_is_used_in() {
        // A party that could read the week's own result before acting would be a party with no
        // expectation at all.
        let mut o = Outlook::new(About::RatePerAnnum, 3.0);
        o.observe(0.05, 7, 7);
    }

    #[test]
    fn the_surprise_is_the_only_thing_that_changes_an_outlook_and_it_is_recorded() {
        let mut o = Outlook::new(About::DemandPerPeriod, 2.0);
        o.observe(50.0, 1, 2);
        let before = o.expects();
        o.observe(90.0, 2, 3);
        assert_eq!(o.surprises().len(), 1);
        assert_eq!(o.surprises()[0].size(), 40.0);
        assert_eq!(o.surprises()[0].week, 2);
        assert_ne!(o.expects(), before);
        // The falsification test — a move with no surprise behind it.
        assert!(!o.moved_without_a_surprise(before));
    }

    #[test]
    fn confidence_is_a_read_of_its_own_surprises_and_never_an_input() {
        let mut steady = Outlook::new(About::IncomePerPeriod, 3.0);
        let mut battered = Outlook::new(About::IncomePerPeriod, 3.0);
        // A party with no surprises yet has no width to read: absence, not certainty.
        assert!(steady.confidence().is_none());
        for n in 1..=4u32 {
            steady.observe(100.0, n, n + 1);
            battered.observe(if n.is_multiple_of(2) { 20.0 } else { 180.0 }, n, n + 1);
        }
        let (s, b) = (steady.confidence().unwrap(), battered.confidence().unwrap());
        // The one surprised often and widely does not trust its own outlook.
        assert!(b > s, "battered {b} should be wider than steady {s}");
        assert_eq!(s, 0.0, "nothing ever surprised it");
    }

    #[test]
    #[should_panic(expected = "is not a memory")]
    fn memory_is_the_one_preference_and_a_memory_of_nothing_is_not_one() {
        Outlook::new(About::IncomePerPeriod, 0.0);
    }
}
