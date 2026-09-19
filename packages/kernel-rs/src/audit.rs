//! The audit: independent families, run every period, reporting owner + size + period + citation.

use crate::ids::{HoldingId, InstrumentId, PartyId};
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
    /// Every family this audit is accountable for. A world that assembled no contribution to a
    /// family must not read as a world with no violations in it, and the only way to know a family
    /// is missing is to have the list of them — so the list is here, beside the enum, and
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

    /// What an unbuilt family is WAITING FOR. A family whose blocker is named is a different thing
    /// from one nobody has looked at, and the difference belongs in the report rather than only in a
    /// plan file — `Audit::over` puts it where the contributor's name goes, because for an unbuilt
    pub fn waits_on(self) -> &'static str {
        match self {
            // Everything anyone marks has a price that came out of a mechanism. Nothing marks, so
            // there is nothing to check a mark against.
            Family::Prices => "waits on 0n — nothing is marked",
            // The same economic thing reached two ways. There is one such thing and it is the
            // index, whose module nothing imports.
            Family::CrossMarket => "waits on 0r — no economic thing is reachable twice",
            // Equity as a stated ACCOUNT moved by named events, against the residual read from the
            // register. Defining equity as the residual makes it a read of one thing against
            Family::Accounts => "waits on 0n.5 — equity is the residual and nothing else",
            // Part XII: derivative marks sum to zero per contract and in aggregate. Nothing marks a
            // derivative position.
            Family::ZeroSum => "waits on 0r — no derivative marks",
            Family::Liveness => "waits on 0p — no party's own view moves a price",
            // The five the kernel builds. `Audit::over` never reaches these.
            Family::Money | Family::Ownership | Family::Names | Family::Flows | Family::Units => {
                "built"
            }
        }
    }

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

/// A violation names its owner, its size, its period and the clause it is about. A finding with no
/// size cannot be ranked and one with no owner cannot be chased.
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
/// wire's own history, the instruments, the parties. A family that needs to know WHY something moved
/// reads the legs here — which is its own pass over its own source, and the independence Audit C3 is
pub struct Sources<'a> {
    pub wire: &'a Settlement,
    pub register: &'a Register,
    pub instruments: &'a Instruments,
    pub parties: &'a Parties,
    pub period: u32,
}


/// A contribution to a family. `visit` is called once per holding on the ONE traversal; `finish` is
/// where a family that needs the whole picture states what it found.
pub trait Contribution {
    fn family(&self) -> Family;
    fn contributor(&self) -> &'static str;
    /// A family nobody has built says so. It is never green by default.
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

    /// THE ONLY WAY TO BUILD AN AUDIT OF A WORLD. It takes what the kernel and the modules
    /// contributed and declares `NotBuilt` for every family nobody contributed to — so an unbuilt
    /// family is IN the report saying it is unbuilt, and cannot be absent from it.
    pub fn over(contributions: Vec<Box<dyn Contribution>>) -> Audit {
        let mut audit = Audit::new();
        for c in contributions {
            audit.add(c);
        }
        for family in Family::ALL {
            if audit.families.iter().any(|f| f.family() == family) {
                continue;
            }
            // And it says what it is waiting for. "nobody" was true and told a reader nothing; a
            // family with a named blocker has been looked at.
            audit.add(Box::new(NotBuilt { family, contributor: family.waits_on() }));
        }
        audit
    }

