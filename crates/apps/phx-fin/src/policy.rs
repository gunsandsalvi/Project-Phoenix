//! `-F policy`: the design point's dated policy schedules — ten thousand, each a country's policy of one owner — a few
//! changes announced a day, each day opened once, advancing only the schedules a change takes effect in, and every
//! rule's read of a value in force by its handle.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_core::calendar::Calendar;
use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::{PolicyBook, PolicyH};
use phx_id::{CountryId, Date, Day, PartyRef, Slot, Weekday};
use phx_rand::uniform::below_u64;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{day_of, index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the schedules are measured under.
pub const BASE: &str = "policy";

/// Schedules at the design point: policies × countries × owners.
const SCHEDULES: u32 = 10_000;

/// Countries the schedules are spread over.
const COUNTRIES: u8 = 3;

/// Changes announced a day, each in force some days on.
const CHANGES: u64 = 5;
const LEAD_DAYS: u64 = 30;

/// Reads of a value in force a day, a schedule's each by a hundred rules.
const READS_PER_SCHEDULE: u32 = 100;

/// The calendar's first year, and the first day the measure opens, a year in.
const EPOCH_YEAR: i32 = 2026;
const FIRST_DAY: u32 = 365;

/// The schedules, their handles and owners, the calendar the changes are placed by, the streams, and the day it is.
#[derive(Debug, Default)]
pub struct Policy {
    book: PolicyBook<i64>,
    handles: Vec<(PolicyH<i64>, PartyRef)>,
    calendar: Option<Calendar>,
    streams: Option<Streams>,
    today: u32,
    folded: i64,
}

impl FinBase for Policy {
    fn name(&self) -> &'static str {
        BASE
    }

    /// Ten thousand schedules, each opening at a drawn value, owned by a party of its own in one of three countries.
    fn fill(&mut self, _design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let epoch = Date::new(EPOCH_YEAR, 1, 1).ok_or_else(|| FinError("no epoch".to_owned()))?;
        let rules = || CountryRules {
            weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] },
            holidays: Vec::new(),
        };
        let countries = (0..COUNTRIES).map(|c| (CountryId::new(c), rules())).collect();
        self.calendar = Some(Calendar::new(epoch, countries, EPOCH_YEAR).map_err(FinError)?);
        let mut d = streams.draws(BASE, 0, 0);
        for s in 0..SCHEDULES {
            let owner = PartyRef::new(1, 0, Slot::new(s));
            let country = CountryId::new(u8::try_from(s % u32::from(COUNTRIES)).map_err(|e| FinError(e.to_string()))?);
            let opening = i64::try_from(below_u64(&mut d, 1 << 20)).map_err(|e| FinError(e.to_string()))?;
            self.handles.push((self.book.open(opening, country, owner), owner));
        }
        self.streams = Some(*streams);
        self.today = FIRST_DAY;
        Ok(Filled { rows: u64::from(SCHEDULES) })
    }

    /// The day's changes announced, the day opened, and every rule's reads.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(calendar), Some(streams)) = (self.calendar.as_ref(), self.streams) else {
            return Err(FinError("the schedules measured before their fill".to_owned()));
        };
        let today = Day::new(self.today);
        let mut d = streams.draws(BASE, u64::from(self.today), day_of(day)?);
        for _ in 0..CHANGES {
            let Some(&(h, owner)) = self.handles.get(index(below_u64(&mut d, wide(self.handles.len())))?) else {
                return Err(FinError("a schedule past the book".to_owned()));
            };
            let lead = u32::try_from(2 + below_u64(&mut d, LEAD_DAYS)).map_err(|e| FinError(e.to_string()))?;
            let effective = calendar.on_or_after(CountryId::new(0), Day::new(self.today + lead));
            let value = i64::try_from(below_u64(&mut d, 1 << 20)).map_err(|e| FinError(e.to_string()))?;
            // A change placed too soon for its country's calendar is refused, as the rule would be; the measure goes on.
            let _ = self.book.announce((h, owner), calendar, (today, effective, value));
        }
        let book = &mut self.book;
        m.read(BASE, "resolve", 1, || book.open_day(today));
        let (book, handles) = (&self.book, &self.handles);
        let reads = u64::from(SCHEDULES) * u64::from(READS_PER_SCHEDULE);
        self.folded ^= m.read(BASE, "read", reads, || {
            let mut fold = 0_i64;
            for _ in 0..READS_PER_SCHEDULE {
                for (h, _) in handles {
                    fold ^= book.read(*h);
                }
            }
            black_box(fold)
        });
        self.today += 1;
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: phx_store::StoreStats::bytes(&self.book), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
}
