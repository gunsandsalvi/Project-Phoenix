use libm::{exp, fabs, lgamma, log, log1p, pow, rint};
use phx_rand::float::{floor_to_i64, from_i64};
use phx_rand::normal_quantile;

use crate::consts::{CF_TINY, DECIMAL_RADIX_F64, NUMERIC_STEPS, PARAM_SCALE_F64, PPM};
use crate::register::values::Family;

fn param(raw: i64) -> f64 {
    from_i64(raw) / PARAM_SCALE_F64
}

/// Refuses parameters no distribution has: a scale or shape that is not positive.
pub(crate) fn check(f: &Family) -> Result<(), String> {
    let positive: &[i64] = match f {
        Family::Normal { sd, .. } => &[*sd],
        Family::LogNormal { sigma, .. } => &[*sigma],
        Family::Pareto { x_min, alpha } => &[*x_min, *alpha],
        Family::LogNormalParetoTail { sigma, threshold, alpha, .. } => &[*sigma, *threshold, *alpha],
        Family::Gamma { shape, scale } => &[*shape, *scale],
        Family::Beta { a, b } => &[*a, *b],
        Family::Weibull { k, lambda } => &[*k, *lambda],
        Family::Discrete { .. } | Family::Empirical { .. } => &[],
    };
    if positive.iter().any(|p| *p <= 0) {
        return Err(format!("{f:?} has a parameter that must be positive and is not"));
    }
    Ok(())
}

/// The regularised lower incomplete gamma function P(a, x): a series below a + 1, Lentz's continued fraction above
/// (Press et al., Numerical Recipes, third edition, chapter 6).
fn gamma_p(shape: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let front = exp(-x + shape * log(x) - lgamma(shape));
    if x < shape + 1.0 {
        let (mut term, mut sum, mut next) = (1.0 / shape, 1.0 / shape, shape);
        for _ in 0..NUMERIC_STEPS {
            next += 1.0;
            term *= x / next;
            sum += term;
            if fabs(term) < fabs(sum) * f64::EPSILON {
                break;
            }
        }
        return sum * front;
    }
    let tiny = |v: f64| if fabs(v) < CF_TINY { CF_TINY } else { v };
    let mut denominator = x + 1.0 - shape;
    let (mut upper, mut lower) = (1.0 / CF_TINY, 1.0 / denominator);
    let mut product = lower;
    let mut step = 1.0;
    for _ in 0..NUMERIC_STEPS {
        let numerator = -step * (step - shape);
        denominator += 2.0;
        lower = 1.0 / tiny(numerator * lower + denominator);
        upper = tiny(denominator + numerator / upper);
        let delta = lower * upper;
        product *= delta;
        step += 1.0;
        if fabs(delta - 1.0) < f64::EPSILON {
            break;
        }
    }
    1.0 - front * product
}

/// The continued fraction of the incomplete beta function, by the same method.
fn beta_fraction(alpha: f64, beta: f64, x: f64) -> f64 {
    let (sum, above, below) = (alpha + beta, alpha + 1.0, alpha - 1.0);
    let tiny = |v: f64| if fabs(v) < CF_TINY { CF_TINY } else { v };
    let mut upper = 1.0;
    let mut lower = 1.0 / tiny(1.0 - sum * x / above);
    let mut product = lower;
    let mut step = 1.0;
    for _ in 0..NUMERIC_STEPS {
        let twice = 2.0 * step;
        let even = step * (beta - step) * x / ((below + twice) * (alpha + twice));
        lower = 1.0 / tiny(1.0 + even * lower);
        upper = tiny(1.0 + even / upper);
        product *= lower * upper;
        let odd = -(alpha + step) * (sum + step) * x / ((alpha + twice) * (above + twice));
        lower = 1.0 / tiny(1.0 + odd * lower);
        upper = tiny(1.0 + odd / upper);
        let delta = lower * upper;
        product *= delta;
        step += 1.0;
        if fabs(delta - 1.0) < f64::EPSILON {
            break;
        }
    }
    product
}

/// The regularised incomplete beta function `I_x(a, b)`.
fn beta_i(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = exp(lgamma(a + b) - lgamma(a) - lgamma(b) + a * log(x) + b * log1p(-x));
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_fraction(b, a, 1.0 - x) / b
    }
}