    /// Every family, every period, off ONE walk of the register.
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

// `LotsAgainstQuantity` stood here and it could not fail (0m.1, Audit A1.a: *a read of one thing
// against itself, which always passes*). It summed a row's lots and compared the total with

/// HOLDINGS SUM TO THE ISSUED AMOUNT, per instrument, always.
#[derive(Default)]
pub struct HoldersAgainstIssued {
    /// B1's side, read from its own source once per period.
    issued: Vec<f64>,
    /// And the holdings' side, accumulated on the shared walk — with the terms and the magnitudes
    /// the dust is derived from, because Law 7 wants the error of THIS arithmetic and B2.b forbids a
    /// fraction of the issue.
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
            // what says so. It is not this identity's to report as a shortfall.
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

/// THE SUM OVER ALL ACCOUNTS CHANGES ONLY BY AN ACT OF A MONEY ISSUER.
#[derive(Default)]
pub struct MoneyIsConserved {
    /// Per currency: what the accounts hold now, with the terms and magnitudes the dust comes from.
    now: std::collections::HashMap<u32, (f64, f64, usize)>,
    /// And what they held at the end of the period before, which is what a change is measured
    /// against. Appendix A: a currency that was not there held NOTHING, which is an answer.
    before: std::collections::HashMap<u32, f64>,
    /// What the issuers made this period, off the wire's own legs.
    minted: std::collections::HashMap<u32, (f64, f64, usize)>,
    /// Two periods that are not consecutive have no change between them to compare.
    comparable: bool,
    last_period: Option<u32>,
    found: Vec<Violation>,
}

impl Contribution for MoneyIsConserved {
    fn family(&self) -> Family {
        Family::Money
    }
    fn contributor(&self) -> &'static str {
        "kernel.conservation"
    }

    fn before(&mut self, from: &Sources<'_>) {
        self.comparable = self.last_period == from.period.checked_sub(1);
        self.before = std::mem::take(&mut self.now).into_iter().map(|(c, (sum, _, _))| (c, sum)).collect();
        self.minted.clear();
        for n in from.wire.in_period(from.period) {
            if from.wire.outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in from.wire.legs_of(n) {
                // The money creators are enumerable and few, and on this wire there is exactly one
                // leg that creates money.
                if let crate::ledger::Leg::Mint { money, amount, .. } = *leg {
                    let ccy = from.instruments.ccy_of(money).0;
                    let e = self.minted.entry(ccy).or_insert((0.0, 0.0, 0));
                    e.0 += amount;
                    e.1 += amount.abs();
                    e.2 += 1;
                }
            }
        }
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let line = at.register.instrument_of(at.row);
        if at.instruments.class_of(line) != crate::instruments::Class::Money {
            return;
        }
        let q = at.register.quantity(at.row);
        let e = self.now.entry(at.instruments.ccy_of(line).0).or_insert((0.0, 0.0, 0));
        e.0 += q;
        e.1 += q.abs();
        e.2 += 1;
    }

    fn finish(&mut self, period: u32) -> Vec<Violation> {
        if self.comparable {
            // Every currency either side knows about: one that emptied is as much a finding as one
            // that grew, and reading only what is there now would lose it.
            let mut seen: Vec<u32> = self.now.keys().copied().chain(self.before.keys().copied()).collect();
            seen.sort_unstable();
            seen.dedup();
            for ccy in seen {
                let (sum, magnitude, terms) = match self.now.get(&ccy) {
                    Some(&held) => held,
                    None => (0.0, 0.0, 0),
                };
                let was = match self.before.get(&ccy) {
                    Some(&held) => held,
                    None => 0.0,
                };
                let (made, made_magnitude, made_terms) = match self.minted.get(&ccy) {
                    Some(&m) => m,
                    None => (0.0, 0.0, 0),
                };
                let change = sum - was;
                let dust = (terms + made_terms) as f64
                    * f64::EPSILON
                    * (magnitude + made_magnitude + change.abs() + was.abs());
                if (change - made).abs() <= dust {
                    continue;
                }
                self.found.push(Violation {
                    family: Family::Money,
                    spec: "Audit B1",
                    owner: format!("currency {ccy}"),
                    size: change - made,
                    unit: "money",
                    period,
                    message: format!(
                        "the accounts moved by {change} and the issuers made {made}"
                    ),
                });
            }
        }
        self.last_period = Some(period);
        std::mem::take(&mut self.found)
    }
}

/// One holding, as one number, so the two sides of the flows check meet on the same key.
#[inline]
const fn key(party: PartyId, instrument: InstrumentId) -> u64 {
    ((party.0 as u64) << 32) | (instrument.0 as u64)
}

/// MISSING IS MISSING — and these two are not missing, they are NOTHING, which is an answer. A
/// holding that was not on the register last period held none of the line; a holding no leg
/// mentioned had nothing accounted for.
#[inline]
fn held_nothing_then(before: &std::collections::HashMap<u64, f64>, k: u64) -> f64 {
    match before.get(&k) {
        Some(&q) => q,
        None => 0.0,
    }
}

/// And what the legs accounted for where none of them named this holding: nothing, over no terms.
#[inline]
fn legs_said_nothing(
    moved: &std::collections::HashMap<u64, (f64, f64, u32)>,
    k: u64,
) -> (f64, f64, u32) {
    match moved.get(&k) {
        Some(&seen) => seen,
        None => (0.0, 0.0, 0),
    }
}

/// INSTRUCTIONS IN MINUS OUT EQUALS THE CHANGE IN HOLDINGS.
#[derive(Default)]
pub struct FlowsAreComplete {
    /// What the legs of this period accounted for, per holding, kept as TERMS — Law 7's dust comes
    /// from the terms and a running total alone cannot produce it.
    moved: std::collections::HashMap<u64, (f64, f64, u32)>,
    held: std::collections::HashMap<u64, f64>,
    before: std::collections::HashMap<u64, f64>,
    comparable: bool,
    last_period: Option<u32>,
    found: Vec<Violation>,
}

impl FlowsAreComplete {
    #[inline]
    fn account(&mut self, party: PartyId, instrument: InstrumentId, qty: f64) {
        let e = self.moved.entry(key(party, instrument)).or_insert((0.0, 0.0, 0));
        e.0 += qty;
        e.1 += qty.abs();
        e.2 += 1;
    }
}

impl Contribution for FlowsAreComplete {
    fn family(&self) -> Family {
        Family::Flows
    }
    fn contributor(&self) -> &'static str {
        "kernel.flows"
    }

