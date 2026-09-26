use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::ScheduleDates;
use phx_core::{
    Apportioned, BALANCES, CONTRACTS, Contribution, DECLARATIONS, Opening, OpeningCountry, OpeningPhase, PARTIES, Prim,
    StreamDef, apportion, opening_subject,
};
use phx_id::{CountryId, Date, PartyId};
use phx_ledger::algebra::{Leg, Reference, Repayment, Schedule, Side};
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, ReasonDecl, ReasonId};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::money::MoneyHolders;
use phx_ledger::opening::{currency, derived, key, monthly, open_row, plain_terms as terms, whole, write};
use phx_ledger::rows::{BALANCE, PENDING};
use phx_ledger::terms::TermsId;
use phx_macros::clause;
use phx_num::{Count, Missing, Money, Rate, RatePeriod, violation};
use phx_rand::below_u64;
use std::collections::BTreeMap;

use crate::consts::{LOANS, LOTS, MONTHS_PER_YEAR, PERCENT, PURPOSES, RATE_ONE, SHARE_PARTS, SITES};
use crate::{BANK, OpeningStream, zipf};

pub(crate) const BANKS: &str = "BNK.banks";
const FIRMS: &str = "FRM.firms";
const DEBT: &str = "FRM.debt";
pub(crate) const DEPOSITS: &str = "FRM.deposits";
const LENDERS: &str = "BNK.lenders";
const ACCOUNT: &str = "current account";
/// The kind of the small firms' cells, and what they draw: their banks, their firms, their deposits and their debt.
pub(crate) const SMALL_FIRM: &str = "small_firm";
const SMALL_BANKS: &str = "FRM.small_banks";
const SMALL_COUNTS: &str = "FRM.small_counts";
pub(crate) const SMALL_DEPOSITS: &str = "FRM.small_deposits";
const SMALL_DEBT: &str = "FRM.small_debt";

/// Current accounts: a bank's liability to the firms that bank with it.
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &["central_bank"],
    banks: &[BANK.name],
    treasuries: &["treasury"],
    depositors: &["firm", SMALL_FIRM, phx_core::ESTATE_KIND.name],
    requesters: &["BNK"],
};