/// The point where an increasing distribution function reaches `p`, by bisection over `[0, high]`, `high` doubled
/// from 1 until it is reached.
fn invert(cdf: impl Fn(f64) -> f64, p: f64) -> Result<f64, String> {
    let mut high = 1.0;
    let mut doublings = 0;
    while cdf(high) < p {
        high *= 2.0;
        doublings += 1;
        if doublings > NUMERIC_STEPS {
            return Err(format!("no point reaches probability {p}"));
        }
    }
    let mut low = 0.0;
    for _ in 0..NUMERIC_STEPS {
        let middle = f64::midpoint(low, high);
        if cdf(middle) < p {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(f64::midpoint(low, high))
}

/// The distribution's mean, where its family has one in closed form: a log-normal body's share below the threshold and
/// the Pareto tail's above it, `t·α/(α − 1)` over the tail's mass.
pub(crate) fn mean(f: &Family) -> Result<f64, String> {
    match f {
        Family::LogNormal { mu, sigma } => {
            let (mu, sigma) = (param(*mu), param(*sigma));
            Ok(exp(mu + sigma * sigma / 2.0))
        }
        Family::LogNormalParetoTail { mu, sigma, threshold, alpha } => {
            let (mu, sigma, t, a) = (param(*mu), param(*sigma), param(*threshold), param(*alpha));
            if a <= 1.0 {
                return Err(format!("a Pareto tail of index {a} has no mean"));
            }
            let at_threshold = normal_cdf((log(t) - mu) / sigma);
            let body = exp(mu + sigma * sigma / 2.0) * normal_cdf((log(t) - mu - sigma * sigma) / sigma);
            Ok(body + (1.0 - at_threshold) * t * a / (a - 1.0))
        }
        other => Err(format!("{other:?} has no mean in closed form here")),
    }
}

/// The value below which a share `p` of the distribution lies, for `p` in (0, 1).
pub(crate) fn quantile(f: &Family, p: f64) -> Result<f64, String> {
    let x = match f {
        Family::Normal { mean, sd } => param(*mean) + param(*sd) * normal_quantile(p),
        Family::LogNormal { mu, sigma } => exp(param(*mu) + param(*sigma) * normal_quantile(p)),
        Family::Pareto { x_min, alpha } => param(*x_min) * exp(-log1p(-p) / param(*alpha)),
        Family::LogNormalParetoTail { mu, sigma, threshold, alpha } => {
            let (mu, sigma, t) = (param(*mu), param(*sigma), param(*threshold));
            let at_threshold = normal_cdf((log(t) - mu) / sigma);
            if p <= at_threshold {
                exp(mu + sigma * normal_quantile(p))
            } else {
                t * pow((1.0 - at_threshold) / (1.0 - p), 1.0 / param(*alpha))
            }
        }
        Family::Gamma { shape, scale } => {
            let a = param(*shape);
            param(*scale) * invert(|x| gamma_p(a, x), p)?
        }
        Family::Beta { a, b } => {
            let (a, b) = (param(*a), param(*b));
            invert(|x| beta_i(a, b, x), p)?
        }
        Family::Weibull { k, lambda } => param(*lambda) * pow(-log1p(-p), 1.0 / param(*k)),
        Family::Empirical { values, cumulative_ppm } => {
            let q = p * f64::from(PPM);
            let above = cumulative_ppm.partition_point(|c| f64::from(*c) < q);
            let below = above.checked_sub(1).ok_or_else(|| format!("probability {p} below the empirical points"))?;
            let (Some(c0), Some(c1), Some(v0), Some(v1)) =
                (cumulative_ppm.get(below), cumulative_ppm.get(above), values.get(below), values.get(above))
            else {
                return Err(format!("probability {p} outside the empirical points"));
            };
            let (c0, c1) = (f64::from(*c0), f64::from(*c1));
            param(*v0) + (param(*v1) - param(*v0)) * (q - c0) / (c1 - c0)
        }
        Family::Discrete { .. } => return Err("a discrete distribution's types are declared, not cut".to_owned()),
    };
    if x.is_finite() { Ok(x) } else { Err(format!("the quantile at {p} is not a finite number")) }
}

/// The standard normal distribution function, by the incomplete gamma function: Φ(z) = (1 ± P(½, z²/2)) / 2.
fn normal_cdf(z: f64) -> f64 {
    let half = 1.0 / 2.0;
    let from_middle = gamma_p(half, z * z / 2.0) / 2.0;
    if z < 0.0 { half - from_middle } else { half + from_middle }
}

/// `x` as an integer at `exp` decimal places, rounded half to even; none when it is not finite or out of range.
pub(crate) fn to_places(x: f64, exp: u8) -> Option<i64> {
    let scaled = x * pow(DECIMAL_RADIX_F64, f64::from(exp));
    if !scaled.is_finite() {
        return None;
    }
    floor_to_i64(rint(scaled))
}

#[cfg(test)]
mod tests {
    use super::{Family, beta_i, gamma_p, mean, normal_cdf, quantile};

    const S: i64 = 1_000_000_000_000;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * (1.0 + b.abs())
    }

    /// The closed-form mean is the quantiles' average over a fine grid, for a tail thin enough that the grid holds it.
    #[test]
    fn a_joined_tails_mean_is_its_quantiles_average() {
        let f = Family::LogNormalParetoTail { mu: 0, sigma: S / 2, threshold: 2 * S, alpha: 6 * S };
        let n = 1_000_000_u32;
        let average: f64 =
            (0..n).map(|k| quantile(&f, (f64::from(k) + 0.5) / f64::from(n)).unwrap()).sum::<f64>() / f64::from(n);
        let m = mean(&f).unwrap();
        assert!((m - average).abs() < 1e-3 * m, "closed form {m}, grid {average}");
        assert!(mean(&Family::LogNormalParetoTail { mu: 0, sigma: S, threshold: S, alpha: S }).is_err());
    }

    #[test]
    fn distribution_functions_known_values() {
        // P(1, x) = 1 − e^(−x); P(2, 1) = 1 − 2/e; I_x(1, 1) = x; I_0.5(2, 3) = 11/16.
        assert!(close(gamma_p(1.0, 2.0), 1.0 - (-2.0_f64).exp()));
        assert!(close(gamma_p(2.0, 1.0), 1.0 - 2.0 / std::f64::consts::E));
        assert!(close(gamma_p(3.0, 10.0), 1.0 - 61.0 * (-10.0_f64).exp()));
        assert!(close(beta_i(1.0, 1.0, 0.3), 0.3));
        assert!(close(beta_i(2.0, 3.0, 0.5), 11.0 / 16.0));
        assert!(close(normal_cdf(1.959_963_984_540_054), 0.975));
    }

    #[test]
    fn quantiles_known_values() {
        let q = |f: Family, p| quantile(&f, p).unwrap();
        // Gamma(2, 1)'s median solves 1 − (1 + x)e^(−x) = 1/2: 1.678346990.
        assert!(close(q(Family::Gamma { shape: 2 * S, scale: S }, 0.5), 1.678_346_990_016_661));
        // Beta(2, 2)'s median is 1/2 by symmetry; its 0.25 quantile solves 3x² − 2x³ = 1/4: 0.326351822.
        assert!(close(q(Family::Beta { a: 2 * S, b: 2 * S }, 0.5), 0.5));
        assert!(close(q(Family::Beta { a: 2 * S, b: 2 * S }, 0.25), 0.326_351_822_333_069_6));
        // Weibull(k = 2, λ = 1): sqrt(−ln(1 − p)); Pareto(1, 2): (1 − p)^(−1/2).
        assert!(close(q(Family::Weibull { k: 2 * S, lambda: S }, 0.5), std::f64::consts::LN_2.sqrt()));
        assert!(close(q(Family::Pareto { x_min: S, alpha: 2 * S }, 0.75), 2.0));
        // A log-normal body joined to a Pareto tail at its median: e^μ below, the tail above.
        let joined = Family::LogNormalParetoTail { mu: 0, sigma: S, threshold: S, alpha: 2 * S };
        assert!(close(q(joined.clone(), 0.5), 1.0));
        assert!(close(q(joined, 0.875), 2.0), "(0.5 / 0.125)^(1/2)");
        let empirical = Family::Empirical { values: vec![0, 10 * S], cumulative_ppm: vec![0, 1_000_000] };
        assert!(close(q(empirical, 0.25), 2.5));
    }
}
