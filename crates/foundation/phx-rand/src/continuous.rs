use libm::{exp, fabs, log, sqrt};
use phx_macros::clause;
use phx_num::violation;

use crate::consts::{
    AS241_A, AS241_B, AS241_C, AS241_CONST1, AS241_CONST2, AS241_D, AS241_E, AS241_F, AS241_SPLIT1, AS241_SPLIT2, HALF,
    NINE, ONE_THIRD,
};
use crate::draws::Draws;
use crate::uniform::open_unit;

/// A scale, a shape or a rate must be finite and positive.
fn check_positive(x: f64) {
    if !(x.is_finite() && x > 0.0) {
        violation!(clause = "CHN.2", "a distribution parameter that is not finite and positive");
    }
}

fn check_finite(x: f64) {
    if !x.is_finite() {
        violation!(clause = "CHN.2", "a distribution parameter that is not finite");
    }
}

/// A polynomial by Horner's rule, lowest coefficient first.
fn horner(coefficients: &[f64; 8], x: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |acc, c| acc * x + c)
}

/// The standard normal quantile, Wichura's AS241 (PPND16, 1988): about 16 digits over the whole open interval.
#[must_use]
pub fn normal_quantile(p: f64) -> f64 {
    let q = p - HALF;
    if fabs(q) <= AS241_SPLIT1 {
        let r = AS241_CONST1 - q * q;
        return q * horner(&AS241_A, r) / horner(&AS241_B, r);
    }
    let tail = if q < 0.0 { p } else { 1.0 - p };
    let r = sqrt(-log(tail));
    let value = if r <= AS241_SPLIT2 {
        let s = r - AS241_CONST2;
        horner(&AS241_C, s) / horner(&AS241_D, s)
    } else {
        let s = r - AS241_SPLIT2;
        horner(&AS241_E, s) / horner(&AS241_F, s)
    };
    if q < 0.0 { -value } else { value }
}

/// A standard normal draw, by inversion of one uniform.
#[clause("CHN.7")]
pub fn normal(d: &mut Draws) -> f64 {
    normal_quantile(open_unit(d))
}

#[clause("CHN.7")]
pub fn log_normal(d: &mut Draws, mu: f64, sigma: f64) -> f64 {
    check_finite(mu);
    check_positive(sigma);
    exp(mu + sigma * normal(d))
}

/// Pareto with scale `x_min` and tail index `alpha`: `P(X > x) = (x_min/x)^alpha`.
#[clause("CHN.7")]
pub fn pareto(d: &mut Draws, x_min: f64, alpha: f64) -> f64 {
    check_positive(x_min);
    check_positive(alpha);
    x_min * exp(-log(open_unit(d)) / alpha)
}

#[clause("CHN.7")]
pub fn gumbel(d: &mut Draws, mu: f64, beta: f64) -> f64 {
    check_finite(mu);
    check_positive(beta);
    mu - beta * log(-log(open_unit(d)))
}

#[clause("CHN.7")]
pub fn exponential(d: &mut Draws, rate: f64) -> f64 {
    check_positive(rate);
    -log(open_unit(d)) / rate
}

/// Gamma(shape, scale) by Marsaglia and Tsang (2000); a shape below one is boosted by one and scaled back by
/// U^(1/shape).
#[clause("CHN.7")]
pub fn gamma(d: &mut Draws, shape: f64, scale: f64) -> f64 {
    check_positive(shape);
    check_positive(scale);
    if shape < 1.0 {
        let boosted = gamma(d, shape + 1.0, scale);
        return boosted * exp(log(open_unit(d)) / shape);
    }
    let base = shape - ONE_THIRD;
    let spread = 1.0 / sqrt(NINE * base);
    loop {
        let z = normal(d);
        let root = 1.0 + spread * z;
        if root <= 0.0 {
            continue;
        }
        let cube = root * root * root;
        if log(open_unit(d)) < HALF * z * z + base - base * cube + base * log(cube) {
            return base * cube * scale;
        }
    }
}

/// Beta(a, b) as X/(X + Y) for independent Gamma(a) and Gamma(b).
#[clause("CHN.7")]
pub fn beta(d: &mut Draws, a: f64, b: f64) -> f64 {
    let first = gamma(d, a, 1.0);
    let second = gamma(d, b, 1.0);
    first / (first + second)
}

/// Weibull with shape `k` and scale `lambda`, by inversion: lambda·(−ln U)^(1/k).
#[clause("CHN.7")]
pub fn weibull(d: &mut Draws, k: f64, lambda: f64) -> f64 {
    check_positive(k);
    check_positive(lambda);
    lambda * exp(log(-log(open_unit(d))) / k)
}

#[cfg(test)]
mod tests {
    use libm::{erfc, sqrt, tgamma};

    use super::{beta, exponential, gamma, gumbel, log_normal, normal, normal_quantile, pareto, weibull};
    use crate::float::from_u64;
    use crate::testing::{Z, draws, mean_within_f64, variance_within_f64, violation_clause};

    const N: usize = 100_000;

    fn sample(name: &str, f: impl FnMut(&mut crate::Draws) -> f64) -> Vec<f64> {
        let mut d = draws(name, 0);
        let mut f = f;
        (0..N).map(|_| f(&mut d)).collect()
    }

