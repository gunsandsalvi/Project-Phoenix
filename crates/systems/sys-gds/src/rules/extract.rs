//! How much of a deposit to work, by Hotelling's rule against the extractor's own outlook.

use phx_macros::clause;

/// Whether to work a deposit now: today's price net of the cost of taking a unit is worth at least the net price the
/// extractor expects over the horizon, discounted at the return it requires, since a unit left in the ground can be
/// sold then instead. A deposit without end keeps nothing for later, so the price need only cover the cost.
#[clause("GDS.4")]
#[must_use]
pub fn works(price: f64, cost: f64, outlook: f64, (rate, horizon_years): (f64, f64), finite: bool) -> bool {
    let now = price - cost;
    if now <= 0.0 {
        return false;
    }
    if !finite {
        return true;
    }
    let later = (outlook - cost) * libm::exp(-rate * horizon_years);
    now >= later
}

/// The units to take over a period: what the extractor's plant runs a day for the period's days, no more than the
/// deposit holds, in whole lots of the product's least quantity; none beyond an integer's reach.
#[clause("GDS.4", "GDS.12")]
#[must_use]
pub fn quantity(per_day: i64, days: i64, remaining: Option<i64>, lot: i64) -> i64 {
    let Some(wanted) = per_day.checked_mul(days) else { return 0 };
    let can = match remaining {
        Some(r) if r < wanted => r,
        _ => wanted,
    };
    if lot <= 0 || can <= 0 { 0 } else { can - can % lot }
}

#[cfg(test)]
mod tests {
    use super::{quantity, works};

    #[test]
    fn hotelling_extract_decision() {
        // A price that rises faster than the return required is worth waiting for.
        assert!(!works(100.0, 40.0, 130.0, (0.05, 1.0), true));
        // One that rises slower is worth taking now.
        assert!(works(100.0, 40.0, 62.0, (0.05, 1.0), true));
        // Nothing is worked below its cost, and a deposit without end is worked whenever the price covers it.
        assert!(!works(30.0, 40.0, 10.0, (0.05, 1.0), true));
        assert!(works(50.0, 40.0, 500.0, (0.05, 1.0), false));
    }

    #[test]
    fn a_deposit_gives_no_more_than_it_holds_in_whole_lots() {
        assert_eq!(quantity(250, 7, None, 100), 1700);
        assert_eq!(quantity(250, 7, Some(1234), 100), 1200);
        assert_eq!(quantity(250, 7, Some(0), 100), 0);
    }
}