/// A firm's term loan from its bank, one contract to a line.
const LOAN: LineKindDecl = LineKindDecl {
    name: "firm term loan",
    asset: SideDecl {
        holder_kinds: &[BANK.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    liability: SideDecl {
        holder_kinds: &["firm", SMALL_FIRM, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    dated: true,
    transfer_requesters: &["BNK"],
};

fn subject(country: CountryId, purpose: u32, ordinal: u32) -> phx_rand::Subject {
    opening_subject(u32::from(country.get()) * PURPOSES + purpose, ordinal)
}

/// What the opening's instructions are for: capital on both sides, since they open the books.
const REASON: ReasonDecl = ReasonDecl { name: "BNK opening", order: 0, paid: Effect::Equity, received: Effect::Equity };

fn reason(b: &Books) -> ReasonId {
    b.ledger.reasons.named(REASON.name)
}

/// The banks' declarations in the books: their opening's reason, their deposits' kind and their loans' kind.
#[clause("MON.1", "BNK.1")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "bank declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let b = books::of(opening);
        let _ = b.ledger.reasons.declare(REASON);
        b.ledger.lines.declare_deposits(HOLDERS.deposits(ACCOUNT));
        b.ledger.lines.declare_money(LOAN);
        b.ledger.lines.declare_deposits(crate::households::RETAIL.retail_deposits(crate::households::ACCOUNT));
        b.ledger.lines.declare_money(crate::households::LOAN);
    }
}

pub(crate) fn drawn(b: &Books, name: &str, country: CountryId) -> Vec<(PartyId, u64)> {
    let Some(v) = b.drawn.get(&key(name, country)) else {
        violation!(clause = "GEN.3", "the banks' opening reading a stratum not yet drawn", country = country.get());
    };
    v.clone()
}

pub(crate) fn rate(percent: f64) -> Rate {
    Rate::new(whole(percent / PERCENT * RATE_ONE), RatePeriod::Year)
}

/// `items` in an order drawn from `draws`, every order alike: the firms come largest first, and the banks take them in
/// turn, so without it the largest bank would keep every largest firm.
#[clause("GEN.3", "CHN.1")]
pub(crate) fn shuffle<T>(items: &mut [T], draws: &mut phx_rand::Draws) {
    for last in (1..items.len()).rev() {
        let Ok(n) = u64::try_from(last + 1) else {
            phx_num::capacity_exceeded!("items shuffled", u64::MAX, last);
        };
        let Ok(pick) = usize::try_from(below_u64(draws, n)) else {
            phx_num::capacity_exceeded!("a drawn position", usize::MAX, n);
        };
        items.swap(last, pick);
    }
}

/// The date `months` months before `date`, on the same day of the month or the month's last.
fn months_before(date: Date, months: u32) -> Date {
    let total = i64::from(date.year()) * i64::from(MONTHS_PER_YEAR) + i64::from(date.month()) - 1 - i64::from(months);
    let (Ok(year), Ok(month)) = (
        i32::try_from(total.div_euclid(i64::from(MONTHS_PER_YEAR))),
        u8::try_from(total.rem_euclid(i64::from(MONTHS_PER_YEAR)) + 1),
    ) else {
        violation!(clause = "TIME.2", "a loan's start before the calendar's reach", months = months);
    };
    let Some(last) = Date::days_in_month(year, month) else {
        violation!(clause = "TIME.2", "a loan's start in no month", month = month);
    };
    let day = if date.day() > last { last } else { date.day() };
    let Some(d) = Date::new(year, month, day) else { violation!(clause = "TIME.2", "a loan's start on no date") };
    d
}

/// A loan's start, drawn uniformly over the days of its term that end after `today`, so the loans the opening finds
/// running began on every day of the years before it.
fn started(
    calendar: &phx_core::Calendar,
    date: Date,
    today: phx_id::Day,
    months: u32,
    draws: &mut phx_rand::Draws,
) -> Date {
    let Some(span) =
        calendar.day(months_before(date, months)).and_then(|earliest| calendar.days_between(earliest, today))
    else {
        violation!(clause = "TIME.2", "a loan's start before the calendar's reach", months = months);
    };
    let Ok(back) = u32::try_from(below_u64(draws, u64::from(span))) else {
        violation!(clause = "GEN.2", "a loan's age beyond its term", months = months);
    };
    let Some(start) = calendar.days_before(today, back) else {
        violation!(clause = "TIME.2", "a loan's start before the calendar's reach", months = months);
    };
    calendar.date(start)
}

/// The first date of a schedule after a day, with its index.
fn next_date(dates: &ScheduleDates, calendar: &phx_core::Calendar, day: phx_id::Day) -> (phx_id::Day, u32) {
    let mut k = 1;
    let mut due = dates.nth(calendar, k);
    while due <= day {
        k += 1;
        due = dates.nth(calendar, k);
    }
    (due, k)
}

/// Each country's banks: how many, and each one's share of the banks' assets, from a Zipf law fitted to the
/// published concentration of the three and the five largest; each sited in the country.
#[clause("GEN.2", "GEN.3", "PTY.9")]
#[derive(Debug)]
pub struct Parties;

impl Contribution for Parties {
    fn name(&self) -> &'static str {
        "banks"
    }
    fn phase(&self) -> OpeningPhase {
        PARTIES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[BANKS]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[BANKS]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (countries, day) = (opening.countries, opening.day);
        for c in countries {
            let c5 = derived(c, "GEN.bank_concentration5") / PERCENT;
            let c3 = derived(c, "GEN.bank_top3_of_top5") * c5;
            let (n, a) = zipf::fit(c3, c5);
            let mut banks = Vec::new();
            for (k, share) in (0_u32..).zip(zipf::shares(n, a)) {
                let mut draws = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, SITES, k));
                let site = c.site(&mut draws);
                let bank = books::of(opening).parties.begin(BANK.name, site, day);
                let Ok(weight) = u64::try_from(whole(share * SHARE_PARTS)) else {
                    violation!(clause = "GEN.2", "a bank's share below nothing", bank = bank.get());
                };
                banks.push((bank, weight));
            }
            let (b, report) = books::split(opening);
            b.drawn.insert(key(BANKS, c.id), banks);
            report.distributions.push((
                key(BANKS, c.id),
                format!("{n} banks, a Zipf law of exponent {a:.4} fitted to GEN.bank_concentration5 and GEN.bank_top3_of_top5 (World Bank GFDD)"),
            ));
        }
    }
}

