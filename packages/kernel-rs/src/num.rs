//! The arithmetic that would otherwise be written twice: one writer per formula, in the kernel,
//! where a mechanism can read it without importing another mechanism.
//!
//! @spec Law 4 · Law 7 · Law 15 · Appendix A

/// The sample standard deviation of a population.
pub fn dispersion(of: &[f64]) -> Option<f64> {
    variance(of).map(|v| v.sqrt())
}

pub fn variance(of: &[f64]) -> Option<f64> {
    if of.len() < 2 {
        return None;
    }
    let n = of.len() as f64;
    let mean = of.iter().sum::<f64>() / n;
    let total: f64 = of.iter().map(|x| (x - mean) * (x - mean)).sum();
    Some(total / (n - 1.0))
}

/// The sample covariance of two equally long series.
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

pub fn mean(of: &[f64]) -> Option<f64> {
    if of.is_empty() {
        return None;
    }
    Some(of.iter().sum::<f64>() / of.len() as f64)
}

/// The dust a comparison over these magnitudes is entitled to — terms × ε × Σ|magnitudes|, derived
/// per check.
pub fn dust(terms: usize, magnitudes: &[f64]) -> f64 {
    (terms as f64) * f64::EPSILON * magnitudes.iter().map(|m| m.abs()).sum::<f64>()
}

/// MSER-5: where a series stops being about how it started.
///
/// @spec XI-15 · Law 2 · Law 6
pub fn mser_5(series: &[f64]) -> Option<usize> {
    const BATCH: usize = 5;
    let batches: Vec<f64> = series
        .as_chunks::<BATCH>()
        .0
        .iter()
        .map(|c| c.iter().sum::<f64>() / BATCH as f64)
        .collect();
    // The rule needs something left after the truncation to have a mean of, and the last few batches
    // alone are a mean of nothing (Schruben's own caveat, and why the half is dropped).
    if batches.len() < 4 {
        return None;
    }
    let n = batches.len();
    let last = n / 2;
    let mut best = None;
    let mut best_at = 0usize;
    for d in 0..last {
        let rest = &batches[d..];
        let left = rest.len() as f64;
        let mean = rest.iter().sum::<f64>() / left;
        let spread: f64 = rest.iter().map(|x| (x - mean) * (x - mean)).sum();
        let mser = spread / (left * left);
        match best {
            Some(b) if mser >= b => {}
            _ => {
                best = Some(mser);
                best_at = d;
            }
        }
    }
    // A minimum at the far end of the search is not an answer: the statistic was still falling
    // when the window ran out.
    if best_at + 1 >= last {
        return None;
    }
    // And a truncation is only an answer if what is LEFT has stopped moving.
    let rest = &batches[best_at..];
    let half = rest.len() / 2;
    if half == 0 {
        return None;
    }
    let front = rest[..half].iter().sum::<f64>() / half as f64;
    let back = rest[half..].iter().sum::<f64>() / (rest.len() - half) as f64;
    match dispersion(rest) {
        Some(spread) if (back - front).abs() <= spread => best.map(|_| best_at * BATCH),
        _ => None,
    }
}

/// WHICH OF TWO LEVELS IS THE KEENER, on a side.
pub fn keener(a: f64, b: f64, buying: bool) -> f64 {
    if buying {
        if a > b {
            a
        } else {
            b
        }
    } else if a < b {
        a
    } else {
        b
    }
}

/// And how many of a list a reader may take: a window over what exists, never a cap on a quantity.
pub fn at_most(wanted: usize, there_are: usize) -> usize {
    if wanted < there_are {
        wanted
    } else {
        there_are
    }
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
        // It grows with the terms and with what passed through them, and it is never a percentage of
        // anything.
        let small = dust(2, &[1.0, 1.0]);
        let large = dust(2, &[1e9, 1e9]);
        assert!(large > small);
        assert!(dust(0, &[1e9]) == 0.0);
    }
}
