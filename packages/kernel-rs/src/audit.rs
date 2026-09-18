//! The audit: independent families, run every period, reporting owner + size + period + citation.
//!
//! **It never repairs.** A violation is a finding about a mechanism and never a licence to adjust
//! the number — a residual plugged here is exactly the residual with no holder Appendix B forbids,
//! and it would hide the missing leg rather than report it. A family nobody has built reports **not
//! built**, never green: an unbuilt check that said "no violations" is the most dangerous line in
//! an audit.
//!
//! **ONE TRAVERSAL FEEDS EVERY FAMILY (0g.34).** In TypeScript, `flows`, `accounts`, `currency`,
//! `units` and `ownership` each walked the register after the one before it did — 18,885,889 reads
//! a period, and a family that walks the holdings once costs exactly 544,104 of them. A family's
//! independence is about the SOURCE it reads (Audit C3), not about how many times the register is
//! visited: each one still derives its own answer from the register and the ledger, and none of
//! them may read another's total or a mechanism's running one.

use crate::ids::HoldingId;
use crate::ledger::Settlement;
use crate::register::Register;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    Money,
    Ownership,
    Prices,
    CrossMarket,
    Accounts,
    Names,
    Flows,
    ZeroSum,
    Units,
    Liveness,
}

/// A2: a violation names its owner, its size, its period and the clause it is about. A finding
/// with no size cannot be ranked and one with no owner cannot be chased.
#[derive(Clone, Debug)]
pub struct Violation {
    pub family: Family,
    pub spec: &'static str,
    pub owner: String,
    pub size: f64,
    pub unit: &'static str,
    pub period: u32,
    pub message: String,
}

/// What one contribution found, and whether it is BUILT. An unbuilt family is reported as unbuilt
/// and is never counted as passing.
pub struct Report {
    pub family: Family,
    pub built: bool,
    pub contributors: Vec<&'static str>,
    pub violations: Vec<Violation>,
}

/// What a family is given on the one pass: the row, and the stores to derive its own answer from.
pub struct Visit<'a> {
    pub row: HoldingId,
    pub register: &'a Register,
    pub period: u32,
}

/// What a family is given BEFORE the shared walk, for the sources that are not the register: the
/// wire's own history, and the period. A family that needs to know WHY something moved reads the
/// legs here — which is its own pass over its own source, and the independence Audit C3 is about.
pub struct Sources<'a> {
    pub wire: &'a Settlement,
    pub register: &'a Register,
    pub period: u32,
}


