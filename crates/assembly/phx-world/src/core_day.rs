//! The core's day while the port runs beside the books: each family's contracts due today make their flows, a
//! contract's next date read from its schedule; and each currency's flows settle over the core's accounts on its
//! country's business days, and are committed to what the accounts have pending on its closed days.

use phx_core::calendar::Calendar;
use phx_core::calendar::period::ScheduleDates;
use std::collections::BTreeMap;

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_core::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};
use phx_core::settle::Settle;
use phx_core::store::{Family, books, deposits_of};
use phx_core::{StreamDecl, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Subject, SubjectTag};
use phx_store::{Row, SystemBacking};

use crate::core::Core;

/// A dated contract on the core: its payer and payee, the amount each date pays, which date of its schedule comes
/// next, its schedule among its family's, the payee's person it is, by the person's identity, and what it owes
/// from dates it failed to pay, asked again with its next.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct Due {
    pub ends: [PartyKey; 2],
    pub amount: i64,
    pub nth: u32,
    pub schedule: u32,
    pub person: u64,
    pub arrears: i64,
}

impl Row for Due {
    fn ends(&self) -> [PartyKey; 2] {
        self.ends
    }

    fn set_end(&mut self, side: usize, party: PartyKey) {
        if let Some(e) = self.ends.get_mut(side) {
            *e = party;
        }
    }
}

pub use crate::consts::reason::{ESTATE, PENSION, SEVERANCE, WAGE};

/// A family of dated contracts: its store, the reason its flows carry, and the schedules its contracts' dates are
/// read from, each with its currency and the payment order its payer gives the family's flows; and, for a family of
/// jobs, each schedule's occupation, weekly hours and the year its jobs began.
#[derive(Debug)]
pub struct DatedFamily {
    pub name: &'static str,
    pub store: Family<Due, SystemBacking>,
    pub reason: u8,
    pub schedules: Vec<(ScheduleDates, u8, u8)>,
    pub classes: Vec<[u32; 3]>,
    /// For a family whose dues are reckoned from its terms — a loan's interest on its balance and its part repaid —
    /// each schedule's terms; a contract's amount is then what it still owes.
    pub terms: Vec<Option<phx_ledger::algebra::Terms>>,
    /// The contracts past their last date whose last due is out today, ended once it settles.
    pub finishing: Vec<u32>,
    /// Each schedule's last date by its place, where its contracts end: a benefit's months.
    pub ends_after: Vec<Option<u32>>,
    /// What its contracts reckoned from their terms moved on their creditors' books since the books last read them.
    pub moves: LoanMoves,
    /// What its contracts owed, balances reckoned from terms and arrears, when they closed.
    pub lost: i128,
}

/// A family's moves on its parties' books: each amount lent and balance written off, with the creditor whose book it
/// moves; each principal repaid, with its payer and creditor; and each change in a contract's arrears — what its payer
/// owes and its creditor is owed beyond its dates — with the two, the family's reason and whether its contract is
/// reckoned from terms.
#[derive(Debug, Default)]
pub struct LoanMoves {
    pub lent: Vec<(PartyKey, i64)>,
    pub repaid: Vec<(PartyKey, PartyKey, i64)>,
    /// Each balance written off, with its creditor and its contract.
    pub written_off: Vec<(PartyKey, i64, u32)>,
    pub arrears: Vec<Arrears>,
}

/// A change in a contract's arrears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrears {
    pub payer: PartyKey,
    pub payee: PartyKey,
    pub change: i64,
    pub reason: u8,
    pub terms: bool,
}

