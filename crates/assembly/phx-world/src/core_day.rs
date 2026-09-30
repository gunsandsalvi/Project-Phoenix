//! The core's day while the port runs beside the books: each family's contracts due today make their flows, a
//! contract's next date read from its schedule; and each currency's flows settle over the core's accounts on its
//! country's business days, and are committed to what the accounts have pending on its closed days.

use phx_core::calendar::Calendar;
use phx_core::calendar::period::ScheduleDates;
use std::collections::BTreeMap;

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_core::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};
use phx_core::settle::{Cause, Outcome, Settle};
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
#[derive(Debug, phx_macros::Saved)]
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
    /// Each schedule's place by its currency, class and first date, so one alike is found among few; indexed up to
    /// `alike_upto`, and so rebuilt after a load.
    #[saved(skip)]
    pub alike: BTreeMap<(u8, [u32; 3], phx_id::Date), Vec<u32>>,
    #[saved(skip)]
    pub alike_upto: usize,
}

/// A family's moves on its parties' books: each amount lent and balance written off, with the creditor whose book it
/// moves; each principal repaid, with its payer and creditor; and each change in a contract's arrears — what its payer
/// owes and its creditor is owed beyond its dates — with the two, the family's reason and whether its contract is
/// reckoned from terms.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct LoanMoves {
    pub lent: Vec<(PartyKey, i64)>,
    pub repaid: Vec<(PartyKey, PartyKey, i64)>,
    /// Each balance written off, with its creditor and its contract.
    pub written_off: Vec<(PartyKey, i64, u32)>,
    pub arrears: Vec<Arrears>,
}

/// A change in a contract's arrears.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Arrears {
    pub payer: PartyKey,
    pub payee: PartyKey,
    pub change: i64,
    pub reason: u8,
    pub terms: bool,
}

/// The state's laws on the core, by country: the income tax withheld from wages, the tax on products each final use
/// pays a unit spent, the
/// benefit for a job lost, each country's treasury the taxes are paid to, and the state pension.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct CoreState {
    pub withholding: Vec<Option<phx_ledger::levy::Withholding>>,
    pub consumption: Vec<Option<[f64; crate::consts::final_use::TAXED]>>,
    pub benefit: Vec<Option<if_state::kinds::BenefitLaw>>,
    #[saved(skip)]
    pub claim: Option<&'static phx_core::decisions::DecisionPointDecl<if_state::kinds::ClaimIn, bool>>,
    #[saved(skip)]
    pub included: Option<fn(f64, f64) -> f64>,
    /// Each country's day of the month after a tax is collected by which it is remitted.
    pub remit_day: Vec<Option<u32>>,
    /// Each country's treasury's payment order.
    pub order: Vec<Option<if_state::kinds::PaymentOrder>>,
    pub pension: Vec<Option<Pension>>,
}

/// A country's state pension as a retiree claims it, by sex, female first: the age it is paid from, the share of
/// retirees it covers, and the flat amount it pays monthly.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Pension {
    pub age: [f64; 2],
    pub coverage: [f64; 2],
    pub amount: [i64; 2],
}

/// What the core's day did: the flows made, settled, failed and committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
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
    pub made: i128,
    /// The value settled; what the payers' nets drew; the failures by cause, the payer's and the bank's; and the
    /// closing ring, the parties settled only by what they were paid in the same settlement, with what that paid.
    pub gross: i128,
    pub net: i128,
    pub fails: [u64; 2],
    pub ring: u64,
    pub ring_value: i128,
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

impl CoreDay {
    /// A currency's settlement added to the day's: its flows settled and failed, its values and its failures by cause.
    #[clause("SET.10")]
    fn settled_with(&mut self, out: &Outcome) {
        self.settled += out.settled;
        self.failed += phx_rand::float::len_u64(out.failed.len());
        self.gross += out.values.gross;
        self.net += out.values.net;
        self.ring += out.values.ring;
        self.ring_value += out.values.ring_value;
        for (_, cause) in &out.failed {
            let at = match cause {
                Cause::Payer => 0,
                Cause::Bank => 1,
            };
            if let Some(n) = self.fails.get_mut(at) {
                *n += 1;
            }
        }
    }
}