/// The banks' contracts in each country: the firms, in their drawn order, are apportioned over the banks by their
/// shares, largest remainder with ties by lot, and each keeps its current account at its bank and owes it a term
/// loan, whose term and age are drawn and whose rate is the country's lending rate; a current account pays the
/// deposit rate.
#[clause("GEN.2", "GEN.5", "BNK.1", "BNK.17")]
#[derive(Debug)]
pub struct Contracts {
    pub years: (Prim<Count>, Prim<Count>),
}

impl Contracts {
    fn open_country(&self, opening: &mut Opening<'_>, c: &OpeningCountry, kinds: (u16, u16), reason: ReasonId) {
        let (register, date, calendar, today) = (opening.register, opening.date, opening.calendar, opening.day);
        let (min, max) = (self.years.0.shared(register).get(), self.years.1.shared(register).get());
        let ccy = currency(c.id);
        let mut lot = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, LOTS, 0));
        let mut loans = opening.ctx.draws(&OpeningStream::DECL, subject(c.id, LOANS, 0));
        let (b, report) = books::split(opening);
        let banks = drawn(b, BANKS, c.id);
        let mut firms = drawn(b, FIRMS, c.id);
        let debts = drawn(b, DEBT, c.id);
        let Ok(count) = u64::try_from(firms.len()) else {
            phx_num::capacity_exceeded!("firms of a country", u64::MAX, firms.len());
        };
        let weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
        let counts = apportion(count, &weights, &mut lot);
        shuffle(&mut firms, &mut lot);
        let deposit_terms = b.ledger.terms.intern(terms(
            ccy,
            vec![Leg::RateOnNotional {
                reference: Reference::Fixed(rate(derived(c, "GEN.deposit_rate"))),
                day_count: DayCount::Act365F,
            }],
            Schedule { dates: monthly(date, c.id), count: Missing::Absent },
        ));
        let first_due = (monthly(date, c.id).nth(calendar, 1), 1);
        let lending = rate(derived(c, "GEN.lending_rate"));
        let mut lenders = Vec::with_capacity(firms.len());
        let mut at = 0_usize;
        for ((bank, weight), realised) in banks.iter().zip(&counts) {
            let Ok(n) = usize::try_from(*realised) else {
                violation!(clause = "GEN.4", "a bank's firms beyond the machine", bank = bank.get());
            };
            report.apportioned.push(Apportioned {
                stratum: key(FIRMS, c.id),
                party: *bank,
                drawn: *weight,
                realised: *realised,
            });
            let Some(mine) = firms.get(at..at + n) else {
                violation!(clause = "GEN.4", "firms apportioned beyond them")
            };
            at += n;
            if mine.is_empty() {
                continue;
            }
            let line = b.ledger.lines.open(kinds.0, deposit_terms, Missing::Present(first_due));
            let Ok(k) = u32::try_from(n) else { violation!(clause = "GEN.4", "a bank's firms beyond a row's count") };
            let mut legs = vec![open_row(register, *bank, line, Side::Liability, k, BALANCE)];
            legs.extend(mine.iter().map(|(f, _)| open_row(register, *f, line, Side::Asset, 1, BALANCE | PENDING)));
            b.open(reason, legs, bank.get(), report);
            for (firm, _) in mine {
                let Some(&(_, principal)) = debts.iter().find(|(p, _)| p == firm) else {
                    violation!(clause = "GEN.3", "a firm with no drawn debt", firm = firm.get());
                };
                let years = min + below_u64(&mut loans, max - min + 1);
                let Ok(months) = u32::try_from(years * u64::from(MONTHS_PER_YEAR)) else {
                    violation!(clause = "GEN.2", "a loan's term beyond a schedule's count", years = years);
                };
                let Ok(principal) = i64::try_from(principal) else {
                    violation!(clause = "MON.16", "a loan beyond whole smallest units", firm = firm.get());
                };
                let dates = monthly(started(calendar, date, today, months, &mut loans), c.id);
                let first = next_date(&dates, calendar, today);
                let loan_terms = b.ledger.terms.intern(terms(
                    ccy,
                    vec![
                        Leg::RateOnNotional { reference: Reference::Fixed(lending), day_count: DayCount::Act365F },
                        Leg::Principal { amount: Money::new(principal, ccy), repayment: Repayment::Bullet },
                    ],
                    Schedule { dates, count: Missing::Present(months) },
                ));
                let loan = b.ledger.lines.open(kinds.1, loan_terms, Missing::Present(first));
                let legs = vec![
                    open_row(register, *bank, loan, Side::Asset, 1, BALANCE),
                    open_row(register, *firm, loan, Side::Liability, 1, BALANCE),
                ];
                b.open(reason, legs, firm.get(), report);
                lenders.push((*firm, bank.get()));
            }
        }
        b.drawn.insert(key(LENDERS, c.id), lenders);
        let small_loans = b.ledger.terms.intern(terms(
            ccy,
            vec![Leg::RateOnNotional { reference: Reference::Fixed(lending), day_count: DayCount::Act365F }],
            Schedule { dates: monthly(date, c.id), count: Missing::Absent },
        ));
        Small { kinds, deposits: deposit_terms, loans: small_loans, first: Missing::Present(first_due) }.open(
            b,
            (register, reason),
            c,
            report,
        );
    }
}

