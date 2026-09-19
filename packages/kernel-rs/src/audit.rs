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

use crate::ids::{HoldingId, InstrumentId};
use crate::instruments::Instruments;
use crate::ledger::Settlement;
use crate::parties::Parties;
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

impl Family {
    /// **Audit E2, 22e.2: every family this audit is accountable for.** A world that assembled no
    /// contribution to a family must not read as a world with no violations in it, and the only way
    /// to know a family is missing is to have the list of them — so the list is here, beside the
    /// enum, and `Audit::over` fills the gaps with `NotBuilt` rather than leaving them silent.
    ///
    /// The lie this exists to stop was measured: four of nine families were tautologies until item 4
    /// and were reported green for thirteen `done` rows, so an "audit green" in the record from
    /// before item 4 is not evidence (21.134.D15).
    pub const ALL: [Family; 10] = [
        Family::Money,
        Family::Ownership,
        Family::Prices,
        Family::CrossMarket,
        Family::Accounts,
        Family::Names,
        Family::Flows,
        Family::ZeroSum,
        Family::Units,
        Family::Liveness,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Money => "money",
            Family::Ownership => "ownership",
            Family::Prices => "prices",
            Family::CrossMarket => "cross-market",
            Family::Accounts => "accounts",
            Family::Names => "names",
            Family::Flows => "flows",
            Family::ZeroSum => "zero-sum",
            Family::Units => "units",
            Family::Liveness => "liveness",
        }
    }
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
#[derive(Debug)]
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
    pub instruments: &'a Instruments,
    pub parties: &'a Parties,
    pub period: u32,
}

