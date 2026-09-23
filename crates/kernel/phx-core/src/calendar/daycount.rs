use phx_id::{Date, days_from_civil};
use phx_macros::clause;
use phx_num::{DayFraction, RatePeriod, violation};

use crate::consts::{DAY_30, DAY_31, DAYS_30, DAYS_360, DAYS_365, DAYS_366};

/// A day count: how an accrual period's length is read from the calendar's dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DayCount {
    Act360,
    Act365F,
    ActActIsda,
    /// 30/360 as bond markets use it: a first date on the 31st counts as the 30th; a last date on the 31st counts as
    /// the 30th when the first is the 30th or 31st.
    Thirty360Bond,
    /// 30E/360: both dates on the 31st count as the 30th.
    Thirty360E,
}

fn thirty_360(start: Date, end: Date, d1: u8, d2: u8) -> i64 {
    DAYS_360 * (i64::from(end.year()) - i64::from(start.year()))
        + DAYS_30 * (i64::from(end.month()) - i64::from(start.month()))
        + (i64::from(d2) - i64::from(d1))
}

/// The fraction of a year from `start` to `end` by the day count, exactly, as a numerator over a denominator.
#[clause("TIME.4")]
pub fn day_fraction(start: Date, end: Date, dc: DayCount) -> DayFraction {
    let (s, e) = (days_from_civil(start), days_from_civil(end));
    if e < s {
        violation!(clause = "TIME.4", "an accrual period that ends before it starts", start = s, end = e);
    }
    let (num, den) = match dc {
        DayCount::Act360 => (e - s, DAYS_360),
        DayCount::Act365F => (e - s, DAYS_365),
        DayCount::ActActIsda => {
            // Each year's days over that year's length, summed over the common denominator 365·366.
            let (mut leap, mut common) = (0, 0);
            for year in start.year()..=end.year() {
                let (Some(first), Some(next)) = (Date::new(year, 1, 1), Date::new(year + 1, 1, 1)) else { continue };
                let from = if year == start.year() { s } else { days_from_civil(first) };
                let to = if year == end.year() { e } else { days_from_civil(next) };
                if Date::is_leap_year(year) {
                    leap += to - from;
                } else {
                    common += to - from;
                }
            }
            (common * DAYS_366 + leap * DAYS_365, DAYS_365 * DAYS_366)
        }
        DayCount::Thirty360Bond => {
            let d1 = if start.day() == DAY_31 { DAY_30 } else { start.day() };
            let d2 = if end.day() == DAY_31 && d1 >= DAY_30 { DAY_30 } else { end.day() };
            (thirty_360(start, end, d1, d2), DAYS_360)
        }
        DayCount::Thirty360E => {
            let d1 = if start.day() == DAY_31 { DAY_30 } else { start.day() };
            let d2 = if end.day() == DAY_31 { DAY_30 } else { end.day() };
            (thirty_360(start, end, d1, d2), DAYS_360)
        }
    };
    DayFraction::new(num, den, RatePeriod::Year)
}

#[cfg(test)]
mod tests {
    use phx_id::Date;

    use super::{DayCount, day_fraction};

    fn frac(a: (i32, u8, u8), b: (i32, u8, u8), dc: DayCount) -> (i64, i64) {
        let f = day_fraction(Date::new(a.0, a.1, a.2).unwrap(), Date::new(b.0, b.1, b.2).unwrap(), dc);
        (f.num(), f.den())
    }

    #[test]
    fn day_fraction_known() {
        assert_eq!(frac((2025, 1, 1), (2025, 7, 1), DayCount::Act360), (181, 360));
        assert_eq!(frac((2023, 12, 15), (2024, 1, 15), DayCount::ActActIsda), (17 * 366 + 14 * 365, 365 * 366));
        assert_eq!(frac((2025, 1, 31), (2025, 2, 28), DayCount::Thirty360Bond), (28, 360));
        assert_eq!(frac((2025, 1, 30), (2025, 3, 31), DayCount::Thirty360E), (60, 360));
        assert_eq!(
            frac((2024, 1, 1), (2025, 1, 1), DayCount::ActActIsda),
            (366 * 365, 365 * 366),
            "a whole leap year is one"
        );
        assert_eq!(frac((2025, 1, 1), (2026, 1, 1), DayCount::Act365F), (365, 365));
    }
}