/// The state's laws on the core, by country: the income tax withheld from wages, the consumption tax's rate, the
/// benefit for a job lost, and each country's treasury the taxes are paid to.
#[derive(Debug, Default)]
pub struct CoreState {
    pub withholding: Vec<Option<phx_ledger::levy::Withholding>>,
    pub consumption: Vec<Option<f64>>,
    pub benefit: Vec<Option<if_state::kinds::BenefitLaw>>,
    pub claim: Option<&'static phx_core::decisions::DecisionPointDecl<if_state::kinds::ClaimIn, bool>>,
    pub included: Option<fn(f64, f64) -> f64>,
    /// Each country's day of the month after a tax is collected by which it is remitted.
    pub remit_day: Vec<Option<u32>>,
}

/// What the core's day did: the flows made, settled, failed and committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreDay {
    pub day: Day,
    pub flows: u64,
    pub settled: u64,
    pub failed: u64,
    pub committed: u64,
    /// The contracts that failed a due and hold it in arrears.
    pub arrears: u64,
    /// The failed flows by their reason.
    pub failed_by: [u64; crate::consts::reason::REASONS],
    /// The wages the day paid and did not fail.
    pub wages: i64,
    /// The money the day's flows move, whatever came of them.
    pub gross: i128,
    /// The estates settled and ended.
    pub estates: u64,
    /// The money family's breaks found after settlement: a bank owing other than its customers hold, money made or
    /// lost among the parties, or reserves moving other than by what customers paid those held at the issuer.
    pub breaks: u64,
    /// The core's day's time, its chance and settlement together, by the run's clock.
    pub ns: u64,
    /// The loans disbursed, and of them those a bank other than the borrower's own disbursed.
    pub lent: u64,
    pub lent_elsewhere: u64,
}

/// The day's working state, kept across days so a day allocates nothing once the heaviest has sized it.
#[derive(Debug, Default)]
pub struct Work {
    pub flows: FlowBufs,
    pub settle: Settle,
    pub due: Vec<u32>,
}

/// A loan's due on its `k`th date, by its terms' shape: what it pays, and of that what repays its balance.
#[clause("REG.5", "REG.11", "BNK.17")]
fn reckoned(
    terms: &phx_ledger::algebra::Terms,
    (balance, ccy): (i64, u8),
    k: u32,
    (day, calendar): (Day, &Calendar),
) -> (i64, i64) {
    use phx_ledger::algebra::{Amount, DueBuf, Leg};
    let (shape, own) = phx_ledger::algebra::shape_of(terms);
    let plan = phx_ledger::algebra::shape_plan(&shape, Some(k), day, calendar);
    if plan.general {
        violation!(clause = "REG.5", "a loan whose dues wait on an event");
    }
    let mut buf = DueBuf::default();
    phx_ledger::algebra::due_by_shape(&plan, &own, phx_num::Money::new(balance, phx_num::Ccy::new(ccy)), &mut buf);
    let (mut paid, mut repaid) = (0, 0);
    for d in buf.iter() {
        let Amount::Money(m) = d.amount else { continue };
        paid += m.amt();
        if matches!(terms.legs.get(usize::from(d.leg)), Some(Leg::Amortising | Leg::Principal { .. })) {
            repaid += m.amt();
        }
    }
    (paid, repaid)
}

impl Core {
    /// The wages the day's dues made and did not fail, recorded in their countries' months; returns them.
    fn record_wages_settled(&mut self, failed: &[Flow]) -> i64 {
        let wage = crate::consts::reason::WAGE;
        let mut by: BTreeMap<u8, i64> = BTreeMap::new();
        if let Some(buf) = self.work.flows.chunks_mut().first_mut() {
            for f in buf.iter().filter(|f| f.reason == wage && f.denomination.is_money()) {
                *by.entry(f.denomination.ccy()).or_insert(0) += f.amount;
            }
        }
        for f in failed.iter().filter(|f| f.reason == wage && f.denomination.is_money()) {
            *by.entry(f.denomination.ccy()).or_insert(0) -= f.amount;
        }
        let mut all = 0;
        for (ccy, amount) in by {
            self.record_wages(ccy, amount);
            all += amount;
        }
        all
    }

