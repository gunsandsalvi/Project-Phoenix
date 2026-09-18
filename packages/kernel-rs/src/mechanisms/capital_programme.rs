//! The capital programme's audit contribution: PLANT MOVES ONLY FOR A REASON.
//!
//! @spec Capital Programme A6.b, Capital Programme D1, Law 5, Law 19
//!
//! Two independent records are compared: the register's own walk over the holdings, and the LEGS
//! that said why anything moved. A stock that moved without a leg has nowhere to hide, which is
//! what makes commissioning, wearing out and a sale from an estate the only ways plant can appear
//! or go. It MEASURES and never repairs — a residual plugged here would be exactly the residual
//! with no holder Appendix B forbids, and it would hide the missing leg rather than report it.
//!
//! **This is the first module ported (0g.42), and it is the one the projection rests on.** In
//! TypeScript it is **2,049 ms of self time — 3.75% of a period and 95% of its own module** — over
//! 497,338 legs, 1,034,257 distinct (party, instrument) keys and 21,490 holdings on 479 capital
//! lines. 0g.40 measured a standalone port at 10.9× translating; this is the same family inside
//! the real kernel, sharing the audit's one traversal.
//!
//! The key is a PACKED PAIR, not a built string. The TypeScript family builds `${party}|${instrument}`
//! and hashes it once per leg and once per holding — a million built-and-hashed strings a period,
//! and a million `Qty[]` arrays behind them.

use crate::audit::{Contribution, Family, Sources, Violation, Visit};
use crate::ids::{InstrumentId, PartyId};
use crate::ledger::Leg;
use std::collections::HashMap;

/// Appendix A: MISSING IS MISSING — and these two are not missing, they are NOTHING, which is an
/// answer. A holding that was not on the register last period held none of the line; a holding no
/// leg mentioned had nothing accounted for. Both are named here rather than written as a default
/// at the site, because the difference between "nobody said" and "the answer is nothing" is the
/// whole of the rule (`zeroIfNone` is the same read in the TypeScript engine).
#[inline]
fn held_nothing_then(before: &HashMap<u64, f64>, k: u64) -> f64 {
    match before.get(&k) {
        Some(&q) => q,
        None => 0.0,
    }
}

/// And what the legs accounted for where none of them named this holding: nothing, over no terms.
#[inline]
fn legs_said_nothing(moved: &HashMap<u64, (f64, f64, u32)>, k: u64) -> (f64, f64, u32) {
    match moved.get(&k) {
        Some(&seen) => seen,
        None => (0.0, 0.0, 0),
    }
}

#[inline]
const fn key(party: PartyId, instrument: InstrumentId) -> u64 {

    ((party.0 as u64) << 32) | (instrument.0 as u64)
}

/// A6.b: what the legs say moved, per holding, kept as TERMS rather than a running total — Law 7's
/// dust is derived from the terms and a total alone cannot produce it.
#[derive(Default)]
pub struct PlantMoves {
    /// Which lines are capital. A fact about the INSTRUMENTS, declared by the module that owns
    /// them, and never a branch on a kind inside the check (Law 15).
    capital: Vec<bool>,
    /// What the legs accounted for, this period.
    moved: HashMap<u64, (f64, f64, u32)>,
    /// What the register holds now, for the lines that are capital.
    held: HashMap<u64, f64>,
    /// And what it held last period, which is what a change is measured against.
    before: HashMap<u64, f64>,
    /// Whether the two periods are consecutive; without that there is nothing to compare.
    comparable: bool,
    last_period: Option<u32>,
    found: Vec<Violation>,
}

impl PlantMoves {
    /// The lines this module says are capital, by row. It is DATA in a registry, handed in, so the
    /// check never asks an instrument what kind it is (Law 15).
    pub fn over(capital: Vec<bool>) -> Self {
        Self { capital, ..Default::default() }
    }

    #[inline]
    fn is_capital(&self, i: InstrumentId) -> bool {
        matches!(self.capital.get(i.row()), Some(true))
    }

    #[inline]
    fn account(&mut self, party: PartyId, instrument: InstrumentId, qty: f64) {
        if !self.is_capital(instrument) {
            return;
        }
        let e = self.moved.entry(key(party, instrument)).or_insert((0.0, 0.0, 0));
        e.0 += qty;
        e.1 += qty.abs();
        e.2 += 1;
    }
}

