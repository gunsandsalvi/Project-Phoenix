//! One calendar (Money G1–G4): an epoch, a period length, one mapping from period to date, and
//! every periodicity placed on that grid BY DATE and never by a count of periods (G3.a).
//!
//! Time within a period is cycles (G1, G2); nothing finer exists. There is no default period
//! (G4.a), which is why `Period` is a value a caller must have rather than a number it can omit.
//!
//! **A `Day` CARRIES A CIVIL DATE** (22c.0). It was a day count from the epoch and nothing else — no
//! weekday, no month, no year — which is enough for everything placed by elapsed time (a maturity, an
//! accrual, a year fraction) and not enough for any convention that NAMES one: a contract expiring on
//! the third Friday of a delivery month, a fixing on the last business day, a quarter end. A ladder
//! anchored to the epoch instead was written, never called, and deleted at 21.132.OP1, because a
//! convention stated differently from the market is worse than none — a caller would have believed it.
//!
//! The mapping is the proleptic Gregorian one, as integer arithmetic over the day count. It reaches
//! for no clock and no library: `Date` and `std::time` are forbidden in the engine for the reason
//! this arithmetic exists — a world whose dates came from the machine it runs on is a world that runs
//! differently tomorrow.

/// A period index on the one calendar.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Period(pub u32);

/// A settlement cycle within a period, 0-based.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Cycle(pub u16);

/// A civil date, as a day number from the epoch — one mapping, and day counts come from dates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Day(pub i64);

/// 22c.0: the days of the week a market convention names. A venue that opens on a Wednesday, a
/// fixing on the last business day, a contract expiring on the third Friday: none of them could be
/// written down while a `Day` was only a count.
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
    /// Whether a market is open on it at all. A weekend is not a short week — it is a day on which
    /// nothing settles and nothing fixes, which is why *the last business day* is a convention.
    pub fn is_a_business_day(self) -> bool {
        !matches!(self, Weekday::Saturday | Weekday::Sunday)
    }
}

/// 22c.0: a date as a market names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Civil {
    pub year: i64,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
}

/// The day the epoch IS, in civil terms: 1 January 2000, a Saturday. It is a RESOLUTION — the world's
/// path must not turn on which day zero is — and it is stated once here so that every civil read in
/// the engine comes from one mapping (Law 4).
const EPOCH_YEAR: i64 = 2000;
const EPOCH_MONTH: u32 = 1;
const EPOCH_DAY: u32 = 1;

/// Days from 1970-01-01 to the civil date, proleptic Gregorian. Integer arithmetic, no clock: the
/// engine may not read the machine's date, and a world that did would run differently tomorrow.
const fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// And back. The inverse of `days_from_civil`, so the two cannot disagree about a date (Law 4).
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
    /// The civil date this day IS. One mapping (Money G3), and every convention reads through it.
    pub fn civil(self) -> Civil {
        civil_from_days(days_from_civil(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY) + self.0)
    }

    /// The day of the week. 1970-01-01 was a Thursday, which is what anchors the cycle.
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

    /// **22c.0: the nth such weekday of this day's month** — *the third Friday of the delivery
    /// month*, which is how an exchange states an expiry and which could not be written at all
    /// before. `None` where the month has no nth one, which is an answer about that month.
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

    /// **22c.0: the last business day of this day's month** — how a fixing and a quarter end are
    /// stated. A month always has one, so this is not an Option.
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
    /// The day the world opened. Everything is placed against this and nothing against "now".
    epoch: Day,
    /// G3: the period is 7 days. It is a RESOLUTION, tested by invariance, not a preference.
    days_per_period: u32,
    /// G2: the settlement cycles within a period.
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

    /// G3: the day a period starts on. One mapping, and every day count is taken from it.
    pub fn start_of(&self, at: Period) -> Day {
        Day(self.epoch.0 + i64::from(at.0) * i64::from(self.days_per_period))
    }

    /// G3.a: the first period at or after this day — a periodicity is placed BY DATE.
    pub fn period_on(&self, day: Day) -> Period {
        let since = day.0 - self.epoch.0;
        assert!(since >= 0, "Money G4: a day before the world opened is not a period");
        let whole = since / i64::from(self.days_per_period);
        let exact = since % i64::from(self.days_per_period) == 0;
        Period(if exact { whole as u32 } else { (whole + 1) as u32 })
    }

    /// Law 8, G3.a: how much of a year lies between two days, from the DATES and never from a
    /// count of periods. ACT/365F, which is the convention this kernel states once.
    pub fn year_fraction(&self, from: Day, to: Day) -> f64 {
        (to.0 - from.0) as f64 / 365.0
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_periodicity_is_placed_by_date_and_never_by_a_count_of_periods() {
        let cal = Calendar::new(Day(0), 7, 3);
        assert_eq!(cal.start_of(Period(0)), Day(0));
        assert_eq!(cal.start_of(Period(4)), Day(28));
        // A day inside a period belongs to the period that has not started yet, by date.
        assert_eq!(cal.period_on(Day(28)), Period(4));
        assert_eq!(cal.period_on(Day(29)), Period(5));
        assert!((cal.year_fraction(Day(0), Day(365)) - 1.0).abs() <= crate::num::dust(2, &[1.0]));
    }

    #[test]
    fn a_day_carries_a_civil_date_and_the_two_mappings_are_inverses() {
        // 22c.0: the epoch IS 1 January 2000, a Saturday, and every civil read goes through the one
        // mapping — so a date turned into a day and back is the day it started as (Law 4).
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
        // **22c.0: the third Friday of the delivery month, and the last business day.** Neither
        // could be written down while a `Day` was only a count, so no dated venue could exist.
        // March 2024: the Fridays are the 1st, 8th, 15th, 22nd and 29th.
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