    /// Each failed flow a dated contract made held on it in arrears, asked again with its next due: a contract past
    /// its last date is kept, due again on its schedule's next date, until it is paid; one whose last due settled
    /// ends. Returns the contracts in arrears.
    #[clause("HH.13", "BNK.17")]
    fn hold_arrears(&mut self, day: Day, calendar: &Calendar, failed: &[Flow]) -> u64 {
        let mut n = 0;
        for f in failed {
            let Some(family) = self
                .families
                .iter_mut()
                .find(|x| x.reason == f.reason && x.store.kinds.first() == Some(&f.payer.kind()))
            else {
                continue;
            };
            let slot = Slot::new(f.source);
            if !family.store.edges.is_open(slot) {
                continue;
            }
            let Some(row) = family.store.edges.rows_mut().get_mut(usize::try_from(f.source).unwrap_or(usize::MAX))
            else {
                continue;
            };
            if row.ends != [f.payer, f.payee] {
                continue;
            }
            row.arrears += f.amount;
            let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
            let terms = family.terms.get(at).is_some_and(Option::is_some);
            family.moves.arrears.push(Arrears {
                payer: f.payer,
                payee: f.payee,
                change: f.amount,
                reason: family.reason,
                terms,
            });
            n += 1;
            if let Some(i) = family.finishing.iter().position(|e| *e == f.source) {
                family.finishing.swap_remove(i);
                let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
                if let Some((dates, _, _)) = family.schedules.get(at) {
                    let next = dates.nth(calendar, row.nth);
                    if next > day {
                        family.store.wheel.schedule(f.source, next);
                    }
                }
            }
        }
        for family in &mut self.families {
            for edge in std::mem::take(&mut family.finishing) {
                family.close_contract(Slot::new(edge));
            }
        }
        n
    }

    /// What a party's contracts take from it from today until `until`, each contract's next due where it falls by
    /// then: its amount, or what its terms reckon it pays.
    pub(crate) fn owed_until(&self, party: PartyKey, (day, until): (Day, Day), calendar: &Calendar) -> i64 {
        let mut owed = 0;
        for family in &self.families {
            if family.store.kinds.first() != Some(&party.kind())
                || family.store.heads.first().is_none_or(Option::is_none)
            {
                continue;
            }
            for edge in family.store.of(0, party.slot()) {
                let Some(row) = family.store.edges.row(edge) else { continue };
                if row.ends.first() != Some(&party) {
                    continue;
                }
                owed += row.arrears;
                let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
                let Some((dates, ccy, _)) = family.schedules.get(at) else { continue };
                let date = dates.nth(calendar, row.nth);
                if date < day || date > until {
                    continue;
                }
                owed += match family.terms.get(at).and_then(Option::as_ref) {
                    Some(terms) => reckoned(terms, (row.amount, *ccy), row.nth, (date, calendar)).0,
                    None => row.amount,
                };
            }
        }
        owed
    }
}

impl DatedFamily {
    /// A contract closed; one reckoned from its terms that still owes a balance has it written off its creditor's
    /// book.
    #[clause("BNK.11")]
    pub(crate) fn close_contract(&mut self, edge: Slot) {
        if let Some(row) = self.store.edges.row(edge).filter(|_| self.store.edges.is_open(edge)) {
            let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
            let terms = self.terms.get(at).is_some_and(Option::is_some);
            if terms && row.amount > 0 {
                self.moves.written_off.push((row.ends[1], row.amount, edge.get()));
                self.lost += i128::from(row.amount);
            }
            self.lost += i128::from(row.arrears);
            if row.arrears != 0 {
                let [debtor, creditor] = row.ends;
                self.moves.arrears.push(Arrears {
                    payer: debtor,
                    payee: creditor,
                    change: -row.arrears,
                    reason: self.reason,
                    terms,
                });
            }
        }
        self.store.close(edge);
    }