/// The small firms' lines at their banks: each bank's current account for the small firms that bank with it and a
/// loan line they owe on, interest on the balance alone, a cell's row counting its firms; each cell's deposit and
/// debt written on them.
struct Small {
    kinds: (u16, u16),
    deposits: TermsId,
    loans: TermsId,
    first: Missing<(phx_id::Day, u32)>,
}

impl Small {
    #[clause("BNK.1", "FRM.23", "GEN.4")]
    fn open(
        &self,
        b: &mut Books,
        (register, reason): (&phx_core::Register, ReasonId),
        c: &OpeningCountry,
        report: &mut phx_core::GenReport,
    ) {
        let ccy = currency(c.id);
        let (banked, counts) = (drawn(b, SMALL_BANKS, c.id), drawn(b, SMALL_COUNTS, c.id));
        let (deposits, debts) = (drawn(b, SMALL_DEPOSITS, c.id), drawn(b, SMALL_DEBT, c.id));
        // Each agent's drawn amounts and firms found by its identity, as there are millions of agents.
        let (deposits, debts): (BTreeMap<PartyId, u64>, BTreeMap<PartyId, u64>) =
            (deposits.into_iter().collect(), debts.into_iter().collect());
        let counts: BTreeMap<PartyId, u64> = counts.into_iter().collect();
        let amount = |list: &BTreeMap<PartyId, u64>, cell: PartyId| {
            let Some(&a) = list.get(&cell) else {
                violation!(clause = "GEN.3", "a small firms' cell with no drawn amount", cell = cell.get());
            };
            let Ok(a) = i64::try_from(a) else {
                violation!(clause = "MON.16", "an amount beyond whole smallest units", cell = cell.get());
            };
            a
        };
        let mut by_bank: BTreeMap<u64, Vec<PartyId>> = BTreeMap::new();
        for (cell, bank) in &banked {
            by_bank.entry(*bank).or_default().push(*cell);
        }
        for (bank, banked) in by_bank {
            let bank_party = PartyId::new(bank);
            let cells: Vec<(PartyId, u32)> = banked
                .iter()
                .map(|cell| {
                    let Some(&n) = counts.get(cell) else {
                        violation!(clause = "GEN.3", "a small firms' cell with no drawn firms", cell = cell.get());
                    };
                    let Ok(n) = u32::try_from(n) else {
                        phx_num::capacity_exceeded!("firms of a cell", u32::MAX, n);
                    };
                    (*cell, n)
                })
                .collect();
            let total: u64 = cells.iter().map(|(_, n)| u64::from(*n)).sum();
            let Ok(total) = u32::try_from(total) else {
                phx_num::capacity_exceeded!("small firms of a bank", u32::MAX, total);
            };
            let account = b.ledger.lines.open(self.kinds.0, self.deposits, self.first);
            let loan = b.ledger.lines.open(self.kinds.1, self.loans, self.first);
            let mut legs = vec![
                open_row(register, bank_party, account, Side::Liability, total, BALANCE),
                open_row(register, bank_party, loan, Side::Asset, total, BALANCE),
            ];
            for (cell, n) in &cells {
                legs.push(open_row(register, *cell, account, Side::Asset, *n, BALANCE | PENDING));
                legs.push(open_row(register, *cell, loan, Side::Liability, *n, BALANCE));
            }
            b.open(reason, legs, bank, report);
            for (cell, _) in &cells {
                let (deposit, owed) = (amount(&deposits, *cell), amount(&debts, *cell));
                let id = cell.get();
                let legs = vec![
                    write(*cell, account, Side::Asset, deposit, ccy, id),
                    write(bank_party, account, Side::Liability, -deposit, ccy, id),
                    write(bank_party, loan, Side::Asset, owed, ccy, id),
                    write(*cell, loan, Side::Liability, -owed, ccy, id),
                ];
                b.open(reason, legs, id, report);
            }
        }
    }
}

