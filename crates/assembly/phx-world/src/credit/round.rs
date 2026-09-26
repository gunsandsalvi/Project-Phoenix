//! The day's round of credit at 5c, in order: the quotes made the day before chosen among by their borrowers; the
//! applications sent the day before quoted or declined by their banks; and the borrowers whose loans fall due within
//! their lead applying to refinance them. At 7c each loan taken is written; on the first round of a month each bank
//! reviews its book. A loan therefore takes three days from its application.

use std::collections::BTreeMap;

use if_credit::decisions::{ChooseIn, DeclineIn, QuoteIn, StandardIn};
use if_credit::kind::CreditKind;
use if_credit::law::Law;
use phx_core::{FactStore, SubStep};
use phx_id::{CountryId, Day, LineId, PartyId};
use phx_ledger::algebra::{Leg, Reference, Repayment, Schedule, Side};
use phx_ledger::apply::ApplyAt;
use phx_macros::clause;
use phx_num::{Missing, Money, violation};
use phx_rand::{Draws, Subject, SubjectTag};

use super::book::{Application, Bank, Quote, Written};
use crate::world::World;

/// The fact an applicant's required return is read from.
const REQUIRED_RETURN: &str = "FRM.required_return";

impl World {
    /// Each firm loan's maturity, read from the world as it opens or loads.
    #[clause("BNK.19")]
    pub(crate) fn credit_rebuild(&mut self) {
        let Some(kind) = self.credit.kind else { return };
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(kind.loan);
        let mut maturities: BTreeMap<Day, Vec<LineId>> = BTreeMap::new();
        for line in (0..lines.len()).filter_map(|i| u32::try_from(i).ok().map(LineId::new)) {
            if lines.kind_of(line) != k {
                continue;
            }
            let terms = self.books.ledger.terms.get(lines.terms(line));
            let Missing::Present(n) = terms.schedule.count else { continue };
            let last = terms.schedule.dates.nth(&self.calendar, n);
            if last > self.today {
                maturities.entry(last).or_default().push(line);
            }
        }
        self.credit.maturities = maturities;
    }

    /// A stream the credit kind names, opened for a subject on the day.
    fn credit_draws(&self, name: &str, subject: Subject, day: Day) -> Draws {
        let Some(stream) = self.streams.named(name) else {
            violation!(clause = "CHN.1", "credit drawing from a stream never declared");
        };
        self.streams.open(&stream, subject, day, SubStep::S5c.ordinal())
    }