    fn before(&mut self, from: &Sources<'_>) {
        self.moved.clear();
        self.comparable = self.last_period == from.period.checked_sub(1);
        self.before = std::mem::take(&mut self.held);
        for n in from.wire.in_period(from.period) {
            // A refused instruction moved NOTHING, and its legs are on the wire because the wire is
            // the history of what was tried. Counting them would report every fail as a holding
            if from.wire.outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in from.wire.legs_of(n) {
                match *leg {
                    // Goods B, E4: a thing coming into existence or leaving it. ONE side.
                    crate::ledger::Leg::Create { party, instrument, qty, .. } => {
                        self.account(party, instrument, qty)
                    }
                    crate::ledger::Leg::Destroy { party, instrument, qty, .. } => {
                        self.account(party, instrument, -qty)
                    }
                    // A move between two holders is two sides of one fact.
                    crate::ledger::Leg::Asset { from: seller, to: buyer, instrument, qty, .. } => {
                        self.account(buyer, instrument, qty);
                        self.account(seller, instrument, -qty);
                    }
                    // Money is `MoneyIsConserved`'s, and a pledge moves no units at all.
                    crate::ledger::Leg::Money { .. }
                    | crate::ledger::Leg::Mint { .. }
                    | crate::ledger::Leg::Pledge { .. } => {}
                }
            }
        }
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let instrument = at.register.instrument_of(at.row);
        if at.instruments.class_of(instrument) == crate::instruments::Class::Money {
            return;
        }
        let holder = at.register.holder_of(at.row);
        self.held.insert(key(holder, instrument), at.register.quantity(at.row));
    }

    fn finish(&mut self, period: u32) -> Vec<Violation> {
        if self.comparable {
            let mut say = |k: u64, change: f64, seen: (f64, f64, u32), extra: f64| {
                let (accounted, magnitude, terms) = seen;
                let dust =
                    (terms as f64 + 2.0) * f64::EPSILON * (magnitude + change.abs() + extra);
                if (change - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Flows,
                        spec: "Audit B7",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: change - accounted,
                        unit: "units",
                        period,
                        message: format!(
                            "the holding moved by {change} and its legs account for {accounted}"
                        ),
                    });
                }
            };
            for (&k, &now) in &self.held {
                let was = held_nothing_then(&self.before, k);
                say(k, now - was, legs_said_nothing(&self.moved, k), now.abs() + was.abs());
            }
            // A holding that went to NOTHING still has to have a leg behind it.
            for (&k, &was) in &self.before {
                if self.held.contains_key(&k) {
                    continue;
                }
                say(k, -was, legs_said_nothing(&self.moved, k), was.abs());
            }
        }
        self.last_period = Some(period);
        std::mem::take(&mut self.found)
    }
}