impl Contribution for Contracts {
    fn name(&self) -> &'static str {
        "bank accounts and loans"
    }
    fn phase(&self) -> OpeningPhase {
        CONTRACTS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[BANKS, FIRMS, DEBT, SMALL_BANKS, SMALL_COUNTS, SMALL_DEPOSITS, SMALL_DEBT]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[LENDERS]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[LENDERS]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let countries = opening.countries;
        let b = books::of(opening);
        let reason = reason(b);
        let kinds = (b.ledger.lines.kind_index(HOLDERS.deposits(ACCOUNT).name), b.ledger.lines.kind_index(LOAN.name));
        for c in countries {
            self.open_country(opening, c, kinds, reason);
        }
    }
}

/// The balances of the banks' contracts, as the firms drew them: each firm's deposit on its current account, and
/// its loan, owed to its bank.
#[clause("GEN.4", "MON.7")]
#[derive(Debug)]
pub struct Balances;

impl Contribution for Balances {
    fn name(&self) -> &'static str {
        "bank balances"
    }
    fn phase(&self) -> OpeningPhase {
        BALANCES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[LENDERS, DEBT, DEPOSITS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &["BNK.balances"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["BNK.balances"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let countries = opening.countries;
        let (b, report) = books::split(opening);
        let reason = reason(b);
        for c in countries {
            let ccy = currency(c.id);
            let (lenders, debts, deposits) = (drawn(b, LENDERS, c.id), drawn(b, DEBT, c.id), drawn(b, DEPOSITS, c.id));
            for (firm, bank) in lenders {
                let bank = PartyId::new(bank);
                let amount = |list: &[(PartyId, u64)]| {
                    let Some(&(_, a)) = list.iter().find(|(p, _)| *p == firm) else {
                        violation!(clause = "GEN.3", "a firm with no drawn amount", firm = firm.get());
                    };
                    let Ok(a) = i64::try_from(a) else {
                        violation!(clause = "MON.16", "an amount beyond whole smallest units", firm = firm.get());
                    };
                    a
                };
                let (loan, deposit) = (amount(&debts), amount(&deposits));
                let (Some((account, _)), Some((line, _))) = (b.row_on(firm, ACCOUNT), b.row_on(firm, LOAN.name)) else {
                    violation!(clause = "GEN.4", "a firm with no account or no loan", firm = firm.get());
                };
                let id = firm.get();
                let legs = vec![
                    write(firm, account, Side::Asset, deposit, ccy, id),
                    write(bank, account, Side::Liability, -deposit, ccy, id),
                    write(bank, line, Side::Asset, loan, ccy, id),
                    write(firm, line, Side::Liability, -loan, ccy, id),
                ];
                b.open(reason, legs, id, report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::shuffle;

    fn draws(seed: u64) -> Draws {
        Draws::new(stream_key(Seed::new(seed), "BNK.lots"), Subject::new(SubjectTag::World, 0), 0, 0)
    }

    #[test]
    fn shuffle_keeps_every_item_once() {
        for seed in 0..20 {
            let mut items: Vec<u32> = (0..100).collect();
            shuffle(&mut items, &mut draws(seed));
            let mut sorted = items.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..100).collect::<Vec<u32>>(), "a permutation of what it was handed");
        }
    }

    #[test]
    fn shuffle_moves_the_first_item_anywhere() {
        let mut first_at = [0_u32; 4];
        for seed in 0..4000 {
            let mut items: Vec<usize> = (0..4).collect();
            shuffle(&mut items, &mut draws(seed));
            let Some(at) = items.iter().position(|i| *i == 0) else { panic!("the first item lost") };
            first_at[at] += 1;
        }
        assert!(first_at.iter().all(|n| (850..1150).contains(n)), "every position alike: {first_at:?}");
    }
}