    /// The contracts due today made flows, each rescheduled at its next date.
    #[clause("SET.4", "TIME.4")]
    fn dues(&mut self, day: Day, calendar: &Calendar, due: &mut Vec<u32>, out: &mut Vec<Flow>) -> u64 {
        self.store.wheel.take(day, due, None);
        let mut made = 0;
        for edge in due.iter().copied() {
            let slot = Slot::new(edge);
            if !self.store.edges.is_open(slot) {
                continue;
            }
            let Some(row) = self.store.edges.rows_mut().get_mut(usize::try_from(edge).unwrap_or(usize::MAX)) else {
                continue;
            };
            let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
            let Some((dates, ccy, order)) = self.schedules.get(at) else {
                violation!(clause = "TIME.4", "a contract with no schedule", edge = edge);
            };
            let amount = match self.terms.get(at).and_then(Option::as_ref) {
                Some(terms) => {
                    let (paid, repaid) = reckoned(terms, (row.amount, *ccy), row.nth, (day, calendar));
                    row.amount -= repaid;
                    if repaid > 0 {
                        self.moves.repaid.push((row.ends[0], row.ends[1], repaid));
                    }
                    paid
                }
                None => row.amount,
            } + row.arrears;
            // What it owed is asked again now; a failure puts it back.
            if row.arrears != 0 {
                let terms = self.terms.get(at).is_some_and(Option::is_some);
                let [debtor, creditor] = row.ends;
                self.moves.arrears.push(Arrears {
                    payer: debtor,
                    payee: creditor,
                    change: -row.arrears,
                    reason: self.reason,
                    terms,
                });
            }
            row.arrears = 0;
            if amount > 0 {
                out.push(Flow {
                    payer: row.ends[0],
                    payee: row.ends[1],
                    amount,
                    source: edge,
                    denomination: Denom::money(*ccy),
                    reason: self.reason,
                    order: *order,
                });
                made += 1;
            }
            let last = self.ends_after.get(at).copied().flatten().or_else(|| {
                self.terms.get(at).and_then(Option::as_ref).and_then(|t| match t.schedule.count {
                    phx_num::Missing::Present(n) => Some(n),
                    phx_num::Missing::Absent => None,
                })
            });
            row.nth += 1;
            if last.is_some_and(|n| row.nth > n) {
                // A contract past its last date ends once nothing it owes is left to fail.
                if amount == 0 {
                    self.close_contract(slot);
                } else {
                    self.finishing.push(edge);
                }
                continue;
            }
            let next = dates.nth(calendar, row.nth);
            if next <= day {
                violation!(clause = "TIME.4", "a contract's next date not after today", edge = edge);
            }
            self.store.wheel.schedule(edge, next);
        }
        made
    }
}

impl Core {
    /// Income tax withheld from each wage the day's dues pay: the band's levy on its year taken from what the
    /// household is paid and paid by the employer to its country's treasury.
    #[clause("TAX.2", "TAX.7")]
    fn withhold(&mut self, buf: &mut [Flow]) {
        let mut arising = Vec::new();
        for f in buf.iter_mut().filter(|f| f.reason == WAGE && f.denomination.is_money()) {
            let ccy = f.denomination.ccy();
            let Some(Some(w)) = self.state.withholding.get(usize::from(ccy)) else { continue };
            let gross = f.amount;
            let tax = w.on_payment(gross);
            if tax <= 0 || tax > gross {
                continue;
            }
            f.amount -= tax;
            arising.push(crate::core_taxes::Arising {
                collector: f.payer,
                payer: f.payee,
                base: crate::core_taxes::INCOME,
                tax,
                ccy,
                on: (f.payer, f.payee, f.amount, f.reason, f.source),
            });
            let id = self.kinds.get(usize::from(f.payee.kind())).and_then(|k| k.parties.id(f.payee.slot()));
            if id.is_some_and(crate::core_rates::sampled) {
                self.taxes.sample.push((gross, tax, ccy));
            }
        }
        self.taxes.arising.append(&mut arising);
    }

