//! The audit: independent families, run every week, reporting owner + size + week + citation.

use crate::ids::{HoldingId, InstrumentId, PartyId};
use crate::instruments::Instruments;
use crate::ledger::Settlement;
use crate::parties::Parties;
use crate::prices::Prints;
use crate::register::Register;
use crate::stores::{Agreements, Claims, DueId, DueState, Schedules};

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
    /// Every family this audit is accountable for.
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

/// A violation names its owner, its size, its week and the clause it is about.
#[derive(Clone, Debug)]
pub struct Violation {
    pub family: Family,
    pub spec: &'static str,
    pub owner: String,
    pub size: f64,
    pub unit: &'static str,
    pub week: u32,
    pub message: String,
}

/// What one contribution found, and whether it is BUILT.
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
    pub week: u32,
}

/// What a family is given BEFORE the shared walk, for the sources that are not one holding: the
/// wire's own history, the instruments, the parties.
pub struct Sources<'a> {
    pub wire: &'a Settlement,
    pub register: &'a Register,
    pub instruments: &'a Instruments,
    pub parties: &'a Parties,
    pub week: u32,
    pub prints: Option<&'a Prints>,
    pub claims: Option<&'a Claims>,
    pub schedules: Option<&'a Schedules>,
    pub agreements: Option<&'a Agreements>,
    pub sessions: Option<&'a [crate::session::Session]>,
    /// Audit B5's other record: what each party's account was moved by.
    pub equity: Option<&'a crate::stores::Equity>,
    /// 49 H1: the physical world the nine geography contributions measure.
    pub geography: Option<&'a crate::geography::Geography>,
}

#[derive(Default)]
pub struct CrossMarketValues {
    found: Vec<Violation>,
}

/// Each completed clearing is reconciled from the transfers settlement actually accepted.
#[derive(Default)]
pub struct ClearingReconciles {
    found: Vec<Violation>,
}

