//! The central bank's facilities on the core. Each country's corridor is two standing facilities, each position an
//! overnight contract between its central bank and a bank: the deposit facility pays its rate on reserves placed with
//! it, the lending facility lends overnight against a bank's firm loans at its rate. At each business day's fund stage,
//! after settlement, every position is returned with its days' interest and each bank asks anew; the central bank's net
//! interest is remitted to its treasury on a month's first fund stage, a loss kept against its equity. A bank's
//! reserves may fall below nothing within a day by its intraday credit; one left below nothing after the fund stage has
//! the shortfall standing as the central bank's overdue claim.

use std::collections::BTreeMap;

use if_credit::central::{Corridor, RequestIn};
use phx_core::calendar::Calendar;
use phx_core::calendar::daycount::DayCount;
use phx_core::flows::{Denom, Flow, Grouped, Ranges};
use phx_core::slots::DaySlot;
use phx_core::store::{books, deposits_of};
use phx_core::{OpeningCountry, Register, StreamDecl, WorldStreams};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_ledger::algebra::{Leg, Reference, Schedule};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};

use crate::consts::reason::{LENT, REMITTED, REPAID};
use crate::consts::{DAYS_A_YEAR, KIND_ROWS_PER_CHUNK};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due};
use phx_core::capacity::KIND_ROWS;

/// The deposit facility's positions, each owed by a central bank to a bank.
pub const DEPOSIT_FACILITY: &str = crate::consts::families::DEPOSIT_FACILITY;
/// The lending facility's positions, each owed by a bank to its central bank.
pub const LENDING_FACILITY: &str = crate::consts::families::LENDING_FACILITY;

/// A country's fund stage of a day: what its banks placed and borrowed and how many did each, what stood overdue after
/// it and in how many banks, the positions a bank could not return, and what the central bank remitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct FundDay {
    pub day: u32,
    pub country: u8,
    pub placed: i64,
    pub borrowed: i64,
    pub placing: u64,
    pub borrowing: u64,
    pub overdue: i64,
    pub overdue_banks: u64,
    pub unreturned: u64,
    pub remitted: i64,
}

/// The issuer's money by what it owes it as: banks' reserves, the state's accounts — its treasury's and its
/// agencies' — and banknotes.
pub const RESERVES: usize = 0;
pub const TREASURY_ACCOUNT: usize = 1;
pub const NOTES: usize = 2;

/// The central banks on the core: each country's corridor, the day each position opened, each central bank's net interest since its last remittance and the losses it kept,
/// the month it last remitted in, the fund stages' days, and the issuers' money as they record it.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Central {
    pub corridors: Vec<Corridor>,
    opened: BTreeMap<(usize, u32), Day>,
    pub income: Vec<i128>,
    pub kept: Vec<i128>,
    remitted_in: Vec<Option<i64>>,
    pub days: Vec<FundDay>,
    pub recorded: Option<[i128; 3]>,
}

/// One move of the fund stage, by the flow it makes: a position returned with its interest, a position opened, or a
/// central bank's income remitted.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    Return {
        family: usize,
        edge: u32,
        principal: i64,
        interest: i64,
        country: usize,
        lending: bool,
    },
    Open {
        family: usize,
        bank: PartyKey,
        issuer: PartyKey,
        amount: i64,
        lending: bool,
        country: usize,
    },
    Remit {
        country: usize,
        amount: i64,
    },
    /// Bills a bank won at its country's auction: how many, at what price a unit of face, and what it pays.
    Buy {
        bank: PartyKey,
        treasury: PartyKey,
        bills: i64,
        price: f64,
        pay: i64,
        country: usize,
    },
}

