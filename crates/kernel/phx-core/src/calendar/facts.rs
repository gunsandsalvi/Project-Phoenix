//! A day's facts, each country's: its date, weekday, whether it is a business day, the periods it closes, its periods'
//! indices from the epoch, and the days each business-day convention moves onto it. Computed once at the day's open
//! and read from there, so no rule converts a date or tests a business day per call.

use phx_id::{CountryId, Date, Day, Weekday};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::calendar::Calendar;
use crate::calendar::bizday::BusinessDayConvention;
use crate::consts::{CONVENTIONS, MONTHS, MONTHS_PER_QUARTER};

/// The calendar days a convention moves onto a business day, first to last: after a closed weekend, Saturday to
/// Monday under the following convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RolledRange {
    pub first: Day,
    pub last: Day,
}

/// The longest period a day closes: none, its month, its quarter and month, or its year, quarter and month.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PeriodEnd {
    Within,
    Month,
    Quarter,
    Year,
}

/// One country's facts about a day.
#[clause("TIME.2", "TIME.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayFacts {
    pub country: CountryId,
    pub date: Date,
    pub weekday: Weekday,
    pub business: bool,
    pub closes: PeriodEnd,
    /// Months, quarters and years since the epoch's.
    pub month: u32,
    pub quarter: u32,
    pub year: i32,
    /// For each convention in `BusinessDayConvention::ALL`'s order, the days it moves onto this one; none on a day
    /// no date moves onto, as a closed day is under all but the unadjusted.
    pub rolled: [Missing<RolledRange>; CONVENTIONS],
}

/// Months from year zero's first to a date's month.
fn months(date: Date) -> i64 {
    i64::from(date.year()) * i64::from(MONTHS) + i64::from(date.month()) - 1
}

/// A count of periods since the epoch's, which a day on or after the epoch never makes negative.
fn since(now: i64, epoch: i64) -> u32 {
    match u32::try_from(now - epoch) {
        Ok(n) => n,
        Err(_) => violation!(clause = "TIME.2", "a period before the epoch's", since = now - epoch),
    }
}

impl Calendar {
    /// A country's facts about a day, from its rules and business days.
    #[must_use]
    pub fn facts_of(&self, country: CountryId, day: Day) -> DayFacts {
        let date = self.date(day);
        let tomorrow = self.date(day.succ());
        let (now, epoch) = (months(date), months(self.epoch()));
        let quarters = |m: i64| m.div_euclid(i64::from(MONTHS_PER_QUARTER));
        let business = self.is_business(country, day);
        let rolled = BusinessDayConvention::ALL.map(|c| self.rolled_onto(country, day, (business, c)));
        DayFacts {
            country,
            date,
            weekday: date.weekday(),
            business,
            closes: if tomorrow.year() != date.year() {
                PeriodEnd::Year
            } else if quarters(months(tomorrow)) != quarters(now) {
                PeriodEnd::Quarter
            } else if tomorrow.month() != date.month() {
                PeriodEnd::Month
            } else {
                PeriodEnd::Within
            },
            month: since(now, epoch),
            quarter: since(quarters(now), quarters(epoch)),
            year: date.year() - self.epoch().year(),
            rolled,
        }
    }

    /// The days a convention moves onto `day`: the run of closed days on either side of it, those of them it moves
    /// here, and the day itself. They are one run, since a convention moves a closed day to the nearest business day
    /// on one side.
    fn rolled_onto(
        &self,
        country: CountryId,
        day: Day,
        (business, c): (bool, BusinessDayConvention),
    ) -> Missing<RolledRange> {
        if c == BusinessDayConvention::Unadjusted {
            return Missing::Present(RolledRange { first: day, last: day });
        }
        if !business {
            return Missing::Absent;
        }
        let here = |d: Day| self.adjust(country, self.date(d), c) == day;
        let mut first = day;
        while let Some(before) = first.get().checked_sub(1).map(Day::new) {
            if self.is_business(country, before) || !here(before) {
                break;
            }
            first = before;
        }
        let mut last = day;
        loop {
            let after = last.succ();
            if self.is_business(country, after) || !here(after) {
                break;
            }
            last = after;
        }
        Missing::Present(RolledRange { first, last })
    }

    /// Opens a day: every country's facts about it, computed once and kept until the next day opens.
    pub fn open_day(&mut self, day: Day) -> &[DayFacts] {
        self.today.clear();
        for c in 0..self.countries() {
            let Ok(id) = u8::try_from(c) else {
                violation!(clause = "TIME.2", "a country past the calendar's identities", country = c);
            };
            let facts = self.facts_of(CountryId::new(id), day);
            self.today.push(facts);
        }
        &self.today
    }

    /// The facts of the day last opened, each country's in its order; none before a day opens.
    #[must_use]
    pub fn today(&self) -> &[DayFacts] {
        &self.today
    }
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod tests;