    /// An estate begun with a party's money at its bank, to settle on its country's next business day.
    #[clause("PTY.9")]
    pub(crate) fn open_estate(&mut self, (bank, money): (u32, i64), (country, day): (CountryId, Day)) -> PartyKey {
        let Some(place) = self.names.iter().position(|n| *n == phx_core::ESTATE_KIND.name) else {
            violation!(clause = "PTY.9", "an estate with no kind to hold it");
        };
        let id = phx_id::PartyId::new(self.next_id);
        self.next_id += 1;
        let Some(store) = self.kinds.get_mut(place) else {
            violation!(clause = "PTY.9", "an estate kind with no store");
        };
        let party = store.begin(id, &[], Some(phx_core::store::Opening { bank, balance: money }));
        let key = PartyKey::new(u8::try_from(place).unwrap_or(u8::MAX), party.slot());
        let at = self.keys.partition_point(|(i, _)| *i < id);
        self.keys.insert(at, (id, key));
        self.estates.push((key, country, day));
        key
    }

    /// Each estate opened before today, on its country's business day, pays its claims by rank, each rank in proportion
    /// to what it is owed as far as the money goes, and the rest to the party the law names where no heir is drawn —
    /// its country's treasury, a firm's owners not yet being parties to it on the core — and is ended after the day's
    /// settlement once it holds nothing.
    #[clause("PTY.9", "POP.15", "L3")]
    fn estates_pay(&mut self, day: Day, calendar: &Calendar, out: &mut Vec<Flow>) -> Vec<PartyKey> {
        let mut settling = Vec::new();
        let treasury = self.names.iter().position(|n| *n == crate::consts::HEIRLESS_DESTINATION);
        for (estate, country, opened) in &self.estates {
            if *opened >= day || !calendar.is_business(*country, day) {
                continue;
            }
            let Some(to) = treasury.and_then(|t| {
                self.treasuries
                    .get(usize::from(country.get()))
                    .copied()
                    .flatten()
                    .filter(|k| usize::from(k.kind()) == t)
            }) else {
                violation!(
                    clause = "POP.15",
                    "an estate's country with no heirless destination",
                    country = country.get()
                );
            };
            let held = self
                .kinds
                .get(usize::from(estate.kind()))
                .and_then(|k| k.accounts.as_ref())
                .and_then(|a| Some(a.balance.get(estate.slot())? + a.pending.get(estate.slot())?));
            let mut left = held.unwrap_or(0);
            let claims = self.insolvency.claims.get(estate).map_or(&[][..], Vec::as_slice);
            for (payee, amount, reason) in crate::core_default::shares(claims, left) {
                left -= amount;
                out.push(Flow {
                    payer: *estate,
                    payee,
                    amount,
                    source: estate.slot().get(),
                    denomination: Denom::money(country.get()),
                    reason,
                    order: 0,
                });
            }
            if left > 0 {
                let amount = left;
                out.push(Flow {
                    payer: *estate,
                    payee: to,
                    amount,
                    source: estate.slot().get(),
                    denomination: Denom::money(country.get()),
                    reason: ESTATE,
                    order: 0,
                });
            }
            settling.push(*estate);
        }
        settling
    }

    /// The money every party but the banks holds, and the banks' reserves with every account held at the issuer:
    /// the first changes only by the issuer's own flows, the second only by what the issuer pays or is paid.
    #[must_use]
    pub fn money_totals(&self) -> i128 {
        let mut parties = 0_i128;
        for (k, store) in self.kinds.iter().enumerate() {
            let Some(a) = store.accounts.as_ref() else { continue };
            if self.bank_kind.is_some_and(|b| usize::from(b) == k) {
                continue;
            }
            parties += a
                .balance
                .slice()
                .iter()
                .zip(a.pending.slice())
                .map(|(m, p)| i128::from(*m) + i128::from(*p))
                .sum::<i128>();
        }
        parties
    }

