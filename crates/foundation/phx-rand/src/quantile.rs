use libm::{exp, fabs, lgamma, log};
use phx_macros::clause;
use phx_num::violation;

use crate::consts::{QUANTILE_HALVINGS, SPECIAL_EPSILON, SPECIAL_MAX_TERMS, SPECIAL_TINY};

fn check_shape(x: f64) {
    if !(x.is_finite() && x > 0.0) {
        violation!(clause = "CHN.2", "a distribution parameter that is not finite and positive");
    }
}

fn check_probability(p: f64) {
    if !(p > 0.0 && p < 1.0) {
        violation!(clause = "CHN.2", "a quantile outside the open unit interval");
    }
}

/// A value kept off zero, so Lentz's continued fraction never divides by it.
fn off_zero(v: f64) -> f64 {
    if fabs(v) < SPECIAL_TINY { SPECIAL_TINY } else { v }
}

/// Lentz's evaluation of a continued fraction: each step takes the next partial numerator and denominator and returns
/// the factor it moved the value by.
struct Lentz {
    ratio: f64,
    inverse: f64,
    value: f64,
}

impl Lentz {
    fn start(first: f64) -> Lentz {
        let inverse = 1.0 / off_zero(first);
        Lentz { ratio: 1.0 / SPECIAL_TINY, inverse, value: inverse }
    }

    fn start_unit(first: f64) -> Lentz {
        let inverse = 1.0 / off_zero(first);
        Lentz { ratio: 1.0, inverse, value: inverse }
    }

    fn step(&mut self, numerator: f64, denominator: f64) -> f64 {
        self.inverse = 1.0 / off_zero(denominator + numerator * self.inverse);
        self.ratio = off_zero(denominator + numerator / self.ratio);
        let factor = self.inverse * self.ratio;
        self.value *= factor;
        factor
    }
}

/// The regularised lower incomplete gamma P(a, x): its series below a + 1, its continued fraction above.
#[must_use]
pub fn gamma_cdf(x: f64, shape: f64) -> f64 {
    check_shape(shape);
    if x <= 0.0 {
        return 0.0;
    }
    let front = shape * log(x) - x - lgamma(shape);
    if x < shape + 1.0 {
        let (mut term, mut sum, mut next) = (1.0 / shape, 1.0 / shape, shape);
        for _ in 0..SPECIAL_MAX_TERMS {
            next += 1.0;
            term *= x / next;
            sum += term;
            if fabs(term) < fabs(sum) * SPECIAL_EPSILON {
                break;
            }
        }
        return sum * exp(front);
    }
    let mut denominator = x + 1.0 - shape;
    let mut fraction = Lentz::start(denominator);
    for i in 1..=SPECIAL_MAX_TERMS {
        let n = f64::from(i);
        denominator += 2.0;
        if fabs(fraction.step(-n * (n - shape), denominator) - 1.0) < SPECIAL_EPSILON {
            break;
        }
    }
    1.0 - exp(front) * fraction.value
}

/// The continued fraction of the incomplete beta function, by Lentz's method.
fn beta_fraction(x: f64, a: f64, b: f64) -> f64 {
    let (sum, above, below) = (a + b, a + 1.0, a - 1.0);
    let mut fraction = Lentz::start_unit(1.0 - sum * x / above);
    for i in 1..=SPECIAL_MAX_TERMS {
        let m = f64::from(i);
        let twice = 2.0 * m;
        fraction.step(m * (b - m) * x / ((below + twice) * (a + twice)), 1.0);
        let odd = -(a + m) * (sum + m) * x / ((a + twice) * (above + twice));
        if fabs(fraction.step(odd, 1.0) - 1.0) < SPECIAL_EPSILON {
            break;
        }
    }
    fraction.value
}

/// The regularised incomplete beta `I_x(a, b)`, its fraction taken on the side where it converges fast.
#[must_use]
pub fn beta_cdf(x: f64, a: f64, b: f64) -> f64 {
    check_shape(a);
    check_shape(b);
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = exp(lgamma(a + b) - lgamma(a) - lgamma(b) + a * log(x) + b * log(1.0 - x));
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_fraction(x, a, b) / a
    } else {
        1.0 - front * beta_fraction(1.0 - x, b, a) / b
    }
}

