//! One calendar (Money G1–G4): an epoch, a period length, one mapping from period to date, and
//! every periodicity placed on that grid BY DATE and never by a count of periods (G3.a).
//!
//! Time within a period is cycles (G1, G2); nothing finer exists. There is no default period
//! (G4.a), which is why `Period` is a value a caller must have rather than a number it can omit.

/// A period index on the one calendar.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Period(pub u32);

/// A settlement cycle within a period, 0-based.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Cycle(pub u16);

/// A civil date, as a day number from the epoch — one mapping, and day counts come from dates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Day(pub i64);

pub struct Calendar {
    /// The day the world opened. Everything is placed against this and nothing against "now".
    epoch: Day,
    /// G3: the period is 7 days. It is a RESOLUTION, tested by invariance, not a preference.
    days_per_period: u32,
    /// G2: the settlement cycles within a period.
    cycles_per_period: u16,
}

impl Calendar {
    pub fn new(epoch: Day, days_per_period: u32, cycles_per_period: u16) -> Self {
        assert!(days_per_period > 0, "Money G3: a period is some days long");
        assert!(cycles_per_period > 0, "Money G2: a period has cycles in it");
        Self { epoch, days_per_period, cycles_per_period }
    }

    pub fn cycles_per_period(&self) -> u16 {
        self.cycles_per_period
    }

    /// G3: the day a period starts on. One mapping, and every day count is taken from it.
    pub fn start_of(&self, at: Period) -> Day {
        Day(self.epoch.0 + i64::from(at.0) * i64::from(self.days_per_period))
    }

    /// G3.a: the first period at or after this day — a periodicity is placed BY DATE.
    pub fn period_on(&self, day: Day) -> Period {
        let since = day.0 - self.epoch.0;
        assert!(since >= 0, "Money G4: a day before the world opened is not a period");
        let whole = since / i64::from(self.days_per_period);
        let exact = since % i64::from(self.days_per_period) == 0;
        Period(if exact { whole as u32 } else { (whole + 1) as u32 })
    }

    /// Law 8, G3.a: how much of a year lies between two days, from the DATES and never from a
    /// count of periods. ACT/365F, which is the convention this kernel states once.
    pub fn year_fraction(&self, from: Day, to: Day) -> f64 {
        (to.0 - from.0) as f64 / 365.0
    }

    /// The first period at or after `at` that is a whole number of `every` from the epoch — an
    /// exchange's ladder, so everything written between two dates settles into the same book.
    pub fn next_cycle(&self, at: Period, every: u32) -> Period {
        assert!(every > 0, "Law 8: a contract cycle of no periods");
        Period(((at.0 + 1).div_ceil(every)) * every)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_periodicity_is_placed_by_date_and_never_by_a_count_of_periods() {
        let cal = Calendar::new(Day(0), 7, 3);
        assert_eq!(cal.start_of(Period(0)), Day(0));
        assert_eq!(cal.start_of(Period(4)), Day(28));
        // A day inside a period belongs to the period that has not started yet, by date.
        assert_eq!(cal.period_on(Day(28)), Period(4));
        assert_eq!(cal.period_on(Day(29)), Period(5));
        assert!((cal.year_fraction(Day(0), Day(365)) - 1.0).abs() <= crate::num::dust(2, &[1.0]));
    }

    #[test]
    fn a_ladder_settles_into_the_same_book() {
        let cal = Calendar::new(Day(0), 7, 3);
        // Everything written in periods 1..13 settles at 13 when the ladder is thirteen periods.
        assert_eq!(cal.next_cycle(Period(0), 13), Period(13));
        assert_eq!(cal.next_cycle(Period(11), 13), Period(13));
        assert_eq!(cal.next_cycle(Period(12), 13), Period(13));
        assert_eq!(cal.next_cycle(Period(13), 13), Period(26));
    }
}
