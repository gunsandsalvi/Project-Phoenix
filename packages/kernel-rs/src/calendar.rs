//! The kernel's single executable clock and its civil-date boundary adapter.
//!
//! Executable state uses [`Week`] exclusively. [`CivilDate`] exists only long enough to map an
//! external date onto the first weekly tick on or after it. Financial day counts are derived from
//! the seven-day distance between weekly boundaries; they are measurements, not another clock.

/// A weekly tick since the fixed epoch (Saturday, 1 January 2000).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Week(pub i64);

impl Week {
    pub const DAYS: i64 = 7;

    /// Advance by a number of fixed weekly ticks.
    pub fn after(self, weeks: u32) -> Self {
        Self(self.0.checked_add(i64::from(weeks)).expect("week overflow"))
    }

    /// Whole civil days between weekly boundaries, for financial measurement only.
    pub fn elapsed_days_until(self, to: Self) -> i64 {
        to.0.checked_sub(self.0)
            .expect("end week precedes start week")
            * Self::DAYS
    }
}

/// An external Gregorian date. This is a boundary value and is never stored in executable state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CivilDate {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

impl CivilDate {
    /// A day the target month does not have becomes its last, because there is no thirty-first of
    /// February to land on.
    fn after_months(self, months: u32) -> CivilDate {
        let month = self.month as i64 - 1 + i64::from(months);
        let year = self.year + month / 12;
        let month = (month % 12) as u32 + 1;
        let last = civil_from_days(days_from_civil(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        ) - 1)
        .day;
        CivilDate {
            year,
            month,
            day: if self.day > last { last } else { self.day },
        }
    }
}

const EPOCH_YEAR: i64 = 2000;
const EPOCH_MONTH: u32 = 1;
const EPOCH_DAY: u32 = 1;

const fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

const fn civil_from_days(z: i64) -> CivilDate {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    CivilDate {
        year: if m <= 2 { y + 1 } else { y },
        month: m,
        day: d,
    }
}

/// Fixed weekly calendar. It has no runtime configuration.
#[derive(Clone, Copy, Debug, Default)]
pub struct Calendar;

impl Calendar {
    pub const fn new() -> Self {
        Self
    }

    /// Normalize a weekly tick (the fixed calendar has no offset or variable step).
    pub const fn at(&self, week: Week) -> Week {
        week
    }

    /// Convert an external civil date to the first weekly boundary on or after it.
    pub fn week_on_or_after(&self, date: CivilDate) -> Week {
        assert!((1..=12).contains(&date.month), "invalid civil month");
        assert!((1..=31).contains(&date.day), "invalid civil day");
        let days = days_from_civil(date.year, date.month, date.day)
            - days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY);
        assert!(days >= 0, "civil date precedes simulation epoch");
        Week((days + 6) / 7)
    }

    /// PLACE A PERIODICITY: the tick a date this many months after `from` falls on. A month is a
    /// month and a quarter is three of them, neither a count of weeks, so the advance happens on
    /// the civil date and only the answer is a tick — which is why no caller holds the date.
    pub fn months_after(&self, from: Week, months: u32) -> Week {
        self.week_on_or_after(self.civil_date(from).after_months(months))
    }

    /// Present a weekly boundary as a civil date for output.
    pub fn civil_date(&self, week: Week) -> CivilDate {
        civil_from_days(days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY) + week.0 * 7)
    }
}

/// A market day-count convention. This measures an interval between weekly boundaries; it does not
/// schedule executable work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Convention {
    Actual360,
    Actual365,
}
impl Convention {
    pub fn year(self) -> f64 {
        match self {
            Self::Actual360 => 360.0,
            Self::Actual365 => 365.0,
        }
    }
    pub fn year_fraction(self, from: Week, to: Week) -> f64 {
        from.elapsed_days_until(to) as f64 / self.year()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn civil_dates_round_up_at_the_boundary() {
        let cal = Calendar::new();
        assert_eq!(
            cal.week_on_or_after(CivilDate {
                year: 2000,
                month: 1,
                day: 1
            }),
            Week(0)
        );
        assert_eq!(
            cal.week_on_or_after(CivilDate {
                year: 2000,
                month: 1,
                day: 2
            }),
            Week(1)
        );
        assert_eq!(
            cal.civil_date(Week(1)),
            CivilDate {
                year: 2000,
                month: 1,
                day: 8
            }
        );
    }
    #[test]
    fn day_count_is_measurement_between_weekly_boundaries() {
        assert_eq!(Week(0).elapsed_days_until(Week(52)), 364);
        assert!(
            (Convention::Actual365.year_fraction(Week(0), Week(52)) - 364.0 / 365.0).abs()
                < f64::EPSILON
        );
    }
}
