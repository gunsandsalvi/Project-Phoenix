//! One calendar: an epoch, a period length, one mapping from period to date, and every periodicity
//! placed on that grid BY DATE and never by a count of periods.

/// A period index on the one calendar.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Period(pub u32);

/// A settlement cycle within a period, 0-based.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Cycle(pub u16);

/// A civil date, as a day number from the epoch — one mapping, and day counts come from dates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Day(pub i64);

/// The days of the week a market convention names.
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
    /// Whether a market is open on it at all.
    pub fn is_a_business_day(self) -> bool {
        !matches!(self, Weekday::Saturday | Weekday::Sunday)
    }
}

/// A date as a market names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Civil {
    pub year: i64,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
}

/// The day the epoch IS, in civil terms: 1 January 2000, a Saturday.
const EPOCH_YEAR: i64 = 2000;
const EPOCH_MONTH: u32 = 1;
const EPOCH_DAY: u32 = 1;

/// Days from 1970-01-01 to the civil date, proleptic Gregorian.
const fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// And back.
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
    Civil { year: if m <= 2 { y + 1 } else { y }, month: m, day: d }
}

impl Day {
    /// The civil date this day IS.
    pub fn civil(self) -> Civil {
        civil_from_days(days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY) + self.0)
    }

    /// The day of the week.
    pub fn weekday(self) -> Weekday {
        let since_epoch = days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY) + self.0;
        match (since_epoch + 4).rem_euclid(7) {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }

    /// The day this civil date is, so a convention stated in civil terms comes back as a day count.
    pub fn of(year: i64, month: u32, day: u32) -> Day {
        assert!((1..=12).contains(&month), "22c.0: there is no month {month}");
        assert!((1..=31).contains(&day), "22c.0: there is no day {day} of a month");
        Day(days_from_civil(year, month, day) - days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY))
    }

    /// The nth such weekday of this day's month — *the third Friday of the delivery month*, which is
    /// how an exchange states an expiry and which could not be written at all before.
    pub fn nth_weekday_of_its_month(self, nth: u32, want: Weekday) -> Option<Day> {
        assert!(nth > 0, "22c.0: there is no zeroth Friday of a month");
        let Civil { year, month, .. } = self.civil();
        let first = Day::of(year, month, 1);
        let mut found = 0;
        for step in 0..31 {
            let d = Day(first.0 + step);
            if d.civil().month != month {
                break;
            }
            if d.weekday() == want {
                found += 1;
                if found == nth {
                    return Some(d);
                }
            }
        }
        None
    }

    /// The last business day of this day's month — how a fixing and a quarter end are stated.
    pub fn plus_months(self, months: i64) -> Day {
        let c = self.civil();
        let whole = (c.year * 12 + i64::from(c.month) - 1) + months;
        let year = whole.div_euclid(12);
        let month = (whole.rem_euclid(12) + 1) as u32;
        // The last day of that month, found by stepping back from the first of the next — no table
        // of month lengths, and the leap year falls out of the civil mapping.
        let next = if month == 12 { Day::of(year + 1, 1, 1) } else { Day::of(year, month + 1, 1) };
        let last = Day(next.0 - 1).civil().day;
        Day::of(year, month, if c.day < last { c.day } else { last })
    }

    pub fn last_business_day_of_its_month(self) -> Day {
        let Civil { year, month, .. } = self.civil();
        let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let mut d = Day(Day::of(next_year, next_month, 1).0 - 1);
        while !d.weekday().is_a_business_day() {
            d = Day(d.0 - 1);
        }
        d
    }
}

pub struct Calendar {
    /// The day the world opened.
    epoch: Day,
    days_per_period: u32,
    /// The settlement cycles within a period.
    cycles_per_period: u16,
}

impl Calendar {
    pub fn new(epoch: Day, days_per_period: u32, cycles_per_period: u16) -> Self {
        assert!(days_per_period > 0, "Money G3: a period is some days long");
        assert!(cycles_per_period > 0, "Money G2: a period has cycles in it");
        Self { epoch, days_per_period, cycles_per_period }
    }

    pub fn cycles_per_period(&self) -> u16 {
        self.cycles_per_period
    }

    /// The day a period starts on.
    pub fn start_of(&self, at: Period) -> Day {
        Day(self.epoch.0 + i64::from(at.0) * i64::from(self.days_per_period))
    }

    /// The first period at or after this day — a periodicity is placed BY DATE.
    pub fn period_on(&self, day: Day) -> Period {
        let since = day.0 - self.epoch.0;
        assert!(since >= 0, "Money G4: a day before the world opened is not a period");
        let whole = since / i64::from(self.days_per_period);
        let exact = since % i64::from(self.days_per_period) == 0;
        Period(if exact { whole as u32 } else { (whole + 1) as u32 })
    }

}

/// HOW A MARKET COUNTS A YEAR. A day count is a market CONVENTION, which is data, and at a short
/// tenor it is a material part of the number rather than a detail of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Convention {
    /// Money-market: actual days over 360.
    Actual360,
    /// Bond-equivalent: actual days over 365.
    Actual365,
}

