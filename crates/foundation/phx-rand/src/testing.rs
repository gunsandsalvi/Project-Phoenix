#![cfg(test)]

use crate::draws::Draws;
use crate::float::from_u64;
use crate::key::{Seed, Subject, SubjectTag, stream_key};

/// Two-sided normal bound: P(|Z| > 6.1) ≈ 1.1e-9, so a correct sampler fails a test about once in a billion runs,
/// and with fixed keys never changes its verdict.
pub const Z: f64 = 6.1;

/// A fixed-key draw source for test `i` of a sampler.
pub fn draws(name: &str, i: usize) -> Draws {
    let subject = Subject::new(SubjectTag::World, u64::try_from(i).unwrap());
    Draws::new(stream_key(Seed::new(20_260_923), name), subject, 0, 0)
}

pub fn ln_choose(n: u64, k: u64) -> f64 {
    libm::lgamma(from_u64(n) + 1.0) - libm::lgamma(from_u64(k) + 1.0) - libm::lgamma(from_u64(n - k) + 1.0)
}

fn moments(xs: &[f64]) -> (f64, f64, f64) {
    let n = from_u64(u64::try_from(xs.len()).unwrap());
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0);
    (n, mean, var)
}

/// The sample mean is normal with sd σ/√N by the central limit theorem: |x̄ − μ| ≤ Z·σ/√N.
pub fn mean_within_f64(xs: &[f64], mean: f64, var: f64) -> bool {
    let (n, m, _) = moments(xs);
    (m - mean).abs() <= Z * (var / n).sqrt()
}

/// The sample variance has asymptotic sd √((μ₄ − σ⁴)/N): |s² − σ²| ≤ Z·√((μ₄ − σ⁴)/N).
pub fn variance_within_f64(xs: &[f64], var: f64, mu4: f64) -> bool {
    let (n, _, v) = moments(xs);
    (v - var).abs() <= Z * ((mu4 - var * var) / n).sqrt()
}

pub fn as_f64(xs: &[u64]) -> Vec<f64> {
    xs.iter().map(|x| from_u64(*x)).collect()
}

pub fn mean_within(xs: &[u64], mean: f64, var: f64) -> bool {
    mean_within_f64(&as_f64(xs), mean, var)
}

pub fn variance_within(xs: &[u64], var: f64, mu4: f64) -> bool {
    variance_within_f64(&as_f64(xs), var, mu4)
}

/// Pearson's X² over the bins whose expected count is at least 5, the rest pooled into one. The critical value is
/// Wilson and Hilferty's normal approximation to the cube root of a χ² variable, df·(1 − 2/(9df) + Z·√(2/(9df)))³,
/// accurate far into the tail; an outcome the pmf gives no chance fails outright.
pub fn chi_square_counts(counts: &[u64], probs: &[f64]) -> bool {
    let total = from_u64(counts.iter().sum());
    let mut stat = 0.0;
    let mut bins = 0_u32;
    let (mut pooled_obs, mut pooled_exp) = (0.0, 0.0);
    for (c, p) in counts.iter().zip(probs) {
        let (obs, exp) = (from_u64(*c), total * p);
        if exp <= 0.0 && obs > 0.0 {
            return false;
        }
        if exp >= 5.0 {
            stat += (obs - exp) * (obs - exp) / exp;
            bins += 1;
        } else {
            pooled_obs += obs;
            pooled_exp += exp;
        }
    }
    if pooled_exp > 0.0 {
        stat += (pooled_obs - pooled_exp) * (pooled_obs - pooled_exp) / pooled_exp;
        bins += 1;
    }
    if bins < 2 {
        return true;
    }
    let df = f64::from(bins - 1);
    let critical = df * (1.0 - 2.0 / (9.0 * df) + Z * (2.0 / (9.0 * df)).sqrt()).powi(3);
    stat <= critical
}

/// Counts of each outcome, then `chi_square_counts`; an outcome beyond the pmf's support fails.
pub fn chi_square_passes(xs: &[u64], probs: &[f64]) -> bool {
    let mut counts = vec![0_u64; probs.len()];
    for x in xs {
        match counts.get_mut(usize::try_from(*x).unwrap()) {
            Some(c) => *c += 1,
            None => return false,
        }
    }
    chi_square_counts(&counts, probs)
}

/// Runs `f`, which must violate, and returns the clause its payload names.
pub fn violation_clause<R>(f: impl FnOnce() -> R + std::panic::UnwindSafe) -> &'static str {
    let Err(payload) = std::panic::catch_unwind(f) else { panic!("expected a violation") };
    payload.downcast_ref::<phx_num::Violation>().expect("the payload is a Violation").clause
}
