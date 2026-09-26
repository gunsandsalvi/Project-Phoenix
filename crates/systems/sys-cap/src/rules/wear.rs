//! A kind of plant held by condition class: classes of equal span over the kind's service life, each with the
//! efficiency its units work at and the value they keep, units leaving each class for the next at the rate that makes
//! a unit's life, on average, the kind's service life.

use phx_macros::clause;

/// The mid-age of a condition class, in years.
#[clause("CAP.1", "REP.24")]
#[must_use]
pub fn mid_age(class: usize, classes: usize, life: f64) -> f64 {
    let (c, n) = (index(class), index(classes));
    (2.0 * c + 1.0) * life / (2.0 * n)
}

/// A unit's efficiency at an age: the hyperbolic age-efficiency (L − a) ÷ (L − β·a) of a service life L, which falls
/// slowly at first and fast towards the end of the life, the faster the smaller β.
#[clause("CAP.1", "CAP.6")]
#[must_use]
pub fn efficiency(age: f64, life: f64, beta: f64) -> f64 {
    (life - age) / (life - beta * age)
}

/// A unit's value at an age, as a share of what it cost new: the geometric decline of the kind's depreciation rate.
#[clause("CAP.6")]
#[must_use]
pub fn value(age: f64, depreciation: f64) -> f64 {
    libm::exp(-depreciation * age)
}

/// The yearly rate at which units leave each class, so that a unit passes all the classes in the service life on
/// average.
#[clause("CAP.6")]
#[must_use]
pub fn leaving_rate(classes: usize, life: f64) -> f64 {
    index(classes) / life
}

/// Each class's units relative to the newest's in a stock that has grown at `growth` a year for as long as its oldest
/// units have lived: each class holds the units that entered its span's length earlier, fewer by the growth since.
#[clause("CAP.1")]
#[must_use]
pub fn steady_weights(classes: usize, life: f64, growth: f64) -> Vec<f64> {
    let rate = leaving_rate(classes, life);
    let ratio = rate / (growth + rate);
    let mut w = 1.0;
    (0..classes)
        .map(|_| {
            let this = w;
            w *= ratio;
            this
        })
        .collect()
}

/// The units in each class of a stock worth `worth`, spread by the classes' weights, each unit valued at its class's
/// share of its cost new, one currency unit a unit new.
#[clause("CAP.1", "GEN.5")]
#[must_use]
pub fn steady_units(worth: f64, weights: &[f64], values: &[f64]) -> Vec<f64> {
    let per_unit: f64 = weights.iter().zip(values).map(|(w, v)| w * v).sum();
    weights.iter().map(|w| worth * w / per_unit).collect()
}

fn index(n: usize) -> f64 {
    phx_rand::float::from_u64(phx_rand::float::len_u64(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIFE: f64 = 15.0;

    #[test]
    fn class_efficiency_and_value_fall_with_age() {
        let ages: Vec<f64> = (0..4).map(|c| mid_age(c, 4, LIFE)).collect();
        assert!((ages[0] - 1.875).abs() < 1e-12 && (ages[3] - 13.125).abs() < 1e-12, "mid-ages of equal spans");
        let eff: Vec<f64> = ages.iter().map(|a| efficiency(*a, LIFE, 0.5)).collect();
        assert!(eff.windows(2).all(|w| w[1] < w[0]) && eff[0] < 1.0 && eff[3] > 0.0);
        assert!(efficiency(1.0, LIFE, 0.75) > efficiency(1.0, LIFE, 0.5), "a larger β keeps efficiency longer");
        let val: Vec<f64> = ages.iter().map(|a| value(*a, 0.11)).collect();
        assert!(val.windows(2).all(|w| w[1] < w[0]));
    }

    #[test]
    fn steady_classes_sum_to_the_stock_value() {
        let w = steady_weights(4, LIFE, 0.02);
        assert!((w[0] - 1.0).abs() < 1e-12 && w.windows(2).all(|p| p[1] < p[0]), "fewer in older classes");
        let values: Vec<f64> = (0..4).map(|c| value(mid_age(c, 4, LIFE), 0.11)).collect();
        let units = steady_units(1_000_000.0, &w, &values);
        let worth: f64 = units.iter().zip(&values).map(|(u, v)| u * v).sum();
        assert!((worth - 1_000_000.0).abs() < 1e-6);
        let flat = steady_weights(4, LIFE, 0.0);
        assert!(flat.iter().all(|x| (x - 1.0).abs() < 1e-12), "no growth, equal classes");
    }

    #[test]
    fn wear_moves_whole_twins_and_carries_value() {
        use phx_core::wear::{carried, leaving};
        let rate = leaving_rate(4, LIFE);
        assert_eq!(leaving(3_650, 30, rate, 365), Some(79), "3 650 × (1 − e^(−4/15 × 30/365))");
        assert_eq!(leaving(10, 10_000, rate, 365), Some(10), "all of a class over many lives, and never more");
        assert_eq!(leaving(0, 30, rate, 365), Some(0));
        assert_eq!(carried(1_000, 0.8, 0.6), Some(750));
    }
}
