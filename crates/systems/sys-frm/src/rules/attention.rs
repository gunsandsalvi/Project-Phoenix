//! How often a firm reviews its price: the rate at which the loss from a price left standing, its curvature read from
//! the firm's own markup, grows with the variance of what the price should be, against what a review costs.

use phx_macros::clause;
use phx_num::Missing;

/// The daily chance of a price review, or none while the firm's markup gives its loss no curvature or it expects no
/// revenue.
#[clause("REP.38", "REP.21")]
pub fn review_chance(revenue_per_day: f64, markup: f64, (var_own, var_public): (f64, f64), cost: f64) -> Missing<f64> {
    if revenue_per_day <= 0.0 {
        return Missing::Absent;
    }
    let Missing::Present(psi) = super::price::curvature(revenue_per_day, markup) else {
        return Missing::Absent;
    };
    let g = phx_val::attention::gain(psi, cost);
    Missing::Present(phx_val::attention::probability(phx_val::attention::intensity(g, var_own, var_public)))
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::review_chance;

    #[test]
    fn a_bigger_stake_and_a_cheaper_review_look_more_often() {
        let chance = |r, var, cost| match review_chance(r, 0.25, (var, 0.0), cost) {
            Missing::Present(c) => c,
            Missing::Absent => panic!("a chance"),
        };
        let base = chance(1000.0, 1e-4, 50.0);
        assert!(chance(2000.0, 1e-4, 50.0) > base);
        assert!(chance(1000.0, 4e-4, 50.0) > base);
        assert!(chance(1000.0, 1e-4, 100.0) < base);
        assert_eq!(review_chance(0.0, 0.25, (1e-4, 0.0), 50.0), Missing::Absent);
        assert_eq!(review_chance(1000.0, 0.0, (1e-4, 0.0), 50.0), Missing::Absent);
    }
}