    /// The money family on the core after the day's settlement and fund stage, reading only: each bank owes what its
    /// customers hold; the money the parties hold moved only by what they, the banks and the issuers paid each other;
    /// and the issuers' money — reserves, the treasuries' accounts and banknotes — what their own record of the flows
    /// that moved it says.
    #[clause("MON.5", "MON.7", "MON.8", "MON.9", "N1", "REP.14")]
    fn money_breaks(
        &mut self,
        (day, before): (Day, i128),
        (net, classes): (i128, [i128; 3]),
        deposits: &mut [i64],
    ) -> u64 {
        let owed = deposits_of(self.kinds.iter(), deposits.len());
        let mut found = Vec::new();
        for (slot, (a, b)) in (0_u32..).zip(owed.iter().zip(deposits.iter())) {
            if a == b {
                continue;
            }
            let party = self.bank_kind.and_then(|k| self.kinds.get(usize::from(k))?.parties.id(Slot::new(slot)));
            found.push(finding(
                (day, "Law 2"),
                party.map_or(FindingOwner::Run, FindingOwner::Party),
                i128::from(*a) - i128::from(*b),
                format!("a bank owes {a} where its customers hold {b}"),
            ));
        }
        let parties = self.money_totals();
        if parties != before + net {
            let detail = format!("the parties hold {parties} where the day's flows leave {}", before + net);
            found.push(finding((day, "Law 2"), FindingOwner::Run, parties - before - net, detail));
        }
        let held = self.issuer_held();
        if let Some(recorded) = self.central.recorded.as_mut() {
            for (r, c) in recorded.iter_mut().zip(classes) {
                *r += c;
            }
            let names = ["reserves", "treasuries' accounts", "banknotes"];
            for ((h, r), (class, name)) in held.iter().zip(recorded.iter()).zip(names.iter().enumerate()) {
                if h != r {
                    let clause = if class == crate::core_central::NOTES { "MON.9" } else { "MON.7" };
                    let detail = format!("the issuers owe {h} in {name} where their record of it holds {r}");
                    found.push(finding((day, clause), FindingOwner::Run, h - r, detail));
                }
            }
        }
        let n = phx_rand::float::len_u64(found.len());
        self.found.extend(found);
        n
    }

    /// The day's settled flows entered as income, then the fund stage; returns what both moved of the parties' and
    /// the issuers' money.
    fn after_settle(
        &mut self,
        work: &mut Work,
        failed: &[Flow],
        at: (Day, &Calendar, &Streams, &StreamDecl),
        ranges: &Ranges,
    ) -> (i128, [i128; 3]) {
        let mut moved = (0_i128, [0_i128; 3]);
        if let Some(buf) = work.flows.chunks_mut().first() {
            self.account_flows(buf, failed);
            self.accrue_taxes((at.0, at.1), buf, failed);
            moved = self.money_moves(buf, failed);
        }
        let (fund, fund_failed) = self.fund_stage(work, at, ranges);
        let fund_moved = self.money_moves(&fund, &fund_failed);
        moved.0 += fund_moved.0;
        for (m, f) in moved.1.iter_mut().zip(fund_moved.1) {
            *m += f;
        }
        moved
    }