impl Convention {
    /// What this convention calls a year, in days.
    pub fn year(self) -> f64 {
        match self {
            Convention::Actual360 => 360.0,
            Convention::Actual365 => 365.0,
        }
    }

    /// How much of a year lies between two days, from the DATES and never from a count of periods.
    pub fn year_fraction(self, from: Day, to: Day) -> f64 {
        (to.0 - from.0) as f64 / self.year()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_is_three_months_of_calendar_and_not_ninety_one_days() {
        // A quarter is three MONTHS, which is a whole number of days only by accident — 90 in one
        // and 92 in another — so a fiscal calendar advances the month.
        let opens = Day::of(2000, 1, 1);
        let closes = Day(opens.plus_months(3).0 - 1);
        assert_eq!(closes.civil(), Civil { year: 2000, month: 3, day: 31 });
        assert_eq!(closes.0 - opens.0 + 1, 91, "2000 was a leap year");
        let next = Day(closes.0 + 1);
        assert_eq!(Day(next.plus_months(3).0 - 1).civil(), Civil { year: 2000, month: 6, day: 30 });
        // Four quarters make a year, and adding 91 days four times does not.
        assert_eq!(opens.plus_months(12), Day::of(2001, 1, 1));
        assert_ne!(Day(opens.0 + 4 * 91), Day::of(2001, 1, 1));

        // A date the target month has not got is its LAST day, which is what a day that does not
        // exist means rather than one in the month after.
        assert_eq!(Day::of(2000, 1, 31).plus_months(1).civil(), Civil { year: 2000, month: 2, day: 29 });
        assert_eq!(Day::of(2001, 1, 31).plus_months(1).civil(), Civil { year: 2001, month: 2, day: 28 });
        // And it walks backwards, over a year boundary.
        assert_eq!(Day::of(2000, 1, 15).plus_months(-3).civil(), Civil { year: 1999, month: 10, day: 15 });
    }

    #[test]
    fn a_periodicity_is_placed_by_date_and_never_by_a_count_of_periods() {
        let cal = Calendar::new(Day(0), 7, 3);
        assert_eq!(cal.start_of(Period(0)), Day(0));
        assert_eq!(cal.start_of(Period(4)), Day(28));
        // A day inside a period belongs to the period that has not started yet, by date.
        assert_eq!(cal.period_on(Day(28)), Period(4));
        assert_eq!(cal.period_on(Day(29)), Period(5));
    }

    #[test]
    fn the_day_count_is_part_of_the_rate_and_not_a_detail_of_it() {
        // Actual over 365 and actual over 360 are two different numbers for the same days, which is
        // why the convention is declared rather than assumed.
        let (from, to) = (Day(0), Day(365));
        assert!(
            (Convention::Actual365.year_fraction(from, to) - 1.0).abs() <= crate::num::dust(2, &[1.0])
        );
        assert!(Convention::Actual360.year_fraction(from, to) > 1.0);
    }

    #[test]
    fn a_day_carries_a_civil_date_and_the_two_mappings_are_inverses() {
        // The epoch IS 1 January 2000, a Saturday, and every civil read goes through the one mapping
        // — so a date turned into a day and back is the day it started as.
        assert_eq!(Day(0).civil(), Civil { year: 2000, month: 1, day: 1 });
        assert_eq!(Day(0).weekday(), Weekday::Saturday);
        assert!(!Day(0).weekday().is_a_business_day());
        // A leap year, a century that is not a leap year, and a century that is.
        assert_eq!(Day::of(2000, 2, 29).civil(), Civil { year: 2000, month: 2, day: 29 });
        assert_eq!(Day::of(2100, 3, 1).0 - Day::of(2100, 2, 28).0, 1, "2100 is not a leap year");
        assert_eq!(Day::of(2024, 3, 1).0 - Day::of(2024, 2, 28).0, 2, "2024 is");
        for d in [Day(0), Day(1), Day(12_345), Day(-4_000)] {
            let c = d.civil();
            assert_eq!(Day::of(c.year, c.month, c.day), d);
        }
    }

    #[test]
    fn a_convention_that_names_a_weekday_can_be_stated() {
        // The third Friday of the delivery month, and the last business day.
        let march = Day::of(2024, 3, 7);
        assert_eq!(march.nth_weekday_of_its_month(3, Weekday::Friday), Some(Day::of(2024, 3, 15)));
        assert_eq!(march.nth_weekday_of_its_month(5, Weekday::Friday), Some(Day::of(2024, 3, 29)));
        // A month has no sixth Friday, and that is an answer about that month.
        assert_eq!(march.nth_weekday_of_its_month(6, Weekday::Friday), None);
        // 31 March 2024 is a Sunday, so the last business day is the 29th.
        assert_eq!(march.last_business_day_of_its_month(), Day::of(2024, 3, 29));
        // And a month ending on a weekday ends on it.
        assert_eq!(Day::of(2024, 4, 2).last_business_day_of_its_month(), Day::of(2024, 4, 30));
    }

}
