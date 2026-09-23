#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::needless_pass_by_value,
    reason = "gungraun's harness prints and exits, and hands each benchmark its setup by value"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_num::{
    Ccy, DayFraction, Money, Price, Qty, Rate, RatePeriod, Round, UnitId, UnitTable, accrue, div_round, split_total,
    value_of,
};

const C: Ccy = Ccy::new(0);
const U: UnitId = UnitId::new(0);

fn units() -> UnitTable {
    let Ok(units) = UnitTable::new(vec![3]) else { unreachable!("exponent 3 is within the declared bound") };
    units
}

#[library_benchmark]
#[bench::retail(units())]
fn ir_value_of(units: UnitTable) -> Money {
    black_box(value_of(black_box(Qty::new(3, U)), black_box(Price::new(3_333, C, U)), &units, Round::HalfEven))
}

#[library_benchmark]
fn ir_accrue() -> Money {
    let rate = Rate::new(50_000_000_000, RatePeriod::Year);
    let f = DayFraction::new(31, 365, RatePeriod::Year);
    black_box(accrue(black_box(Money::new(1_000_000, C)), black_box(rate), black_box(f), Round::HalfEven))
}

#[library_benchmark]
fn ir_div_round() -> i128 {
    black_box(div_round(black_box(-7_777_777), black_box(13), Round::HalfEven))
}

#[library_benchmark]
fn ir_split_total() -> (i64, i64) {
    black_box(split_total(black_box(1_000_003), black_box(37), black_box(97), Round::HalfEven))
}

library_benchmark_group!(name = arithmetic, benchmarks = [ir_value_of, ir_accrue, ir_div_round, ir_split_total]);

main!(library_benchmark_groups = arithmetic);
