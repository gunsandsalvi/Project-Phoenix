use phx_macros::clause;
use phx_num::violation;

use crate::consts::{EXPONENT_CEILING, FIT_STEPS, MOST_BANKS, TOP_FIVE, TOP_THREE};

/// The sum of the first `n` ranks' weights under a Zipf law of exponent `a`.
#[must_use]
pub fn harmonic(n: u32, a: f64) -> f64 {
    (1..=n).map(|k| f64::from(k).powf(-a)).sum()
}

/// A country's banks as a Zipf law: how many there are and the exponent, fitted so the three largest hold `c3` and
/// the five largest `c5` of the banks' assets, both shares of a whole.
#[clause("GEN.2")]
#[must_use]
pub fn fit(c3: f64, c5: f64) -> (u32, f64) {
    let ratio = c3 / c5;
    let (mut lo, mut hi) = (0.0_f64, EXPONENT_CEILING);
    for _ in 0..FIT_STEPS {
        let mid = f64::midpoint(lo, hi);
        if harmonic(TOP_THREE, mid) / harmonic(TOP_FIVE, mid) < ratio {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let a = f64::midpoint(lo, hi);
    let whole = harmonic(TOP_THREE, a) / c3;
    let mut sum = harmonic(TOP_FIVE, a);
    let mut n = TOP_FIVE;
    while sum < whole {
        n += 1;
        if n > MOST_BANKS {
            violation!(clause = "GEN.2", "a bank concentration no Zipf law of a country's banks can hold", banks = n);
        }
        sum += f64::from(n).powf(-a);
    }
    (n, a)
}

/// Each bank's share of the assets under the fitted law, largest first.
#[must_use]
pub fn shares(n: u32, a: f64) -> Vec<f64> {
    let h = harmonic(n, a);
    (1..=n).map(|k| f64::from(k).powf(-a) / h).collect()
}

#[cfg(test)]
mod tests {
    use super::{fit, harmonic, shares};

    #[test]
    fn zipf_fit_reproduces_the_concentrations() {
        let (banks, exponent) = (40, 1.3);
        let drawn = shares(banks, exponent);
        let (c3, c5) = (drawn[..3].iter().sum::<f64>(), drawn[..5].iter().sum::<f64>());
        let (fitted, fitted_exponent) = fit(c3, c5);
        assert!((fitted_exponent - exponent).abs() < 1e-9, "exponent {fitted_exponent}");
        assert!(fitted.abs_diff(banks) <= 1, "banks {fitted}");
        assert!((harmonic(3, fitted_exponent) / harmonic(fitted, fitted_exponent) - c3).abs() < 0.01);
    }
}
