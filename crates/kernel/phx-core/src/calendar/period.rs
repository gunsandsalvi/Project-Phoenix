use phx_id::{CountryId, Date, Day, civil_from_days, days_from_civil};
use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::calendar::Calendar;
use crate::calendar::bizday::BusinessDayConvention;
use crate::consts::{DAYS_PER_WEEK, MONTHS};

/// A periodicity: whole months or whole days, never both, so a month is never taken as a count of days.
#[clause("TIME.3", "TIME.12")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, phx_macros::Saved)]
pub struct Period {
    months: u16,
    days: u16,
}

impl Period {
    /// `n` months, or none for zero.
    #[must_use]
    pub fn months(n: u16) -> Option<Period> {
        (n > 0).then_some(Period { months: n, days: 0 })
    }

    /// `n` days, for the lags and weekly periods stated in days; none for zero.
    #[must_use]
    pub fn days(n: u16) -> Option<Period> {
        (n > 0).then_some(Period { months: 0, days: n })
    }

    #[must_use]
    pub fn weeks(n: u16) -> Option<Period> {
        n.checked_mul(DAYS_PER_WEEK).and_then(Period::days)
    }

    #[must_use]
    pub fn month_count(self) -> u16 {
        self.months
    }

    #[must_use]
    pub fn day_count(self) -> u16 {
        self.days
    }
}

/// How a month period treats an anchor on a month's last day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, phx_macros::Saved)]
pub enum EndOfMonth {
    /// The anchor's day, cut to the target month's length.
    Plain,
    /// As `Plain`, and an anchor on its month's last day moves to the target month's last day.
    Keep,
}

/// The `n`-th date from the anchor: every instance is computed from the anchor itself, so a short month never pulls
/// the later ones earlier.
#[clause("TIME.3")]
pub fn advance(anchor: Date, period: Period, n: u32, eom: EndOfMonth) -> Date {
    if period.months == 0 {
        let step = i64::from(period.days) * i64::from(n);
        return civil_from_days(days_from_civil(anchor) + step);
    }
    let months = i64::from(anchor.year()) * i64::from(MONTHS) + i64::from(anchor.month()) - 1
        + i64::from(period.months) * i64::from(n);
    let (year, month_index) = (months.div_euclid(i64::from(MONTHS)), months.rem_euclid(i64::from(MONTHS)));
    let (Ok(year), Ok(month)) = (i32::try_from(year), u8::try_from(month_index + 1)) else {
        capacity_exceeded!("calendar years", i32::MAX, year);
    };
    let (Some(target_len), Some(anchor_len)) =
        (Date::days_in_month(year, month), Date::days_in_month(anchor.year(), anchor.month()))
    else {
        capacity_exceeded!("calendar months", MONTHS, month);
    };
    let keep_end = eom == EndOfMonth::Keep && anchor.day() == anchor_len;
    let day = if keep_end || anchor.day() > target_len { target_len } else { anchor.day() };
    let Some(date) = Date::new(year, month, day) else {
        capacity_exceeded!("calendar days", target_len, day);
    };
    date
}

/// A dated schedule's days: each date advanced from the anchor, then moved to a business day by the convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct ScheduleDates {
    pub anchor: Date,
    pub period: Period,
    pub eom: EndOfMonth,
    pub convention: BusinessDayConvention,
    pub country: CountryId,
}

impl ScheduleDates {
    /// The `n`-th date's day, `n` counted from the anchor.
    pub fn nth(&self, calendar: &Calendar, n: u32) -> Day {
        calendar.adjust(self.country, advance(self.anchor, self.period, n, self.eom), self.convention)
    }

    /// The days from the `from`-th date on.
    pub fn days<'a>(&'a self, calendar: &'a Calendar, from: u32) -> impl Iterator<Item = Day> + 'a {
        (from..).map(move |n| self.nth(calendar, n))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{CountryId, Date};

    use super::{EndOfMonth, Period, ScheduleDates, advance};
    use crate::calendar::bizday::BusinessDayConvention;
    use crate::calendar::testing::calendar;

    #[test]
    fn schedule_dates_are_adjusted_from_the_anchor() {
        let cal = calendar(2020);
        let schedule = ScheduleDates {
            anchor: Date::new(2025, 1, 31).unwrap(),
            period: Period::months(1).unwrap(),
            eom: EndOfMonth::Keep,
            convention: BusinessDayConvention::ModifiedFollowing,
            country: CountryId::new(0),
        };
        let dates: Vec<Date> = schedule.days(&cal, 1).take(4).map(|d| cal.date(d)).collect();
        // 28 Feb (Fri), 31 Mar (Mon), 30 Apr (Wed), 31 May is a Saturday and moves back to 30 May.
        let expected: Vec<Date> =
            [(2, 28), (3, 31), (4, 30), (5, 30)].iter().map(|(m, d)| Date::new(2025, *m, *d).unwrap()).collect();
        assert_eq!(dates, expected);
    }

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    #[test]
    fn advance_eom_modes() {
        let month = Period::months(1).unwrap();
        for eom in [EndOfMonth::Plain, EndOfMonth::Keep] {
            assert_eq!(advance(date(2023, 1, 31), month, 1, eom), date(2023, 2, 28));
            assert_eq!(advance(date(2024, 1, 31), month, 1, eom), date(2024, 2, 29));
        }
        assert_eq!(advance(date(2023, 1, 31), month, 2, EndOfMonth::Plain), date(2023, 3, 31));
        assert_eq!(advance(date(2023, 2, 28), month, 1, EndOfMonth::Keep), date(2023, 3, 31));
        assert_eq!(advance(date(2023, 2, 28), month, 1, EndOfMonth::Plain), date(2023, 3, 28));
    }

    #[test]
    fn advance_from_anchor_never_drifts() {
        let month = Period::months(1).unwrap();
        let anchor = date(2023, 1, 31);
        let mut stepped = anchor;
        for n in 1..=24_u32 {
            let direct = advance(anchor, month, n, EndOfMonth::Plain);
            stepped = advance(stepped, month, 1, EndOfMonth::Plain);
            // Stepping from the previous date drifts to the 28th after February; the anchor never does.
            assert!(direct.day() >= stepped.day());
            assert_eq!(direct.day(), Date::days_in_month(direct.year(), direct.month()).unwrap());
        }
        assert_eq!(advance(date(2023, 12, 31), month, 12, EndOfMonth::Plain), date(2024, 12, 31));
        assert_eq!(advance(date(2023, 1, 1), Period::weeks(2).unwrap(), 3, EndOfMonth::Plain), date(2023, 2, 12));
        assert_eq!((Period::months(0), Period::days(0)), (None, None));
    }
}