impl Core {
    fn facility_family(&mut self, name: &'static str, kinds: [u8; 2], today: Day) -> DatedFamily {
        let rows = kinds.map(|k| self.kind_rows(usize::from(k)));
        DatedFamily {
            name,
            store: phx_core::store::Family::new(
                &mut self.space,
                (kinds, rows),
                (KIND_ROWS, KIND_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), phx_core::capacity::WHEEL_DAYS),
            ),
            reason: REPAID,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            alike: BTreeMap::new(),
            alike_upto: 0,
            moves: crate::core_day::LoanMoves::default(),
            lost: 0,
        }
    }

    /// The facilities opened: each country's corridor; the two facilities' families; each bank's intraday credit, as
    /// much as every bank of its country holds in reserves at the opening; and the central bank's loans to banks the
    /// country's sheet holds, shared over its banks by their weights, each a position at the lending facility.
    ///
    /// # Errors
    /// A primitive of the corridor the register does not hold.
    #[clause("CB.7", "MON.3", "GEN.2")]
    pub fn open_central(
        &mut self,
        register: &Register,
        (countries, sheets): (&[OpeningCountry], &[crate::opening::sheet::Sheet]),
        (calendar, today): (&Calendar, Day),
    ) -> Result<(), String> {
        let (Some(cb), Some(bank)) = (self.bound.kinds.central_bank, self.bank_kind) else {
            return Ok(());
        };
        let corridors =
            countries.iter().map(|c| sys_cb::central::corridor(register, c)).collect::<Result<Vec<_>, String>>()?;
        let n = countries.len();
        self.central = Central {
            corridors,
            income: vec![0; n],
            kept: vec![0; n],
            remitted_in: vec![None; n],
            ..Central::default()
        };
        let deposit = self.facility_family(DEPOSIT_FACILITY, [kind_number(cb), bank], today);
        let lending = self.facility_family(LENDING_FACILITY, [bank, kind_number(cb)], today);
        self.add_family(deposit);
        self.add_family(lending);
        for (c, (country, sheet)) in countries.iter().zip(sheets).enumerate() {
            let banks = self.banks_of.get(c).cloned().unwrap_or_default();
            let reserves: i64 = banks
                .iter()
                .filter_map(|(s, _)| self.kinds.get(usize::from(bank))?.accounts.as_ref()?.balance.get(Slot::new(*s)))
                .sum();
            if let Some(a) = self.kinds.get_mut(usize::from(bank)).and_then(|k| k.accounts.as_mut()) {
                for (s, _) in &banks {
                    a.facility.set(Slot::new(*s), reserves);
                }
            }
            let total = phx_ledger::opening::whole(
                sheet.at(crate::consts::sheet::CENTRAL_BANK_LOANS, crate::consts::sheet::CENTRAL_BANK) * country.gdp,
            );
            let weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
            let parts = self.apportion(("central bank loans", country.id.get()), total, &weights);
            let Some(issuer) = self.issuers.get(c).copied() else { continue };
            for ((s, _), amount) in banks.iter().zip(parts).filter(|(_, a)| *a > 0) {
                let step = Step::Open {
                    family: self.families.len() - 1,
                    bank: PartyKey::new(bank, Slot::new(*s)),
                    issuer,
                    amount,
                    lending: true,
                    country: c,
                };
                self.open_position(step, (today, calendar.date(today)));
            }
        }
        Ok(())
    }

    /// A position opened at its facility's rate, on its day.
    fn open_position(&mut self, step: Step, (day, date): (Day, phx_id::Date)) {
        let Step::Open { family, bank, issuer, amount, lending, country } = step else { return };
        let Some(corridor) = self.central.corridors.get(country).copied() else { return };
        let rate = if lending { corridor.lending_rate } else { corridor.deposit_rate };
        let Ok(ccy) = u8::try_from(country) else { return };
        let Some(days) = phx_core::calendar::period::Period::days(1) else {
            violation!(clause = "TIME.4", "a day that is no period");
        };
        let dates = phx_core::calendar::period::ScheduleDates {
            anchor: date,
            period: days,
            eom: phx_core::calendar::period::EndOfMonth::Plain,
            convention: phx_core::calendar::bizday::BusinessDayConvention::Following,
            country: CountryId::new(ccy),
        };
        let terms = phx_ledger::opening::plain_terms(
            phx_ledger::opening::currency(CountryId::new(ccy)),
            vec![
                Leg::RateOnNotional {
                    reference: Reference::Fixed(sys_bnk::rate(rate * crate::consts::PERCENT)),
                    day_count: DayCount::Act365F,
                },
                Leg::Amortising,
            ],
            Schedule { dates, count: Missing::Present(1) },
        );
        let ends = if lending { [bank, issuer] } else { [issuer, bank] };
        let Some(f) = self.families.get_mut(family) else { return };
        let schedule = f.schedule_in(ccy, [0, 0, 0], terms);
        let edge = f.store.open(Due { ends, amount, nth: 1, schedule, person: 0, arrears: 0 }, None);
        f.moves.lent.push((ends[1], amount));
        self.central.opened.insert((family, edge.get()), day);
    }

    /// The day's fund stage, after settlement, for each country on its business day: every position returned with its
    /// interest over the days it stood, each central bank's income remitted to its treasury on a month's first stage,
    /// and each bank's request at the facilities — its chief executive's, from its reserves after the returns, its
    /// target and its collateral — all settled together between the banks and their issuer. Returns the flows made and
    /// those that failed.
    #[clause("CB.7", "CB.10", "MON.3", "MON.6", "TIME.6", "BFL.10", "MKT.8")]
    pub(crate) fn fund_stage(
        &mut self,
        work: &mut crate::core_day::Work,
        ((day, calendar, streams, order), pool): (
            (Day, &Calendar, &WorldStreams, &StreamDecl),
            Option<&phx_exec::Pool>,
        ),
        ranges: &Ranges,
    ) -> (Vec<Flow>, Vec<Flow>) {
        let (Some(deposit), Some(lending)) =
            (self.bound.families.deposit_facility, self.bound.families.lending_facility)
        else {
            return (Vec::new(), Vec::new());
        };
        let open: Vec<usize> = (0..self.issuers.len())
            .filter(|c| u8::try_from(*c).is_ok_and(|c| calendar.is_business(CountryId::new(c), day)))
            .collect();
        if open.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let mut steps = self.returns((deposit, lending), &open, day);
        steps.extend(self.remittances(&open, calendar.date(day)));
        let buys = self.auctions(&open, (day, calendar), &steps);
        steps.extend(buys);
        steps.extend(self.requests((deposit, lending), &open, &steps));
        let flows: Vec<Flow> = (0_u32..).zip(&steps).filter_map(|(i, s)| self.flow_of(*s, i)).collect();
        work.fund.reset(1);
        if let Some(buf) = work.fund.chunks_mut().first_mut() {
            buf.extend_from_slice(&flows);
        }
        let banks = self.bank_kind.map_or(0, |b| usize::try_from(self.directory.high_water(b)).unwrap_or(0));
        let mut deposits = deposits_of(self.kinds.iter(), banks);
        let closed = vec![false; banks];
        let mut failed: Vec<Flow> = Vec::new();
        for c in &open {
            let (Ok(ccy), Some(issuer)) = (u8::try_from(*c), self.issuers.get(*c).copied()) else { continue };
            work.fund.group(pool, ranges, Denom::money(ccy));
            let grouped = Grouped::new(&[&work.fund], ranges);
            let mut b = books(&mut self.kinds, self.bank_kind.unwrap_or(u8::MAX), (&mut deposits, &closed), issuer);
            let lot = |p: PartyKey| {
                streams.open_at(
                    order,
                    Subject::new(SubjectTag::Party, u64::from(p.word())),
                    day,
                    DaySlot::S8e.ordinal(),
                )
            };
            let out = work.settle.settle(pool, &grouped, ranges, &mut b, &lot);
            failed.extend(out.failed.iter().map(|(f, _)| *f));
        }
        let unpaid: std::collections::BTreeSet<u32> = failed.iter().map(|f| f.source).collect();
        self.after_fund_stage(&steps, &unpaid, day, calendar.date(day));
        self.issue_bills(&steps, &unpaid, (day, calendar));
        self.account_flows(&flows, &failed);
        self.record_fund_days(&steps, &unpaid, &open, day);
        (flows, failed)
    }

    /// Every position of the open countries returned, with its interest over the days since it opened.
    fn returns(&self, (deposit, lending): (usize, usize), open: &[usize], day: Day) -> Vec<Step> {
        let mut steps = Vec::new();
        for (family, is_lending) in [(deposit, false), (lending, true)] {
            let Some(f) = self.families.get(family) else { continue };
            for edge in f.store.edges.open_slots() {
                let Some(row) = f.store.edges.row(edge) else { continue };
                let issuer = if is_lending { row.ends[1] } else { row.ends[0] };
                let Some(country) = self.issuers.iter().position(|i| *i == issuer).filter(|c| open.contains(c)) else {
                    continue;
                };
                let Some(corridor) = self.central.corridors.get(country) else { continue };
                let rate = if is_lending { corridor.lending_rate } else { corridor.deposit_rate };
                let since = self.central.opened.get(&(family, edge.get())).copied().unwrap_or(day);
                let days = phx_rand::float::from_i64(i64::from(day.get()) - i64::from(since.get()));
                let interest =
                    phx_ledger::opening::whole(phx_rand::float::from_i64(row.amount) * rate * days / DAYS_A_YEAR);
                steps.push(Step::Return {
                    family,
                    edge: edge.get(),
                    principal: row.amount,
                    interest,
                    country,
                    lending: is_lending,
                });
            }
        }
        steps
    }

    /// On each open country's first fund stage of a month, its central bank's net interest since its last: a surplus
    /// remitted to its treasury, a loss kept against its equity.
    fn remittances(&mut self, open: &[usize], date: phx_id::Date) -> Vec<Step> {
        let month = i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month());
        let mut steps = Vec::new();
        for c in open {
            let Some(last) = self.central.remitted_in.get_mut(*c) else { continue };
            let first = last.is_none();
            if *last == Some(month) {
                continue;
            }
            *last = Some(month);
            if first {
                continue;
            }
            let income = self.central.income.get(*c).copied().unwrap_or(0);
            if let Some(i) = self.central.income.get_mut(*c) {
                *i = 0;
            }
            match i64::try_from(income) {
                Ok(amount) if amount > 0 => steps.push(Step::Remit { country: *c, amount }),
                _ => {
                    if let Some(k) = self.central.kept.get_mut(*c) {
                        *k += income;
                    }
                }
            }
        }
        steps
    }

    /// Each bank of the open countries asks the facilities, from its reserves after the stage's returns: what it holds
    /// above its target placed, what it lacks borrowed as far as its collateral lends — its firm loans' balances less
    /// the corridor's haircut. Its target is its reserves over its deposits at its first fund stage, times its
    /// deposits now.
    #[clause("CB.7", "CB.6", "MND.20")]
    fn requests(&mut self, (deposit, lending): (usize, usize), open: &[usize], before: &[Step]) -> Vec<Step> {
        let Some(bank) = self.bank_kind else { return Vec::new() };
        let after = self.reserves_moved(before);
        let mut collateral: BTreeMap<PartyKey, i128> = BTreeMap::new();
        if let Some(f) = self.bound.families.firm_loans.and_then(|i| self.families.get(i)) {
            for edge in f.store.edges.open_slots() {
                if let Some(row) = f.store.edges.row(edge) {
                    *collateral.entry(row.ends[1]).or_insert(0) += i128::from(row.amount);
                }
            }
        }
        let deposits = self.bank_deposits();
        let requesting = self.point(|p| p.request, &sys_bnk::points::REQUEST);
        let mut steps = Vec::new();
        for c in open {
            let (Some(issuer), Some(corridor)) =
                (self.issuers.get(*c).copied(), self.central.corridors.get(*c).copied())
            else {
                continue;
            };
            for (s, _) in self.banks_of.get(*c).cloned().unwrap_or_default() {
                let key = PartyKey::new(bank, Slot::new(s));
                let Some((reserves, target)) = self.reserves_and_target(key, &after, &deposits) else { continue };
                let lends = phx_ledger::opening::whole(
                    (1.0 - corridor.haircut)
                        * i64::try_from(collateral.get(&key).copied().unwrap_or(0))
                            .map_or(0.0, phx_rand::float::from_i64),
                );
                let r = self.decide(requesting, key, |_| RequestIn { reserves, target, collateral: lends });
                if r.place > 0 {
                    steps.push(Step::Open {
                        family: deposit,
                        bank: key,
                        issuer,
                        amount: r.place,
                        lending: false,
                        country: *c,
                    });
                }
                if r.borrow > 0 {
                    steps.push(Step::Open {
                        family: lending,
                        bank: key,
                        issuer,
                        amount: r.borrow,
                        lending: true,
                        country: *c,
                    });
                }
            }
        }
        steps
    }

    /// What the stage's earlier steps move each bank's reserves by: its positions returned, the bills it buys.
    pub(crate) fn reserves_moved(&self, steps: &[Step]) -> BTreeMap<PartyKey, i64> {
        let mut after: BTreeMap<PartyKey, i64> = BTreeMap::new();
        for s in steps {
            match *s {
                Step::Return { family, edge, principal, interest, lending: is_lending, .. } => {
                    let Some(row) = self.families.get(family).and_then(|f| f.store.edges.row(Slot::new(edge))) else {
                        continue;
                    };
                    let (b, v) = if is_lending {
                        (row.ends[0], -(principal + interest))
                    } else {
                        (row.ends[1], principal + interest)
                    };
                    *after.entry(b).or_insert(0) += v;
                }
                Step::Buy { bank, pay, .. } => *after.entry(bank).or_insert(0) -= pay,
                _ => {}
            }
        }
        after
    }

    /// Each bank's deposits, what it owes its customers.
    pub(crate) fn bank_deposits(&self) -> Vec<i64> {
        let banks = self.bank_kind.map_or(0, |b| usize::try_from(self.directory.high_water(b)).unwrap_or(0));
        deposits_of(self.kinds.iter(), banks)
    }

    /// A bank's reserves after the stage's earlier steps, and its target: its reserves over its deposits at its first
    /// fund stage, times its deposits now.
    pub(crate) fn reserves_and_target(
        &mut self,
        key: PartyKey,
        after: &BTreeMap<PartyKey, i64>,
        deposits: &[i64],
    ) -> Option<(i64, i64)> {
        let a = self.kinds.get(usize::from(key.kind()))?.accounts.as_ref()?;
        let reserves = a.balance.get(key.slot())? + a.pending.get(key.slot())? + after.get(&key).copied().unwrap_or(0);
        let owed = deposits.get(usize::try_from(key.slot().get()).unwrap_or(usize::MAX)).copied().unwrap_or(0);
        // Its target is set at its first fund stage, its row's word the share every stage reads after.
        let banks = self.banks.as_mut().filter(|b| b.kind() == key.kind())?;
        if banks.reserve_target(key.slot()).is_none() {
            let first =
                if owed > 0 { phx_rand::float::from_i64(reserves) / phx_rand::float::from_i64(owed) } else { 0.0 };
            banks.set_reserve_target(key.slot(), first);
        }
        let ratio = banks.reserve_target(key.slot())?;
        Some((reserves, phx_ledger::opening::whole(ratio * phx_rand::float::from_i64(owed))))
    }

    /// The flow a step makes, its source the step's place: a return first in its payer's order, then what it opens.
    fn flow_of(&self, step: Step, at: u32) -> Option<Flow> {
        let (from, to, amount, reason, order, country) = match step {
            Step::Return { family, edge, principal, interest, country, .. } => {
                let row = self.families.get(family)?.store.edges.row(Slot::new(edge))?;
                (row.ends[0], row.ends[1], principal + interest, REPAID, 0, country)
            }
            Step::Open { bank, issuer, amount, lending, country, .. } => {
                let (lender, borrower) = if lending { (issuer, bank) } else { (bank, issuer) };
                (lender, borrower, amount, LENT, 1, country)
            }
            Step::Remit { country, amount } => {
                (*self.issuers.get(country)?, (*self.treasuries.get(country)?)?, amount, REMITTED, 0, country)
            }
            Step::Buy { bank, treasury, pay, country, .. } => (bank, treasury, pay, LENT, 0, country),
        };
        let ccy = u8::try_from(country).ok()?;
        Some(Flow { payer: from, payee: to, amount, source: at, denomination: Denom::money(ccy), reason, order })
    }

    /// The stage's steps that settled made: a position returned closed, its principal off its creditor's book and its
    /// interest in its central bank's income; a position asked for opened. A return that failed leaves its position
    /// standing, overdue.
    fn after_fund_stage(
        &mut self,
        steps: &[Step],
        unpaid: &std::collections::BTreeSet<u32>,
        day: Day,
        date: phx_id::Date,
    ) {
        for (at, step) in (0_u32..).zip(steps) {
            if unpaid.contains(&at) {
                continue;
            }
            match *step {
                Step::Return { family, edge, principal, interest, country, lending } => {
                    let Some(f) = self.families.get_mut(family) else { continue };
                    let Some(row) = f.store.edges.row(Slot::new(edge)) else { continue };
                    f.moves.repaid.push((row.ends[0], row.ends[1], principal));
                    f.store.close(Slot::new(edge));
                    self.central.opened.remove(&(family, edge));
                    if let Some(i) = self.central.income.get_mut(country) {
                        *i += if lending { i128::from(interest) } else { -i128::from(interest) };
                    }
                }
                Step::Open { .. } => self.open_position(*step, (day, date)),
                Step::Remit { .. } | Step::Buy { .. } => {}
            }
        }
    }

    /// Each open country's fund stage recorded: its placements and borrowings, the positions left unreturned, what
    /// was remitted, and each bank's reserves below nothing after it, the central bank's overdue claim.
    fn record_fund_days(&mut self, steps: &[Step], unpaid: &std::collections::BTreeSet<u32>, open: &[usize], day: Day) {
        let mut by: BTreeMap<usize, FundDay> = open
            .iter()
            .map(|c| {
                (*c, FundDay { day: day.get(), country: u8::try_from(*c).unwrap_or(u8::MAX), ..FundDay::default() })
            })
            .collect();
        for (at, step) in (0_u32..).zip(steps) {
            let paid = !unpaid.contains(&at);
            match *step {
                Step::Return { country, .. } if !paid => {
                    if let Some(d) = by.get_mut(&country) {
                        d.unreturned += 1;
                    }
                }
                Step::Open { amount, lending, country, .. } if paid => {
                    if let Some(d) = by.get_mut(&country) {
                        if lending {
                            (d.borrowed, d.borrowing) = (d.borrowed + amount, d.borrowing + 1);
                        } else {
                            (d.placed, d.placing) = (d.placed + amount, d.placing + 1);
                        }
                    }
                }
                Step::Remit { country, amount } if paid => {
                    if let Some(d) = by.get_mut(&country) {
                        d.remitted += amount;
                    }
                }
                _ => {}
            }
        }
        if let Some(bank) = self.bank_kind
            && let Some(a) = self.kinds.get(usize::from(bank)).and_then(|k| k.accounts.as_ref())
        {
            for (c, d) in &mut by {
                for (s, _) in self.banks_of.get(*c).map_or(&[][..], Vec::as_slice) {
                    let slot = Slot::new(*s);
                    let held = a.balance.get(slot).unwrap_or(0) + a.pending.get(slot).unwrap_or(0);
                    if held < 0 {
                        (d.overdue, d.overdue_banks) = (d.overdue - held, d.overdue_banks + 1);
                    }
                }
            }
        }
        self.central.days.extend(by.into_values());
    }

    /// What the issuers owe as money, by class: every bank's reserves, the treasuries' accounts at them, and the
    /// banknotes every other party holds there.
    #[must_use]
    pub fn issuer_held(&self) -> [i128; 3] {
        let mut held = [0_i128; 3];
        for (k, store) in self.kinds.iter().enumerate() {
            let Some(a) = store.accounts.as_ref() else { continue };
            let traits = crate::core_kinds::of(&self.declared.kinds, k);
            let bank = traits.takes_deposits;
            for ((b, m), p) in a.bank.slice().iter().zip(a.balance.slice()).zip(a.pending.slice()) {
                let class = if bank {
                    RESERVES
                } else if *b != phx_core::settle::AT_ISSUER {
                    continue;
                } else if traits.owners == phx_core::kinds::Owners::State {
                    TREASURY_ACCOUNT
                } else {
                    NOTES
                };
                if let Some(h) = held.get_mut(class) {
                    *h += i128::from(*m) + i128::from(*p);
                }
            }
        }
        held
    }

    /// The class of the issuer's money a flow's side moves: a bank's reserves, or those of the bank its account is at;
    /// a treasury's account; banknotes; none for the issuer itself.
    pub(crate) fn money_class(&self, p: PartyKey) -> Option<usize> {
        if self.issuers.contains(&p) {
            return None;
        }
        let traits = crate::core_kinds::of(&self.declared.kinds, usize::from(p.kind()));
        if traits.takes_deposits {
            return Some(RESERVES);
        }
        let b = self.kinds.get(usize::from(p.kind()))?.accounts.as_ref()?.bank.get(p.slot())?;
        if b != phx_core::settle::AT_ISSUER {
            Some(RESERVES)
        } else if traits.owners == phx_core::kinds::Owners::State {
            Some(TREASURY_ACCOUNT)
        } else {
            Some(NOTES)
        }
    }
}