impl Contribution for ClearingReconciles {
    fn family(&self) -> Family {
        Family::Flows
    }
    fn contributor(&self) -> &'static str {
        "kernel.clearing-settlement-reconciliation"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        let Some(sessions) = from.sessions else {
            return;
        };
        for session in sessions
            .iter()
            .filter(|session| session.week == from.week && session.settled > 0)
        {
            for (size, unit, message) in [
                (
                    session.bought - session.sold,
                    "units",
                    "bought does not equal sold",
                ),
                (
                    session.cash_paid - session.cash_received,
                    "money",
                    "cash paid does not equal cash received",
                ),
            ] {
                let dust = crate::num::dust(2, &[size, 0.0]);
                if size.abs() > dust {
                    self.found.push(Violation {
                        family: Family::Flows,
                        spec: "Clearing D5",
                        owner: format!("market {}", session.market.0),
                        size,
                        unit,
                        week: from.week,
                        message: message.to_string(),
                    });
                }
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

impl Contribution for CrossMarketValues {
    fn family(&self) -> Family {
        Family::CrossMarket
    }
    fn contributor(&self) -> &'static str {
        "kernel.session-price-reconciliation"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        let (Some(sessions), Some(prints)) = (from.sessions, from.prints) else {
            return;
        };
        for session in sessions
            .iter()
            .filter(|session| session.week == from.week && session.settled > 0)
        {
            let crate::clearing::Outcome::Cleared { price, .. } = session.outcome else {
                continue;
            };
            // The key answers which book the level came from, so nothing compares it afterwards.
            let printed = prints.latest(session.market, session.subject, from.week);
            let Some(print) = printed.filter(|print| print.week == from.week) else {
                self.found.push(Violation {
                    family: Family::CrossMarket,
                    spec: "Audit B4",
                    owner: format!("market {}", session.market.0),
                    size: price,
                    unit: "missing print",
                    week: from.week,
                    message: "a cleared session cannot be reached through the price store"
                        .to_string(),
                });
                continue;
            };
            let dust = crate::num::dust(2, &[price, print.price]);
            if (price - print.price).abs() > dust {
                self.found.push(Violation {
                    family: Family::CrossMarket,
                    spec: "Audit B4",
                    owner: format!("instrument {}", session.subject.0),
                    size: print.price - price,
                    unit: "money per unit",
                    week: from.week,
                    message: "the session and price store give different values for one clearing"
                        .to_string(),
                });
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct BilateralDerivativesAreZeroSum {
    found: Vec<Violation>,
}

impl Contribution for BilateralDerivativesAreZeroSum {
    fn family(&self) -> Family {
        Family::ZeroSum
    }
    fn contributor(&self) -> &'static str {
        "kernel.bilateral-derivative-ownership"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        let Some(agreements) = from.agreements else {
            return;
        };
        for row in 0..agreements.len() as u32 {
            let agreement = crate::stores::AgreementId(row);
            if !matches!(
                agreements.terms(agreement),
                crate::stores::AgreementTerms::PriceForward { .. }
                    | crate::stores::AgreementTerms::CreditDefaultSwap { .. }
                    | crate::stores::AgreementTerms::FxForward { .. }
            ) {
                continue;
            }
            let (one, other) = agreements.between(agreement);
            if one == other || one.row() >= from.parties.len() || other.row() >= from.parties.len()
            {
                self.found.push(Violation {
                    family: Family::ZeroSum,
                    spec: "Audit B8",
                    owner: format!("agreement {row}"),
                    size: 1.0,
                    unit: "unpaired derivative side",
                    week: from.week,
                    message: "a derivative does not resolve to two different live account owners"
                        .to_string(),
                });
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct MarketDecisionLiveness {
    found: Vec<Violation>,
}

impl Contribution for MarketDecisionLiveness {
    fn family(&self) -> Family {
        Family::Liveness
    }
    fn contributor(&self) -> &'static str {
        "kernel.order-clearing-print-trace"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        let (Some(sessions), Some(prints)) = (from.sessions, from.prints) else {
            return;
        };
        for session in sessions
            .iter()
            .filter(|session| session.week == from.week && !session.submitted.is_empty())
        {
            if session.settled == 0
                || !matches!(session.outcome, crate::clearing::Outcome::Cleared { .. })
            {
                continue;
            }
            if prints
                .latest(session.market, session.subject, from.week)
                .is_none_or(|print| print.week != from.week)
            {
                self.found.push(Violation {
                    family: Family::Liveness,
                    spec: "Audit D4",
                    owner: format!("market {}", session.market.0),
                    size: session.submitted.len() as f64,
                    unit: "untraced orders",
                    week: from.week,
                    message: "a deciding party's orders cleared without a consumable print"
                        .to_string(),
                });
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

pub trait Contribution {
    fn family(&self) -> Family;
    fn contributor(&self) -> &'static str;
    /// A family nobody has built says so.
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

    /// THE ONLY WAY TO BUILD AN AUDIT OF A WORLD.
    pub fn over(contributions: Vec<Box<dyn Contribution>>) -> Audit {
        let mut audit = Audit::new();
        for c in contributions {
            audit.add(c);
        }
        for family in Family::ALL {
            if audit.families.iter().any(|f| f.family() == family) {
                continue;
            }
            audit.add(Box::new(NotBuilt {
                family,
                contributor: "not built",
            }));
        }
        audit
    }

    /// Every family, every week, off ONE walk of the register.
    pub fn run(&mut self, from: &Sources<'_>) -> Vec<Report> {
        let (register, week) = (from.register, from.week);
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
                week,
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
            let violations = if built { f.finish(week) } else { Vec::new() };
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

/// Market-carried positions must have an observable market value; absence is a finding, never a
/// basis substitution.
#[derive(Default)]
pub struct MarketValuesExist {
    violations: Vec<Violation>,
}

impl Contribution for MarketValuesExist {
    fn family(&self) -> Family {
        Family::Prices
    }
    fn contributor(&self) -> &'static str {
        "market values exist independently of carrying basis"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.violations.clear();
        let Some(prints) = from.prints else { return };
        for row in from.register.all() {
            if from.register.carrying(row) != Some(crate::register::Carrying::Market)
                || from.register.quantity(row) == 0.0
                || crate::instruments::worth(
                    row,
                    from.register,
                    from.instruments,
                    prints,
                    from.week,
                )
                .is_some()
            {
                continue;
            }
            self.violations.push(Violation {
                family: Family::Prices,
                spec: "XI-6",
                owner: format!("holding {}", row.0),
                size: from.register.quantity(row),
                unit: "units without market value",
                week: from.week,
                message: "a market-carried position has no applicable price".to_string(),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.violations)
    }
}

/// Audit B5: what the register and the ledger leave, against what the account was moved by. Two
/// records of one number, built from different things — stocks on one side, named events on the
/// other — so equality is a check and not a restatement (B5.a).
#[derive(Default)]
pub struct AccountsBalance {
    violations: Vec<Violation>,
}

impl Contribution for AccountsBalance {
    fn family(&self) -> Family {
        Family::Accounts
    }
    fn contributor(&self) -> &'static str {
        "the equity account equals the residual"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.violations.clear();
        let (Some(prints), Some(claims), Some(equity)) = (from.prints, from.claims, from.equity)
        else {
            return;
        };
        for row in 0..from.parties.len() as u32 {
            let party = PartyId::at(row);
            let residual = crate::instruments::booked_equity(
                party,
                from.register,
                from.instruments,
                prints,
                claims,
                from.week,
            );
            let Some(residual) = residual else {
                self.violations.push(Violation {
                    family: Family::Accounts,
                    spec: "Audit B5",
                    owner: format!("party {row}"),
                    size: 1.0,
                    unit: "unreadable account",
                    week: from.week,
                    message: "booked equity cannot be read under the declared treatments"
                        .to_string(),
                });
                continue;
            };
            // A party with no account at all is not a party whose two records disagree: it is one
            // nobody has opened an account for, and the seed is where that is said.
            if !equity.opened(party) {
                continue;
            }
            let account = equity.balance_of(party);
            let gap = residual - account;
            // The dust of the walk that produced them: the account is the sum of its movements and
            // the residual is the sum of the holdings it read.
            let terms = equity.movements_of(party) + from.register.of_holder(party).len();
            if gap.abs() <= crate::num::dust(terms, &[residual, account]) {
                continue;
            }
            self.violations.push(Violation {
                family: Family::Accounts,
                spec: "Audit B5",
                owner: format!("party {row}"),
                size: gap,
                unit: "money",
                week: from.week,
                message: format!(
                    "the residual is {residual} and the account it was moved to is {account}"
                ),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.violations)
    }
}

/// Contractual state is a projection of wire outcomes, not a second account somebody may update
/// independently. Claimed balances must also have reached the estate claims store.
#[derive(Default)]
pub struct ScheduleOutcomesMatch {
    violations: Vec<Violation>,
}

impl Contribution for ScheduleOutcomesMatch {
    fn family(&self) -> Family {
        Family::Flows
    }
    fn contributor(&self) -> &'static str {
        "schedule outcomes reconcile with wire and claims"
    }

    fn before(&mut self, from: &Sources<'_>) {
        self.violations.clear();
        let Some(schedules) = from.schedules else {
            return;
        };
        let mut latest = std::collections::HashMap::<u32, crate::ledger::Outcome>::new();
        let mut settled = std::collections::HashMap::<u32, f64>::new();
        for n in 0..from.wire.len() {
            let Some(due) = from.wire.due_of(n) else {
                continue;
            };
            let outcome = from.wire.outcome_of(n);
            latest.insert(due.0, outcome);
            if outcome == crate::ledger::Outcome::Settled {
                let paid: f64 = from
                    .wire
                    .legs_of(n)
                    .iter()
                    .filter_map(|leg| match leg {
                        crate::ledger::Leg::Money { amount, .. } => Some(amount.get()),
                        _ => None,
                    })
                    .sum();
                *settled.entry(due.0).or_default() += paid;
            }
        }

        for row in 0..schedules.len() as u32 {
            let due = DueId(row);
            if let Some(&paid) = settled.get(&row) {
                let recovered = schedules.recovered(due);
                let dust = crate::num::dust(2, &[paid, recovered]);
                if (paid - recovered).abs() > dust {
                    self.violations.push(Violation {
                        family: Family::Flows,
                        spec: "Audit B7 · XI-9",
                        owner: format!("due {row}"),
                        size: recovered - paid,
                        unit: "money",
                        week: from.week,
                        message: format!(
                            "the wire settled {paid}, but the schedule recovered {recovered}"
                        ),
                    });
                }
            }
            let Some(&outcome) = latest.get(&row) else {
                continue;
            };
            let matches = match (outcome, schedules.state(due)) {
                (crate::ledger::Outcome::Settled, DueState::Settled { .. }) => true,
                (crate::ledger::Outcome::Queued, DueState::Queued { .. }) => true,
                (crate::ledger::Outcome::ShortOfMoney, DueState::Failed { outcome, .. })
                | (crate::ledger::Outcome::BankCouldNotSettle, DueState::Failed { outcome, .. })
                | (
                    crate::ledger::Outcome::NoAccountInThatMoney,
                    DueState::Failed { outcome, .. },
                )
                | (crate::ledger::Outcome::Encumbered, DueState::Failed { outcome, .. })
                | (crate::ledger::Outcome::NoCapacity, DueState::Failed { outcome, .. })
                | (crate::ledger::Outcome::ShortOfUnits, DueState::Failed { outcome, .. }) => {
                    outcome == latest[&row]
                }
                _ => false,
            };
            if !matches {
                self.violations.push(Violation {
                    family: Family::Flows,
                    spec: "Audit B7 · XI-9",
                    owner: format!("due {row}"),
                    size: 1.0,
                    unit: "state mismatch",
                    week: from.week,
                    message: format!(
                        "latest wire outcome {outcome:?} disagrees with schedule state {:?}",
                        schedules.state(due)
                    ),
                });
            }
        }

        let Some(claims) = from.claims else { return };
        let mut claimed = std::collections::HashMap::<u32, f64>::new();
        for row in 0..schedules.len() as u32 {
            let due = DueId(row);
            if schedules.claimed(due) {
                *claimed.entry(schedules.owed_by(due).0).or_default() +=
                    schedules.amount(due) - schedules.recovered(due);
            }
        }
        for (estate, scheduled) in claimed {
            let estate = PartyId::at(estate);
            let claimed: f64 = claims
                .on_estate(estate)
                .iter()
                .map(|row| claims.owed(crate::stores::ClaimId(*row)))
                .sum();
            let dust = crate::num::dust(2, &[scheduled, claimed]);
            if claimed + dust < scheduled {
                self.violations.push(Violation {
                    family: Family::Flows,
                    spec: "XI-8 · Audit B7",
                    owner: format!("estate {}", estate.0),
                    size: scheduled - claimed,
                    unit: "money without claim",
                    week: from.week,
                    message: "a schedule marked claimed has no matching estate-claim coverage"
                        .to_string(),
                });
            }
        }
    }

    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.violations)
    }
}

// `LotsAgainstQuantity` stood here and it could not fail (0m.1, Audit A1.a: *a read of one thing
// against itself, which always passes*).

/// HOLDINGS SUM TO THE ISSUED AMOUNT, per instrument, always.
#[derive(Default)]
pub struct HoldersAgainstIssued {
    /// B1's side, read from its own source once per week.
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
        self.issued
            .extend((0..lines).map(|row| from.instruments.issued_of(InstrumentId(row as u32))));
        self.held.clear();
        self.held.resize(lines, (0.0, 0.0, 0));
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let line = at.register.instrument_of(at.row).row();
        let Some(side) = self.held.get_mut(line) else {
            // A holding of a line this store never issued is Register A4's, and the Names family is
            // what says so.
            return;
        };
        let q = at.register.quantity(at.row);
        side.0 += q;
        side.1 += q.abs();
        side.2 += 1;
    }

    fn finish(&mut self, week: u32) -> Vec<Violation> {
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
                week,
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
    /// And what they held at the end of the week before, which is what a change is measured
    /// against.
    before: std::collections::HashMap<u32, f64>,
    /// What the issuers made this week, off the wire's own legs.
    minted: std::collections::HashMap<u32, (f64, f64, usize)>,
    /// Two weeks that are not consecutive have no change between them to compare.
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
        self.comparable = self.last_period == from.week.checked_sub(1);
        self.before = std::mem::take(&mut self.now)
            .into_iter()
            .map(|(c, (sum, _, _))| (c, sum))
            .collect();
        self.minted.clear();
        for n in from.wire.in_period(from.week) {
            if from.wire.outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in from.wire.legs_of(n) {
                // The money creators are enumerable and few, and on this wire there is exactly one
                // leg that creates money.
                if let crate::ledger::Leg::Mint { money, amount, .. } = *leg {
                    let ccy = from.instruments.ccy_of(money).0;
                    let e = self.minted.entry(ccy).or_insert((0.0, 0.0, 0));
                    e.0 += amount.get();
                    e.1 += amount.get().abs();
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
        let e = self
            .now
            .entry(at.instruments.ccy_of(line).0)
            .or_insert((0.0, 0.0, 0));
        e.0 += q;
        e.1 += q.abs();
        e.2 += 1;
    }

    fn finish(&mut self, week: u32) -> Vec<Violation> {
        if self.comparable {
            // Every currency either side knows about: one that emptied is as much a finding as one
            // that grew, and reading only what is there now would lose it.
            let mut seen: Vec<u32> = self
                .now
                .keys()
                .copied()
                .chain(self.before.keys().copied())
                .collect();
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
                    week,
                    message: format!("the accounts moved by {change} and the issuers made {made}"),
                });
            }
        }
        self.last_period = Some(week);
        std::mem::take(&mut self.found)
    }
}

/// One holding, as one number, so the two sides of the flows check meet on the same key.
#[inline]
const fn key(party: PartyId, instrument: InstrumentId) -> u64 {
    ((party.0 as u64) << 32) | (instrument.0 as u64)
}

/// MISSING IS MISSING — and these two are not missing, they are NOTHING, which is an answer.
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
    /// What the legs of this week accounted for, per holding, kept as TERMS — Law 7's dust comes
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
        let e = self
            .moved
            .entry(key(party, instrument))
            .or_insert((0.0, 0.0, 0));
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
        self.comparable = self.last_period == from.week.checked_sub(1);
        self.before = std::mem::take(&mut self.held);
        for n in from.wire.in_period(from.week) {
            // A refused instruction moved NOTHING, and its legs are on the wire because the wire is
            // the history of what was tried.
            if from.wire.outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in from.wire.legs_of(n) {
                match *leg {
                    // Goods B, E4: a thing coming into existence or leaving it.
                    crate::ledger::Leg::Create {
                        party,
                        instrument,
                        qty,
                        ..
                    } => self.account(party, instrument, qty.get()),
                    crate::ledger::Leg::Destroy {
                        party,
                        instrument,
                        qty,
                        ..
                    } => self.account(party, instrument, -qty.get()),
                    // A move between two holders is two sides of one fact.
                    crate::ledger::Leg::Asset {
                        from: seller,
                        to: buyer,
                        instrument,
                        qty,
                        ..
                    } => {
                        self.account(buyer, instrument, qty.get());
                        self.account(seller, instrument, -qty.get());
                    }
                    crate::ledger::Leg::Money {
                        from: payer,
                        to,
                        instrument,
                        amount,
                        ..
                    } => {
                        self.account(payer, instrument, -amount.get());
                        if crate::ledger::is_exchange_leg(
                            leg,
                            from.wire.legs_of(n),
                            from.instruments,
                        ) {
                            self.account(to, instrument, amount.get());
                        } else {
                            match crate::ledger::across(
                                from.parties,
                                from.instruments,
                                to,
                                instrument,
                            ) {
                                crate::ledger::Across::Same => {
                                    self.account(to, instrument, amount.get())
                                }
                                crate::ledger::Across::Banks {
                                    payers_bank,
                                    payees_bank,
                                    payees_money,
                                    reserves,
                                } => {
                                    self.account(to, payees_money, amount.get());
                                    self.account(payers_bank, reserves, -amount.get());
                                    self.account(payees_bank, reserves, amount.get());
                                }
                                crate::ledger::Across::Refused(..) => {
                                    unreachable!("settled money leg was pre-checked")
                                }
                            }
                        }
                    }
                    crate::ledger::Leg::Mint {
                        issuer,
                        money,
                        amount,
                    } => {
                        self.account(issuer, money, amount.get());
                    }
                    crate::ledger::Leg::Pledge { .. }
                    | crate::ledger::Leg::Depreciate { .. }
                    | crate::ledger::Leg::Dispatch { .. } => {}
                }
            }
        }
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let instrument = at.register.instrument_of(at.row);
        let holder = at.register.holder_of(at.row);
        self.held
            .insert(key(holder, instrument), at.register.quantity(at.row));
    }

    fn finish(&mut self, week: u32) -> Vec<Violation> {
        if self.comparable {
            let mut say = |k: u64, change: f64, seen: (f64, f64, u32), extra: f64| {
                let (accounted, magnitude, terms) = seen;
                let dust = (terms as f64 + 2.0) * f64::EPSILON * (magnitude + change.abs() + extra);
                if (change - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Flows,
                        spec: "Audit B7",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: change - accounted,
                        unit: "units",
                        week,
                        message: format!(
                            "the holding moved by {change} and its legs account for {accounted}"
                        ),
                    });
                }
            };
            for (&k, &now) in &self.held {
                let was = held_nothing_then(&self.before, k);
                say(
                    k,
                    now - was,
                    legs_said_nothing(&self.moved, k),
                    now.abs() + was.abs(),
                );
            }
            // A holding that went to NOTHING still has to have a leg behind it.
            for (&k, &was) in &self.before {
                if self.held.contains_key(&k) {
                    continue;
                }
                say(k, -was, legs_said_nothing(&self.moved, k), was.abs());
            }
        }
        self.last_period = Some(week);
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
        // No holding without a holder.
        if holder.row() >= at.parties.len() {
            self.found.push(Violation {
                family: Family::Names,
                spec: "Audit B6",
                owner: format!("party {}", holder.0),
                size: at.register.quantity(at.row),
                unit: "units",
                week: at.week,
                message: format!(
                    "holding {} is held by a party that does not exist",
                    at.row.row()
                ),
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
                week: at.week,
                message: format!(
                    "holding {} is of a line that was never issued",
                    at.row.row()
                ),
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
                week: at.week,
                message: format!("its issuer, party {}, does not exist", issuer.0),
            });
        }
    }

    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

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
                week: at.week,
                message: format!("a money account carries {} lots", lots.len()),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// No unit is encumbered beyond what is held.
#[derive(Default)]
pub struct NoCollateralCountedTwice {
    found: Vec<Violation>,
}

/// Population-cell weights change only through the named lattice transition doors.
#[derive(Default)]
pub struct CellWeightsConserve {
    found: Vec<Violation>,
}

/// A merged cell is a durable identity tombstone, never an economic owner.
#[derive(Default)]
pub struct CellOwnedUnitsReachLiveRows {
    found: Vec<Violation>,
}

impl Contribution for CellOwnedUnitsReachLiveRows {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "kernel.population-cell-instrument-ownership"
    }
    fn before(&mut self, _from: &Sources<'_>) {
        self.found.clear();
    }
    fn visit(&mut self, at: &Visit<'_>) {
        let holder = at.register.holder_of(at.row);
        if at.parties.merged_into(holder).is_none() || at.register.quantity(at.row) == 0.0 {
            return;
        }
        self.found.push(Violation {
            family: Family::Units,
            spec: "XI-15",
            owner: format!("merged cell {}", holder.0),
            size: at.register.quantity(at.row),
            unit: "instrument units",
            week: at.week,
            message: "instrument units remained on a consumed population cell".to_string(),
        });
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// A merged cell cannot remain named by a live agreement.
#[derive(Default)]
pub struct CellAgreementsReachLiveRows {
    found: Vec<Violation>,
}

impl Contribution for CellAgreementsReachLiveRows {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "kernel.population-cell-agreement-ownership"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        let Some(agreements) = from.agreements else {
            return;
        };
        for row in 0..from.parties.len() as u32 {
            let party = PartyId(row);
            if from.parties.merged_into(party).is_none() {
                continue;
            }
            for agreement in agreements.of_party(party) {
                if !agreements.live(crate::stores::AgreementId(*agreement)) {
                    continue;
                }
                self.found.push(Violation {
                    family: Family::Units,
                    spec: "XI-15 · XI-10",
                    owner: format!("merged cell {}", party.0),
                    size: 1.0,
                    unit: "live agreements",
                    week: from.week,
                    message: format!(
                        "live agreement {agreement} remained on a consumed population cell"
                    ),
                });
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

impl Contribution for CellWeightsConserve {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "kernel.population-cell-weights"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        for ((kind, region), gap) in from.parties.weight_conservation_gaps() {
            self.found.push(Violation {
                family: Family::Units,
                spec: "XI-15",
                owner: format!("party kind {kind} in region {}", region.0),
                size: gap as f64,
                unit: "people",
                week: from.week,
                message: "the cells that stand differ from what the population's events moved"
                    .to_string(),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// Appendix A: a cell is HOMOGENEOUS, so what it holds is `weight × what one member holds`. A total
/// that does not divide is an amount no member could hold — the average this mechanism exists to
/// forbid — and it arrives one indivisible payment at a time.
#[derive(Default)]
pub struct CellHoldingsDivideByWeight {
    found: Vec<Violation>,
}

impl CellHoldingsDivideByWeight {
    /// What is left over once the total is read as a whole count per member, or nothing where it
    /// divides to the dust of that one multiplication.
    fn remainder(total: f64, weight: f64) -> Option<f64> {
        let whole = (total / weight).round() * weight;
        let left = total - whole;
        (left.abs() > crate::num::dust(2, &[total, whole])).then_some(left)
    }
}

impl Contribution for CellHoldingsDivideByWeight {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "kernel.population-cell-divisibility"
    }
    fn before(&mut self, _from: &Sources<'_>) {
        self.found.clear();
    }
    fn visit(&mut self, at: &Visit<'_>) {
        let holder = at.register.holder_of(at.row);
        let crate::parties::Representation::Cell(weight) = at.parties.representation_of(holder)
        else {
            return;
        };
        if !at.parties.alive(holder) {
            return;
        }
        // A money account is divisible into what a money is divided into, which a holding does not
        // carry, so the pieces are what can be checked here and the account is not.
        let weight = f64::from(weight.get());
        if !at.register.is_total(at.row) {
            if let Some(left) = Self::remainder(at.register.quantity(at.row), weight) {
                self.found.push(Violation {
                    family: Family::Units,
                    spec: "Appendix A · XI-15",
                    owner: format!("cell {}", holder.0),
                    size: left,
                    unit: "pieces",
                    week: at.week,
                    message: format!(
                        "instrument {} is held in an amount no member of {} could hold",
                        at.register.instrument_of(at.row).0,
                        weight
                    ),
                });
            }
        }
        // And what is pledged moves with the members who pledged it, so it divides the same way.
        for lien in at.register.liens(at.row) {
            if let Some(left) = Self::remainder(lien.qty, weight) {
                self.found.push(Violation {
                    family: Family::Units,
                    spec: "Appendix A · XI-15",
                    owner: format!("cell {}", holder.0),
                    size: left,
                    unit: "pieces",
                    week: at.week,
                    message: format!(
                        "a lien to party {} is for an amount no member of {} could have pledged",
                        lien.to.0, weight
                    ),
                });
            }
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
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
                week: at.week,
                message: format!("pledged {} beyond what is held", -free),
            });
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// A family that has not been built.
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

// THE AUDIT NEVER REPAIRS, and that is the borrow rather than an assertion: `Sources` holds `&`
// references to the register, the instruments, the parties and the wire, so a contribution has no
// way to write the world it is reading.
//
// An unbuilt family is never green for the same reason: `Audit::over` fills every gap in
// `Family::ALL` with `NotBuilt`, whose `built()` is false, so a family nobody contributed to cannot
// be absent from the report.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_total_no_member_could_hold_is_what_is_left_over() {
        // Three members holding four pieces each is twelve, and thirteen is nobody's four.
        assert_eq!(CellHoldingsDivideByWeight::remainder(12.0, 3.0), None);
        assert_eq!(CellHoldingsDivideByWeight::remainder(13.0, 3.0), Some(1.0));
        // Signed, so a cell holding one too few and one too many are different findings.
        assert_eq!(CellHoldingsDivideByWeight::remainder(11.0, 3.0), Some(-1.0));
    }

    #[test]
    fn every_family_has_a_name_a_reader_can_read() {
        for family in Family::ALL {
            assert!(!family.name().is_empty());
        }
    }

    #[test]
    fn the_families_are_distinct_so_two_cannot_report_as_one() {
        let mut names: Vec<&str> = Family::ALL.iter().map(|f| f.name()).collect();
        names.sort_unstable();
        let all = names.len();
        names.dedup();
        assert_eq!(
            all,
            names.len(),
            "two families under one name report as one"
        );
    }

    #[test]
    fn a_claimed_due_without_estate_claim_coverage_is_a_flow_break() {
        let mut schedules = Schedules::new();
        let due = schedules.owes(
            crate::stores::Owed::To(PartyId::at(2)),
            PartyId::at(1),
            crate::ids::CurrencyCode::at(0),
            crate::stores::Payment {
                from: crate::calendar::Week(0),
                due: crate::calendar::Week(1),
                amount: 10.0,
                of: crate::stores::Owing::Principal,
            },
        );
        schedules.claim(due);
        let wire = (Settlement::new)(1);
        let register = Register::default();
        let instruments = Instruments::default();
        let parties = Parties::default();
        let claims = Claims::new();
        let mut check = ScheduleOutcomesMatch::default();
        check.before(&Sources {
            wire: &wire,
            register: &register,
            instruments: &instruments,
            parties: &parties,
            week: 1,
            prints: None,
            claims: Some(&claims),
            schedules: Some(&schedules),
            agreements: None,
            sessions: None,
            equity: None,
            geography: None,
        });
        let found = check.finish(1);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].size, 10.0);
    }
}
