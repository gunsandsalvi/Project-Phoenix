#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "gungraun's harness prints and exits; a setup that fails is a broken benchmark"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_core::{
    BusinessDayConvention, Calendar, CountryRules, DayCount, EndOfMonth, Period, ScheduleDates, WeekendRule,
};
use phx_id::{CountryId, Date, Day, Weekday};
use phx_ledger::algebra::{
    DefaultDefinition, DueBuf, DueState, Leg, PaymentOrder, Reference, Repayment, Schedule, Seniority, Termination,
    Terms, due_on,
};
use phx_num::{Ccy, Missing, Money, Rate, RatePeriod};

const EUR: Ccy = Ccy::new(0);

fn calendar() -> Calendar {
    let rules =
        CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
    Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2020).unwrap()
}

/// A ten-year bond paying a fixed coupon a year and its principal at the end, asked for its dues on its fifth
/// coupon date: two legs, one due.
fn bond() -> (Calendar, Terms, Day) {
    let cal = calendar();
    let schedule = Schedule {
        dates: ScheduleDates {
            anchor: Date::new(2026, 1, 15).unwrap(),
            period: Period::months(12).unwrap(),
            eom: EndOfMonth::Plain,
            convention: BusinessDayConvention::Following,
            country: CountryId::new(0),
        },
        count: Missing::Present(10),
    };
    let terms = Terms {
        ccy: EUR,
        legs: vec![
            Leg::RateOnNotional {
                reference: Reference::Fixed(Rate::new(50_000_000_000, RatePeriod::Year)),
                day_count: DayCount::Thirty360Bond,
            },
            Leg::Principal { amount: Money::new(1_000_000, EUR), repayment: Repayment::Bullet },
        ],
        schedule,
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 30 },
        underlying: Missing::Absent,
    };
    let day = schedule.day(&cal, 5);
    (cal, terms, day)
}

/// The same bond on the day after its fifth coupon date: nothing due, so only the search for a date is paid.
fn bond_between() -> (Calendar, Terms, Day) {
    let (cal, terms, day) = bond();
    (cal, terms, day.succ())
}

fn dues(setup: (Calendar, Terms, Day)) -> usize {
    let (cal, terms, day) = setup;
    let state = DueState {
        calendar: &cal,
        outstanding: Money::new(1_000_000, EUR),
        elected: &|_, _| false,
        occurred: &|_, _| false,
        in_state_since: &|_, _| Missing::Absent,
    };
    let mut out = DueBuf::default();
    due_on(black_box(&terms), black_box(day), &state, &mut out);
    black_box(out.iter().count())
}

#[library_benchmark]
#[bench::fresh(bond())]
fn ir_due_on_coupon(setup: (Calendar, Terms, Day)) -> usize {
    dues(setup)
}

#[library_benchmark]
#[bench::fresh(bond_between())]
fn ir_due_on_between(setup: (Calendar, Terms, Day)) -> usize {
    dues(setup)
}

library_benchmark_group!(name = ledger, benchmarks = [ir_due_on_coupon, ir_due_on_between]);

main!(library_benchmark_groups = ledger);
