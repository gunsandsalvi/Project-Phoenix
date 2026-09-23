use phx_id::{CountryId, Day};
use phx_macros::clause;
use phx_num::violation;

use crate::calendar::Calendar;
use crate::calendar::bizday::BusinessDayConvention;
use crate::calendar::period::{EndOfMonth, Period, advance};
use crate::consts::{MONTHS, SHORTEST_MONTH_DAYS};

/// Whether a decision is taken on business days only or on any day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunsOn {
    Business,
    Any,
}

/// When a kind of party reviews a kind of decision: every period, moved to a day its decision point runs.
#[clause("TIME.5", "TIME.13")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionSchedule {
    pub period: Period,
    pub convention: BusinessDayConvention,
    pub runs_on: RunsOn,
}

/// A party's own offset within each of a schedule's periods, so parties of one kind do not all decide on one day.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phase {
    offset_days: u16,
}

impl Phase {
    /// An offset within the period's shortest instance, or none beyond it.
    #[must_use]
    pub fn within(period: Period, offset_days: u16) -> Option<Phase> {
        let shortest = if period.month_count() > 0 {
            u32::from(period.month_count()) * u32::from(SHORTEST_MONTH_DAYS)
        } else {
            u32::from(period.day_count())
        };
        (u32::from(offset_days) < shortest).then_some(Phase { offset_days })
    }

    #[must_use]
    pub fn offset_days(self) -> u16 {
        self.offset_days
    }
}

/// What may wake a party outside its schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WakeKind {
    Message,
    Surprise,
    PlayerIntent,
    KinkDay,
    EventConcerning,
}

/// The day of a schedule's `k`-th instance, its start placed from the epoch by the calendar's dates.
fn instance(calendar: &Calendar, country: CountryId, s: DecisionSchedule, phase: Phase, k: u32) -> Day {
    let Some(start) = calendar.day(advance(calendar.epoch(), s.period, k, EndOfMonth::Plain)) else {
        violation!(clause = "TIME.5", "a schedule instance before the epoch", k = k);
    };
    let due = Period::days(phase.offset_days).map_or(start, |offset| calendar.plus(start, offset));
    match s.runs_on {
        RunsOn::Any => due,
        RunsOn::Business => calendar.adjust(country, calendar.date(due), s.convention),
    }
}

/// The first day strictly after `after` on which the party takes the decision: its phase into the period's next
/// instance, moved by the schedule's convention.
#[clause("TIME.5")]
pub fn next_due(calendar: &Calendar, country: CountryId, s: DecisionSchedule, phase: Phase, after: Day) -> Day {
    // Start one instance before the one `after` falls in, so a convention that moves a date back is still seen.
    let elapsed = if s.period.month_count() > 0 {
        let (e, a) = (calendar.epoch(), calendar.date(after));
        let months = (a.year() - e.year()) * MONTHS + i32::from(a.month()) - i32::from(e.month());
        let Ok(months) = u32::try_from(months) else {
            violation!(clause = "TIME.5", "a day before the epoch", day = after.get());
        };
        months / u32::from(s.period.month_count())
    } else {
        after.get() / u32::from(s.period.day_count())
    };
    let mut k = if elapsed > 0 { elapsed - 1 } else { 0 };
    loop {
        let due = instance(calendar, country, s, phase, k);
        if due > after {
            return due;
        }
        k += 1;
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{CountryId, Date};

    use super::{DecisionSchedule, Phase, RunsOn, next_due};
    use crate::calendar::bizday::BusinessDayConvention;
    use crate::calendar::period::Period;
    use crate::calendar::testing::calendar;

    #[test]
    fn next_due_respects_phase_and_convention() {
        let cal = calendar(2020);
        let c = CountryId::new(0);
        let day = |y, m, d| cal.day(Date::new(y, m, d).unwrap()).unwrap();
        let monthly = DecisionSchedule {
            period: Period::months(1).unwrap(),
            convention: BusinessDayConvention::Following,
            runs_on: RunsOn::Business,
        };
        let phase = Phase::within(monthly.period, 14).unwrap();
        // Each month's 15th, moved to a business day: 15 March 2025 is a Saturday, so Monday 17 March.
        assert_eq!(next_due(&cal, c, monthly, phase, day(2025, 3, 1)), day(2025, 3, 17));
        assert_eq!(next_due(&cal, c, monthly, phase, day(2025, 3, 17)), day(2025, 4, 15));
        let any = DecisionSchedule { runs_on: RunsOn::Any, ..monthly };
        assert_eq!(next_due(&cal, c, any, phase, day(2025, 3, 1)), day(2025, 3, 15));
        let weekly = DecisionSchedule { period: Period::weeks(1).unwrap(), ..any };
        let friday = next_due(&cal, c, weekly, Phase::within(weekly.period, 5).unwrap(), day(2025, 3, 1));
        assert_eq!(cal.date(friday).weekday(), phx_id::Weekday::Friday, "the epoch was a Sunday; five days on");
        assert!(Phase::within(monthly.period, 28).is_none());
    }
}
