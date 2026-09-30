//! A day's facts over hand-given calendars: what each convention rolls onto a day, per country, the periods closed and
//! counted, day zero, and the same facts however often they are opened.
#![cfg(test)]

use phx_id::{CountryId, Date, Day, Weekday};
use phx_num::Missing;

use super::{DayFacts, PeriodEnd, RolledRange};
use crate::calendar::Calendar;
use crate::calendar::bizday::BusinessDayConvention;
use crate::calendar::period::Period;
use crate::calendar::rules::{CountryRules, WeekendRule};
use crate::calendar::testing::calendar;

const C: CountryId = CountryId::new(0);

fn day(cal: &Calendar, y: i32, m: u8, d: u8) -> Day {
    cal.day(Date::new(y, m, d).unwrap()).unwrap()
}

fn rolled(f: &DayFacts, c: BusinessDayConvention) -> Missing<RolledRange> {
    let at = BusinessDayConvention::ALL.iter().position(|x| *x == c).unwrap();
    f.rolled[at]
}

#[test]
fn following_rolls_weekend_onto_monday() {
    let cal = calendar(2020);
    // Monday 3 March 2025: the Saturday and Sunday before it roll onto it under the following convention.
    let monday = day(&cal, 2025, 3, 3);
    let f = cal.facts_of(C, monday);
    assert_eq!((f.weekday, f.business), (Weekday::Monday, true));
    let sat = day(&cal, 2025, 3, 1);
    assert_eq!(
        rolled(&f, BusinessDayConvention::Following),
        Missing::Present(RolledRange { first: sat, last: monday })
    );
    // The preceding convention rolls them back onto the Friday instead.
    let friday = day(&cal, 2025, 2, 28);
    let fri = cal.facts_of(C, friday);
    let sunday = day(&cal, 2025, 3, 2);
    assert_eq!(
        rolled(&fri, BusinessDayConvention::Preceding),
        Missing::Present(RolledRange { first: friday, last: sunday })
    );
    assert_eq!(
        rolled(&f, BusinessDayConvention::Preceding),
        Missing::Present(RolledRange { first: monday, last: monday })
    );
    // A closed day has nothing rolled onto it but itself, unadjusted.
    let s = cal.facts_of(C, sat);
    assert_eq!(rolled(&s, BusinessDayConvention::Following), Missing::Absent);
    assert_eq!(rolled(&s, BusinessDayConvention::Unadjusted), Missing::Present(RolledRange { first: sat, last: sat }));
}

#[test]
fn modified_following_month_end() {
    let cal = calendar(2020);
    // Saturday 31 May 2025 would follow into June, so it rolls back onto Friday 30 May.
    let friday = day(&cal, 2025, 5, 30);
    let f = cal.facts_of(C, friday);
    let may31 = day(&cal, 2025, 5, 31);
    assert_eq!(
        rolled(&f, BusinessDayConvention::ModifiedFollowing),
        Missing::Present(RolledRange { first: friday, last: may31 })
    );
    // June's first Monday takes June's Sunday only.
    let monday = day(&cal, 2025, 6, 2);
    let m = cal.facts_of(C, monday);
    let june1 = day(&cal, 2025, 6, 1);
    assert_eq!(
        rolled(&m, BusinessDayConvention::ModifiedFollowing),
        Missing::Present(RolledRange { first: june1, last: monday })
    );
    // Each rolled day adjusts onto the day whose facts hold it.
    for d in [f, m] {
        for c in BusinessDayConvention::ALL {
            if let Missing::Present(r) = rolled(&d, c) {
                for x in r.first.get()..=r.last.get() {
                    assert_eq!(cal.adjust(C, cal.date(Day::new(x)), c), cal.day(d.date).unwrap(), "{c:?}");
                }
            }
        }
    }
}

#[test]
fn business_in_one_country() {
    // Two countries, one closed on Fridays and Saturdays, the other on Saturdays and Sundays.
    let rules = |days: Vec<Weekday>| CountryRules { weekend: WeekendRule { days }, holidays: Vec::new() };
    let mut cal = Calendar::new(
        Date::new(2020, 1, 1).unwrap(),
        vec![
            (CountryId::new(0), rules(vec![Weekday::Friday, Weekday::Saturday])),
            (CountryId::new(1), rules(vec![Weekday::Saturday, Weekday::Sunday])),
        ],
        2025,
    )
    .unwrap();
    let friday = day(&cal, 2025, 3, 7);
    let facts = cal.open_day(friday).to_vec();
    assert_eq!(
        facts.iter().map(|f| (f.country, f.business)).collect::<Vec<_>>(),
        vec![(CountryId::new(0), false), (CountryId::new(1), true)]
    );
    assert_eq!(cal.today(), facts.as_slice());
}

#[test]
fn period_indices_match_plus() {
    let cal = calendar(2020);
    let epoch = cal.epoch();
    let mut d = day(&cal, 2000, 1, 31);
    let first = cal.facts_of(C, d);
    assert_eq!(first.month, u32::try_from((2000 - epoch.year()) * 12).unwrap());
    for n in 1..40_u32 {
        d = cal.plus(d, Period::months(1).unwrap());
        let f = cal.facts_of(C, d);
        assert_eq!(f.month, first.month + n);
        assert_eq!(f.quarter, first.quarter + n / 3);
        assert_eq!(f.year, first.year + i32::try_from(n / 12).unwrap());
    }
    // A quarter's and a year's last day close them.
    let closes = |y, m, d| cal.facts_of(C, day(&cal, y, m, d)).closes;
    assert_eq!(closes(2024, 12, 31), PeriodEnd::Year);
    assert_eq!(closes(2024, 3, 31), PeriodEnd::Quarter);
    assert_eq!(closes(2024, 2, 29), PeriodEnd::Month);
    assert_eq!(closes(2024, 2, 28), PeriodEnd::Within, "2024 is a leap year");
}

#[test]
fn day_zero_facts() {
    // Day zero is the day before the first: its date is the epoch.
    let cal = calendar(2020);
    let f = cal.facts_of(C, Day::new(0));
    assert_eq!((f.date, f.month, f.quarter, f.year), (cal.epoch(), 0, 0, 0));
}

#[test]
fn facts_rebuilt_equal() {
    let mut cal = calendar(2020);
    let d = day(&cal, 2025, 4, 18);
    let before = cal.open_day(d).to_vec();
    let mut again = cal.clone();
    let _ = again.open_day(day(&cal, 2025, 4, 19));
    assert_eq!(again.open_day(d), before.as_slice(), "a calendar rebuilt opens the same facts");
    assert!(!before[0].business, "Good Friday is a holiday");
}