    /// 5c: the day's round of credit, a step of each part, and the banks' reviews on a month's first round.
    #[clause("BNK.4", "BNK.5", "BNK.6", "BNK.19", "TIME.10")]
    pub(crate) fn credit_round(&mut self, day: Day) {
        let Some(kind) = self.credit.kind else { return };
        let date = self.calendar.date(day);
        let month =
            u32::try_from(i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month())).unwrap_or(0);
        if month != self.credit.book.reviewed {
            self.credit.book.reviewed = month;
            self.review_banks(&kind);
        }
        self.choose_quotes(day, &kind);
        self.answer_applications(day, &kind);
        self.apply_for_refinancing(day, &kind);
    }

    /// The bank and the borrower of a loan line: its asset side's one holder and its liability side's.
    fn loan_parties(&self, line: LineId) -> Option<(PartyId, PartyId)> {
        let lines = &self.books.ledger.lines;
        let bank = lines.sole_holder(line, Side::Asset).map(|k| self.books.party_of_key(k))?;
        let borrower = lines.sole_holder(line, Side::Liability).map(|k| self.books.party_of_key(k))?;
        Some((bank, borrower))
    }

    /// A party's balance on its row of a line's side.
    fn balance_on(&self, party: PartyId, line: LineId, side: Side) -> Missing<i64> {
        let (place, slot) = self.books.parties.row(party);
        match phx_ledger::rows::find(self.books.parties.holder(place), slot, line, side).map(|r| r.optional.balance) {
            Some(b) => b,
            None => Missing::Absent,
        }
    }

    /// A borrower's interest cover as a bank reads it: its earnings before interest over the interest its loans
    /// charge a year, its earnings its period's income to date grossed up to a year with the interest it paid; none
    /// before a day of its period has passed or where it owes no interest.
    fn cover(&self, borrower: PartyId, day: Day) -> Missing<f64> {
        let Some(kind) = self.credit.kind else { return Missing::Absent };
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(kind.loan);
        let (place, slot) = self.books.parties.row(borrower);
        let mut interest = 0.0;
        for r in phx_ledger::rows::rows(self.books.parties.holder(place), slot) {
            if r.side() != Side::Liability || lines.kind_of(r.row.line) != k {
                continue;
            }
            let terms = self.books.ledger.terms.get(lines.terms(r.row.line));
            let Missing::Present(balance) = r.optional.balance else { continue };
            for leg in &terms.legs {
                if let Leg::RateOnNotional { reference: Reference::Fixed(rate), .. } = leg {
                    interest += -phx_rand::float::from_i64(balance) * yearly(*rate);
                }
            }
        }
        let Missing::Present(tally) = self.accounts.tally_of(borrower) else { return Missing::Absent };
        let days = i64::from(day.get()) - tally.opened;
        if interest <= 0.0 || days <= 0 {
            return Missing::Absent;
        }
        let year = crate::consts::DAYS_A_YEAR / phx_rand::float::from_i64(days);
        let income = wide(tally.income) * year;
        Missing::Present((income + interest) / interest)
    }

    /// A bank's state, begun at the standards its published statistics admit: every class but those in default.
    fn bank_state(&mut self, bank: PartyId, law: &Law) -> &mut Bank {
        self.credit.book.banks.entry(bank).or_insert_with(|| {
            let first = law.default_rates.iter().position(|r| *r < 1.0).unwrap_or(0);
            Bank {
                standard: u32::try_from(first).unwrap_or(0),
                loan_years: vec![0.0; law.default_rates.len()],
                defaults: vec![0; law.default_rates.len()],
                ..Bank::default()
            }
        })
    }

    /// A bank's capital and its loans weighted by their risk: its net assets, and the balances it holds on firm loans.
    fn capital_of(&self, kind: &CreditKind, bank: PartyId) -> (f64, f64) {
        let s = phx_acct::statement::statement(&self.accounts, &self.books, bank);
        let capital = match s.net_assets {
            Missing::Present(v) => wide(v),
            Missing::Absent => 0.0,
        };
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(kind.loan);
        let (place, slot) = self.books.parties.row(bank);
        let loans: f64 = phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Asset && lines.kind_of(r.row.line) == k)
            .filter_map(|r| match r.optional.balance {
                Missing::Present(b) => Some(phx_rand::float::from_i64(b)),
                Missing::Absent => None,
            })
            .sum();
        (capital, loans)
    }

    /// The applications sent the day before, each quoted or declined by its bank.
    fn answer_applications(&mut self, day: Day, kind: &CreditKind) {
        let (due, later): (Vec<Application>, Vec<Application>) =
            std::mem::take(&mut self.credit.book.applications).into_iter().partition(|a| a.sent < day);
        self.credit.book.applications = later;
        let mut capital: BTreeMap<PartyId, (f64, f64)> = BTreeMap::new();
        let mut declined: BTreeMap<PartyId, u64> = BTreeMap::new();
        for a in due {
            if !self.live(a.bank) || !self.live(a.borrower) {
                continue;
            }
            let Missing::Present(country) = self.country_of_party(a.borrower) else { continue };
            let law = super::law_of(&self.credit.laws, country).clone();
            let class = (kind.class_of)(&law, self.cover(a.borrower, day));
            let (held, weighted) = *capital.entry(a.bank).or_insert_with(|| self.capital_of(kind, a.bank));
            let principal = phx_rand::float::from_i64(a.principal);
            let bank = self.bank_state(a.bank, &law).clone();
            let refused = (kind.decline)(&DeclineIn {
                class,
                standard: bank.standard,
                capital: held,
                weighted: (weighted + principal) * law.risk_weight,
                capital_requirement: law.capital_requirement,
            });
            if refused {
                self.bank_state(a.bank, &law).declined += 1;
                *declined.entry(a.bank).or_insert(0) += 1;
                self.credit.day.declines += 1;
                continue;
            }
            let (default_rate, loss) = outlooks(kind, &law, &bank, class);
            let rate = (kind.quote)(&QuoteIn {
                default_rate,
                loss_given_default: loss,
                cost_of_funds: law.cost_of_funds,
                risk_weight: law.risk_weight,
                capital_requirement: law.capital_requirement,
                required_return: law.required_return,
                loan_cost: law.loan_cost,
                principal,
                years: f64::from(a.months) / crate::consts::MONTHS_A_YEAR,
                rate_step: law.rate_step,
            });
            if let Some(e) = capital.get_mut(&a.bank) {
                e.1 += principal;
            }
            self.credit.book.quotes.push(Quote {
                borrower: a.borrower,
                bank: a.bank,
                refinances: a.refinances,
                principal: a.principal,
                months: a.months,
                rate,
                made: day,
            });
            self.credit.day.quotes += 1;
        }
        self.credit.day.declined_by_bank.extend(declined);
    }

    /// The quotes made the day before, each borrower taking the one it values most, or none.
    fn choose_quotes(&mut self, day: Day, kind: &CreditKind) {
        let (due, later): (Vec<Quote>, Vec<Quote>) =
            std::mem::take(&mut self.credit.book.quotes).into_iter().partition(|q| q.made < day);
        self.credit.book.quotes = later;
        let mut by_borrower: BTreeMap<PartyId, Vec<Quote>> = BTreeMap::new();
        for q in due {
            by_borrower.entry(q.borrower).or_default().push(q);
        }
        for (borrower, quotes) in by_borrower {
            if !self.live(borrower) {
                continue;
            }
            let Missing::Present(required) = self.required_return(borrower) else { continue };
            let Missing::Present(country) = self.country_of_party(borrower) else { continue };
            let step = super::law_of(&self.credit.laws, country).rate_step;
            let mut d = self.credit_draws(kind.taste_stream, Subject::new(SubjectTag::Party, borrower.get()), day);
            let tastes: Vec<f64> = quotes.iter().map(|_| phx_rand::gumbel(&mut d, 0.0, 1.0)).collect();
            let input = ChooseIn {
                rates: quotes.iter().map(|q| q.rate).collect(),
                tastes,
                required_return: required,
                rate_step: step,
            };
            let Missing::Present(n) = (kind.choose)(&input) else { continue };
            let Some(q) = usize::try_from(n).ok().and_then(|i| quotes.get(i)) else { continue };
            self.credit.book.lending.push(q.clone());
            self.credit.day.accepted += 1;
        }
    }

    /// A firm's required return, from its fact; none before its opening gives it one.
    fn required_return(&mut self, party: PartyId) -> Missing<f64> {
        let (place, slot) = self.books.parties.row(party);
        if place >= self.books.parties.first_cell_place() {
            return Missing::Absent;
        }
        let store: &mut dyn FactStore = self.books.parties.table_mut(place);
        match store.read(REQUIRED_RETURN, slot) {
            Missing::Present(v) => {
                let scale = (0..crate::consts::HURDLE_EXP).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
                Missing::Present(phx_rand::float::from_i64(v) / scale)
            }
            Missing::Absent => Missing::Absent,
        }
    }

    /// The borrowers whose loans fall due within their lead, each applying once to refinance the loan's balance for
    /// its months again: to its own bank and to as many more of its country's banks as the lenders it asks.
    fn apply_for_refinancing(&mut self, day: Day, kind: &CreditKind) {
        let Some(lead) = self.credit.laws.iter().map(|l| l.lead_days).reduce(|a, b| if b > a { b } else { a }) else {
            return;
        };
        let Some(horizon) = u16::try_from(lead).ok().and_then(phx_core::calendar::period::Period::days) else {
            violation!(clause = "BNK.19", "a refinancing lead beyond a period", days = lead);
        };
        let until = self.calendar.plus(day, horizon);
        let due: Vec<LineId> = self.credit.maturities.range(day..=until).flat_map(|(_, l)| l.iter().copied()).collect();
        let banks: Vec<PartyId> = self.books.parties.of_kind(kind.lender).collect();
        for line in due {
            if self.credit.book.tried.contains(&line) {
                continue;
            }
            let Some((own, borrower)) = self.loan_parties(line) else { continue };
            let Missing::Present(country) = self.country_of_party(borrower) else { continue };
            let law = super::law_of(&self.credit.laws, country).clone();
            let Some(last) = self.maturity_of(line) else { continue };
            if self.calendar.days_between(day, last).is_none_or(|d| d > law.lead_days) {
                continue;
            }
            if self.required_return(borrower) == Missing::Absent {
                continue;
            }
            let Missing::Present(balance) = self.balance_on(own, line, Side::Asset) else { continue };
            if balance <= 0 {
                continue;
            }
            self.credit.book.tried.insert(line);
            let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(line));
            let Missing::Present(months) = terms.schedule.count else { continue };
            let mut d = self.credit_draws(kind.asked_stream, Subject::new(SubjectTag::Party, borrower.get()), day);
            let asked = asked(&law.lenders_asked, phx_rand::open_unit(&mut d));
            let mut others: Vec<PartyId> = banks
                .iter()
                .copied()
                .filter(|b| *b != own && self.country_of_party(*b) == Missing::Present(country))
                .collect();
            let mut chosen = vec![own];
            while chosen.len() < asked && !others.is_empty() {
                let n = u64::try_from(others.len()).unwrap_or(u64::MAX);
                let at = usize::try_from(phx_rand::uniform::below_u64(&mut d, n)).unwrap_or(0);
                chosen.push(others.swap_remove(at));
            }
            for bank in chosen {
                self.credit.book.applications.push(Application {
                    borrower,
                    bank,
                    refinances: line,
                    principal: balance,
                    months,
                    sent: day,
                });
                self.credit.day.applications += 1;
            }
        }
    }

    /// A loan line's last date.
    fn maturity_of(&self, line: LineId) -> Option<Day> {
        let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(line));
        let Missing::Present(n) = terms.schedule.count else { return None };
        Some(terms.schedule.dates.nth(&self.calendar, n))
    }

    /// 7c: each loan taken written, its bank's claim and the deposit it creates for the borrower, on a line of its
    /// own whose rate is the quote's and whose principal falls due at its end, its interest monthly from today.
    #[clause("BNK.8", "BNK.17", "BNK.11", "MON.6")]
    pub(crate) fn credit_settle(&mut self, day: Day) {
        let Some(kind) = self.credit.kind else { return };
        let Missing::Present(reason) = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.lent))
        else {
            violation!(clause = "BNK.8", "loans under a reason never declared");
        };
        for q in std::mem::take(&mut self.credit.book.lending) {
            if !self.live(q.bank) || !self.live(q.borrower) {
                continue;
            }
            let Missing::Present(country) = self.country_of_party(q.borrower) else { continue };
            let ccy = phx_ledger::opening::currency(country);
            let line = self.loan_line(kind, country, (q.rate, q.principal, q.months));
            let before = self.books.money_held(q.borrower, ccy);
            let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S7c));
            if self
                .books
                .lend((q.bank, q.borrower, line), (q.principal, ccy), (reason, m), self.audit.stream())
                .is_err()
            {
                continue;
            }
            let after = self.books.money_held(q.borrower, ccy);
            let booked = match self.balance_on(q.bank, line, Side::Asset) {
                Missing::Present(b) => b,
                Missing::Absent => 0,
            };
            if let (Missing::Present(before), Missing::Present(after)) = (before, after) {
                self.credit.day.written.push(Written {
                    bank: q.bank,
                    borrower: q.borrower,
                    principal: q.principal,
                    before,
                    after,
                    booked,
                });
            }
            if let Some(last) = self.maturity_of(line) {
                self.credit.maturities.entry(last).or_default().push(line);
            }
        }
    }

    /// A new loan line: the quoted yearly rate on the balance and the principal at its end, monthly from today.
    fn loan_line(
        &mut self,
        kind: CreditKind,
        country: CountryId,
        (rate, principal, months): (f64, i64, u32),
    ) -> LineId {
        let ccy = phx_ledger::opening::currency(country);
        let dates = phx_ledger::opening::monthly(self.calendar.date(self.today), country);
        let raw = phx_ledger::opening::whole(rate * wide(phx_num::consts::RATE_SCALE));
        let mut terms = phx_ledger::opening::plain_terms(
            ccy,
            vec![
                Leg::RateOnNotional {
                    reference: Reference::Fixed(phx_num::Rate::new(raw, phx_num::RatePeriod::Year)),
                    day_count: phx_core::calendar::daycount::DayCount::Act365F,
                },
                Leg::Principal { amount: Money::new(principal, ccy), repayment: Repayment::Bullet },
            ],
            Schedule { dates, count: Missing::Present(months) },
        );
        terms.class = Vec::new();
        let id = self.books.ledger.terms.intern(terms);
        let mut n = 1_u32;
        while dates.nth(&self.calendar, n) <= self.today {
            n += 1;
        }
        let k = self.books.ledger.lines.kind_index(kind.loan);
        self.books.ledger.lines.open(k, id, Missing::Present((dates.nth(&self.calendar, n), n)))
    }

    /// Each bank's review of its book on a month's first round: each firm loan's month counted in its borrower's
    /// class, each loan whose borrower has defaulted since counted a default of the class it was in, and the bank's
    /// standard moved against what its book's defaults cost next to what the published statistics priced.
    #[clause("BNK.5", "BNK.20", "BNK.15")]
    fn review_banks(&mut self, kind: &CreditKind) {
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(kind.loan);
        let loans: Vec<LineId> = (0..lines.len())
            .filter_map(|i| u32::try_from(i).ok().map(LineId::new))
            .filter(|l| lines.kind_of(*l) == k)
            .collect();
        let day = self.today;
        for line in loans {
            let Some((bank, borrower)) = self.loan_parties(line) else { continue };
            let Missing::Present(country) = self.country_of_party(bank) else { continue };
            let law = super::law_of(&self.credit.laws, country).clone();
            let Missing::Present(balance) = self.balance_on(bank, line, Side::Asset) else { continue };
            if balance <= 0 {
                continue;
            }
            let (place, _) = self.books.parties.row(borrower);
            let estate = self.books.parties.holder(place).kind() == phx_core::ESTATE_KIND.name;
            let cover = if estate { Missing::Absent } else { self.cover(borrower, day) };
            let state = self.bank_state(bank, &law);
            if estate {
                if !state.defaulted.contains_key(&line) {
                    let class = state.classes.get(&line).copied().unwrap_or(0);
                    if let Some(d) = usize::try_from(class).ok().and_then(|c| state.defaults.get_mut(c)) {
                        *d += 1;
                    }
                    state.defaulted.insert(line, balance);
                }
                continue;
            }
            let class = (kind.class_of)(&law, cover);
            if let Some(y) = usize::try_from(class).ok().and_then(|c| state.loan_years.get_mut(c)) {
                *y += 1.0 / crate::consts::MONTHS_A_YEAR;
            }
            state.classes.insert(line, class);
        }
        let laws = self.credit.laws.clone();
        let banks: Vec<PartyId> = self.credit.book.banks.keys().copied().collect();
        for bank in banks {
            let Missing::Present(country) = self.country_of_party(bank) else { continue };
            let Some(law) = laws.get(usize::from(country.get())) else { continue };
            let Some(state) = self.credit.book.banks.get(&bank).cloned() else { continue };
            let (_, lost) = outlooks(kind, law, &state, 0);
            let seen: f64 = state.defaults.iter().map(|d| phx_rand::float::from_u64(*d) * lost).sum();
            let priced: f64 =
                state.loan_years.iter().zip(&law.default_rates).map(|(y, r)| y * r * law.loss_given_default).sum();
            let classes = u32::try_from(law.default_rates.len()).unwrap_or(0);
            let next = (kind.standard)(&StandardIn {
                seen_loss: seen,
                priced_loss: priced,
                standard: state.standard,
                classes,
            });
            if let Some(s) = self.credit.book.banks.get_mut(&bank) {
                s.standard = next;
            }
            self.credit.day.reviews += 1;
        }
    }

    /// What each written-off loan cost its bank, learned as a share of the balance it held when its borrower defaulted.
    #[clause("BNK.20", "BNK.14")]
    pub(crate) fn credit_losses(&mut self, lost: &[(PartyId, LineId, i64)]) {
        for (creditor, line, amount) in lost {
            let Some(state) = self.credit.book.banks.get_mut(creditor) else { continue };
            let Some(balance) = state.defaulted.remove(line) else { continue };
            if balance > 0 {
                state.lost += phx_rand::float::from_i64(*amount) / phx_rand::float::from_i64(balance);
                state.recoveries += 1;
            }
        }
    }
}

