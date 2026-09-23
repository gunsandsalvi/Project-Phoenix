use std::collections::BTreeMap;

use phx_core::consts::CALENDAR_WINDOW_YEARS;
use phx_id::{CountryId, Date, Day};
use phx_world::Inspector;
use serde_json::{Value, json};

pub fn date_text(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

/// Whether a day is some country's last business day of its month, the day it pays and settles its month.
fn month_end_payday(w: Inspector<'_>, day: Day, countries: usize) -> bool {
    (0..countries).filter_map(|c| u8::try_from(c).ok()).map(CountryId::new).any(|c| {
        let next = w.calendar().next_business(c, day);
        w.is_business(c, day) && w.date(next).month() != w.date(day).month()
    })
}

/// The calendar's turns over its window of years: the longest run of days that are business days nowhere, the
/// lengths of all such runs, and each turn that ends on a month-end payday after days holidays closed.
pub fn measure(w: Inspector<'_>) -> Value {
    let countries = w.countries().len();
    let first = w.day_zero().succ();
    let start_year = w.date(first).year();
    let Some(end) = Date::new(start_year + CALENDAR_WINDOW_YEARS - 1, 12, 31).and_then(|d| w.calendar().day(d)) else {
        return json!({ "error": "the window's last day is not a day of the calendar" });
    };
    let mut lengths: BTreeMap<u32, u32> = BTreeMap::new();
    let mut longest = (0_u32, first, first);
    let mut coincidences = Vec::new();
    let mut day = first;
    while day <= end {
        let block_start = day;
        let mut closed = 0_u32;
        let mut holiday_on_weekday = false;
        while !w.any_business(day) {
            let weekday = w.date(day).weekday();
            holiday_on_weekday |= !matches!(weekday, phx_id::Weekday::Saturday | phx_id::Weekday::Sunday);
            closed += 1;
            day = day.succ();
        }
        if closed > 0 {
            *lengths.entry(closed).or_default() += 1;
            if closed > longest.0 {
                longest = (closed, block_start, day);
            }
        }
        if closed >= 2 && holiday_on_weekday && month_end_payday(w, day, countries) {
            let month = w.date(day).month();
            coincidences.push(json!({
                "turn_ends": date_text(w.date(day)),
                "closed_days": closed,
                "quarter_end": month.is_multiple_of(3),
            }));
        }
        day = day.succ();
    }
    let (days, from, to) = longest;
    json!({
        "step": "S0.11",
        "from": date_text(w.date(first)),
        "to": date_text(w.date(end)),
        "countries": w.countries().iter().map(|c| c.level.dir()).collect::<Vec<_>>(),
        "longest_closed_run": {
            "days": days,
            "first": date_text(w.date(from)),
            "turn_ends": date_text(w.date(to)),
            "turn_days": days + 1,
            "ends_on_month_end_payday": month_end_payday(w, to, countries),
        },
        "closed_runs_by_length": lengths.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "paydays_after_holidays": coincidences,
    })
}
