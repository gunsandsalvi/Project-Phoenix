use phx_id::{CountryId, Date, Day};
use phx_macros::clause;
use phx_num::violation;

use crate::calendar::Calendar;

/// Where a date that is not a business day moves to, as ISDA defines the conventions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, phx_macros::Saved)]
pub enum BusinessDayConvention {
    Following,
    /// The following business day, unless that is in the next month; then the preceding one.
    ModifiedFollowing,
    Preceding,
    /// The preceding business day, unless that is in the previous month; then the following one.
    ModifiedPreceding,
    Unadjusted,
}

impl Calendar {
    fn preceding(&self, country: CountryId, day: Day) -> Day {
        let Some(d) = self.on_or_before(country, day) else {
            violation!(clause = "TIME.3", "no business day between the epoch and a date to adjust", day = day.get());
        };
        d
    }

    /// The day a payment dated `date` falls on in `country` by the convention.
    #[clause("TIME.3", "TIME.13")]
    pub fn adjust(&self, country: CountryId, date: Date, convention: BusinessDayConvention) -> Day {
        let day = self.day_of(date);
        let same_month = |d: Day| self.date(d).month() == date.month();
        match convention {
            BusinessDayConvention::Unadjusted => day,
            BusinessDayConvention::Following => self.on_or_after(country, day),
            BusinessDayConvention::Preceding => self.preceding(country, day),
            BusinessDayConvention::ModifiedFollowing => {
                let next = self.on_or_after(country, day);
                if same_month(next) { next } else { self.preceding(country, day) }
            }
            BusinessDayConvention::ModifiedPreceding => {
                let before = self.preceding(country, day);
                if same_month(before) { before } else { self.on_or_after(country, day) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{CountryId, Date};

    use super::BusinessDayConvention;
    use crate::calendar::testing::calendar;

    #[test]
    fn adjust_modified_following_stays_in_month() {
        let cal = calendar(2020);
        let c = CountryId::new(0);
        let day = |y, m, d| cal.day(Date::new(y, m, d).unwrap()).unwrap();
        let may31 = Date::new(2025, 5, 31).unwrap();
        assert_eq!(cal.adjust(c, may31, BusinessDayConvention::ModifiedFollowing), day(2025, 5, 30));
        assert_eq!(cal.adjust(c, may31, BusinessDayConvention::Following), day(2025, 6, 2));
        assert_eq!(cal.adjust(c, may31, BusinessDayConvention::Unadjusted), day(2025, 5, 31));
        let nov1 = Date::new(2025, 11, 1).unwrap();
        assert_eq!(cal.adjust(c, nov1, BusinessDayConvention::Preceding), day(2025, 10, 31));
        assert_eq!(cal.adjust(c, nov1, BusinessDayConvention::ModifiedPreceding), day(2025, 11, 3));
    }
}
