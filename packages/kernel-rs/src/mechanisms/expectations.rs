//! WHAT A PARTY EXPECTS, formed from its own history and nobody else's.
//!
//! @spec 46 A1, A2, A2.a, A2.b, A3, A4, A5, B1, B1.a, B1.b, B2, B2.a, B3, B4, B5 · XI-16 · Law 2, Law 8, Law 17
//!
//! Expectations are heterogeneous and the heterogeneity is LOAD-BEARING. Different
//! histories make different outlooks, and different outlooks are the different reasons a market
//! needs to have two sides. *A world in which every party expected the same thing would trade once
//! and stop.* That is why the outlook is per party and there is no door onto anybody else's.
//!
//! Three refusals are structural here rather than enforced:
//!
//! - A2.b, no global expectation. There is no *the* expected inflation rate in this world. An
//!  `Outlook` belongs to one party and one subject; nothing aggregates them and there is nowhere a
//!  decision could consult an aggregate if something did.
//! - A4, no model forecast. An outlook is formed from the PAST by the party. `observe` takes
//!  what a party saw and the period it saw it in, and B4's "before the period it is used in" is
//!  a refusal at the call: a party that could read the period's own result before acting would
//!  be a party with no expectation at all.
//! - B1.b, no second primitive. Memory is the one PREFERENCE. Confidence, sentiment, optimism,
//!  an anchor, a target: each is either a read of the surprises or a number nobody could derive,
//!  and the second kind is Law 2's defect. `confidence` is a READ and there is no setter.

/// An expectation carries its unit and its periodicity. What is expected is named by the
/// subject; how wide the horizon is, is part of the number.
use crate::ids::PartyId;
use crate::ledger::Leg;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::about;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum About {
    /// What it will be paid, per period.
    IncomePerPeriod,
    /// What a named line will fetch, per piece.
    PricePerPiece(u32),
    /// What it will pay to borrow, per annum.
    RatePerAnnum,
    /// What it will sell, per period.
    DemandPerPeriod,
}

/// Observed minus expected, per party, per variable, per period. It is the only thing that
/// changes an outlook, and it is a real event rather than an intermediate value.
#[derive(Clone, Copy, Debug)]
pub struct Surprise {
    pub about: About,
    pub period: u32,
    pub expected: f64,
    pub observed: f64,
}

impl Surprise {
    pub fn size(&self) -> f64 {
        self.observed - self.expected
    }
}

/// One party's outlook on one subject. It is the party's own: there is no constructor that
/// takes another party's history and no read that reaches one.
pub struct Outlook {
    about: About,
    /// How many of its OWN periods it weighs. The one preference this system admits, drawn
    /// once at entry and dispersed across parties — a sector whose members all remembered the same
    /// way would move as one.
    memory: f64,
    /// What it expects. Missing until it has seen something: a party that has observed nothing has
    /// no expectation, which is an absence and not a zero.
    expects: Option<f64>,
    /// Its own surprises, in order. The history IS the record — confidence is read off it and
    /// nothing stores a confidence.
    surprises: Vec<Surprise>,
    /// The last period it observed, so an outlook cannot be fed the period it is used in.
    through: Option<u32>,
}

impl Outlook {
    /// Memory is drawn once at entry and is the only preference here. A memory of no periods
    /// is not a memory — a party that weighed nothing would have no outlook to correct.
    pub fn new(about: About, memory: f64) -> Self {
        assert!(
            memory >= 1.0 && memory.is_finite(),
            "§46 B1.a: a memory of {memory} periods is not a memory"
        );
        Self { about, memory, expects: None, surprises: Vec::new(), through: None }
    }

    pub fn about(&self) -> About {
        self.about
    }

    /// What the decider sees. Missing until it has observed something — there is no default
    /// expectation, because a default would be an expectation nobody formed.
    pub fn expects(&self) -> Option<f64> {
        self.expects
    }

