//! A merchant's buy, hold and sell decision.

use phx_macros::clause;

/// What a unit bought today is worth to a merchant who holds it over the horizon: the price it expects then, less what
/// spoils at the good's rate and what its storage costs, discounted at the merchant's marginal cost of funds.
#[clause("GDS.6", "GDS.8")]
#[must_use]
pub fn carry_value(outlook: f64, (spoil_rate, storage): (f64, f64), (rate, horizon_years): (f64, f64)) -> f64 {
    (outlook * libm::exp(-spoil_rate * horizon_years) - storage) * libm::exp(-rate * horizon_years)
}

/// Whether to carry a stock: its carry value beats today's price, so the merchant buys, or holds what it has; when it
/// does not, it sells.
#[clause("GDS.6")]
#[must_use]
pub fn carries(price: f64, carry_value: f64) -> bool {
    carry_value > price
}

#[cfg(test)]
mod tests {
    use super::{carries, carry_value};

    #[test]
    fn stockist_carry_condition() {
        // A tenth expected rise over a year beats nothing spoiled, nothing to store and a five per cent cost of funds.
        let v = carry_value(110.0, (0.0, 0.0), (0.05, 1.0));
        assert!(carries(100.0, v));
        // Spoilage at a tenth a year takes the gain away.
        let spoiled = carry_value(110.0, (0.1, 0.0), (0.05, 1.0));
        assert!(!carries(100.0, spoiled));
        // So does storage costing more than the expected rise.
        assert!(!carries(100.0, carry_value(110.0, (0.0, 11.0), (0.0, 1.0))));
    }
}
