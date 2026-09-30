//! Today's calendar and streams (`phx_core::calendar`, `phx_rand`): a day's civil dates and business-day reads, each
//! due reading its date, and a day's draws, each retail want drawing its choice.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::calendar::{Calendar as Civil, civil_date, civil_serial};
use phx_id::{CountryId, Date, Day, Weekday};
use phx_rand::Draws;
use phx_rand::key::{Seed, Subject, SubjectTag, stream_key};

use crate::FinError;
use crate::kept::{BASE, count};
use crate::measure::Measures;

/// The years of days the reads range over, from the calendar's epoch.
const YEARS: i64 = 10;
const DAYS_A_YEAR: i64 = 365;

/// One country's calendar of a weekend of two days.
#[derive(Debug, Default)]
pub struct Calendar {
    calendar: Option<Civil>,
}

impl Calendar {
    /// The calendar from its epoch over the years the reads range over.
    ///
    /// # Errors
    /// An epoch the calendar refuses.
    pub fn fill(&mut self) -> Result<(), FinError> {
        let epoch = Date::new(2026, 1, 1).ok_or_else(|| FinError("no epoch".to_owned()))?;
        let rules = CountryRules {
            weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
            holidays: Vec::new(),
        };
        let last = epoch.year() + i32::try_from(YEARS).map_err(|e| FinError(e.to_string()))?;
        self.calendar = Some(Civil::new(epoch, vec![(CountryId::new(0), rules)], last).map_err(FinError)?);
        Ok(())
    }

    /// A day's `dues` civil and business-day reads, if it has dues, and its `retail` draws.
    ///
    /// # Errors
    /// A day without the counts.
    pub fn day(&mut self, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let draws = count(counts, "retail", "day")?;
        let calendar = self.calendar.as_ref().ok_or_else(|| FinError("a calendar read before its fill".to_owned()))?;
        let span = YEARS * DAYS_A_YEAR;
        let first = civil_serial(calendar.epoch());
        // Each due reads its date, so a day without dues reads none.
        if let Some(&reads) = counts.get("dues") {
            m.read(BASE, "civil", reads, || {
                let (mut open, mut offset) = (0_u64, 0_u32);
                for _ in 0..reads {
                    let date = civil_date(first + i64::from(offset));
                    open += u64::from(calendar.is_business(CountryId::new(0), Day::new(offset)) && date.day() > 0);
                    offset = if i64::from(offset) + 1 < span { offset + 1 } else { 0 };
                }
                black_box(open)
            });
        }
        let mut d = Draws::new(stream_key(Seed::new(1), "fin.kept.draws"), Subject::new(SubjectTag::World, 0), 0, 0);
        m.read(BASE, "draw", draws, || {
            let mut sum = 0_u64;
            for _ in 0..draws {
                sum ^= d.next_u64();
            }
            black_box(sum)
        });
        Ok(())
    }
}
