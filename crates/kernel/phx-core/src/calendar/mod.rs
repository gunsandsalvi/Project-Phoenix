pub mod bizday;
pub mod daycount;
pub mod period;
pub mod prims;
pub mod rules;

use phx_id::{CountryId, Date, Day, civil_from_days, days_from_civil};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::calendar::period::{EndOfMonth, Period, advance};
use crate::calendar::rules::CountryRules;
use crate::consts::{CALENDAR_WINDOW_YEARS, CALENDAR_WORD_BITS as BITS};

/// One country's business days: its rules, which are the calendar's state, and a bitset of a window of years built
/// from them, which is not.
#[derive(Clone, Debug)]
pub struct CountryCalendar {
    pub country: CountryId,
    rules: CountryRules,
    /// Days of the window, from the first day of its first year.
    window_start: i64,
    window_days: i64,
    bits: Vec<u64>,
}

impl CountryCalendar {
    fn build(country: CountryId, rules: CountryRules, first_year: i32) -> CountryCalendar {
        let (Some(first), Some(after)) =
            (Date::new(first_year, 1, 1), Date::new(first_year + CALENDAR_WINDOW_YEARS, 1, 1))
        else {
            capacity_exceeded!("calendar years", i32::MAX, first_year);
        };
        let (start, end) = (days_from_civil(first), days_from_civil(after));
        let Ok(words) = usize::try_from((end - start + BITS - 1) / BITS) else {
            capacity_exceeded!("calendar window words", usize::MAX, end - start);
        };
        let mut bits = vec![0_u64; words];
        for year in first_year..first_year + CALENDAR_WINDOW_YEARS {
            let holidays: Vec<i64> = rules.holidays_in(year).into_iter().map(days_from_civil).collect();
            let (Some(from), Some(to)) = (Date::new(year, 1, 1), Date::new(year + 1, 1, 1)) else {
                capacity_exceeded!("calendar years", i32::MAX, year);
            };
            for s in days_from_civil(from)..days_from_civil(to) {
                if rules.is_weekend(civil_from_days(s)) || holidays.binary_search(&s).is_ok() {
                    continue;
                }
                let at = s - start;
                let Some(word) = usize::try_from(at / BITS).ok().and_then(|w| bits.get_mut(w)) else {
                    violation!(clause = "TIME.2", "a day past the calendar's window", day = at);
                };
                *word |= 1 << (at % BITS);
            }
        }
        CountryCalendar { country, rules, window_start: start, window_days: end - start, bits }
    }

    /// Whether a date is a business day: one bit inside the window, the rules outside it.
    fn is_business_serial(&self, serial: i64) -> bool {
        let at = serial - self.window_start;
        if (0..self.window_days).contains(&at) {
            let Some(word) = usize::try_from(at / BITS).ok().and_then(|w| self.bits.get(w)) else {
                violation!(clause = "TIME.2", "a day past the calendar's window", day = at);
            };
            return word & (1 << (at % BITS)) != 0;
        }
        let date = civil_from_days(serial);
        !self.rules.is_weekend(date) && !self.rules.holidays_in(date.year()).contains(&date)
    }
}

/// The world's one calendar: the epoch, the day-to-date mapping, and each country's business days.
#[clause("TIME.1", "TIME.2", "TIME.11")]
#[derive(Clone, Debug)]
pub struct Calendar {
    epoch: Date,
    /// The epoch's civil day count, which every day's lookup adds to.
    epoch_serial: i64,
    countries: Vec<CountryCalendar>,
}

impl Calendar {
    /// The calendar of the countries in `CountryId` order, their bitsets starting at `first_year`.
    ///
    /// # Errors
    /// When a country's rules cannot describe a calendar, or the countries are not numbered from zero in order.
    pub fn new(epoch: Date, countries: Vec<(CountryId, CountryRules)>, first_year: i32) -> Result<Calendar, String> {
        let mut built = Vec::with_capacity(countries.len());
        for (i, (country, rules)) in countries.into_iter().enumerate() {
            if usize::from(country.get()) != i {
                return Err(format!("country {} declared in place {i}", country.get()));
            }
            rules.validate().map_err(|e| format!("country {}: {e}", country.get()))?;
            built.push(CountryCalendar::build(country, rules, first_year));
        }
        Ok(Calendar { epoch, epoch_serial: days_from_civil(epoch), countries: built })
    }

    /// Moves every country's bitset to start at `first_year`, as each year's start does.
    pub fn move_window(&mut self, first_year: i32) {
        for c in &mut self.countries {
            *c = CountryCalendar::build(c.country, c.rules.clone(), first_year);
        }
    }

    pub fn epoch(&self) -> Date {
        self.epoch
    }

    #[must_use]
    pub fn countries(&self) -> usize {
        self.countries.len()
    }

    pub fn date(&self, day: Day) -> Date {
        day.date(self.epoch)
    }

    /// The day of a date, or none before the epoch.
    #[must_use]
    pub fn day(&self, date: Date) -> Option<Day> {
        Day::from_date(date, self.epoch)
    }

