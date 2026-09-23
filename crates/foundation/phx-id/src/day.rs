use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{
    CENTURY, COMMON_FEBRUARY, DAYS_PER_CENTURY, DAYS_PER_ERA, DAYS_PER_LEAP_CYCLE, DAYS_PER_WEEK, DAYS_PER_YEAR,
    LEAP_EVERY, LEAP_FEBRUARY, LONGEST_MONTH, MARCH, MARCH_BASED_JANUARY, MONTH_SPAN_DEN, MONTH_SPAN_NUM, MONTHS,
    SERIAL_SHIFT, SERIAL_ZERO_WEEKDAY, SHORT_MONTH, THIRTY_DAY_MONTHS, YEARS_PER_ERA,
};

/// A day of the world, counted from the calendar's epoch; there is no default day.
#[clause("TIME.1")]
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Day(u32);

impl Day {
    pub const fn new(raw: u32) -> Day {
        Day(raw)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn succ(self) -> Day {
        let Some(next) = self.0.checked_add(1) else {
            capacity_exceeded!("day count", u32::MAX, u64::from(self.0) + 1);
        };
        Day(next)
    }

    /// The earlier of two days.
    pub fn earlier(a: Day, b: Day) -> Day {
        if a <= b { a } else { b }
    }

    /// The later of two days.
    pub fn later(a: Day, b: Day) -> Day {
        if a >= b { a } else { b }
    }

    /// The day a date falls on, or none when it lies before the epoch or beyond the last countable day.
    #[must_use]
    pub fn from_date(date: Date, epoch: Date) -> Option<Day> {
        u32::try_from(days_from_civil(date) - days_from_civil(epoch)).ok().map(Day)
    }

    pub fn date(self, epoch: Date) -> Date {
        civil_from_days(days_from_civil(epoch) + i64::from(self.0))
    }
}

/// A proleptic Gregorian date; only a real date can be built.
#[clause("TIME.2")]
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

fn is_leap(year: i64) -> bool {
    year.rem_euclid(LEAP_EVERY) == 0 && (year.rem_euclid(CENTURY) != 0 || year.rem_euclid(YEARS_PER_ERA) == 0)
}

fn month_length(year: i32, month: u8) -> u8 {
    if month == 2 {
        if is_leap(i64::from(year)) { LEAP_FEBRUARY } else { COMMON_FEBRUARY }
    } else if THIRTY_DAY_MONTHS.contains(&month) {
        SHORT_MONTH
    } else {
        LONGEST_MONTH
    }
}

impl Date {
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Date> {
        let month_ok = month >= 1 && i64::from(month) <= MONTHS;
        (month_ok && day >= 1 && day <= month_length(year, month)).then_some(Date { year, month, day })
    }

    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }
}

/// Days from 1970-01-01 to a date, by Hinnant's algorithm over years that begin in March.
#[must_use]
pub fn days_from_civil(date: Date) -> i64 {
    let month = i64::from(date.month);
    let year = i64::from(date.year) - i64::from(month < MARCH);
    let era = year.div_euclid(YEARS_PER_ERA);
    let year_of_era = year - era * YEARS_PER_ERA;
    let march_month = (month - MARCH).rem_euclid(MONTHS);
    let day_of_year = (MONTH_SPAN_NUM * march_month + 2) / MONTH_SPAN_DEN + i64::from(date.day) - 1;
    let day_of_era = year_of_era * DAYS_PER_YEAR + year_of_era / LEAP_EVERY - year_of_era / CENTURY + day_of_year;
    era * DAYS_PER_ERA + day_of_era - SERIAL_SHIFT
}

