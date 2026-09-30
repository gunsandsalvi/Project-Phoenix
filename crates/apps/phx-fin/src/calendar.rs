//! `-F calendar`: the day's facts opened for three countries over two years of days — each a civil date, a business
//! day, the periods it closes and what each convention rolls onto it — as the day's open computes them once.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::calendar::Calendar;
use phx_core::calendar::rules::{CountryRules, HolidayRule, WeekendRule};
use phx_id::{CountryId, Date, Day, Weekday};

use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the day's facts are measured under.
pub const BASE: &str = "calendar";

/// The countries a day opens facts for.
const COUNTRIES: u8 = 3;

/// The days each measured day opens: two years.
const DAYS: u32 = 730;

/// The calendar's first year, and the day the opened run starts from, a year in.
const EPOCH_YEAR: i32 = 2026;
const FIRST_DAY: u32 = 365;

/// Three countries' calendars — weekends of Saturday and Sunday, of Friday and Saturday, and of Sunday alone, each
/// with New Year and Christmas — and the business days the opened facts counted.
#[derive(Debug, Default)]
pub struct CalendarBase {
    calendar: Option<Calendar>,
    business: u64,
}

impl FinBase for CalendarBase {
    fn name(&self) -> &'static str {
        BASE
    }

    fn fill(&mut self, _design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        let epoch = Date::new(EPOCH_YEAR, 1, 1).ok_or_else(|| FinError("no epoch".to_owned()))?;
        let weekends =
            [vec![Weekday::Saturday, Weekday::Sunday], vec![Weekday::Friday, Weekday::Saturday], vec![Weekday::Sunday]];
        let holidays = || {
            vec![
                HolidayRule::Fixed { name: "New Year".to_owned(), month: 1, day: 1 },
                HolidayRule::Fixed { name: "Christmas".to_owned(), month: 12, day: 25 },
            ]
        };
        let countries = (0..COUNTRIES)
            .zip(weekends)
            .map(|(c, days)| (CountryId::new(c), CountryRules { weekend: WeekendRule { days }, holidays: holidays() }))
            .collect();
        self.calendar = Some(Calendar::new(epoch, countries, EPOCH_YEAR).map_err(FinError)?);
        Ok(Filled { rows: u64::from(COUNTRIES) })
    }

    /// Two years of days opened, each country's facts computed once a day.
    fn day(&mut self, _day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(calendar) = self.calendar.as_mut() else {
            return Err(FinError("the calendar measured before its fill".to_owned()));
        };
        let opened = u64::from(COUNTRIES) * u64::from(DAYS);
        self.business += m.read(BASE, "open", opened, || {
            let mut business = 0;
            for d in FIRST_DAY..FIRST_DAY + DAYS {
                business += calendar.open_day(Day::new(d)).iter().filter(|f| f.business).count();
            }
            black_box(crate::kept::wide(business))
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: 0, resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
}