/// A contribution to a family. `visit` is called once per holding on the ONE traversal; `finish`
/// is where a family that needs the whole picture states what it found.
pub trait Contribution {
    fn family(&self) -> Family;
    fn contributor(&self) -> &'static str;
    /// Audit E2: a family nobody has built says so. It is never green by default.
    fn built(&self) -> bool {
        true
    }
    /// Before the shared walk: a family's own pass over the sources that are not the register.
    fn before(&mut self, _from: &Sources<'_>) {}
    fn visit(&mut self, _at: &Visit<'_>) {}
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct Audit {
    families: Vec<Box<dyn Contribution>>,
}

impl Audit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, c: Box<dyn Contribution>) {
        self.families.push(c);
    }

    /// Audit C1: every family, every period, off ONE walk of the register.
    pub fn run(&mut self, register: &Register, wire: &Settlement, period: u32) -> Vec<Report> {
        let from = Sources { wire, register, period };
        for f in self.families.iter_mut() {
            if f.built() {
                f.before(&from);
            }
        }
        for row in register.all() {

            let at = Visit { row, register, period };
            for f in self.families.iter_mut() {
                if f.built() {
                    f.visit(&at);
                }
            }
        }
        let mut reports: Vec<Report> = Vec::new();
        for f in self.families.iter_mut() {
            let built = f.built();
            let violations = if built { f.finish(period) } else { Vec::new() };
            match reports.iter_mut().find(|r| r.family == f.family()) {
                Some(r) => {
                    r.built = r.built && built;
                    r.contributors.push(f.contributor());
                    r.violations.extend(violations);
                }
                None => reports.push(Report {
                    family: f.family(),
                    built,
                    contributors: vec![f.contributor()],
                    violations,
                }),
            }
        }
        reports
    }
}

/// Audit B2, Law 19: the lots are summed and compared with the row's own quantity. It reads the
/// SOURCE — the lots — and never the total it is checking, which is what makes it a check and not
/// a restatement.
#[derive(Default)]
pub struct LotsAgainstQuantity {
    found: Vec<Violation>,
}

impl Contribution for LotsAgainstQuantity {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "kernel"
    }
    fn visit(&mut self, at: &Visit<'_>) {
        // Money D2: a money account is a TOTAL and has no lots to sum. Summing them would report
        // every account in the world, which is what the first end-to-end period did.
        if at.register.is_total(at.row) {
            return;
        }
        let lots = at.register.lots(at.row);

        let mut summed = 0.0;
        let mut magnitude = 0.0;
        for l in lots {
            summed += l.qty;
            magnitude += l.qty.abs();
        }
        let held = at.register.quantity(at.row);
        // Law 7: the dust of THIS walk, from its own terms and magnitudes. Never a percentage.
        let dust = (lots.len() as f64 + 2.0) * f64::EPSILON * (magnitude + held.abs());
        if (summed - held).abs() > dust {
            self.found.push(Violation {
                family: Family::Ownership,
                spec: "Register B2",
                owner: format!("holding {}", at.row.row()),
                size: summed - held,
                unit: "pieces",
                period: at.period,
                message: format!("lots sum to {summed} and the row holds {held}"),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// Money D2: a TOTAL account carries no lots. It is the other half of the check above — one of them
/// would be a rule with an exemption, and the two together are the rule.
#[derive(Default)]
pub struct ATotalCarriesNoLots {
    found: Vec<Violation>,
}

impl Contribution for ATotalCarriesNoLots {
    fn family(&self) -> Family {
        Family::Money
    }
    fn contributor(&self) -> &'static str {
        "kernel.totals"
    }
    fn visit(&mut self, at: &Visit<'_>) {
        if !at.register.is_total(at.row) {
            return;
        }
        let lots = at.register.lots(at.row);
        if !lots.is_empty() {
            self.found.push(Violation {
                family: Family::Money,
                spec: "Money D2",
                owner: format!("holding {}", at.row.row()),
                size: lots.len() as f64,
                unit: "lots",
                period: at.period,
                message: format!("a money account carries {} lots", lots.len()),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// Appendix B, Register C3: no unit is encumbered beyond what is held. A lien over more than
/// exists is collateral counted twice.
#[derive(Default)]
pub struct NoCollateralCountedTwice {
    found: Vec<Violation>,
}

impl Contribution for NoCollateralCountedTwice {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "kernel.liens"
    }
    fn visit(&mut self, at: &Visit<'_>) {
        let free = at.register.free(at.row);
        if free < 0.0 {
            self.found.push(Violation {
                family: Family::Ownership,
                spec: "Appendix B",
                owner: format!("holding {}", at.row.row()),
                size: free,
                unit: "pieces",
                period: at.period,
                message: format!("pledged {} beyond what is held", -free),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// A family that has not been built. It reports NOT BUILT and never green (Audit E2).
pub struct NotBuilt {
    pub family: Family,
    pub contributor: &'static str,
}

impl Contribution for NotBuilt {
    fn family(&self) -> Family {
        self.family
    }
    fn contributor(&self) -> &'static str {
        self.contributor
    }
    fn built(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{InstrumentId, PartyId};

    #[test]
    fn one_walk_feeds_every_family_and_each_derives_its_own_answer() {
        let mut reg = Register::new();
        for p in 0..50u32 {
            reg.credit(PartyId::at(p), InstrumentId::at(1), 10.0, 1.0, 1);
        }
        let mut audit = Audit::new();
        audit.add(Box::<LotsAgainstQuantity>::default());
        audit.add(Box::<NoCollateralCountedTwice>::default());
        let reports = audit.run(&reg, &Settlement::new(), 1);
        assert_eq!(reports.len(), 1, "both contribute to ownership");
        assert_eq!(reports[0].contributors.len(), 2);
        assert!(reports[0].built);
        assert!(reports[0].violations.is_empty());
    }

    #[test]
    fn a_violation_is_reported_with_its_owner_and_size_and_never_repaired() {
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(1);
        reg.credit(p, i, 10.0, 1.0, 1);
        // A lien over more than is held: collateral counted twice.
        reg.pledge(p, i, PartyId::at(1), 18.0);
        let before = reg.quantity(reg.row(p, i));
        let mut audit = Audit::new();
        audit.add(Box::<NoCollateralCountedTwice>::default());
        let reports = audit.run(&reg, &Settlement::new(), 4);
        assert_eq!(reports[0].violations.len(), 1);
        let v = &reports[0].violations[0];
        assert_eq!(v.size, -8.0);
        assert_eq!(v.period, 4);
        assert_eq!(v.spec, "Appendix B");
        // THE AUDIT NEVER REPAIRS: the world is exactly as it was.
        assert_eq!(reg.quantity(reg.row(p, i)), before);
        assert_eq!(reg.free(reg.row(p, i)), -8.0);
    }

    #[test]
    fn a_money_account_is_a_total_and_is_not_a_violation() {
        // Money D2: money is one of itself, so its account is a total with no lots. The family that
        // sums lots must not report it, and the family that checks totals must find it clean.
        let mut reg = Register::new();
        let cash = InstrumentId::at(0);
        for p in 0..100u32 {
            reg.money_delta(PartyId::at(p), cash, 1_000.0);
        }
        reg.credit(PartyId::at(0), InstrumentId::at(1), 5.0, 2.0, 1);
        let mut audit = Audit::new();
        audit.add(Box::<LotsAgainstQuantity>::default());
        audit.add(Box::<ATotalCarriesNoLots>::default());
        let reports = audit.run(&reg, &Settlement::new(), 1);
        let found: usize = reports.iter().map(|r| r.violations.len()).sum();
        assert_eq!(found, 0, "a money account is not a defect");
    }

    #[test]
    fn an_unbuilt_family_says_so_and_is_never_green() {

        let reg = Register::new();
        let mut audit = Audit::new();
        audit.add(Box::new(NotBuilt { family: Family::Liveness, contributor: "nobody" }));
        let reports = audit.run(&reg, &Settlement::new(), 1);
        assert_eq!(reports[0].family, Family::Liveness);
        assert!(!reports[0].built, "an unbuilt family is not green");
        assert!(reports[0].violations.is_empty());
    }
}
