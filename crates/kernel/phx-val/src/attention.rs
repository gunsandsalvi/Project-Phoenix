//! Attention (Reis, 2006): how often a cell's members review a lumpy decision is their own choice, from what leaving
//! it costs them and what reviewing costs. A decision left unreviewed for τ days loses ½·ψ·σ²·t a day at t days, where
//! ψ is the loss's curvature in money per unit² per day and σ² the variance per day of what the decision targets;
//! a review costs c. Minimising c ÷ τ + ¼·ψ·σ²·τ gives the review intensity λ = ½·sqrt(ψ·σ² ÷ c) per day.

use libm::{expm1, sqrt};
use phx_macros::clause;
use phx_num::violation;

/// `g = ½·sqrt(ψ ÷ c)`, the part of the intensity the cell holds between its visits. A review costs something real,
/// so a free one is an impossible state, as is a loss that shrinks when the decision drifts.
#[clause("REP.38", "REP.21")]
#[must_use]
pub fn gain(curvature: f64, cost: f64) -> f64 {
    if cost.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        violation!(clause = "REP.21", "a review that costs nothing");
    }
    if curvature.is_nan() || curvature < 0.0 {
        violation!(clause = "REP.38", "a loss that falls as a decision drifts");
    }
    sqrt(curvature / cost) / 2.0
}

/// The review intensity per day: `g·sqrt(σ²_own + σ²_pub)`.
#[clause("REP.38", "REP.35")]
#[must_use]
pub fn intensity(gain: f64, var_own: f64, var_public: f64) -> f64 {
    let var = var_own + var_public;
    if var.is_nan() || var < 0.0 {
        violation!(clause = "REP.38", "a negative variance");
    }
    gain * sqrt(var)
}

/// The daily review probability, `1 − e^(−λ)`, so that −ln(1 − a) = λ adds into the review exposure.
#[clause("REP.21")]
#[must_use]
pub fn probability(intensity: f64) -> f64 {
    -expm1(-intensity)
}

/// A span of days over which the method's public variance held one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub days: u32,
    pub var_public: f64,
}

/// The review exposure accrued over spans since the cell's last visit: one term per span, exact, since the rate is
/// constant within each.
#[clause("REP.21", "REP.38")]
#[must_use]
pub fn exposure(gain: f64, var_own: f64, spans: &[Span]) -> f64 {
    spans.iter().map(|s| intensity(gain, var_own, s.var_public) * f64::from(s.days)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lambda(psi: f64, var: f64, cost: f64) -> f64 {
        intensity(gain(psi, cost), var, 0.0)
    }

    #[test]
    fn attention_rises_with_stake_and_falls_with_cost() {
        let base = lambda(2.0, 0.5, 10.0);
        assert!(lambda(4.0, 0.5, 10.0) > base);
        assert!(lambda(2.0, 1.0, 10.0) > base);
        assert!(lambda(2.0, 0.5, 20.0) < base);
        // Doubling the stake and the cost together leaves attention where it was.
        assert!((lambda(4.0, 0.5, 20.0) - base).abs() < 1e-12);
        assert!((lambda(8.0, 2.0, 4.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn attention_units() {
        let (psi, var, cost) = (3.0, 0.2, 50.0);
        let per_day = lambda(psi, var, cost);
        // Money counted in cents: ψ and c scale alike and λ does not move.
        assert!((lambda(psi * 100.0, var, cost * 100.0) - per_day).abs() < 1e-12);
        // The target in a unit ten times smaller: σ² grows a hundredfold and ψ shrinks as much.
        assert!((lambda(psi / 100.0, var * 100.0, cost) - per_day).abs() < 1e-12);
        // Per week: ψ and σ² are each seven times their daily values, and λ is seven times the daily rate.
        assert!((lambda(psi * 7.0, var * 7.0, cost) - 7.0 * per_day).abs() < 1e-12);
    }

    #[test]
    fn exposure_span_exact() {
        let g = gain(5.0, 12.0);
        let own = 0.3;
        let spans =
            [Span { days: 3, var_public: 0.1 }, Span { days: 1, var_public: 0.9 }, Span { days: 7, var_public: 0.4 }];
        let mut daily = 0.0;
        for s in spans {
            for _ in 0..s.days {
                daily += -(1.0 - probability(intensity(g, own, s.var_public))).ln();
            }
        }
        assert!((exposure(g, own, &spans) - daily).abs() < 1e-12);
    }

    #[test]
    fn a_free_review_is_refused() {
        assert_eq!(crate::testing::refused(|| gain(1.0, 0.0)), Some("REP.21"));
        assert_eq!(crate::testing::refused(|| gain(-1.0, 1.0)), Some("REP.38"));
    }
}
