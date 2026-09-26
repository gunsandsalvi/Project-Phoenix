//! How often a firm reviews its price: the rate at which the loss from a price left standing, its curvature read from
//! the firm's own markup, grows with the variance of what the price should be, against what a review costs.

use phx_macros::clause;
use phx_num::Missing;

/// The daily chance of a price review: none when the firm expects no revenue, since a price left standing then costs it
/// nothing; every day when it sells at or below cost, where a gap's loss has no bound; otherwise the rate its loss's
/// curvature and the variance of what its price should be give against a review's cost. The variance is its own
/// outlook's and that of each public series it reads.
#[clause("REP.38", "REP.21")]
#[must_use]
pub fn review_chance(revenue_per_day: f64, markup: f64, (var_own, public): (f64, &[f64]), cost: f64) -> f64 {
    if revenue_per_day <= 0.0 {
        return 0.0;
    }
    let Missing::Present(psi) = super::price::curvature(revenue_per_day, markup) else {
        return 1.0;
    };
    let g = phx_val::attention::gain(psi, cost);
    let var_public: f64 = public.iter().sum();
    phx_val::attention::probability(phx_val::attention::intensity(g, var_own, var_public))
}

#[cfg(test)]
mod tests {
    use super::review_chance;

    #[test]
    fn a_bigger_stake_and_a_cheaper_review_look_more_often() {
        let chance = |r, var, cost| review_chance(r, 0.25, (var, &[]), cost);
        let base = chance(1000.0, 1e-4, 50.0);
        assert!(chance(2000.0, 1e-4, 50.0) > base);
        assert!(chance(1000.0, 4e-4, 50.0) > base);
        assert!(chance(1000.0, 1e-4, 100.0) < base);
        assert!(review_chance(0.0, 0.25, (1e-4, &[]), 50.0) <= 0.0, "no revenue, no stake");
        assert!(review_chance(1000.0, 0.0, (1e-4, &[]), 50.0) >= 1.0, "at cost, every day");
        assert!(base < review_chance(1000.0, 0.25, (1e-4, &[3e-4]), 50.0), "a public series read adds its variance");
    }
}