impl Contribution for PlantMoves {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "capital-programme"
    }

    /// Its own pass over its own source: the legs of this period, and what each says moved.
    fn before(&mut self, from: &Sources<'_>) {
        self.moved.clear();
        self.comparable = self.last_period == from.period.checked_sub(1);
        self.before = std::mem::take(&mut self.held);
        for n in from.wire.in_period(from.period) {
            for leg in from.wire.legs_of(n) {
                match *leg {
                    // Goods B, E4: a thing coming into existence or leaving it. ONE side.
                    Leg::Create { party, instrument, qty, .. } => self.account(party, instrument, qty),
                    Leg::Destroy { party, instrument, qty, .. } => self.account(party, instrument, -qty),
                    // Law 5: and a move between two holders is two sides of one fact.
                    Leg::Asset { from: seller, to: buyer, instrument, qty, .. } => {
                        self.account(buyer, instrument, qty);
                        self.account(seller, instrument, -qty);
                    }
                    // Minting is money, and money is not a thing this family counts the units of.
                    Leg::Money { .. } | Leg::Mint { .. } | Leg::Pledge { .. } => {}
                }
            }
        }
    }

    /// The shared walk: what the register holds of every capital line, now.
    fn visit(&mut self, at: &Visit<'_>) {
        let instrument = at.register.instrument_of(at.row);
        if !self.is_capital(instrument) {
            return;
        }
        let holder = at.register.holder_of(at.row);
        self.held.insert(key(holder, instrument), at.register.quantity(at.row));
    }

    fn finish(&mut self, period: u32) -> Vec<Violation> {
        if self.comparable {
            // What moved, against what the legs say moved.
            for (&k, &now) in &self.held {
                let was = held_nothing_then(&self.before, k);
                let change = now - was;
                let (accounted, magnitude, terms) = legs_said_nothing(&self.moved, k);
                // Law 7: the dust of THIS comparison, from its own terms and magnitudes.
                let dust = (terms as f64 + 2.0)
                    * f64::EPSILON
                    * (magnitude + change.abs() + now.abs() + was.abs());
                if (change - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Capital Programme A6.b",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: change - accounted,
                        unit: "units",
                        period,
                        message: format!(
                            "plant moved by {change} and its legs account for {accounted}"
                        ),
                    });
                }
            }
            // A holding that went to NOTHING still has to have a leg behind it.
            for (&k, &was) in &self.before {
                if self.held.contains_key(&k) {
                    continue;
                }
                let (accounted, magnitude, terms) = legs_said_nothing(&self.moved, k);
                let dust =
                    (terms as f64 + 2.0) * f64::EPSILON * (magnitude + was.abs());
                if (-was - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Capital Programme A6.b",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: -was - accounted,
                        unit: "units",
                        period,
                        message: format!("plant left at {was} and its legs account for {accounted}"),
                    });
                }
            }
        }
        self.last_period = Some(period);
        std::mem::take(&mut self.found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Audit;
    use crate::journal::Journal;
    use crate::ledger::{Cause, Instruction, Receipt, Settlement};
    use crate::register::Register;

    fn capital(lines: usize) -> Vec<bool> {
        vec![true; lines]
    }

    #[test]
    fn plant_that_moved_with_a_leg_behind_it_is_not_a_violation() {
        let mut reg = Register::new();
        let mut j = Journal::new();
        let ok = j.kinds.declare("instruction.settled");
        let no = j.kinds.declare("instruction.failed");
        let mut wire = Settlement::new();
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let plant = InstrumentId::at(1);
        reg.credit(a, plant, 100.0, 1.0, 0);

        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(capital(4))));
        // Period 1 establishes what is held; nothing is comparable yet.
        audit.run(&reg, &wire, 1);

        // Period 2: it sells 40, and the leg says so.
        let legs = [Leg::Asset { from: a, to: b, instrument: plant, qty: 40.0, price_per_unit: Some(2.0) }];
        wire.settle(&Instruction::free_of_payment(&legs, Cause::Trade), 2, &mut reg, &mut j, ok, no);
        let reports = audit.run(&reg, &wire, 2);
        assert!(
            reports[0].violations.is_empty(),
            "{:?}",
            reports[0].violations.iter().map(|v| v.message.clone()).collect::<Vec<_>>()
        );
        let _ = Receipt::Sale;
    }

    #[test]
    fn plant_that_moved_with_no_leg_behind_it_has_nowhere_to_hide() {
        let mut reg = Register::new();
        let wire = Settlement::new();
        let a = PartyId::at(0);
        let plant = InstrumentId::at(1);
        reg.credit(a, plant, 100.0, 1.0, 0);

        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(capital(4))));
        audit.run(&reg, &wire, 1);

        // Units appear with no instruction behind them — which is what this family exists to find.
        reg.credit(a, plant, 25.0, 1.0, 2);
        let reports = audit.run(&reg, &wire, 2);
        assert_eq!(reports[0].violations.len(), 1);
        assert_eq!(reports[0].violations[0].size, 25.0);
        assert_eq!(reports[0].violations[0].spec, "Capital Programme A6.b");
        // And it never repairs: the units are still there.
        assert_eq!(reg.quantity(reg.row(a, plant)), 125.0);
    }

    #[test]
    fn a_line_that_is_not_capital_is_not_this_familys_business() {
        let mut reg = Register::new();
        let wire = Settlement::new();
        let a = PartyId::at(0);
        let share = InstrumentId::at(2);
        // Only row 1 is capital; row 2 is not.
        let mut which = vec![false; 4];
        which[1] = true;
        reg.credit(a, share, 100.0, 1.0, 0);
        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(which)));
        audit.run(&reg, &wire, 1);
        reg.credit(a, share, 50.0, 1.0, 2);
        let reports = audit.run(&reg, &wire, 2);
        assert!(reports[0].violations.is_empty());
    }
}
