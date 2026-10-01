//! Lending on the core. A firm short of what it pays today applies to its own bank and to as many more of its country's
//! banks as the published shares of lenders asked give it. Each bank reads the firm's interest cover — its earnings
//! over its last year before interest, its filed year's for the part before the opening, against the interest its
//! loans and this one charge a year at its country's lending rate — and declines where the firm's class is worse than
//! its standards admit or its capital cannot carry the loan, else quotes its rate for the class. The firm takes the
//! best quote by its taste for each lender, whatever its return requires, since failing its dues ends it; declines
//! are counted by bank.

use std::collections::BTreeMap;

use if_credit::decisions::{ChooseIn, DeclineIn, QuoteIn};
use if_credit::law::Law;
use phx_core::StreamDef;
use phx_core::slots::DaySlot;
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::Subject;

use crate::consts::firm::{MARKUP, OUTPUT, PART_ONE, PRICE, PRODUCT};
use crate::core::Core;

/// A bank's lending: the worst class it admits; the applications it answered, declined and quoted, and the loans it
/// made; by class, the loan-years its book has held and the defaults it has seen; and what it wrote off since its
/// last review.
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub struct Lender {
    pub standard: u32,
    pub applications: u64,
    pub declined: u64,
    pub quoted: u64,
    pub lent: u64,
    pub loan_years: Vec<f64>,
    pub defaults: Vec<u64>,
    pub written: i128,
}

/// Lending on the core: each country's lending law, each firm's filed earnings a year at the opening, and each bank's
/// lending.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub struct Credit {
    pub laws: Vec<Law>,
    pub filed: BTreeMap<PartyKey, f64>,
    pub lenders: BTreeMap<PartyKey, Lender>,
    /// Each firm loan's class when it was made and the day it was, by its family and contract.
    pub classes: BTreeMap<(usize, u32), (u32, Day)>,
    /// The month the banks last reviewed their standards in, and its first day.
    pub reviewed: Option<(i64, Day)>,
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

/// A class's default rate as a bank has learned it from its own book: the published rate counted as the law's prior
/// loan-years, with the defaults and loan-years it has seen.
fn learned_rate(law: &Law, lender: &Lender, class: usize) -> f64 {
    let published = law.default_rates.get(class).copied().unwrap_or_else(|| {
        phx_num::violation!(clause = "BNK.20", "a class beyond the published default rates", class = class)
    });
    let years = lender.loan_years.get(class).copied().unwrap_or(0.0);
    let defaults = lender.defaults.get(class).map_or(0.0, |d| phx_rand::float::from_u64(*d));
    sys_bnk::credit::learned(published, law.prior_loan_years, defaults, years)
}

/// A quoted yearly rate, a fraction, as the rate its contract carries.
fn as_rate(yearly: f64) -> phx_num::Rate {
    sys_bnk::rate(yearly * crate::consts::PERCENT)
}

