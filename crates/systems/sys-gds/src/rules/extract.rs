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

/// The units to take over a period: what the extractor's plant runs a day for the period's days, in whole units, no
/// more than the deposit holds; none beyond an integer's reach. What it takes gathers until it holds a lot to sell.
#[clause("GDS.4", "GDS.12")]
#[must_use]
pub fn quantity(per_day: f64, days: i64, remaining: Option<i64>) -> i64 {
    let Some(wanted) = phx_rand::float::floor_to_i64(per_day * phx_rand::float::from_i64(days)) else { return 0 };
    let can = match remaining {
        Some(r) if r < wanted => r,
        _ => wanted,
    };
    if can <= 0 { 0 } else { can }
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
    fn a_deposit_gives_no_more_than_it_holds_in_whole_units() {
        assert_eq!(quantity(250.0, 7, None), 1750);
        assert_eq!(quantity(250.0, 7, Some(1234)), 1234);
        assert_eq!(quantity(250.0, 7, Some(0)), 0);
        assert_eq!(quantity(0.13, 7, None), 0, "less than a unit over the period takes none");
        assert_eq!(quantity(0.2, 7, None), 1);
    }
}