/// A bank's outlooks: its class's default frequency and the share of a defaulted balance it expects to lose.
fn outlooks(kind: &CreditKind, law: &Law, bank: &Bank, class: u32) -> (f64, f64) {
    let c = usize::try_from(class).unwrap_or(usize::MAX);
    let published = law.default_rates.get(c).copied().unwrap_or(1.0);
    let years = bank.loan_years.get(c).copied().unwrap_or(0.0);
    let defaults = bank.defaults.get(c).map_or(0.0, |d| phx_rand::float::from_u64(*d));
    let rate = (kind.learned)(published, law.prior_loan_years, defaults, years);
    let lost = (kind.learned)(
        law.loss_given_default,
        law.prior_recoveries,
        bank.lost,
        phx_rand::float::from_u64(bank.recoveries),
    );
    (rate, lost)
}

/// A wide amount as a float; no amount the world keeps passes an i64.
fn wide(v: i128) -> f64 {
    let Ok(n) = i64::try_from(v) else {
        phx_num::capacity_exceeded!("an amount read as a float", i64::MAX, v);
    };
    phx_rand::float::from_i64(n)
}

/// A yearly rate as a fraction.
fn yearly(rate: phx_num::Rate) -> f64 {
    phx_rand::float::from_i64(rate.raw()) / wide(phx_num::consts::RATE_SCALE)
}

/// How many lenders a borrower asks, drawn from the published shares at a uniform draw.
fn asked(shares: &[f64], u: f64) -> usize {
    let mut left = u;
    for (n, s) in shares.iter().enumerate() {
        if left < *s {
            return n + 1;
        }
        left -= s;
    }
    shares.len()
}
