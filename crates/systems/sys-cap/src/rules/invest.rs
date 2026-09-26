//! Whether a firm invests: its own value of a project against what the money costs it at the margin now, by at least
//! its management's hurdle and the value of waiting that its uncertainty about demand gives, and only if it can fund
//! it.

use phx_macros::clause;

/// What money costs a firm at the margin now: its quoted rate for new debt and its owners' required return, weighted
/// by the share of debt its management's leverage tolerance would fund the project with.
#[clause("CAP.3")]
#[must_use]
pub fn cost_of_funds(borrowing: f64, required_return: f64, debt_share: f64) -> f64 {
    debt_share * borrowing + (1.0 - debt_share) * required_return
}

/// How far a project's value must exceed its cost before investing now beats waiting, at the rate `rate` the money
/// costs, the project's yearly payout `payout` over its value, and the volatility `sigma` of its value a year: β ÷
/// (β − 1), where β is the positive root of ½σ²β(β − 1) + (r − δ)β − r = 0 (Dixit and Pindyck, 1994). It rises with
/// the volatility; where waiting gains nothing it is one.
#[clause("CAP.3")]
#[must_use]
pub fn waiting_multiple(rate: f64, payout: f64, sigma: f64) -> f64 {
    let var = sigma * sigma;
    let drift = rate - payout - var / 2.0;
    let denominator = drift + libm::sqrt(drift * drift + 2.0 * rate * var);
    // With no volatility and a payout at or above the rate, waiting is worth nothing: the root runs off to infinity.
    if denominator <= 0.0 {
        return 1.0;
    }
    let beta = 2.0 * rate / denominator;
    beta / (beta - 1.0)
}

/// Whether to invest: the value beats the cost by the hurdle, after the value of waiting, and the firm can fund it.
#[clause("CAP.3", "CAP.11")]
#[must_use]
pub fn invests(value: f64, cost: f64, hurdle: f64, waiting: f64, fundable: bool) -> bool {
    fundable && value >= cost * (1.0 + hurdle) * waiting
}

#[cfg(test)]
mod tests {
    use super::{cost_of_funds, invests, waiting_multiple};

    #[test]
    fn invest_only_above_hurdle_and_funded() {
        assert!(invests(130.0, 100.0, 0.2, 1.0, true));
        assert!(!invests(115.0, 100.0, 0.2, 1.0, true), "below the hurdle");
        assert!(!invests(130.0, 100.0, 0.2, 1.0, false), "not fundable");
        assert!(!invests(130.0, 100.0, 0.2, 1.2, true), "waiting is worth more");
        assert!((cost_of_funds(0.06, 0.12, 0.5) - 0.09).abs() < 1e-12);
    }

    #[test]
    fn waiting_value_rises_with_uncertainty() {
        let calm = waiting_multiple(0.08, 0.06, 0.1);
        let wild = waiting_multiple(0.08, 0.06, 0.4);
        assert!(calm > 1.0 && wild > calm);
        assert!((waiting_multiple(0.08, 0.1, 0.0) - 1.0).abs() < 1e-12, "no volatility, payout above the rate");
        assert!((waiting_multiple(0.08, 0.02, 0.0) - 0.08 / 0.02).abs() < 1e-9, "the deterministic limit r ÷ δ");
    }
}