/// EVERY PARTY REFERENCED EXISTS; EVERY ISSUER OF A HELD INSTRUMENT EXISTS OR HAS A SUCCESSOR.
#[derive(Default)]
pub struct NamesResolve {
    found: Vec<Violation>,
}

impl Contribution for NamesResolve {
    fn family(&self) -> Family {
        Family::Names
    }
    fn contributor(&self) -> &'static str {
        "kernel.names"
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let holder = at.register.holder_of(at.row);
        let line = at.register.instrument_of(at.row);
        // No holding without a holder. A row whose holder is not a party in this world is a
        // position on nobody.
        if holder.row() >= at.parties.len() {
            self.found.push(Violation {
                family: Family::Names,
                spec: "Audit B6",
                owner: format!("party {}", holder.0),
                size: at.register.quantity(at.row),
                unit: "units",
                period: at.period,
                message: format!("holding {} is held by a party that does not exist", at.row.row()),
            });
            return;
        }
        // No holding without an issuer, and A2 — a holding is a claim ON somebody.
        if line.row() >= at.instruments.len() {
            self.found.push(Violation {
                family: Family::Names,
                spec: "Audit B6",
                owner: format!("instrument {}", line.0),
                size: at.register.quantity(at.row),
                unit: "units",
                period: at.period,
                message: format!("holding {} is of a line that was never issued", at.row.row()),
            });
            return;
        }
        let issuer = at.instruments.issuer_of(line);
        if issuer.row() >= at.parties.len() {
            self.found.push(Violation {
                family: Family::Names,
                spec: "Audit B6",
                owner: format!("instrument {}", line.0),
                size: at.register.quantity(at.row),
                unit: "units",
                period: at.period,
                message: format!("its issuer, party {}, does not exist", issuer.0),
            });
        }
    }

    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// A TOTAL account carries no lots. The other half of a rule whose first half is that a row carrying
/// lots answers from them — one of them alone would be a rule with an exemption.
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

/// No unit is encumbered beyond what is held. A lien over more than exists is collateral counted
/// twice.
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