    /// Adaptively, from its own history — the last outlook corrected towards what actually
    /// happened, at the party's OWN speed. The first thing it ever sees IS its outlook: there is
    /// nothing to correct from, and inventing a prior would be a number nobody could derive.
    ///
    /// `at` must be a period it has already lived through. A party that could read the
    /// period's own result before acting would be a party with no expectation at all, so being
    /// handed one is a contract violation and not a value to use.
    pub fn observe(&mut self, observed: f64, at: u32, acting_in: u32) {
        assert!(
            at < acting_in,
            "§46 B4: an outlook acting in period {acting_in} was handed period {at}'s own result"
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
                self.surprises.push(Surprise { about: self.about, period: at, expected, observed });
                // Corrected towards what happened, at its own speed. A long memory moves
                // slowly, which is B5: expectations lag turning points and the lag differs by
                // party with its memory.
                self.expects = Some(expected + (observed - expected) / self.memory);
            }
        }
    }

    /// Its own surprises. The record a VERIFY reads (B2.a: an outlook that moved with no
    /// surprise behind it moved for a reason nobody stated).
    pub fn surprises(&self) -> &[Surprise] {
        &self.surprises
    }

    /// CONFIDENCE IS A READ — how wide its own recent surprises have been. A party
    /// surprised often and widely does not trust its own outlook and acts on that: it holds more
    /// liquidity, spends less of an expected windfall, bids more cautiously.
    ///
    /// It is never stated, never drawn and never an input, which is why this is a function and
    /// there is no field behind it. A party with no surprises yet has no width to read — absence,
    /// not certainty.
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

    /// The falsification test. An outlook that moved with no surprise behind it
    /// moved for a reason nobody stated. This is what a VERIFY reads, and it MEASURES rather than
    /// enforcing: it reports, and never corrects the outlook it is about.
    pub fn moved_without_a_surprise(&self, was: Option<f64>) -> bool {
        match (was, self.expects) {
            (Some(before), Some(now)) => before != now && self.surprises.is_empty(),
            _ => false,
        }
    }
}

// §46 RUNS HERE. `Forming` was in `running.rs` and this file was three types and no
// function, so the module every party's outlook comes from contained no outlook-forming.

/// EVERY DECIDING PARTY FORMS ITS OWN OUTLOOK FROM ITS OWN HISTORY.
///
/// One PREFERENCE — how much weight it gives the surprise — and no global expectation anywhere. What
/// it forms its outlook ABOUT is what it can see: the price its own lines last printed at.
///
/// Outlooks disagree because the parties see different things, which is what gives a market two
/// sides. A world where everybody expected the same would trade once and stop.
pub struct Forming {
    /// The memory — how much of the new observation displaces the old. The one primitive here.
    pub memory: &'static str,
}

impl Mechanism for Forming {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let memory = ctx.params().ratio(self.memory);
        assert!(memory > 0.0 && memory <= 1.0, "§46: a memory outside its own range is not one");
        let mut formed: Vec<(PartyId, u32, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // It looks at ITS OWN rows and the prints those lines actually made. A party
            // that holds nothing has seen nothing and forms nothing — which is not an outlook of
            // zero.
            let mut seen = 0.0;
            let mut lines = 0.0;
            for row in ctx.register().of_holder(who) {
                let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                if let Some(print) = ctx.prints().latest(line, ctx.period()) {
                    seen += print.price;
                    lines += 1.0;
                }
            }
            if lines <= 0.0 {
                continue;
            }
            let now = seen / lines;
            let was = ctx.outlooks().of(who, about::WHAT_IT_SELLS_FOR);
            // Adaptive. The first observation IS the outlook; after that the surprise moves
            // it by the party's own memory.
            let level = match was {
                Some(old) => old + memory * (now - old),
                None => now,
            };
            formed.push((who, about::WHAT_IT_SELLS_FOR, level));
        }

        // 37 B1, §46: and how much it expects to sell, which is a different fact from the price
        // and is the first reason the production decision has. It is formed the same adaptive way,
        // from the one source that is not an inference: what it actually DELIVERED last period, read
        // off the wire. A firm that has never delivered has no view of its demand
        // and forms none — which is missing, not a demand of zero.
        let mut delivered: Vec<(PartyId, f64)> = Vec::new();
        for n in ctx.wire().in_period(ctx.period()) {
            for leg in ctx.wire().legs_of(n) {
                if let Leg::Asset { from, qty, .. } = *leg {
                    match delivered.iter_mut().find(|(who, _)| *who == from) {
                        Some((_, units)) => *units += qty,
                        None => delivered.push((from, qty)),
                    }
                }
            }
        }
        for (who, units) in delivered {
            if !ctx.parties().alive(who) {
                continue;
            }
            let level = match ctx.outlooks().of(who, about::HOW_MUCH_IT_SELLS) {
                Some(old) => old + memory * (units - old),
                None => units,
            };
            formed.push((who, about::HOW_MUCH_IT_SELLS, level));
        }

        for (who, subject, level) in formed {
            ctx.form(who, subject, level);
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
        // The first thing it sees IS its outlook: there is nothing to correct from, and inventing
        // a prior would be a number nobody could derive.
        o.observe(100.0, 1, 2);
        assert_eq!(o.expects(), Some(100.0));
        assert!(o.surprises().is_empty(), "the first observation surprised nobody");
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
        assert!(p < 140.0 && j < 140.0, "neither saw the turn in the period it happened");
    }

    #[test]
    #[should_panic(expected = "was handed period")]
    fn an_outlook_cannot_read_the_period_it_is_used_in() {
        // A party that could read the period's own result before acting would be a party with
        // no expectation at all. It is refused at the call rather than trusted not to.
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
        assert_eq!(o.surprises()[0].period, 2);
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
