//! The arithmetic that would otherwise be written twice: one writer per formula, in the kernel,
//! where a mechanism can read it without importing another mechanism (Law 4, Law 15).
//!
//! @spec Law 4 · Law 7 · Law 15 · Appendix A
//!
//! Two modules measuring the dispersion of two different populations are two facts, but they are not
//! two formulas: a second copy of a sample standard deviation is exactly the parallel formula Law 4
//! says to hunt, and the two copies drift the day one of them is corrected.
//!
//! **Missing is missing.** A dispersion over one observation and a covariance over one pair are not
//! small numbers, they are absent — answering zero would say the population agrees when nothing has
//! been measured (Appendix A).

/// The sample standard deviation of a population. `None` below two observations.
pub fn dispersion(of: &[f64]) -> Option<f64> {
    variance(of).map(|v| v.sqrt())
}

/// The sample variance. `None` below two observations, for the same reason.
pub fn variance(of: &[f64]) -> Option<f64> {
    if of.len() < 2 {
        return None;
    }
    let n = of.len() as f64;
    let mean = of.iter().sum::<f64>() / n;
    let total: f64 = of.iter().map(|x| (x - mean) * (x - mean)).sum();
    Some(total / (n - 1.0))
}

/// The sample covariance of two equally long series. `None` when they are not equally long, or when
/// there is not enough of them for the answer to mean anything.
pub fn covariance(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.len() < 2 {
        return None;
    }
    let n = a.len() as f64;
    let mean_a: f64 = a.iter().sum::<f64>() / n;
    let mean_b: f64 = b.iter().sum::<f64>() / n;
    let total: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - mean_a) * (y - mean_b))
        .sum();
    Some(total / (n - 1.0))
}

/// The mean. `None` over nothing: a mean of no observations is not zero, and a decision at a mean is
/// a defect in its own right (Appendix B) — this exists for statistics that are READS, and the
/// callers that take one say why.
pub fn mean(of: &[f64]) -> Option<f64> {
    if of.is_empty() {
        return None;
    }
    Some(of.iter().sum::<f64>() / of.len() as f64)
}

/// Law 7: the dust a comparison over these magnitudes is entitled to — terms × ε × Σ|magnitudes|,
/// derived per check. Never a percentage, and never widened.
pub fn dust(terms: usize, magnitudes: &[f64]) -> f64 {
    (terms as f64) * f64::EPSILON * magnitudes.iter().map(|m| m.abs()).sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dispersion_over_one_observation_is_absent_and_not_zero() {
        assert!(dispersion(&[1.0]).is_none());
        assert!(dispersion(&[]).is_none());
        assert!(dispersion(&[1.0, 3.0]).is_some());
    }

    #[test]
    fn a_wider_population_disperses_further() {
        let tight = dispersion(&[0.059, 0.060, 0.061]).unwrap();
        let wide = dispersion(&[0.02, 0.10, 0.06]).unwrap();
        assert!(wide > tight);
    }

    #[test]
    fn a_covariance_needs_two_series_of_the_same_length() {
        assert!(covariance(&[1.0, 2.0], &[1.0]).is_none());
        assert!(covariance(&[1.0], &[1.0]).is_none());
        let with_it = covariance(&[100.0, 110.0, 90.0], &[50.0, 56.0, 44.0]).unwrap();
        let against_it = covariance(&[100.0, 110.0, 90.0], &[50.0, 44.0, 56.0]).unwrap();
        assert!(with_it > 0.0 && against_it < 0.0);
    }

    #[test]
    fn a_mean_of_nothing_is_missing() {
        assert!(mean(&[]).is_none());
        assert_eq!(mean(&[2.0, 4.0]), Some(3.0));
    }

    #[test]
    fn the_dust_is_derived_from_the_magnitudes_that_went_through_the_sum() {
        // Law 7: it grows with the terms and with what passed through them, and it is never a
        // percentage of anything.
        let small = dust(2, &[1.0, 1.0]);
        let large = dust(2, &[1e9, 1e9]);
        assert!(large > small);
        assert!(dust(0, &[1e9]) == 0.0);
    }
}
