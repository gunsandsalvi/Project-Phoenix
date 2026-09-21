//! The kernel's single weekly clock.
//!
//! Executable state uses [`Week`] exclusively. Civil dates are input/presentation values and are
//! converted immediately using one rule: select the first weekly tick on or after the civil date.

/// An integer number of seven-day simulation ticks from the epoch (2000-01-01).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Week(pub i64);

impl From<u32> for Week {
    fn from(value: u32) -> Self {
        Self(i64::from(value))
    }
}

/// The days of a civil week. This is presentation/input data, never executable time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}
impl Weekday {
    pub fn is_a_business_day(self) -> bool {
        !matches!(self, Self::Saturday | Self::Sunday)
    }
}

/// A non-schedulable civil date used only at input and presentation boundaries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
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
const fn civil_from_days(z: i64) -> Civil {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    Civil {
        year: if m <= 2 { y + 1 } else { y },
        month: m,
        day: d,
    }
}
impl Civil {
    fn day_number(self) -> i64 {
        assert!((1..=12).contains(&self.month), "22c.0: invalid month");
        assert!((1..=31).contains(&self.day), "22c.0: invalid day");
        days_from_civil(self.year, self.month, self.day)
    }
    pub fn weekday(self) -> Weekday {
        match (self.day_number() + 4).rem_euclid(7) {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }
}

impl Week {
    /// Convert civil input to the first weekly tick on or after it.
    pub fn on_or_after(civil: Civil) -> Self {
        let days = civil.day_number() - days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY);
        Self(days.div_euclid(7) + i64::from(days.rem_euclid(7) != 0))
    }
    /// Convenience input constructor; it applies [`Week::on_or_after`].
    pub fn of(year: i64, month: u32, day: u32) -> Self {
        Self::on_or_after(Civil { year, month, day })
    }
    /// Civil date of this weekly boundary, for presentation only.
    pub fn civil(self) -> Civil {
        civil_from_days(days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY) + self.0 * 7)
    }
    pub fn weekday(self) -> Weekday {
        self.civil().weekday()
    }
    pub fn plus_months(self, months: i64) -> Self {
        let c = self.civil();
        let whole = c.year * 12 + i64::from(c.month) - 1 + months;
        let year = whole.div_euclid(12);
        let month = (whole.rem_euclid(12) + 1) as u32;
        let next = if month == 12 {
            Civil {
                year: year + 1,
                month: 1,
                day: 1,
            }
        } else {
            Civil {
                year,
                month: month + 1,
                day: 1,
            }
        };
        let last = civil_from_days(next.day_number() - 1).day;
        Self::on_or_after(Civil {
            year,
            month,
            day: c.day.min(last),
        })
    }
    pub fn nth_weekday_of_its_month(self, nth: u32, want: Weekday) -> Option<Self> {
        assert!(nth > 0);
        let c = self.civil();
        let first = Civil {
            year: c.year,
            month: c.month,
            day: 1,
        };
        let mut found = 0;
        for step in 0..31 {
            let candidate = civil_from_days(first.day_number() + step);
            if candidate.month != c.month {
                break;
            }
            if candidate.weekday() == want {
                found += 1;
                if found == nth {
                    return Some(Self::on_or_after(candidate));
                }
            }
        }
        None
    }
    pub fn last_business_day_of_its_month(self) -> Self {
        let c = self.civil();
        let next = if c.month == 12 {
            Civil {
                year: c.year + 1,
                month: 1,
                day: 1,
            }
        } else {
            Civil {
                year: c.year,
                month: c.month + 1,
                day: 1,
            }
        };
        let mut day = next.day_number() - 1;
        while !civil_from_days(day).weekday().is_a_business_day() {
            day -= 1;
        }
        Self::on_or_after(civil_from_days(day))
    }
}

/// Stateless weekly calendar. Tick length is deliberately not configurable.
#[derive(Clone, Copy, Debug, Default)]
pub struct Calendar;
impl Calendar {
    pub const fn new() -> Self {
        Self
    }
    pub const fn next(&self, week: Week) -> Week {
        Week(week.0 + 1)
    }
}

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
        ((to.0 - from.0) * 7) as f64 / self.year()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consecutive_ticks_are_exactly_one_week() {
        let calendar = Calendar::new();
        assert_eq!(calendar.next(Week(41)), Week(42));
        assert_eq!(
            calendar.next(Week(41)).civil(),
            Civil {
                year: 2000,
                month: 10,
                day: 21
            }
        );
    }
    #[test]
    fn civil_input_rounds_up_to_the_first_weekly_boundary() {
        assert_eq!(
            Week::on_or_after(Civil {
                year: 2000,
                month: 1,
                day: 1
            }),
            Week(0)
        );
        assert_eq!(
            Week::on_or_after(Civil {
                year: 2000,
                month: 1,
                day: 2
            }),
            Week(1)
        );
        assert_eq!(
            Week::on_or_after(Civil {
                year: 2000,
                month: 1,
                day: 8
            }),
            Week(1)
        );
    }
    #[test]
    fn calendar_has_no_tick_length_configuration() {
        assert_eq!(core::mem::size_of::<Calendar>(), 0);
    }
}