    /// What a day's settled money flows moved: into the parties other than banks from the banks and the issuers, and
    /// each class of the issuers' money, a flow taking from its payer's class and adding to its payee's.
    fn money_moves(&self, flows: &[Flow], failed: &[Flow]) -> (i128, [i128; 3]) {
        let mut unpaid: BTreeMap<(PartyKey, PartyKey, i64, u8, u32), u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        let money_maker = |k: PartyKey| self.bank_kind == Some(k.kind()) || self.issuers.contains(&k);
        let (mut net, mut classes) = (0_i128, [0_i128; 3]);
        for f in flows.iter().filter(|f| f.denomination.is_money()) {
            if let Some(n) = unpaid.get_mut(&(f.payer, f.payee, f.amount, f.reason, f.source)).filter(|n| **n > 0) {
                *n -= 1;
                continue;
            }
            let a = i128::from(f.amount);
            match (money_maker(f.payer), money_maker(f.payee)) {
                (true, false) => net += a,
                (false, true) => net -= a,
                _ => {}
            }
            if let Some(c) = self.money_class(f.payer).and_then(|c| classes.get_mut(c)) {
                *c -= a;
            }
            if let Some(c) = self.money_class(f.payee).and_then(|c| classes.get_mut(c)) {
                *c += a;
            }
        }
        (net, classes)
    }

    /// After the day's settlement: the sales delivered or released, each failed due held in its contract's arrears
    /// and remembered from the day they began. Returns the contracts in arrears.
    fn after_settlement(&mut self, day: Day, calendar: &Calendar, failed: &[Flow]) -> u64 {
        self.deliver_sales(day, failed);
        let n = self.hold_arrears(day, calendar, failed);
        self.note_arrears(day, failed);
        n
    }