impl Core {
    /// Each country's lending law; each firm's filed earnings a year — what its opening output earns at its opening
    /// price over the cost its markup was set on; each bank's standard; and each firm loan the opening holds classed.
    ///
    /// # Errors
    /// A primitive of the lending law the register does not hold.
    #[clause("BNK.16", "BNK.20")]
    pub fn open_credit(
        &mut self,
        register: &phx_core::Register,
        countries: &[phx_core::OpeningCountry],
        today: Day,
    ) -> Result<(), String> {
        let laws = countries.iter().map(|c| sys_bnk::credit::law(register, c)).collect::<Result<Vec<Law>, String>>()?;
        let mut filed = BTreeMap::new();
        if let Some(firm) = self.bound.kinds.firm
            && let Some(store) = self.kinds.get(firm)
        {
            for slot in self.directory.live_slots(crate::core::kind_number(firm)) {
                let word = |at: usize| match store.record(slot).get(at).map(|w| w.get()) {
                    Some(Missing::Present(v)) => Some(phx_rand::float::from_i64(v)),
                    _ => None,
                };
                let (Some(product), Some(price), Some(markup), Some(output)) =
                    (word(PRODUCT), word(PRICE), word(MARKUP), word(OUTPUT))
                else {
                    continue;
                };
                let Some(lot) = phx_rand::float::floor_to_i64(product)
                    .and_then(|p| usize::try_from(p).ok())
                    .and_then(|p| self.goods.lots.get(p))
                    .copied()
                else {
                    continue;
                };
                let markup = markup / PART_ONE;
                let margin = price / lot * markup / (1.0 + markup);
                filed.insert(PartyKey::new(crate::core::kind_number(firm), slot), margin * output);
            }
        }
        let lenders = self.bank_kind.map_or_else(BTreeMap::new, |b| {
            self.directory.live_slots(b).map(|s| (PartyKey::new(b, s), Lender::default())).collect::<BTreeMap<_, _>>()
        });
        self.credit = Credit { laws, filed, lenders, ..Credit::default() };
        let mut founded = Vec::new();
        for (bank, lender) in &mut self.credit.lenders {
            let country = self.banks_of.iter().position(|bs| bs.iter().any(|(s, _)| *s == bank.slot().get()));
            let Some(law) = country.and_then(|c| self.credit.laws.get(c)) else { continue };
            // A bank opens admitting every class but those in default.
            lender.standard =
                law.default_rates.iter().position(|r| *r < 1.0).and_then(|p| u32::try_from(p).ok()).unwrap_or(0);
            lender.loan_years = vec![0.0; law.default_rates.len()];
            lender.defaults = vec![0; law.default_rates.len()];
            // The return its shareholders require is its preference at its founding, every bank's the same.
            founded.push((
                *bank,
                phx_core::Prefs { required_return: Missing::Present(law.required_return), ..phx_core::Prefs::NONE },
            ));
        }
        for (bank, prefs) in founded {
            self.found(bank, prefs);
        }
        self.class_opening_loans(today);
        Ok(())
    }

    /// Each firm loan the opening holds classed as its bank reads its borrower at the opening.
    fn class_opening_loans(&mut self, today: Day) {
        let Some(i) = self.bound.families.firm_loans else { return };
        let Some(family) = self.families.get(i) else { return };
        let loans: Vec<(u32, PartyKey, u8)> = family
            .store
            .edges
            .open_slots()
            .filter_map(|e| {
                let row = family.store.edges.row(e)?;
                let at = usize::try_from(row.schedule).ok()?;
                Some((e.get(), row.ends[0], family.schedules.get(at)?.1))
            })
            .collect();
        for (edge, borrower, country) in loans {
            let Some(law) = self.credit.laws.get(usize::from(country)) else { continue };
            let class = sys_bnk::credit::class_of(law, self.cover(borrower, (0, self.lending_rate(country)), today));
            self.credit.classes.insert((i, edge), (class, today));
        }
    }

    /// A country's lending rate as a yearly fraction.
    fn lending_rate(&self, country: u8) -> f64 {
        self.lending.get(usize::from(country)).map_or(0.0, |(r, _)| {
            phx_rand::float::from_i64(r.raw()) / phx_rand::float::from_i128(phx_num::consts::RATE_SCALE)
        })
    }

    /// A classed loan written off: a default its bank has seen in the loan's class, and what it lost.
    pub(crate) fn loan_defaulted(&mut self, bank: PartyKey, amount: i64, loan: (usize, u32)) {
        let Some((class, _)) = self.credit.classes.remove(&loan) else { return };
        let Some(lender) = self.credit.lenders.get_mut(&bank) else { return };
        if let Some(d) = usize::try_from(class).ok().and_then(|c| lender.defaults.get_mut(c)) {
            *d += 1;
        }
        lender.written += i128::from(amount);
    }