/// The date of a serial counted from 1970-01-01, the inverse of `days_from_civil`.
pub fn civil_from_days(serial: i64) -> Date {
    let Some(shifted) = serial.checked_add(SERIAL_SHIFT) else {
        capacity_exceeded!("civil serial", i64::MAX - SERIAL_SHIFT, serial);
    };
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted - era * DAYS_PER_ERA;
    let year_of_era = (day_of_era - day_of_era / DAYS_PER_LEAP_CYCLE + day_of_era / DAYS_PER_CENTURY
        - day_of_era / (DAYS_PER_ERA - 1))
        / DAYS_PER_YEAR;
    let day_of_year = day_of_era - (DAYS_PER_YEAR * year_of_era + year_of_era / LEAP_EVERY - year_of_era / CENTURY);
    let march_month = (MONTH_SPAN_DEN * day_of_year + 2) / MONTH_SPAN_NUM;
    let day = day_of_year - (MONTH_SPAN_NUM * march_month + 2) / MONTH_SPAN_DEN + 1;
    let month = (march_month + MARCH - 1).rem_euclid(MONTHS) + 1;
    let year = year_of_era + era * YEARS_PER_ERA + i64::from(march_month >= MARCH_BASED_JANUARY);
    let Ok(year) = i32::try_from(year) else {
        capacity_exceeded!("civil year", i32::MAX, year);
    };
    let (Ok(month), Ok(day)) = (u8::try_from(month), u8::try_from(day)) else {
        violation!(clause = "TIME.2", "a civil month or day outside its range", month = month, day = day);
    };
    Date { year, month, day }
}

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

const WEEK: &[Weekday] = &[
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

impl Weekday {
    /// A day's weekday, derived from the epoch's civil date rather than declared.
    pub fn of(day: Day, epoch: Date) -> Weekday {
        let serial = days_from_civil(epoch) + i64::from(day.0);
        let index = (serial + SERIAL_ZERO_WEEKDAY).rem_euclid(DAYS_PER_WEEK);
        let Some(weekday) = usize::try_from(index).ok().and_then(|i| WEEK.get(i)) else {
            violation!(clause = "TIME.2", "a weekday outside the week", index = index);
        };
        *weekday
    }
}

#[cfg(test)]
mod tests {
    use super::{Date, Day, Weekday, civil_from_days, days_from_civil};

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).expect("a real date")
    }

    #[test]
    fn civil_roundtrip() {
        let mut previous = civil_from_days(-1_000_001);
        for serial in -1_000_000..=1_000_000 {
            let d = civil_from_days(serial);
            assert_eq!(days_from_civil(d), serial);
            assert!(d > previous && Date::new(d.year(), d.month(), d.day()) == Some(d), "{serial}");
            previous = d;
        }
        assert_eq!(days_from_civil(date(1970, 1, 1)), 0);
        assert_eq!(days_from_civil(date(2000, 3, 1)) - days_from_civil(date(2000, 2, 28)), 2);
        assert!(Date::new(1900, 2, 29).is_none() && Date::new(2100, 2, 29).is_none());
        assert!(Date::new(2000, 2, 29).is_some() && Date::new(2024, 2, 29).is_some());
        assert_eq!(days_from_civil(date(1900, 3, 1)) - days_from_civil(date(1900, 2, 28)), 1);
        assert_eq!(days_from_civil(date(2100, 3, 1)) - days_from_civil(date(2100, 2, 28)), 1);
        for bad in [(2001, 0, 1), (2001, 13, 1), (2001, 4, 31), (2001, 1, 0), (2001, 1, 32)] {
            assert!(Date::new(bad.0, bad.1, bad.2).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn weekday_known() {
        let epoch = date(1999, 12, 25);
        assert_eq!(Weekday::of(Day::new(7), epoch), Weekday::Saturday);
        assert_eq!(Weekday::of(Day::new(0), date(1970, 1, 1)), Weekday::Thursday);
        assert_eq!(Weekday::of(Day::new(1), date(1969, 12, 31)), Weekday::Thursday);
    }

    #[test]
    fn days_convert_through_the_epoch() {
        let epoch = date(1999, 12, 31);
        assert_eq!(Day::from_date(date(2000, 3, 1), epoch), Some(Day::new(61)));
        assert_eq!(Day::new(61).date(epoch), date(2000, 3, 1));
        assert_eq!(Day::from_date(date(1999, 12, 30), epoch), None);
        assert_eq!(Day::new(4).succ(), Day::new(5));
        assert_eq!(
            (Day::earlier(Day::new(3), Day::new(9)), Day::later(Day::new(3), Day::new(9))),
            (Day::new(3), Day::new(9))
        );
    }
}