/// The point of an increasing function's bracket where it reaches `p`, by bisection until the bracket closes.
fn bisect(p: f64, mut low: f64, mut high: f64, cdf: impl Fn(f64) -> f64) -> f64 {
    for _ in 0..QUANTILE_HALVINGS {
        let mid = low + (high - low) / 2.0;
        if mid <= low || mid >= high {
            break;
        }
        if cdf(mid) < p {
            low = mid;
        } else {
            high = mid;
        }
    }
    high
}

/// The quantile of Gamma(shape, scale): its bracket doubled from the mean until it holds `p`, then bisected.
#[clause("CHN.3")]
#[must_use]
pub fn gamma_quantile(p: f64, shape: f64, scale: f64) -> f64 {
    check_probability(p);
    check_shape(scale);
    let mut high = shape;
    while gamma_cdf(high, shape) < p {
        high *= 2.0;
    }
    scale * bisect(p, 0.0, high, |x| gamma_cdf(x, shape))
}

/// The quantile of Beta(a, b), by bisection of the unit interval.
#[clause("CHN.3")]
#[must_use]
pub fn beta_quantile(p: f64, a: f64, b: f64) -> f64 {
    check_probability(p);
    bisect(p, 0.0, 1.0, |x| beta_cdf(x, a, b))
}

/// The quantile of the Weibull with shape `k` and scale `lambda`: lambda·(−ln(1 − p))^(1/k).
#[clause("CHN.3")]
#[must_use]
pub fn weibull_quantile(p: f64, k: f64, lambda: f64) -> f64 {
    check_probability(p);
    check_shape(k);
    check_shape(lambda);
    lambda * exp(log(-libm::log1p(-p)) / k)
}

#[cfg(test)]
mod tests {
    use libm::exp;

    use super::{beta_cdf, beta_quantile, gamma_cdf, gamma_quantile, weibull_quantile};

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn cdfs_match_closed_forms() {
        for x in [0.01, 0.5, 1.0, 3.0, 20.0] {
            assert!(close(gamma_cdf(x, 1.0), 1.0 - exp(-x), 1e-13), "Gamma(1) is the exponential at {x}");
            assert!(close(gamma_cdf(x, 2.0), 1.0 - exp(-x) * (1.0 + x), 1e-13), "Gamma(2) at {x}");
        }
        for x in [0.05, 0.3, 0.5, 0.8, 0.99] {
            assert!(close(beta_cdf(x, 1.0, 1.0), x, 1e-13), "Beta(1, 1) is uniform at {x}");
            assert!(close(beta_cdf(x, 2.0, 2.0), 3.0 * x * x - 2.0 * x * x * x, 1e-13), "Beta(2, 2) at {x}");
            assert!(close(beta_cdf(x, 0.7, 3.5), 1.0 - beta_cdf(1.0 - x, 3.5, 0.7), 1e-13), "the reflection");
        }
    }

    #[test]
    fn quantiles_invert_their_cdfs() {
        for p in [1e-6, 0.01, 0.3, 0.5, 0.9, 0.999_999] {
            for shape in [0.4, 1.0, 2.5, 30.0] {
                let q = gamma_quantile(p, shape, 3.0);
                assert!(close(gamma_cdf(q / 3.0, shape), p, 1e-12), "gamma {shape} at {p}");
            }
            for (a, b) in [(0.5, 0.5), (2.0, 5.0), (8.0, 1.5)] {
                // Near 1 a steep CDF moves by more than 1e-12 between adjacent doubles, so the bound is looser.
                assert!(close(beta_cdf(beta_quantile(p, a, b), a, b), p, 1e-10), "beta {a} {b} at {p}");
            }
            let w = weibull_quantile(p, 2.0, 5.0);
            assert!(close(1.0 - exp(-(w / 5.0) * (w / 5.0)), p, 1e-12), "Weibull at {p}");
        }
    }
}
