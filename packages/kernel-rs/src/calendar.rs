//! The simulation calendar is an integer grid of weeks. Civil dates belong at import and display
//! boundaries; financial mechanisms never reconstruct days hidden inside a weekly tenor.

/// The single time coordinate used by the simulation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Week(pub i64);

/// A simulation period index.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Period(pub u32);

/// The declared accrual basis. There is deliberately only one weeks-per-year basis.
pub const WEEKS_PER_YEAR: i64 = 52;

/// Convert one explicit, whole-week tenor to years.
pub fn weekly_year_fraction(weeks: i64) -> f64 {
    assert!(weeks >= 0, "a negative weekly tenor cannot accrue");
    weeks as f64 / WEEKS_PER_YEAR as f64
}

/// Weekly market accrual conventions. Names describe products, not alternate hidden day counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Convention {
    MoneyMarketWeekly,
    BondWeekly,
}

impl Convention {
    pub fn year_fraction(self, from: Week, to: Week) -> f64 {
        let _ = self;
        weekly_year_fraction(to.0 - from.0)
    }
}

pub struct Calendar {
    epoch: Week,
    weeks_per_period: u32,
}

impl Calendar {
    pub fn new(epoch: Week, weeks_per_period: u32) -> Self {
        assert!(
            weeks_per_period > 0,
            "Money G3: a period is some weeks long"
        );
        Self {
            epoch,
            weeks_per_period,
        }
    }

    pub fn weeks_per_period(&self) -> i64 {
        i64::from(self.weeks_per_period)
    }

    pub fn start_of(&self, at: Period) -> Week {
        Week(self.epoch.0 + i64::from(at.0) * i64::from(self.weeks_per_period))
    }

    pub fn period_on(&self, week: Week) -> Period {
        let since = week.0 - self.epoch.0;
        assert!(
            since >= 0,
            "Money G4: a week before the world opened is not a period"
        );
        let whole = since / i64::from(self.weeks_per_period);
        let exact = since % i64::from(self.weeks_per_period) == 0;
        Period(if exact {
            whole as u32
        } else {
            (whole + 1) as u32
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accrual_uses_integer_week_differences_on_one_basis() {
        assert_eq!(weekly_year_fraction(52), 1.0);
        assert_eq!(
            Convention::MoneyMarketWeekly.year_fraction(Week(4), Week(17)),
            0.25
        );
        assert_eq!(
            Convention::BondWeekly.year_fraction(Week(4), Week(17)),
            0.25
        );
    }

    #[test]
    fn periods_are_placed_on_the_week_grid() {
        let cal = Calendar::new(Week(0), 1);
        assert_eq!(cal.start_of(Period(4)), Week(4));
        assert_eq!(cal.period_on(Week(4)), Period(4));
    }
}