/// What a family is given BEFORE the shared walk, for the sources that are not one holding: the
/// wire's own history, the instruments, the parties. A family that needs to know WHY something
/// moved reads the legs here — which is its own pass over its own source, and the independence
/// Audit C3 is about.
pub struct Sources<'a> {
    pub wire: &'a Settlement,
    pub register: &'a Register,
    pub instruments: &'a Instruments,
    pub parties: &'a Parties,
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

    /// **Audit E2, 22e.2: THE ONLY WAY TO BUILD AN AUDIT OF A WORLD.** It takes what the kernel and
    /// the modules contributed and declares `NotBuilt` for every family nobody contributed to — so
    /// an unbuilt family is IN the report saying it is unbuilt, and cannot be absent from it.
    ///
    /// Leaving the gaps silent is the lie: a reader counting violations over the families that
    /// happened to be assembled would read a world with one family built and nine missing as a world
    /// with no violations. Filling them here makes that unsayable rather than forbidden.
    pub fn over(contributions: Vec<Box<dyn Contribution>>) -> Audit {
        let mut audit = Audit::new();
        for c in contributions {
            audit.add(c);
        }
        for family in Family::ALL {
            if audit.families.iter().any(|f| f.family() == family) {
                continue;
            }
            audit.add(Box::new(NotBuilt { family, contributor: "nobody" }));
        }
        audit
    }

    /// Audit C1: every family, every period, off ONE walk of the register.
    pub fn run(&mut self, from: &Sources<'_>) -> Vec<Report> {
        let (register, period) = (from.register, from.period);
        for f in self.families.iter_mut() {
            if f.built() {
                f.before(from);
            }
        }
        for row in register.all() {
            let at = Visit {
                row,
                register,
                instruments: from.instruments,
                parties: from.parties,
                period,
            };
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

// **`LotsAgainstQuantity` stood here and it could not fail** (0m.1, Audit A1.a: *a read of one thing
// against itself, which always passes*). It summed a row's lots and compared the total with
// `register.quantity(row)` — and for any row that is not `total_only`, `quantity()` IS
// `self.lots[at..at+len].iter().map(|l| l.qty).sum()`. The same lots, the same slice, the same
// order, the same f64 addition, so the difference was exactly `0.0` and the branch was unreachable.
//
// **It was a real check once, and 22e2 made it a tautology.** The quantity used to be a maintained
// total beside the lots, this family compared the two, and it found a genuine drift —
// `27.143341836734685` against `27.143341836734628`. The fix correctly deleted the total and left
// the family pointed at a `quantity()` that now re-derives from the lots, so the deletion did not
// name the read that replaced it (Law 19). **The read that replaces it is `quantity()` itself**:
// there is one number where there were two, and a drift between two copies cannot happen because
// there is no second copy. What Ownership needs instead is B2's own identity — the holders of a line
// against what the line says is issued — and that is what stands here now.

/// **Register B2, Bond N8.a: HOLDINGS SUM TO THE ISSUED AMOUNT, per instrument, always.**
///
/// The identity the family it replaces could not express, because until 0l there was no issued
/// amount to sum against — and `held_total` IS the sum of the holdings, so a check written against
/// it compares the answer with itself (Audit A1.a). **Two independent things that must agree**: the
/// holdings, walked here; and `Instruments::issued`, moved only by a named event on the wire.
///
/// B2.a says what each direction means, and the message says which: *a shortfall means somebody's
/// claim vanished; a surplus means somebody's was invented.*
///
/// **It will fire, in volume, and that is the point** (22g.1). Every unit this world opens with was
/// placed on the register directly rather than issued over the wire, so every seeded line is short
/// by exactly what was placed. A family that went green on a world like this one would be measuring
/// nothing.
#[derive(Default)]
pub struct HoldersAgainstIssued {
    /// B1's side, read from its own source once per period.
    issued: Vec<f64>,
    /// And the holdings' side, accumulated on the shared walk — with the terms and the magnitudes
    /// the dust is derived from, because Law 7 wants the error of THIS arithmetic and B2.b forbids
    /// a fraction of the issue.
    held: Vec<(f64, f64, usize)>,
    found: Vec<Violation>,
}

impl Contribution for HoldersAgainstIssued {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "kernel.issued"
    }

    fn before(&mut self, from: &Sources<'_>) {
        let lines = from.instruments.len();
        self.issued.clear();
        self.issued.extend((0..lines).map(|row| from.instruments.issued_of(InstrumentId(row as u32))));
        self.held.clear();
        self.held.resize(lines, (0.0, 0.0, 0));
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let line = at.register.instrument_of(at.row).row();
        let Some(side) = self.held.get_mut(line) else {
            // A holding of a line this store never issued is Register A4's, and the Names family is
            // what says so (0m.5). It is not this identity's to report as a shortfall.
            return;
        };
        let q = at.register.quantity(at.row);
        side.0 += q;
        side.1 += q.abs();
        side.2 += 1;
    }

    fn finish(&mut self, period: u32) -> Vec<Violation> {
        for (row, &(sum, magnitude, terms)) in self.held.iter().enumerate() {
            let issued = self.issued[row];
            let dust = (terms as f64 + 2.0) * f64::EPSILON * (magnitude + issued.abs());
            let off = sum - issued;
            if off.abs() <= dust {
                continue;
            }
            self.found.push(Violation {
                family: Family::Ownership,
                spec: "Register B2",
                owner: format!("instrument {row}"),
                size: off,
                unit: "units",
                period,
                message: if off < 0.0 {
                    format!("{sum} held against {issued} issued — a claim vanished")
                } else {
                    format!("{sum} held against {issued} issued — a claim was invented")
                },
            });
        }
        std::mem::take(&mut self.found)
    }
}

/// Money D2: a TOTAL account carries no lots. The other half of a rule whose first half is that a
/// row carrying lots answers from them — one of them alone would be a rule with an exemption.
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
    use crate::ids::PartyId;

    /// The stores an audit derives its answers from, gathered for a call — the same shape
    /// `Settling` has, and for the same reason.
    fn over<'a>(
        register: &'a Register,
        instruments: &'a Instruments,
        parties: &'a Parties,
        wire: &'a Settlement,
        period: u32,
    ) -> Sources<'a> {
        Sources { wire, register, instruments, parties, period }
    }

    #[test]
    fn one_walk_feeds_every_family_and_each_derives_its_own_answer() {
        let mut reg = Register::new();
        for p in 0..50u32 {
            reg.credit(PartyId::at(p), InstrumentId::at(1), 10.0, 1.0, 1);
        }
        let mut audit = Audit::new();
        audit.add(Box::<ATotalCarriesNoLots>::default());
        audit.add(Box::<NoCollateralCountedTwice>::default());
        let reports = audit.run(&over(&reg, &Instruments::new(), &Parties::new(), &Settlement::new(6), 1));
        assert_eq!(reports.len(), 2, "two families, each deriving its own answer");
        assert_eq!(reports[0].contributors.len(), 1);
        assert!(reports[0].built);
        assert!(reports[0].violations.is_empty());
    }

    #[test]
    fn holdings_that_do_not_sum_to_the_issued_amount_name_the_line_and_the_size() {
        use crate::calendar::Day;
        use crate::ids::{CurrencyCode, UnitId};
        use crate::instruments::{Class, Issuance};
        // **Register B2, and it needed 0l to be writable at all.** Two independent things: the
        // holdings, walked; and what the line says is issued, moved only by a named event.
        let mut ins = Instruments::new();
        let issuer = PartyId::at(0);
        let line = ins.issue(issuer, CurrencyCode::at(0), Class::Claim, UnitId::at(0), None, Some(Day(700)));
        ins.moves(line, Issuance::Made, 1_000.0);

        let mut reg = Register::new();
        reg.credit(PartyId::at(1), line, 600.0, 1.0, 1);
        reg.credit(PartyId::at(2), line, 400.0, 1.0, 1);
        let mut audit = Audit::new();
        audit.add(Box::<HoldersAgainstIssued>::default());
        let reports = audit.run(&over(&reg, &ins, &Parties::new(), &Settlement::new(6), 3));
        assert!(reports[0].violations.is_empty(), "1,000 held against 1,000 issued");

        // B2.a: a shortfall means somebody's claim vanished. Take a holder away and the line says
        // so, by name and by size.
        let row = reg.row(PartyId::at(2), line);
        reg.debit(row, 400.0);
        let reports = audit.run(&over(&reg, &ins, &Parties::new(), &Settlement::new(6), 4));
        assert_eq!(reports[0].violations.len(), 1);
        let v = &reports[0].violations[0];
        assert_eq!(v.size, -400.0);
        assert_eq!(v.owner, format!("instrument {}", line.row()));
        assert_eq!(v.spec, "Register B2");
        assert_eq!(v.period, 4);
        assert!(v.message.contains("a claim vanished"), "{}", v.message);

        // And a surplus means somebody's was invented — which is what a register credited outside
        // the wire looks like (22g.1).
        reg.credit(PartyId::at(3), line, 700.0, 1.0, 4);
        let reports = audit.run(&over(&reg, &ins, &Parties::new(), &Settlement::new(6), 5));
        assert_eq!(reports[0].violations[0].size, 300.0);
        assert!(reports[0].violations[0].message.contains("was invented"));
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
        let reports = audit.run(&over(&reg, &Instruments::new(), &Parties::new(), &Settlement::new(6), 4));
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
        audit.add(Box::<ATotalCarriesNoLots>::default());
        audit.add(Box::<NoCollateralCountedTwice>::default());
        let reports = audit.run(&over(&reg, &Instruments::new(), &Parties::new(), &Settlement::new(6), 1));
        let found: usize = reports.iter().map(|r| r.violations.len()).sum();
        assert_eq!(found, 0, "a money account is not a defect");
    }

    #[test]
    fn an_unbuilt_family_says_so_and_is_never_green() {

        let reg = Register::new();
        let mut audit = Audit::new();
        audit.add(Box::new(NotBuilt { family: Family::Liveness, contributor: "nobody" }));
        let reports = audit.run(&over(&reg, &Instruments::new(), &Parties::new(), &Settlement::new(6), 1));
        assert_eq!(reports[0].family, Family::Liveness);
        assert!(!reports[0].built, "an unbuilt family is not green");
        assert!(reports[0].violations.is_empty());
    }
}