/// A family that has not been built. It reports NOT BUILT and never green.
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

    /// The stores an audit derives its answers from, gathered for a call — the same shape `Settling`
    /// has, and for the same reason.
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
        // Register B2, and it needed 0l to be writable at all. Two independent things: the
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

        // A shortfall means somebody's claim vanished. Take a holder away and the line says so, by
        // name and by size.
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
        // the wire looks like.
        reg.credit(PartyId::at(3), line, 700.0, 1.0, 4);
        let reports = audit.run(&over(&reg, &ins, &Parties::new(), &Settlement::new(6), 5));
        assert_eq!(reports[0].violations[0].size, 300.0);
        assert!(reports[0].violations[0].message.contains("was invented"));
    }

    /// A world with one bank, one money and one good, so a family has something to be about.
    fn lines() -> (Instruments, Parties, crate::ids::InstrumentId, crate::ids::InstrumentId) {
        use crate::ids::{CurrencyCode, RegionId, UnitId};
        use crate::instruments::Class;
        use crate::parties::Representation;
        let mut ps = Parties::new();
        let bank = ps.add(0, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        for _ in 0..4 {
            ps.add(0, RegionId::at(0), bank, Representation::Named, 1, 0);
        }
        let mut ins = Instruments::new();
        let cash = ins.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let grain = ins.issue(PartyId::at(1), CurrencyCode::at(0), Class::Good, UnitId::at(0), None, None);
        (ins, ps, cash, grain)
    }

    #[test]
    fn money_that_appears_with_no_issuer_behind_it_is_reported_against_its_currency() {
        // Two independent things: what every account adds up to, and what the
        // issuers made.
        let (ins, ps, cash, _) = lines();
        let mut reg = Register::new();
        reg.money_delta(PartyId::at(1), cash, 500.0);
        let wire = Settlement::new(6);
        let mut audit = Audit::new();
        audit.add(Box::<MoneyIsConserved>::default());
        // Period 1 establishes what is held; two periods that are not consecutive have no change.
        audit.run(&over(&reg, &ins, &ps, &wire, 1));
        let reports = audit.run(&over(&reg, &ins, &ps, &wire, 2));
        assert!(reports[0].violations.is_empty(), "nothing moved and nobody minted");

        // A payment between two accounts moves no money into or out of the world.
        reg.money_delta(PartyId::at(1), cash, -200.0);
        reg.money_delta(PartyId::at(2), cash, 200.0);
        let reports = audit.run(&over(&reg, &ins, &ps, &wire, 3));
        assert!(reports[0].violations.is_empty(), "Money C2.c: a transfer's legs sum to zero");

        // And money out of nowhere is named, by currency and by size.
        reg.money_delta(PartyId::at(3), cash, 90.0);
        let reports = audit.run(&over(&reg, &ins, &ps, &wire, 4));
        assert_eq!(reports[0].violations.len(), 1);
        let v = &reports[0].violations[0];
        assert_eq!(v.size, 90.0);
        assert_eq!(v.owner, "currency 0");
        assert_eq!(v.spec, "Audit B1");
    }

    #[test]
    fn a_holding_that_moved_with_no_leg_behind_it_is_named_with_its_size() {
        // The register's own walk against the LEGS that said why anything
        // moved.
        let (ins, ps, _, grain) = lines();
        let mut reg = Register::new();
        reg.credit(PartyId::at(2), grain, 40.0, 1.0, 1);
        let wire = Settlement::new(6);
        let mut audit = Audit::new();
        audit.add(Box::<FlowsAreComplete>::default());
        audit.run(&over(&reg, &ins, &ps, &wire, 1));
        let reports = audit.run(&over(&reg, &ins, &ps, &wire, 2));
        assert!(reports[0].violations.is_empty(), "it held 40 and it holds 40");

        reg.credit(PartyId::at(2), grain, 15.0, 1.0, 3);
        let reports = audit.run(&over(&reg, &ins, &ps, &wire, 3));
        assert_eq!(reports[0].violations.len(), 1);
        let v = &reports[0].violations[0];
        assert_eq!(v.size, 15.0);
        assert_eq!(v.spec, "Audit B7");
        assert!(v.message.contains("legs account for 0"), "{}", v.message);
    }

    #[test]
    fn a_holding_of_a_line_nobody_issued_is_a_name_that_does_not_resolve() {
        // A holding is a claim ON somebody; a claim on a party that never
        // issued it is money invented in the ownership dimension.
        let (ins, ps, _, grain) = lines();
        let mut reg = Register::new();
        reg.credit(PartyId::at(2), grain, 10.0, 1.0, 1);
        let mut audit = Audit::new();
        audit.add(Box::<NamesResolve>::default());
        let reports = audit.run(&over(&reg, &ins, &ps, &Settlement::new(6), 1));
        assert!(reports[0].violations.is_empty());

        // A line beyond the last one issued, and a holder beyond the last party admitted.
        reg.credit(PartyId::at(2), crate::ids::InstrumentId::at(99), 5.0, 1.0, 1);
        reg.credit(PartyId::at(77), grain, 5.0, 1.0, 1);
        let reports = audit.run(&over(&reg, &ins, &ps, &Settlement::new(6), 2));
        assert_eq!(reports[0].violations.len(), 2);
        assert!(reports[0].violations.iter().any(|v| v.message.contains("never issued")));
        assert!(reports[0].violations.iter().any(|v| v.message.contains("does not exist")));
    }

    #[test]
    fn an_unbuilt_family_names_what_it_is_waiting_for() {
        // "nobody" was true and told a reader nothing. Audit E1 is why these are absences rather
        // than violations — there is nothing to be inconsistent with.
        let audit = Audit::over(Vec::new());
        let reports = Audit::over(Vec::new()).families.len();
        assert_eq!(reports, Family::ALL.len());
        drop(audit);
        for family in Family::ALL {
            assert!(!family.waits_on().is_empty(), "{} says nothing", family.name());
        }
        assert!(Family::Prices.waits_on().contains("0n"));
        assert!(Family::Accounts.waits_on().contains("0n.5"));
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
        // Money is one of itself, so its account is a total with no lots. The family that sums lots
        // must not report it, and the family that checks totals must find it clean.
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
