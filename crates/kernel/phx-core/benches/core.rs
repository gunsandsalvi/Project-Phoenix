#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::needless_pass_by_value,
    clippy::unwrap_used,
    reason = "gungraun's harness prints, exits and passes setups by value; a setup that fails is a broken benchmark"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_core::{
    Agenda, AgendaTableSpec, BusinessDayConvention, Calendar, CountryRules, DecisionSchedule, HolidayRule, Period,
    Phase, RunsOn, WeekendRule, next_due,
};
use phx_id::{CountryId, Date, Day, Slot, TableId, Weekday};
use phx_store::{AddressSpace, HeapBacking};

const ROWS: u32 = 4096;
const C: CountryId = CountryId::new(0);

fn calendar() -> Calendar {
    let fixed = |name: &str, month, day| HolidayRule::Fixed { name: name.to_owned(), month, day };
    let rules = CountryRules {
        weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
        holidays: vec![
            fixed("New Year", 1, 1),
            HolidayRule::EasterOffset { name: "Good Friday".to_owned(), days: -2 },
            fixed("Christmas", 12, 25),
        ],
    };
    Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(C, rules)], 2020).unwrap()
}

/// 4 096 rows with two reasons each, due over the next eight days, gathered on the first.
fn booked_agenda() -> Agenda<HeapBacking> {
    let spec = AgendaTableSpec { table: TableId::new(0), max_rows: ROWS, reasons: 2 };
    let mut a = Agenda::new(&mut AddressSpace::empty(), Day::new(0), &[spec], 1 << 16).unwrap();
    a.grow(spec.table, ROWS);
    for row in 0..ROWS {
        a.set_next(spec.table, Slot::new(row), 0, Day::new(1 + row % 8));
        a.set_next(spec.table, Slot::new(row), 1, Day::new(9 + row % 64));
    }
    a
}

#[library_benchmark]
#[bench::fresh(calendar())]
fn ir_is_business(cal: Calendar) -> usize {
    let from = cal.day(Date::new(2025, 1, 1).unwrap()).unwrap().get();
    black_box((from..from + 64).filter(|d| cal.is_business(C, Day::new(*d))).count())
}

#[library_benchmark]
#[bench::fresh(calendar())]
fn ir_next_due(cal: Calendar) -> Day {
    let monthly = DecisionSchedule {
        period: Period::months(1).unwrap(),
        convention: BusinessDayConvention::ModifiedFollowing,
        runs_on: RunsOn::Business,
    };
    let after = cal.day(Date::new(2025, 3, 1).unwrap()).unwrap();
    black_box(next_due(&cal, C, monthly, Phase::within(monthly.period, 14).unwrap(), after))
}

#[library_benchmark]
#[bench::fresh(booked_agenda())]
fn ir_agenda_gather(mut agenda: Agenda<HeapBacking>) -> usize {
    black_box(agenda.gather(Day::new(1)).per_table.iter().map(|t| t.slots.len()).sum())
}

library_benchmark_group!(name = core, benchmarks = [ir_is_business, ir_next_due, ir_agenda_gather]);

main!(library_benchmark_groups = core);
