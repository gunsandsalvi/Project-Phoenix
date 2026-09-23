use phx_macros::clause;

use crate::consts::RATE_SCALE;
use crate::money::Money;
use crate::round::{Round, div_round};
use crate::violation;

/// The period a rate or a fraction of time is stated per.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RatePeriod {
    Year,
    Month,
    Day,
}

/// A fraction × 10^12 per period, so every rate says its period.
#[clause("Law 8")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    raw: i64,
    per: RatePeriod,
}

impl Rate {
    pub const fn new(raw: i64, per: RatePeriod) -> Rate {
        Rate { raw, per }
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.raw
    }

    pub const fn per(self) -> RatePeriod {
        self.per
    }
}

/// A fraction of a period, `num / den`, as the calendar's day counts produce it.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayFraction {
    num: i64,
    den: i64,
    per: RatePeriod,
}

impl DayFraction {
    pub const fn new(num: i64, den: i64, per: RatePeriod) -> DayFraction {
        DayFraction { num, den, per }
    }

    #[must_use]
    pub const fn num(self) -> i64 {
        self.num
    }

    #[must_use]
    pub const fn den(self) -> i64 {
        self.den
    }

    pub const fn per(self) -> RatePeriod {
        self.per
    }
}

/// Interest on a principal at a rate over a fraction of the rate's own period, rounded once.
#[clause("TIME.4", "MON.16")]
pub fn accrue(principal: Money, rate: Rate, f: DayFraction, r: Round) -> Money {
    if rate.per != f.per {
        violation!(clause = "TIME.4", "a rate applied over a fraction of another period");
    }
    let product =
        i128::from(principal.amt()).checked_mul(i128::from(rate.raw)).and_then(|p| p.checked_mul(i128::from(f.num)));
    let divisor = RATE_SCALE.checked_mul(i128::from(f.den));
    let (Some(product), Some(divisor)) = (product, divisor) else {
        violation!(clause = "Law 7", "an accrual overflows", principal = principal.amt(), raw = rate.raw);
    };
    let Ok(amt) = i64::try_from(div_round(product, divisor, r)) else {
        violation!(clause = "Law 7", "an accrual overflows money", principal = principal.amt(), raw = rate.raw);
    };
    Money::new(amt, principal.ccy())
}

#[cfg(test)]
mod tests {
    use super::{DayFraction, Rate, RatePeriod, accrue};
    use crate::money::{Ccy, Money};
    use crate::round::Round;
    use crate::violation::testing::violated_clause;

    const C: Ccy = Ccy::new(0);
    const FIVE_PERCENT: i64 = 50_000_000_000;

    #[test]
    fn accrue_known() {
        let f = DayFraction::new(31, 365, RatePeriod::Year);
        let interest = accrue(Money::new(1_000_000, C), Rate::new(FIVE_PERCENT, RatePeriod::Year), f, Round::HalfEven);
        assert_eq!(interest, Money::new(4_247, C));
    }

    #[test]
    fn accrue_refuses_period_mismatch() {
        let f = DayFraction::new(31, 365, RatePeriod::Year);
        let daily = Rate::new(FIVE_PERCENT, RatePeriod::Day);
        assert_eq!(violated_clause(|| accrue(Money::new(1, C), daily, f, Round::HalfEven)), "TIME.4");
    }
}
