use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{
    Apportioned, BALANCES, CONTRACTS, Contribution, Opening, OpeningCountry, OpeningPhase, PARTIES, Prim, StreamDef,
    apportion, opening_subject,
};
use phx_id::{CountryId, Date, PartyId};
use phx_ledger::algebra::{
    DefaultDefinition, Facility, Leg, PaymentOrder, Reference, Repayment, Schedule, Seniority, Side, Termination, Terms,
};
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, ReasonDecl, ReasonId};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::money::MoneyHolders;
use phx_ledger::opening::{currency, derived, key, open_row, whole, write};
use phx_ledger::rows::{BALANCE, PENDING};
use phx_macros::clause;
use phx_num::{Count, Missing, Money, Rate, RatePeriod, violation};
use phx_rand::below_u64;

use crate::consts::{LOANS, LOTS, MONTHS_PER_YEAR, PERCENT, PURPOSES, RATE_ONE, SHARE_PARTS, SITES};
use crate::{BANK, OpeningStream, zipf};

const BANKS: &str = "BNK.banks";
const FIRMS: &str = "FRM.firms";
const DEBT: &str = "FRM.debt";
const DEPOSITS: &str = "FRM.deposits";
const LENDERS: &str = "BNK.lenders";
const ACCOUNT: &str = "current account";

/// Current accounts: a bank's liability to the firms that bank with it.
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &["central_bank"],
    banks: &[BANK.name],
    treasuries: &["treasury"],
    depositors: &["firm"],
    requesters: &["BNK"],
};

/// A firm's term loan from its bank, one contract to a line.
const LOAN: LineKindDecl = LineKindDecl {
    name: "firm term loan",
    asset: SideDecl { holder_kinds: &[BANK.name], words: BALANCE, holder_list: true },
    liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
    dated: true,
    transfer_requesters: &["BNK"],
};

fn subject(country: CountryId, purpose: u32, ordinal: u32) -> phx_rand::Subject {
    opening_subject(u32::from(country.get()) * PURPOSES + purpose, ordinal)
}

fn reason(b: &mut Books) -> ReasonId {
    b.ledger.reasons.declare(ReasonDecl {
        name: "BNK opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    })
}

fn drawn(b: &Books, name: &str, country: CountryId) -> Vec<(PartyId, u64)> {
    let Some(v) = b.drawn.get(&key(name, country)) else {
        violation!(clause = "GEN.3", "the banks' opening reading a stratum not yet drawn", country = country.get());
    };
    v.clone()
}

fn rate(percent: f64) -> Rate {
    Rate::new(whole(percent / PERCENT * RATE_ONE), RatePeriod::Year)
}

fn monthly(anchor: Date, country: CountryId) -> ScheduleDates {
    let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
    ScheduleDates {
        anchor,
        period: months,
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country,
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

fn terms(ccy: phx_num::Ccy, legs: Vec<Leg>, schedule: Schedule) -> Terms {
    Terms {
        ccy,
        legs,
        schedule,
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 0 },
        underlying: Missing::Absent,
        facility: Missing::<Facility>::Absent,
        stay: Missing::Absent,
    }
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
        let firms = drawn(b, FIRMS, c.id);
        let debts = drawn(b, DEBT, c.id);
        let Ok(count) = u64::try_from(firms.len()) else {
            phx_num::capacity_exceeded!("firms of a country", u64::MAX, firms.len());
        };
        let weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
        let counts = apportion(count, &weights, &mut lot);
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
        &[BANKS, FIRMS, DEBT]
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
        let kinds = (
            b.ledger.lines.declare_deposits(HOLDERS.deposits(ACCOUNT)).index(),
            b.ledger.lines.declare_money(LOAN).index(),
        );
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
