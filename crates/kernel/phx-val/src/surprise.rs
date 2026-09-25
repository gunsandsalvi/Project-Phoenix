//! Surprises and confidence: observed less expected, and the width of recent surprises as the only confidence there
//! is.

use phx_macros::clause;
use phx_num::Missing;

#[clause("VAL.4")]
#[must_use]
pub fn surprise(observed: f64, expected: f64) -> f64 {
    observed - expected
}

/// The width after a surprise: the exponentially weighted mean of absolute surprises at the party's memory speed; the
/// first surprise, with no width yet, is the width.
#[clause("VAL.4", "VAL.19")]
#[must_use]
pub fn width(previous: Missing<f64>, surprise: f64, lambda: f64) -> f64 {
    let size = surprise.abs();
    match previous {
        Missing::Present(w) => w + lambda * (size - w),
        Missing::Absent => size,
    }
}

/// Whether a surprise wakes its party: larger than its type's attention sensitivity times its width.
#[clause("REP.35")]
#[must_use]
pub fn wakes(surprise: f64, width: f64, sensitivity: f64) -> bool {
    surprise.abs() > sensitivity * width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surprise_width_ewma() {
        assert!((surprise(3.0, 2.0) - 1.0).abs() < f64::EPSILON);
        let mut w = Missing::Absent;
        for s in [2.0, -2.0, 2.0] {
            w = Missing::Present(width(w, s, 0.5));
        }
        assert_eq!(w, Missing::Present(2.0));
        assert_eq!(Missing::Present(width(Missing::Present(1.0), -3.0, 0.25)), Missing::Present(1.5));
        let mut w = 0.0;
        for _ in 0..200 {
            w = width(Missing::Present(w), 4.0, 0.1);
        }
        assert!((w - 4.0).abs() < 1e-6);
    }

    #[test]
    fn a_surprise_wakes_beyond_its_sensitivity() {
        assert!(wakes(-3.1, 1.0, 3.0));
        assert!(!wakes(2.9, 1.0, 3.0));
    }
}