    /// On a month's first day each bank reviews its standard: its loan-years since its last review entered by class,
    /// and the loss its write-offs showed against the loss its classes' default rates, as it has learned them, priced
    /// for its book over the month; a class tighter where it lost more, a class looser where less.
    #[clause("BNK.5", "BNK.20")]
    pub(crate) fn review_lenders(&mut self, day: Day, month: i64) {
        let Some((last, since)) = self.credit.reviewed else {
            self.credit.reviewed = Some((month, day));
            return;
        };
        if last == month {
            return;
        }
        self.credit.reviewed = Some((month, day));
        let families = &self.families;
        self.credit
            .classes
            .retain(|(i, e), _| families.get(*i).is_some_and(|f| f.store.edges.is_open(phx_id::Slot::new(*e))));
        let year = crate::consts::DAYS_A_YEAR;
        let span = phx_rand::float::from_i64(i64::from(day.get()) - i64::from(since.get()));
        let mut held: BTreeMap<PartyKey, Vec<(f64, f64)>> = BTreeMap::new();
        for ((i, e), (class, opened)) in &self.credit.classes {
            let Some(row) = self.families.get(*i).and_then(|f| f.store.edges.row(phx_id::Slot::new(*e))) else {
                continue;
            };
            // A loan made since the last review has been held from its day.
            let from = if *opened > since { *opened } else { since };
            let days = phx_rand::float::from_i64(i64::from(day.get()) - i64::from(from.get()));
            let c = usize::try_from(*class).unwrap_or(usize::MAX);
            let by = held.entry(row.ends[1]).or_default();
            if by.len() <= c {
                by.resize(c + 1, (0.0, 0.0));
            }
            if let Some(slot) = by.get_mut(c) {
                slot.0 += days / year;
                slot.1 += phx_rand::float::from_i64(row.amount);
            }
        }
        let reviewing = self.point(|p| p.standard, &sys_bnk::points::STANDARD);
        let countries: Vec<(PartyKey, usize)> = self
            .credit
            .lenders
            .keys()
            .filter_map(|b| {
                self.banks_of.iter().position(|bs| bs.iter().any(|(s, _)| *s == b.slot().get())).map(|c| (*b, c))
            })
            .collect();
        for (bank, country) in countries {
            let Some(law) = self.credit.laws.get(country).cloned() else { continue };
            let Some(mut lender) = self.credit.lenders.remove(&bank) else { continue };
            let by = held.remove(&bank).unwrap_or_default();
            let mut book = 0.0;
            let mut priced = 0.0;
            for (c, (years, balance)) in by.iter().enumerate() {
                if let Some(y) = lender.loan_years.get_mut(c) {
                    *y += years;
                }
                let rate = learned_rate(&law, &lender, c);
                book += balance;
                priced += balance * rate * law.loss_given_default * span / year;
            }
            let written = i64::try_from(std::mem::take(&mut lender.written)).map_or(0.0, phx_rand::float::from_i64);
            if book > 0.0 {
                lender.standard = self.decide(reviewing, bank, |_| if_credit::decisions::StandardIn {
                    seen_loss: written / book,
                    priced_loss: priced / book,
                    standard: lender.standard,
                    classes: u32::try_from(law.default_rates.len()).unwrap_or(u32::MAX),
                });
            }
            self.credit.lenders.insert(bank, lender);
        }
    }

    /// A firm's interest cover as a bank reads it: its earnings over its last year before interest — its filed year's
    /// for the part of the year before the opening, its recognised income since — against the interest its loans
    /// and this one charge a year at its country's lending rate; none where it would owe no interest.
    fn cover(&self, key: PartyKey, (principal, rate): (i64, f64), today: Day) -> Missing<f64> {
        let Some(opened) = self.accounts.opened else { return Missing::Absent };
        let days = phx_rand::float::from_i64(i64::from(today.get()) - i64::from(opened.get()));
        let year = crate::consts::DAYS_A_YEAR;
        let seen = self.accounts.income.get(key).map_or(0.0, |i| {
            let ebit = i.net() + i.interest_paid;
            i64::try_from(ebit).map_or(0.0, phx_rand::float::from_i64)
        });
        let earnings = if days < year {
            let Some(filed) = self.credit.filed.get(&key).copied() else { return Missing::Absent };
            filed * (year - days) / year + seen
        } else {
            seen * year / days
        };
        // Only its loans charge interest; what it owes in taxes collected does not.
        let owed: i64 =
            self.debts_of(key).iter().filter(|c| c.reason == crate::consts::reason::REPAID).map(|c| c.amount).sum();
        let interest = phx_rand::float::from_i64(owed + principal) * rate;
        if interest <= 0.0 {
            return Missing::Absent;
        }
        Missing::Present(earnings / interest)
    }

