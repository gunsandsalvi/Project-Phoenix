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
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::{Subject, SubjectTag};

use crate::consts::firm::{MARKUP, OUTPUT, PART_ONE, PRICE, PRODUCT};
use crate::core::Core;

/// A bank's lending: the worst class it admits, and the applications it answered, declined and quoted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lender {
    pub standard: u32,
    pub applications: u64,
    pub declined: u64,
    pub quoted: u64,
    pub lent: u64,
}

/// Lending on the core: each country's lending law, each firm's filed earnings a year at the opening, and each bank's
/// lending.
#[derive(Clone, Debug, Default)]
pub struct Credit {
    pub laws: Vec<Law>,
    pub filed: BTreeMap<PartyKey, f64>,
    pub lenders: BTreeMap<PartyKey, Lender>,
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

/// A quoted yearly rate, a fraction, as the rate its contract carries.
fn as_rate(yearly: f64) -> phx_num::Rate {
    sys_bnk::rate(yearly * crate::consts::PERCENT)
}

impl Core {
    /// Each country's lending law, and each firm's filed earnings a year: what its opening output earns at its
    /// opening price over the cost its markup was set on.
    ///
    /// # Errors
    /// A primitive of the lending law the register does not hold.
    #[clause("BNK.16", "BNK.20")]
    pub fn open_credit(
        &mut self,
        register: &phx_core::Register,
        countries: &[phx_core::OpeningCountry],
    ) -> Result<(), String> {
        let laws = countries.iter().map(|c| sys_bnk::credit::law(register, c)).collect::<Result<Vec<Law>, String>>()?;
        let mut filed = BTreeMap::new();
        if let Some(firm) = self.names.iter().position(|n| *n == "firm")
            && let Some(store) = self.kinds.get(firm)
        {
            for slot in store.parties.live_slots() {
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
        let lenders = self.bank_kind.and_then(|b| self.kinds.get(usize::from(b)).map(|k| (b, k))).map_or_else(
            BTreeMap::new,
            |(b, k)| {
                k.parties.live_slots().map(|s| (PartyKey::new(b, s), Lender::default())).collect::<BTreeMap<_, _>>()
            },
        );
        self.credit = Credit { laws, filed, lenders };
        for (bank, lender) in &mut self.credit.lenders {
            let country = self.banks_of.iter().position(|bs| bs.iter().any(|(s, _)| *s == bank.slot().get()));
            let law = country.and_then(|c| self.credit.laws.get(c));
            // A bank opens admitting every class but those in default.
            lender.standard = law
                .and_then(|l| l.default_rates.iter().position(|r| *r < 1.0))
                .and_then(|p| u32::try_from(p).ok())
                .unwrap_or(0);
        }
        Ok(())
    }

    /// A firm's interest cover as a bank reads it: its earnings over its last year before interest — its filed year's
    /// for the part of the year before the opening, its recognised income since — against the interest its loans
    /// and this one charge a year at its country's lending rate; none where it would owe no interest.
    fn cover(&self, key: PartyKey, (principal, rate): (i64, f64), today: Day) -> Missing<f64> {
        let Some(opened) = self.accounts.opened else { return Missing::Absent };
        let days = phx_rand::float::from_i64(i64::from(today.get()) - i64::from(opened.get()));
        let year = crate::consts::DAYS_A_YEAR;
        let seen = self.accounts.income.get(&key).map_or(0.0, |i| {
            let ebit = i.net() + i.interest_paid;
            i64::try_from(ebit).map_or(0.0, phx_rand::float::from_i64)
        });
        let earnings = if days < year {
            let Some(filed) = self.credit.filed.get(&key).copied() else { return Missing::Absent };
            filed * (year - days) / year + seen
        } else {
            seen * year / days
        };
        let owed: i64 = self.debts_of(key).iter().map(|c| c.amount).sum();
        let interest = phx_rand::float::from_i64(owed + principal) * rate;
        if interest <= 0.0 {
            return Missing::Absent;
        }
        Missing::Present(earnings / interest)
    }

    /// A firm's application for a loan of `principal` over `years`: its own bank and the others it asks each decline
    /// or quote, and it takes the best quote by its taste; the lender and the rate, none where every bank declined.
    #[clause("BNK.4", "BNK.5", "BNK.6", "BNK.20", "REP.22")]
    pub(crate) fn apply_for_loan(
        &mut self,
        (key, country): (PartyKey, u8),
        (principal, years): (i64, u64),
        (day, streams): (Day, &phx_core::Streams),
    ) -> Option<(PartyKey, phx_num::Rate)> {
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
        let mut draws = streams.open(&asked_stream, Subject::new(SubjectTag::Party, u64::from(key.word())), day, 0);
        let count = asked(&law.lenders_asked, phx_rand::open_unit(&mut draws));
        let mut chosen = vec![own];
        while chosen.len() < count && !others.is_empty() {
            let at = phx_rand::float::index(phx_rand::below_u64(&mut draws, phx_rand::float::len_u64(others.len())));
            chosen.push(others.swap_remove(at));
        }
        let lending = self.lending.get(usize::from(country)).map_or(0.0, |(r, _)| {
            phx_rand::float::from_i64(r.raw()) / phx_rand::float::from_i128(phx_num::consts::RATE_SCALE)
        });
        let class = sys_bnk::credit::class_of(&law, self.cover(key, (principal, lending), day));
        let amount = phx_rand::float::from_i64(principal);
        let mut quotes: Vec<(PartyKey, f64)> = Vec::new();
        for bank in chosen {
            let capital =
                self.accounts.equity(bank).map_or(0.0, |e| i64::try_from(e).map_or(0.0, phx_rand::float::from_i64));
            let book = self
                .loan_books
                .get(&bank)
                .map_or(0.0, |b| i64::try_from(b.book).map_or(0.0, phx_rand::float::from_i64));
            let Some(lender) = self.credit.lenders.get_mut(&bank) else { continue };
            lender.applications += 1;
            let refused = sys_bnk::credit::decline(&DeclineIn {
                class,
                standard: lender.standard,
                capital,
                weighted: (book + amount) * law.risk_weight,
                capital_requirement: law.capital_requirement,
            });
            if refused {
                lender.declined += 1;
                continue;
            }
            let Some(default_rate) = usize::try_from(class).ok().and_then(|c| law.default_rates.get(c)).copied() else {
                phx_num::violation!(clause = "BNK.20", "a class beyond the published default rates", class = class);
            };
            let rate = sys_bnk::credit::quote(&QuoteIn {
                default_rate,
                loss_given_default: law.loss_given_default,
                cost_of_funds: law.cost_of_funds,
                risk_weight: law.risk_weight,
                capital_requirement: law.capital_requirement,
                required_return: law.required_return,
                loan_cost: law.loan_cost,
                principal: amount,
                years: phx_rand::float::from_u64(years),
                rate_step: law.rate_step,
            });
            lender.quoted += 1;
            quotes.push((bank, rate));
        }
        let taste_stream = streams.named(sys_bnk::TasteStream::DECL.name)?;
        let mut taste = streams.open(&taste_stream, Subject::new(SubjectTag::Party, u64::from(key.word())), day, 0);
        let tastes: Vec<f64> = quotes.iter().map(|_| phx_rand::gumbel(&mut taste, 0.0, 1.0)).collect();
        let pick = sys_bnk::credit::choose(&ChooseIn {
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
        Some((bank, as_rate(rate)))
    }
}