/// The day's working state, kept across days so a day allocates nothing once the heaviest has sized it.
#[derive(Debug, Default)]
pub struct Work {
    pub flows: FlowBufs,
    /// The fund stage's flows, apart from the day's, which the day's reads still read after it.
    pub fund: FlowBufs,
    pub settle: Settle,
    /// The families' contracts due today, one family after another, each ending where `due_ends` says; the take's
    /// own buffer they are copied from.
    pub due: Vec<u32>,
    pub due_ends: Vec<usize>,
    pub taken: Vec<u32>,
    /// The hazards' follows of the day.
    pub(crate) follows: crate::pop_rules::Follows,
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
        let agencies: Vec<PartyKey> = self.agencies.iter().flatten().copied().collect();
        let mut by: BTreeMap<u8, (i64, i64)> = BTreeMap::new();
        let mut add = |f: &Flow, sign: i64| {
            let e = by.entry(f.denomination.ccy()).or_insert((0, 0));
            e.0 += sign * f.amount;
            if agencies.contains(&f.payer) {
                e.1 += sign * f.amount;
            }
        };
        for f in self.work.flows.slices().flatten().filter(|f| f.reason == wage && f.denomination.is_money()) {
            add(f, 1);
        }
        for f in failed.iter().filter(|f| f.reason == wage && f.denomination.is_money()) {
            add(f, -1);
        }
        let mut all = 0;
        for (ccy, paid) in by {
            self.record_wages(ccy, paid);
            all += paid.0;
        }
        all
    }

    /// Each failed flow a dated contract made held on it in arrears, asked again with its next due: a contract past
    /// its last date is kept, due again on its schedule's next date, until it is paid; one whose last due settled
    /// ends. Returns the contracts in arrears.
    #[clause("HH.13", "BNK.17", "SET.16")]
    fn hold_arrears(&mut self, day: Day, calendar: &Calendar, failed: &[Flow]) -> u64 {
        let mut n = 0;
        // Where each contract finishing today stands in its family's list, kept as the list is taken from.
        let mut places: Vec<BTreeMap<u32, usize>> = self
            .families
            .iter()
            .map(|x| {
                let mut at = BTreeMap::new();
                for (i, e) in x.finishing.iter().enumerate() {
                    at.entry(*e).or_insert(i);
                }
                at
            })
            .collect();
        for f in failed {
            let Some((family, at_finishing)) = self
                .families
                .iter_mut()
                .zip(places.iter_mut())
                .find(|(x, _)| x.reason == f.reason && x.store.kinds.first() == Some(&f.payer.kind()))
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
            if let Some(i) = at_finishing.remove(&f.source) {
                family.finishing.swap_remove(i);
                if let Some(moved) = family.finishing.get(i) {
                    at_finishing.insert(*moved, i);
                }
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
    fn dues(&mut self, day: Day, calendar: &Calendar, due: &[u32], out: &mut Vec<Flow>) -> u64 {
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
            let law = self.state.withholding.get(usize::from(f.denomination.ccy())).and_then(Option::as_ref);
            let Some((a, gross)) = crate::core_taxes::withheld(f, law) else { continue };
            let id = self.kinds.get(usize::from(a.payer.kind())).and_then(|k| k.parties.id(a.payer.slot()));
            if id.is_some_and(crate::core_rates::sampled) {
                self.taxes.sample.push((gross, a.tax, a.ccy));
            }
            arising.push(a);
        }
        self.taxes.arising.append(&mut arising);
    }

    /// An estate begun with a party's money at its bank, to settle on its country's next business day.
    #[clause("PTY.9")]
    pub(crate) fn open_estate(&mut self, (bank, money): (u32, i64), (country, day): (CountryId, Day)) -> PartyKey {
        let Some(place) = self.bound.kinds.estate else {
            violation!(clause = "PTY.9", "an estate with no kind to hold it");
        };
        let id = phx_id::PartyId::new(self.next_id);
        self.next_id += 1;
        let Some(store) = self.kinds.get_mut(place) else {
            violation!(clause = "PTY.9", "an estate kind with no store");
        };
        // The estate's one record word is its country, where its kind declares its place.
        let record = [phx_num::MaybeI64::present(i64::from(country.get()))];
        let party = store.begin(id, &record, Some(phx_core::store::Opening { bank, balance: money }));
        let key = PartyKey::new(u8::try_from(place).unwrap_or(u8::MAX), party.slot());
        let at = self.keys.partition_point(|(i, _)| *i < id);
        self.keys.insert(at, (id, key));
        self.estates.push((key, country, day));
        key
    }

    /// Each estate opened before today, on its country's business day, pays its claims by rank, each rank in proportion
    /// to what it is owed as far as the money goes, and the rest to its owners, a share each — a firm's estate's are the
    /// firm's — or, where it has none, to the institution its country's inheritance law names, to which what it owns
    /// passes too; it is ended after the day's settlement once it holds nothing.
    #[clause("PTY.9", "POP.15", "L3", "TIME.7")]
    fn estates_pay(&mut self, day: Day, calendar: &Calendar, out: &mut Vec<Flow>) -> Vec<PartyKey> {
        let mut settling = Vec::new();
        let mut heirless = Vec::new();
        for (estate, country, opened) in &self.estates {
            if *opened >= day || !calendar.is_business(*country, day) {
                continue;
            }
            let c = usize::from(country.get());
            let institutions = [
                self.issuers.get(c).copied(),
                self.treasuries.get(c).copied().flatten(),
                self.agencies.get(c).copied().flatten(),
            ];
            let Some(to) =
                self.declared.heirless.get(c).and_then(|kind| crate::core_kinds::heirless_party(&institutions, *kind))
            else {
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
            let owners = self.owners.holders_of(*estate);
            let rest: Vec<(PartyKey, i64)> = if owners.is_empty() {
                vec![(to, left)]
            } else {
                let shares = crate::core_firms::apportion_amount(left, &vec![1; owners.len()]);
                owners.iter().copied().zip(shares).collect()
            };
            for (payee, amount) in rest.into_iter().filter(|(_, a)| *a > 0) {
                out.push(Flow {
                    payer: *estate,
                    payee,
                    amount,
                    source: estate.slot().get(),
                    denomination: Denom::money(country.get()),
                    reason: ESTATE,
                    order: 0,
                });
            }
            heirless.push((*estate, to));
            settling.push(*estate);
        }
        for (estate, to) in heirless {
            self.pass_holdings(estate, to);
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
    #[clause("MON.5", "MON.7", "MON.8", "MON.9", "MON.11", "N1", "REP.14")]
    pub(crate) fn money_breaks(
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
        (at, pool): ((Day, &Calendar, &Streams, &StreamDecl), Option<&phx_exec::Pool>),
        ranges: &Ranges,
    ) -> (i128, [i128; 3]) {
        let flows = &work.flows;
        phx_exec::trace::span("settle.account_flows", || self.account_flows(flows.slices().flatten(), failed));
        phx_exec::trace::span("settle.accrue_taxes", || {
            self.accrue_taxes((at.0, at.1), flows.slices().flatten(), failed);
        });
        let mut moved =
            phx_exec::trace::span("settle.money_moves", || self.money_moves(flows.slices().flatten(), failed));
        let (fund, fund_failed) =
            phx_exec::trace::span("settle.fund_stage", || self.fund_stage(work, (at, pool), ranges));
        phx_exec::trace::note(
            "settle.funding",
            &[("flows", phx_exec::trace::count(fund.len())), ("failed", phx_exec::trace::count(fund_failed.len()))],
        );
        let fund_moved = self.money_moves(&fund, &fund_failed);
        moved.0 += fund_moved.0;
        for (m, f) in moved.1.iter_mut().zip(fund_moved.1) {
            *m += f;
        }
        moved
    }

    /// The day's flows gathered beside its dues: the estates' payments, the day's sales and severance, the loans the
    /// firms short of their dues take, the agencies' funding, all ranked in their payers' orders; the day's lending
    /// and gross counted. Returns the estates paying out today.
    fn gather(
        &mut self,
        (day, calendar, streams): (Day, &Calendar, &Streams),
        (flows, families): (&mut FlowBufs, usize),
        record: &mut CoreDay,
    ) -> Vec<PartyKey> {
        // The families' dues are the first chunks, read as the day's; what the day adds goes in the last.
        let (made, rest) = flows.chunks_mut().split_at_mut(families);
        let Some(buf) = rest.first_mut() else {
            violation!(clause = "SET.4", "a day's flows with no chunk for what the day made", families = families);
        };
        let dues: usize = made.iter().map(Vec::len).sum();
        let settling = phx_exec::trace::span("settle.estates_pay", || self.estates_pay(day, calendar, buf));
        let (estates, pending) = (buf.len(), self.pending.len());
        buf.append(&mut self.pending);
        phx_exec::trace::span("settle.lend_shortfalls", || {
            self.lend_shortfalls((day, calendar, streams), (made, buf));
        });
        let lent = buf.len() - estates - pending;
        phx_exec::trace::span("settle.fund_agencies", || self.fund_agencies((made, buf)));
        phx_exec::trace::span("settle.order_payments", || self.order_payments((made, buf)));
        let count = phx_exec::trace::count;
        phx_exec::trace::note(
            "settle.gathered",
            &[
                ("dues", count(dues)),
                ("estates", count(estates)),
                ("pending", count(pending)),
                ("loans", count(lent)),
                ("all", count(dues + buf.len())),
            ],
        );
        for f in buf.iter().filter(|f| f.reason == crate::consts::reason::LENT) {
            record.lent += 1;
            if self.bank_of(f.payee) != Some(f.payer) {
                record.lent_elsewhere += 1;
            }
        }
        // Every flow the day settles is one it made: its dues, the taxes withheld from them, estates and sales.
        record.flows = phx_rand::float::len_u64(dues + buf.len());
        let today = made.iter().flatten().chain(buf.iter());
        record.made = today.filter(|f| f.denomination.is_money()).map(|f| i128::from(f.amount)).sum();
        settling
    }

    /// Each public agency funded by its treasury for what it pays today beyond what it holds: the state keeps one purse,
    /// its agencies drawing on it as they pay, within their appropriations.
    #[clause("SOC.8", "TRS.10")]
    fn fund_agencies(&self, (dues, buf): (&[Vec<Flow>], &mut Vec<Flow>)) {
        // Each agency's wages and its other payments due today, apart, since its treasury ranks them apart.
        let mut due: BTreeMap<PartyKey, ([i64; 2], u8)> = BTreeMap::new();
        let today = dues.iter().flatten().chain(buf.iter());
        for f in today.filter(|f| f.denomination.is_money() && self.agencies.contains(&Some(f.payer))) {
            let e = due.entry(f.payer).or_insert(([0, 0], f.denomination.ccy()));
            if let Some(v) = e.0.get_mut(usize::from(f.reason != WAGE)) {
                *v += f.amount;
            }
        }
        for (agency, ([wages, other], ccy)) in due {
            let (Some(Some(treasury)), Some(held)) = (
                self.treasuries.get(usize::from(ccy)).copied(),
                self.kinds
                    .get(usize::from(agency.kind()))
                    .and_then(|k| k.accounts.as_ref())
                    .and_then(|a| Some(a.balance.get(agency.slot())? + a.pending.get(agency.slot())?)),
            ) else {
                continue;
            };
            let for_wages = if wages > held { wages - held } else { 0 };
            let left = if held > wages { held - wages } else { 0 };
            let for_other = if other > left { other - left } else { 0 };
            for amount in [for_wages, for_other] {
                if amount > 0 {
                    buf.push(Flow {
                        payer: treasury,
                        payee: agency,
                        amount,
                        source: agency.slot().get(),
                        denomination: Denom::money(ccy),
                        reason: crate::consts::reason::FUNDED,
                        order: 0,
                    });
                }
            }
        }
    }

    /// The treasuries' and their agencies' payments ranked by each treasury's payment order: its debt service, its
    /// pensions, its benefits, its staff's wages and its purchases, each at its declared rank, so a treasury short of
    /// cash leaves unpaid the last of them first.
    #[clause("TRS.10", "TRS.5")]
    fn order_payments(&self, (dues, buf): (&mut [Vec<Flow>], &mut [Flow])) {
        use crate::consts::reason::{BENEFIT, FUNDED, PENSION, REPAID};
        let state = |p: PartyKey| self.treasuries.contains(&Some(p)) || self.agencies.contains(&Some(p));
        // An agency's first funding of the day is for its wages, the second for the rest.
        let mut funded_wages: std::collections::BTreeSet<PartyKey> = std::collections::BTreeSet::new();
        let today = dues.iter_mut().flatten().chain(buf.iter_mut());
        for f in today.filter(|f| f.denomination.is_money() && state(f.payer)) {
            let Some(Some(order)) = self.state.order.get(usize::from(f.denomination.ccy())).copied() else { continue };
            let agency = self.agencies.contains(&Some(f.payer));
            f.order = match f.reason {
                REPAID => order.debt_service,
                PENSION => order.pensions,
                BENEFIT => order.benefits,
                WAGE => order.wages,
                FUNDED if funded_wages.insert(f.payee) => order.wages,
                _ if agency || f.reason == FUNDED => order.purchases,
                _ => f.order,
            };
        }
    }

    /// What a day's settled money flows moved: into the parties other than banks from the banks and the issuers, and
    /// each class of the issuers' money, a flow taking from its payer's class and adding to its payee's.
    fn money_moves<'f>(&self, flows: impl IntoIterator<Item = &'f Flow>, failed: &[Flow]) -> (i128, [i128; 3]) {
        let mut unpaid: BTreeMap<(PartyKey, PartyKey, i64, u8, u32), u32> = BTreeMap::new();
        for f in failed {
            *unpaid.entry((f.payer, f.payee, f.amount, f.reason, f.source)).or_insert(0) += 1;
        }
        let money_maker = |k: PartyKey| self.bank_kind == Some(k.kind()) || self.issuers.contains(&k);
        let (mut net, mut classes) = (0_i128, [0_i128; 3]);
        for f in flows.into_iter().filter(|f| f.denomination.is_money()) {
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
        phx_exec::trace::span("settle.deliver_sales", || self.deliver_sales(day, failed));
        phx_exec::trace::span("settle.depart_shipments", || self.depart_shipments(day, failed));
        let n = phx_exec::trace::span("settle.hold_arrears", || self.hold_arrears(day, calendar, failed));
        phx_exec::trace::span("settle.note_arrears", || self.note_arrears(day, failed));
        n
    }

    /// The day's books kept: the contracts' moves entered as income and on the loan books, every account and loan
    /// book held to what the positions show, and on a month's first day the banks' review of their standards.
    fn keep_books(&mut self, day: Day, calendar: &Calendar) {
        phx_exec::trace::span("books.audit_taxes", || self.audit_taxes(day));
        phx_exec::trace::span("books.audit_debt", || self.audit_debt(day));
        phx_exec::trace::span("books.account_moves", || self.account_moves());
        phx_exec::trace::span("books.book_loans", || self.book_loans(day));
        phx_exec::trace::span("books.audit_accounts", || self.audit_accounts(day));
        let date = calendar.date(day);
        let month = i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month());
        phx_exec::trace::span("books.review_lenders", || self.review_lenders(day, month));
    }

    /// Each estate that paid all it held today ended; returns them.
    fn end_settled(&mut self, settling: Vec<PartyKey>) -> u64 {
        let mut ended = 0;
        let projects = if settling.is_empty() { BTreeMap::new() } else { self.project_costs() };
        let mut gone: std::collections::BTreeSet<PartyKey> = std::collections::BTreeSet::new();
        for estate in settling {
            // Its claims are settled as far as its money went; what it could not pay is the creditors' loss.
            self.insolvency.claims.remove(&estate);
            let empty = self.kinds.get(usize::from(estate.kind())).and_then(|k| k.accounts.as_ref()).is_some_and(|a| {
                a.balance.get(estate.slot()).unwrap_or(0) == 0 && a.pending.get(estate.slot()).unwrap_or(0) == 0
            });
            // Goods and rights to deposits an estate holds wait for its liquidation, which sells them; until then it
            // stays.
            let goods = self.goods.stocks.holdings(estate).any(|h| h.units != 0)
                || self.deposits.held.contains_key(&estate)
                || projects.get(&estate).is_some_and(|c| *c != 0);
            if !empty || goods {
                let why = if goods { crate::core::Waits::Liquidation } else { crate::core::Waits::PayingOut };
                self.waiting.insert(estate, why);
            }
            if empty && !goods {
                self.waiting.remove(&estate);
                self.goods.stocks.end(estate);
                gone.insert(estate);
                if let Some(k) = self.kinds.get_mut(usize::from(estate.kind()))
                    && let Some(r) = k.parties.at(estate.slot())
                {
                    k.parties.end(r);
                }
                ended += 1;
            }
        }
        self.estates_ended(&gone);
        self.estates.retain(|(e, _, _)| !gone.contains(e));
        ended
    }

    /// Each currency's flows grouped and settled on its country's business day, or committed on its closed day; the
    /// flows that failed.
    fn pay_currencies(
        &mut self,
        (work, pool): (&mut Work, Option<&phx_exec::Pool>),
        (ranges, deposits, closed): (&Ranges, &mut [i64], &[bool]),
        (day, calendar, streams, order): (Day, &Calendar, &Streams, &StreamDecl),
        record: &mut CoreDay,
    ) -> Vec<Flow> {
        let mut failed: Vec<Flow> = Vec::new();
        for (country, issuer) in self.issuers.iter().enumerate() {
            let Ok(ccy) = u8::try_from(country) else { continue };
            phx_exec::trace::span("settle.group", || work.flows.group(pool, ranges, Denom::money(ccy)));
            let grouped = Grouped::new(&[&work.flows], ranges);
            let mut b = books(&mut self.kinds, self.bank_kind.unwrap_or(u8::MAX), (&mut *deposits, closed), *issuer);
            let business = calendar.is_business(CountryId::new(ccy), day);
            let count = phx_exec::trace::count;
            phx_exec::trace::note(
                "settle.currency",
                &[("currency", i64::from(ccy)), ("flows", count(grouped.end())), ("business", i64::from(business))],
            );
            if business {
                let lot = |p: PartyKey| {
                    streams.open(
                        order,
                        Subject::new(SubjectTag::Party, u64::from(p.word())),
                        day,
                        SubStep::S7b.ordinal(),
                    )
                };
                let out = phx_exec::trace::span("settle.fixed_point", || {
                    work.settle.settle(pool, &grouped, ranges, &mut b, &lot)
                });
                note_outcome(ccy, &out);
                record.settled_with(&out);
                failed.extend(out.failed.iter().map(|(f, _)| *f));
            } else {
                work.settle.commit(pool, &grouped, ranges, &mut b);
                record.committed += phx_rand::float::len_u64(grouped.end());
            }
        }
        failed
    }

    /// Each family's contracts due today taken, then its dues made into its own chunk of the day's flows, a family a
    /// chunk on the pool, and the wages among them withheld.
    fn make_dues(
        &mut self,
        work: &mut Work,
        (day, calendar, pool): (Day, &Calendar, Option<&phx_exec::Pool>),
        record: &mut CoreDay,
    ) {
        let families = self.families.len();
        work.due.clear();
        work.due_ends.clear();
        for family in &mut self.families {
            family.store.wheel.take(day, &mut work.taken, pool);
            work.due.extend_from_slice(&work.taken);
            work.due_ends.push(work.due.len());
        }
        // Each family makes its dues into its own chunk of the day's flows, a family a chunk on the pool.
        let (due, ends, flows) = (&work.due, &work.due_ends, work.flows.chunks_mut());
        let Some(bufs) = flows.get_mut(..families) else {
            violation!(clause = "SET.4", "a day's flows with no chunk for a family's dues", families = families);
        };
        phx_exec::for_each_pair(pool, (self.families.as_mut_slice(), bufs), |i, family, buf| {
            let from = match i.checked_sub(1) {
                Some(before) => ends.get(before).copied(),
                None => Some(0),
            };
            let Some(due) = from.zip(ends.get(i)).and_then(|(from, to)| due.get(from..*to)) else {
                violation!(clause = "TIME.4", "a family with no dues taken", family = i);
            };
            let _ = family.dues(day, calendar, due, buf);
        });
        // A family's chunk begins the day empty, so what it holds is what it made.
        for (family, buf) in self.families.iter().zip(work.flows.slices()) {
            let n = u64::try_from(buf.len()).unwrap_or(u64::MAX);
            phx_exec::trace::note(family.name, &[("dues", i64::try_from(n).unwrap_or(i64::MAX))]);
            record.flows += n;
        }
        for buf in work.flows.chunks_mut().iter_mut().take(families) {
            self.withhold(buf);
        }
    }

    /// Runs the core's day: every family's dues made flows, then each currency's flows settled on its country's
    /// business day or committed on its closed day.
    #[clause("SET.4", "SET.6", "MON.5")]
    pub fn run_day(
        &mut self,
        (day, calendar, streams, order): (Day, &Calendar, &Streams, &StreamDecl),
        (clock, pool): (Option<&dyn phx_exec::Clock>, Option<&phx_exec::Pool>),
    ) -> CoreDay {
        let mut work = std::mem::take(&mut self.work);
        // A chunk for each family's dues, then one for the flows the day's stages made: fixed by the makers, never by
        // the workers, so a payer's flows keep one order however many run them.
        let families = self.families.len();
        work.flows.reset(families + 1);
        let mut record = CoreDay {
            day,
            flows: 0,
            settled: 0,
            failed: 0,
            committed: 0,
            failed_by: [0; crate::consts::reason::REASONS],
            arrears: 0,
            wages: 0,
            made: 0,
            gross: 0,
            net: 0,
            fails: [0; 2],
            ring: 0,
            ring_value: 0,
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
        self.timed(clock, "settle.dues", |c| c.make_dues(&mut work, (day, calendar, pool), &mut record));
        let settling = self.timed(clock, "settle.gather", |c| {
            c.gather((day, calendar, streams), (&mut work.flows, families), &mut record)
        });
        work.flows.split_last(phx_core::consts::FLOW_GROUP_COST);
        let high: Vec<u32> = self.kinds.iter().map(|k| k.parties.high_water()).collect();
        let ranges = Ranges::new(self.range_bits, &high);
        let banks = self.bank_kind.map_or(0, |b| {
            usize::try_from(self.kinds.get(usize::from(b)).map_or(0, |k| k.parties.high_water())).unwrap_or(0)
        });
        let mut deposits = deposits_of(self.kinds.iter(), banks);
        let closed = vec![false; banks];
        let failed = self.timed(clock, "settle.money", |c| {
            c.pay_currencies(
                (&mut work, pool),
                (&ranges, deposits.as_mut_slice(), &closed),
                (day, calendar, streams, order),
                &mut record,
            )
        });
        let moved = self.timed(clock, "settle.after", |c| {
            c.after_settle(&mut work, &failed, ((day, calendar, streams, order), pool), &ranges)
        });
        self.work = work;
        for f in &failed {
            if let Some(n) = record.failed_by.get_mut(usize::from(f.reason)) {
                *n += 1;
            }
        }
        self.timed(clock, "settle.close", |c| {
            record.arrears = c.after_settlement(day, calendar, &failed);
            record.wages = phx_exec::trace::span("settle.wages", || c.record_wages_settled(&failed));
            record.breaks +=
                phx_exec::trace::span("settle.money_breaks", || c.money_breaks((day, before), moved, &mut deposits));
            record.estates += phx_exec::trace::span("settle.end_settled", || c.end_settled(settling));
            phx_exec::trace::span("settle.keep_books", || c.keep_books(day, calendar));
        });
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

/// What a currency's settlement came to, for the bench's trace.
fn note_outcome(ccy: u8, out: &Outcome) {
    let n = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
    let count = phx_exec::trace::count;
    phx_exec::trace::note(
        "settle.outcome",
        &[
            ("currency", i64::from(ccy)),
            ("settled", n(out.settled)),
            ("failed", count(out.failed.len())),
            ("held", count(out.held.len())),
            ("rounds", n(out.rounds)),
            ("visits", n(out.visits)),
        ],
    );
}

impl CoreDay {
    /// The day's settlement in counts, for the bench's trace.
    pub(crate) fn note(&self) {
        let n = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
        phx_exec::trace::note(
            "settle",
            &[
                ("flows", n(self.flows)),
                ("settled", n(self.settled)),
                ("failed", n(self.failed)),
                ("committed", n(self.committed)),
                ("arrears", n(self.arrears)),
                ("estates", n(self.estates)),
                ("breaks", n(self.breaks)),
            ],
        );
    }
}

/// A finding of the money family.
fn finding((day, clause): (Day, &'static str), owner: FindingOwner, size: i128, detail: String) -> Finding {
    Finding { family: "money", clause, owner, size, unit: Unit::Count, day, detail }
}