    /// A frequency of an event of probability p over N draws is within Z binomial standard deviations.
    fn frequency_within(xs: &[f64], event: impl Fn(f64) -> bool, p: f64) -> bool {
        let n = from_u64(u64::try_from(xs.len()).unwrap());
        let hits = from_u64(u64::try_from(xs.iter().filter(|x| event(**x)).count()).unwrap());
        (hits - n * p).abs() <= Z * sqrt(n * p * (1.0 - p))
    }

    #[test]
    fn normal_quantile_inverts_the_cdf() {
        // Φ(x) = erfc(−x/√2)/2. Below the median p is exact in relative terms, and AS241 is good to about 1e-16
        // relative, so 1e-12 leaves room for erfc's own error.
        let mut x = -8.0;
        while x <= 0.0 {
            let p = 0.5 * erfc(-x / std::f64::consts::SQRT_2);
            assert!((normal_quantile(p) - x).abs() <= 1e-12 * (1.0 + x.abs()), "x={x}");
            // Above the median, 1 − p is rounded to the spacing of doubles below 1, ε/2, which moves the quantile by
            // up to ε/(2φ(x)); twice that, plus the 1e-12 above, bounds the difference.
            let density = libm::exp(-x * x / 2.0) / sqrt(2.0 * std::f64::consts::PI);
            let tolerance = f64::EPSILON / density + 1e-12 * (1.0 + x.abs());
            assert!((normal_quantile(1.0 - p) + x).abs() <= tolerance, "symmetric x={x}");
            x += 0.125;
        }
    }

    #[test]
    fn normal_moments_and_tails() {
        let xs = sample("normal", normal);
        // Standard normal: mean 0, variance 1, fourth moment 3; P(|Z| > 3) = 0.002_699_796.
        assert!(mean_within_f64(&xs, 0.0, 1.0));
        assert!(variance_within_f64(&xs, 1.0, 3.0));
        assert!(frequency_within(&xs, |x| x.abs() > 3.0, 0.002_699_796));
        assert!(frequency_within(&xs, |x| x < -1.0, 0.158_655_254));
    }

    #[test]
    fn pareto_tail_index() {
        // With x_min = 1 and alpha = 2.5, P(X > x) = x^-2.5.
        let xs = sample("pareto", |d| pareto(d, 1.0, 2.5));
        assert!(xs.iter().all(|x| *x >= 1.0));
        for x in [2.0_f64, 4.0, 10.0] {
            assert!(frequency_within(&xs, |v| v > x, x.powf(-2.5)), "x={x}");
        }
    }

    #[test]
    fn gamma_beta_weibull_moments() {
        // Gamma(k, θ): mean kθ, variance kθ², fourth central moment 3k(k + 2)θ⁴.
        for (i, (k, theta)) in [(0.5, 2.0), (3.0, 1.5), (1.0, 1.0)].into_iter().enumerate() {
            let xs = sample(&format!("gamma{i}"), |d| gamma(d, k, theta));
            let var = k * theta * theta;
            assert!(mean_within_f64(&xs, k * theta, var), "gamma mean k={k}");
            assert!(variance_within_f64(&xs, var, 3.0 * k * (k + 2.0) * theta.powi(4)), "gamma var k={k}");
        }
        // Beta(2, 5): mean 2/7, variance ab/((a + b)²(a + b + 1)).
        let xs = sample("beta", |d| beta(d, 2.0, 5.0));
        assert!(xs.iter().all(|x| *x > 0.0 && *x < 1.0));
        assert!(mean_within_f64(&xs, 2.0 / 7.0, 10.0 / (49.0 * 8.0)));
        // Weibull(k = 1.5, λ = 2): mean λΓ(1 + 1/k), variance λ²(Γ(1 + 2/k) − Γ(1 + 1/k)²).
        let (k, lambda) = (1.5, 2.0);
        let xs = sample("weibull", |d| weibull(d, k, lambda));
        let g1 = tgamma(1.0 + 1.0 / k);
        let var = lambda * lambda * (tgamma(1.0 + 2.0 / k) - g1 * g1);
        assert!(mean_within_f64(&xs, lambda * g1, var));
    }

    #[test]
    fn exponential_gumbel_log_normal_means() {
        let xs = sample("exponential", |d| exponential(d, 4.0));
        assert!(mean_within_f64(&xs, 0.25, 0.0625));
        // Gumbel(μ, β): mean μ + βγ, variance π²β²/6.
        let xs = sample("gumbel", |d| gumbel(d, 1.0, 2.0));
        let euler_gamma = 0.577_215_664_901_532_9;
        assert!(mean_within_f64(&xs, 1.0 + 2.0 * euler_gamma, std::f64::consts::PI.powi(2) * 4.0 / 6.0));
        // Log-normal(μ, σ): mean e^(μ + σ²/2), variance (e^σ² − 1)e^(2μ + σ²).
        let (mu, sigma) = (0.1, 0.5_f64);
        let xs = sample("log_normal", |d| log_normal(d, mu, sigma));
        let s2 = sigma * sigma;
        assert!(mean_within_f64(&xs, (mu + s2 / 2.0).exp(), (s2.exp() - 1.0) * (2.0 * mu + s2).exp()));
        assert_eq!(violation_clause(|| gamma(&mut draws("gamma", 9), -1.0, 1.0)), "CHN.2");
    }
}