    /// The day of a date the world has placed, which must lie on the calendar.
    pub(crate) fn day_of(&self, date: Date) -> Day {
        let Some(day) = self.day(date) else {
            violation!(clause = "TIME.2", "a date before the epoch", year = date.year(), month = date.month());
        };
        day
    }

    fn serial(&self, day: Day) -> i64 {
        self.epoch_serial + i64::from(day.get())
    }

    fn country(&self, country: CountryId) -> &CountryCalendar {
        let Some(c) = self.countries.get(usize::from(country.get())) else {
            violation!(clause = "TIME.2", "a country the calendar does not hold", country = country.get());
        };
        c
    }

    #[clause("TIME.2")]
    #[must_use]
    pub fn is_business(&self, country: CountryId, day: Day) -> bool {
        self.country(country).is_business_serial(self.serial(day))
    }

    #[must_use]
    pub fn any_business(&self, day: Day) -> bool {
        let s = self.serial(day);
        self.countries.iter().any(|c| c.is_business_serial(s))
    }

    /// The first business day strictly after `day`.
    pub fn next_business(&self, country: CountryId, day: Day) -> Day {
        self.on_or_after(country, day.succ())
    }

    /// `day` if it is a business day, else the first after it.
    pub fn on_or_after(&self, country: CountryId, day: Day) -> Day {
        let mut d = day;
        while !self.is_business(country, d) {
            d = d.succ();
        }
        d
    }

    /// `day` if it is a business day, else the last before it; none when the epoch comes first.
    #[must_use]
    pub fn on_or_before(&self, country: CountryId, day: Day) -> Option<Day> {
        let mut d = day;
        while !self.is_business(country, d) {
            d = Day::new(d.get().checked_sub(1)?);
        }
        Some(d)
    }

    /// The first day after `day` that is a business day in any country: the next day a turn ends on.
    pub fn next_turn_day(&self, day: Day) -> Day {
        let mut d = day.succ();
        while !self.any_business(d) {
            d = d.succ();
        }
        d
    }

    /// A day plus a period: the one way a period is added to a day, months by the calendar's dates.
    #[clause("TIME.3", "TIME.11")]
    pub fn plus(&self, day: Day, period: Period) -> Day {
        let date = advance(self.date(day), period, 1, EndOfMonth::Plain);
        let Some(d) = self.day(date) else {
            capacity_exceeded!("calendar days", u32::MAX, day.get());
        };
        d
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use phx_id::{CountryId, Date, Weekday};

    use super::Calendar;
    use crate::calendar::rules::{CountryRules, HolidayRule, WeekendRule};

    /// A calendar of one country with a Saturday and Sunday weekend and a few holidays, from 1950.
    pub fn calendar(first_year: i32) -> Calendar {
        let rules = CountryRules {
            weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
            holidays: vec![
                HolidayRule::Fixed { name: "New Year".to_owned(), month: 1, day: 1 },
                HolidayRule::EasterOffset { name: "Good Friday".to_owned(), days: -2 },
                HolidayRule::Fixed { name: "Christmas".to_owned(), month: 12, day: 25 },
            ],
        };
        Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], first_year).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{CountryId, Date, Day};

    use super::testing::calendar;
    use crate::calendar::period::Period;

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    const C: CountryId = CountryId::new(0);

    #[test]
    fn business_days_follow_rules() {
        let cal = calendar(2020);
        let day = |y, m, d| cal.day(date(y, m, d)).unwrap();
        assert!(!cal.is_business(C, day(2025, 1, 1)), "New Year");
        assert!(!cal.is_business(C, day(2025, 4, 18)), "Good Friday 2025");
        assert!(!cal.is_business(C, day(2025, 5, 31)), "a Saturday");
        assert!(cal.is_business(C, day(2025, 5, 30)));
        assert_eq!(cal.next_business(C, day(2025, 12, 24)), day(2025, 12, 26));
        assert_eq!(cal.on_or_before(C, day(2025, 12, 28)), Some(day(2025, 12, 26)));
        assert_eq!(cal.next_turn_day(day(2025, 12, 31)), day(2026, 1, 2));
        assert_eq!(cal.plus(day(2024, 1, 31), Period::months(1).unwrap()), day(2024, 2, 29));
        assert_eq!(cal.day(date(1949, 12, 31)), None);
    }

    #[test]
    fn beyond_window_matches_rules() {
        let near = calendar(2020);
        let far = calendar(2060);
        // 2090 lies beyond the first calendar's window, which ends with 2083, and inside the second's.
        let from = near.day(date(2090, 1, 1)).unwrap().get();
        for d in from..from + 1000 {
            assert_eq!(near.is_business(C, Day::new(d)), far.is_business(C, Day::new(d)), "day {d}");
        }
        let mut moved = calendar(2020);
        moved.move_window(2060);
        assert_eq!(moved.is_business(C, Day::new(from)), far.is_business(C, Day::new(from)));
    }
}