    /// The day's books kept: the contracts' moves entered as income and on the loan books, every account and loan
    /// book held to what the positions show, and on a month's first day the banks' review of their standards.
    fn keep_books(&mut self, day: Day, calendar: &Calendar) {
        self.audit_taxes(day);
        self.audit_debt(day);
        self.account_moves();
        self.book_loans(day);
        self.audit_accounts(day);
        let date = calendar.date(day);
        self.review_lenders(day, i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month()));
    }

    /// Each estate that paid all it held today ended; returns them.
    fn end_settled(&mut self, settling: Vec<PartyKey>) -> u64 {
        let mut ended = 0;
        for estate in settling {
            // Its claims are settled as far as its money went; what it could not pay is the creditors' loss.
            self.insolvency.claims.remove(&estate);
            let empty = self.kinds.get(usize::from(estate.kind())).and_then(|k| k.accounts.as_ref()).is_some_and(|a| {
                a.balance.get(estate.slot()).unwrap_or(0) == 0 && a.pending.get(estate.slot()).unwrap_or(0) == 0
            });
            // Goods an estate holds wait for its liquidation, which sells them; until then it stays.
            let goods = self.goods.stocks.holdings(estate).any(|h| h.units != 0);
            if !empty || goods {
                let why = if goods { "its goods wait for their liquidation" } else { "its money waits to be paid out" };
                self.waiting.insert(estate, why);
            }
            if empty && !goods {
                self.waiting.remove(&estate);
                self.goods.stocks.end(estate);
                self.estates.retain(|(e, _, _)| *e != estate);
                if let Some(k) = self.kinds.get_mut(usize::from(estate.kind()))
                    && let Some(r) = k.parties.at(estate.slot())
                {
                    k.parties.end(r);
                }
                ended += 1;
            }
        }
        ended
    }

    /// Runs the core's day: every family's dues made flows, then each currency's flows settled on its country's
    /// business day or committed on its closed day.
    #[clause("SET.4", "SET.6", "MON.5")]
    pub fn run_day(&mut self, day: Day, calendar: &Calendar, streams: &Streams, order: &StreamDecl) -> CoreDay {
        let mut work = std::mem::take(&mut self.work);
        work.flows.reset(1);
        let mut record = CoreDay {
            day,
            flows: 0,
            settled: 0,
            failed: 0,
            committed: 0,
            failed_by: [0; crate::consts::reason::REASONS],
            arrears: 0,
            wages: 0,
            gross: 0,
            estates: 0,
            breaks: 0,
            ns: 0,
            lent: 0,
            lent_elsewhere: 0,
        };
        let before = self.money_totals();
        if self.central.recorded.is_none() {
            self.central.recorded = Some(self.issuer_held());
        }
        for family in &mut self.families {
            if let Some(buf) = work.flows.chunks_mut().first_mut() {
                record.flows += family.dues(day, calendar, &mut work.due, buf);
            }
        }
        if let Some(buf) = work.flows.chunks_mut().first_mut() {
            let mut taken = std::mem::take(buf);
            self.withhold(&mut taken);
            if let Some(buf) = work.flows.chunks_mut().first_mut() {
                *buf = taken;
            }
        }
        let mut settling = Vec::new();
        if let Some(buf) = work.flows.chunks_mut().first_mut() {
            settling = self.estates_pay(day, calendar, buf);
            buf.append(&mut self.pending);
            self.lend_shortfalls((day, calendar, streams), buf);
            for f in buf.iter().filter(|f| f.reason == crate::consts::reason::LENT) {
                record.lent += 1;
                if self.bank_of(f.payee) != Some(f.payer) {
                    record.lent_elsewhere += 1;
                }
            }
            // Every flow the day settles is one it made: its dues, the taxes withheld from them, estates and sales.
            record.flows = phx_rand::float::len_u64(buf.len());
            record.gross = buf.iter().filter(|f| f.denomination.is_money()).map(|f| i128::from(f.amount)).sum();
        }
        let high: Vec<u32> = self.kinds.iter().map(|k| k.parties.high_water()).collect();
        let ranges = Ranges::new(self.range_bits, &high);
        let banks = self.bank_kind.map_or(0, |b| {
            usize::try_from(self.kinds.get(usize::from(b)).map_or(0, |k| k.parties.high_water())).unwrap_or(0)
        });
        let mut deposits = deposits_of(self.kinds.iter(), banks);
        let closed = vec![false; banks];
        let mut failed: Vec<Flow> = Vec::new();
        for (country, issuer) in self.issuers.iter().enumerate() {
            let Ok(ccy) = u8::try_from(country) else { continue };
            work.flows.group(None, &ranges, Denom::money(ccy));
            let grouped = Grouped::new(&[&work.flows], &ranges);
            let mut b = books(&mut self.kinds, self.bank_kind.unwrap_or(u8::MAX), (&mut deposits, &closed), *issuer);
            if calendar.is_business(CountryId::new(ccy), day) {
                let lot = |p: PartyKey| {
                    streams.open(
                        order,
                        Subject::new(SubjectTag::Party, u64::from(p.word())),
                        day,
                        SubStep::S7b.ordinal(),
                    )
                };
                let out = work.settle.settle(None, &grouped, &ranges, &mut b, &lot);
                record.settled += out.settled;
                record.failed += phx_rand::float::len_u64(out.failed.len());
                failed.extend(out.failed.iter().map(|(f, _)| *f));
            } else {
                work.settle.commit(None, &grouped, &ranges, &mut b);
                record.committed += phx_rand::float::len_u64(grouped.end());
            }
        }
        let moved = self.after_settle(&mut work, &failed, (day, calendar, streams, order), &ranges);
        self.work = work;
        for f in &failed {
            if let Some(n) = record.failed_by.get_mut(usize::from(f.reason)) {
                *n += 1;
            }
        }
        record.arrears = self.after_settlement(day, calendar, &failed);
        record.wages = self.record_wages_settled(&failed);
        record.breaks += self.money_breaks((day, before), moved, &mut deposits);
        record.estates += self.end_settled(settling);
        self.keep_books(day, calendar);
        for k in &mut self.kinds {
            k.parties.close_day();
        }
        for f in &mut self.families {
            f.store.edges.close_day();
        }
        self.days.push(record);
        record
    }
}

/// A finding of the money family.
fn finding((day, clause): (Day, &'static str), owner: FindingOwner, size: i128, detail: String) -> Finding {
    Finding { family: "money", clause, owner, size, unit: Unit::Count, day, detail }
}