    /// A firm's application for a loan of `principal` over `years`: its own bank and the others it asks each decline
    /// or quote, and it takes the best quote by its taste; the lender and the rate, none where every bank declined.
    #[clause("BNK.4", "BNK.5", "BNK.6", "BNK.20", "REP.22", "MKT.7")]
    pub(crate) fn apply_for_loan(
        &mut self,
        (key, country): (PartyKey, u8),
        (principal, years): (i64, u64),
        (day, streams): (Day, &phx_core::WorldStreams),
    ) -> Option<(PartyKey, phx_num::Rate, u32)> {
        let law = self.credit.laws.get(usize::from(country))?.clone();
        let own = self.bank_of(key)?;
        let mut others: Vec<PartyKey> = self
            .banks_of
            .get(usize::from(country))
            .map(|bs| {
                bs.iter().filter_map(|(s, _)| self.bank_kind.map(|b| PartyKey::new(b, phx_id::Slot::new(*s)))).collect()
            })
            .unwrap_or_default();
        others.retain(|b| *b != own);
        let asked_stream = streams.named(sys_bnk::AskedStream::DECL.name)?;
        let Some(party) = self.reference(key) else {
            violation!(clause = "PTY.1", "a borrower the directory never held", key = key.word());
        };
        let subject = Subject::from(party);
        let mut draws = streams.open_at(&asked_stream, subject, day, DaySlot::S5c.ordinal());
        let count = asked(&law.lenders_asked, phx_rand::open_unit(&mut draws));
        let mut chosen = vec![own];
        while chosen.len() < count && !others.is_empty() {
            let at = phx_rand::float::index(phx_rand::below_u64(&mut draws, phx_rand::float::len_u64(others.len())));
            chosen.push(others.swap_remove(at));
        }
        let class = sys_bnk::credit::class_of(&law, self.cover(key, (principal, self.lending_rate(country)), day));
        let amount = phx_rand::float::from_i64(principal);
        let mut quotes: Vec<(PartyKey, f64)> = Vec::new();
        let (declining, quoting, choosing) = (
            self.point(|p| p.decline, &sys_bnk::points::DECLINE),
            self.point(|p| p.quote, &sys_bnk::points::QUOTE),
            self.point(|p| p.choose, &sys_bnk::points::CHOOSE),
        );
        for bank in chosen {
            let capital =
                self.accounts.equity(bank).map_or(0.0, |e| i64::try_from(e).map_or(0.0, phx_rand::float::from_i64));
            let book = self
                .loan_books
                .get(&bank)
                .map_or(0.0, |b| i64::try_from(b.book).map_or(0.0, phx_rand::float::from_i64));
            let Some(lender) = self.credit.lenders.get(&bank) else { continue };
            let standard = lender.standard;
            let default_rate = learned_rate(&law, lender, usize::try_from(class).unwrap_or(usize::MAX));
            let refused = self.decide(declining, bank, |_| DeclineIn {
                class,
                standard,
                capital,
                weighted: (book + amount) * law.risk_weight,
                capital_requirement: law.capital_requirement,
            });
            let rate = (!refused).then(|| {
                self.decide(quoting, bank, |prefs| QuoteIn {
                    default_rate,
                    loss_given_default: law.loss_given_default,
                    cost_of_funds: law.cost_of_funds,
                    risk_weight: law.risk_weight,
                    capital_requirement: law.capital_requirement,
                    required_return: match prefs.required_return {
                        Missing::Present(r) => r,
                        Missing::Absent => {
                            violation!(clause = "MND.16", "a bank quoting with no return required at its founding")
                        }
                    },
                    loan_cost: law.loan_cost,
                    principal: amount,
                    years: phx_rand::float::from_u64(years),
                    rate_step: law.rate_step,
                })
            });
            let Some(lender) = self.credit.lenders.get_mut(&bank) else { continue };
            lender.applications += 1;
            match rate {
                None => lender.declined += 1,
                Some(rate) => {
                    lender.quoted += 1;
                    quotes.push((bank, rate));
                }
            }
        }
        let taste_stream = streams.named(sys_bnk::TasteStream::DECL.name)?;
        let mut taste = streams.open_at(&taste_stream, subject, day, DaySlot::S5c.ordinal());
        let tastes: Vec<f64> = quotes.iter().map(|_| phx_rand::gumbel(&mut taste, 0.0, 1.0)).collect();
        let pick = self.decide(choosing, key, |_| ChooseIn {
            rates: quotes.iter().map(|q| q.1).collect(),
            tastes,
            required_return: f64::MAX,
            rate_step: law.rate_step,
        });
        let Missing::Present(i) = pick else { return None };
        let (bank, rate) = quotes.get(usize::try_from(i).ok()?).copied()?;
        if let Some(l) = self.credit.lenders.get_mut(&bank) {
            l.lent += 1;
        }
        Some((bank, as_rate(rate), class))
    }
}
