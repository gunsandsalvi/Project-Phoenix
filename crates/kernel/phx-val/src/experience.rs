//! Experience weighting (Malmendier and Nagel, 2016): a member weights the years it lived through, and only those,
//! with the older years counting less by its memory type's exponent.

use libm::pow;
use phx_macros::clause;
use phx_num::Missing;

/// The weight of the year `k` years back (0 the last closed year) for a member who has lived `lived` years:
/// `(lived − k)^theta`, unnormalised; a year before its life weighs nothing, since it never saw it.
#[clause("VAL.23")]
#[must_use]
pub fn weight(lived: u32, k: u32, theta: f64) -> f64 {
    if k >= lived {
        return 0.0;
    }
    pow(f64::from(lived - k), theta)
}

/// The experience-weighted long mean of a series' annual means, newest first, over the years the member lived and
/// the series has; absent when the member has lived no year the series covers.
#[clause("VAL.23", "VAL.5")]
pub fn long_mean(annual: &[f64], lived: u32, theta: f64) -> Missing<f64> {
    let mut total = 0.0;
    let mut weights = 0.0;
    for (k, x) in (0_u32..).zip(annual.iter()) {
        if k >= lived {
            break;
        }
        let w = weight(lived, k, theta);
        total += w * x;
        weights += w;
    }
    if weights > 0.0 { Missing::Present(total / weights) } else { Missing::Absent }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share_beyond(lived: u32, years_back: u32, theta: f64) -> f64 {
        let all: f64 = (0..lived).map(|k| weight(lived, k, theta)).sum();
        let old: f64 = (years_back..lived).map(|k| weight(lived, k, theta)).sum();
        old / all
    }

    #[test]
    fn experience_weights_by_age() {
        // An older class puts more of its weight on years long past.
        for theta in [0.5, 1.0, 2.0] {
            assert!(share_beyond(60, 10, theta) > share_beyond(30, 10, theta));
        }
        // A year before a member's life weighs nothing.
        assert!(weight(20, 25, 1.0).abs() < f64::EPSILON);
        // Recent years weigh more for a positive exponent.
        assert!(weight(40, 0, 1.0) > weight(40, 30, 1.0));
    }

    #[test]
    fn long_mean_reads_only_lived_years() {
        let annual = [2.0, 2.0, 10.0];
        // Two lived years never see the third.
        assert_eq!(long_mean(&annual, 2, 1.0), Missing::Present(2.0));
        // The snapshot's one value is the whole history at the opening.
        assert!(matches!(long_mean(&[3.0], 50, 1.5), Missing::Present(m) if (m - 3.0).abs() < 1e-12));
        assert_eq!(long_mean(&[], 50, 1.0), Missing::Absent);
        let m = match long_mean(&annual, 3, 1.0) {
            Missing::Present(m) => m,
            Missing::Absent => f64::NAN,
        };
        assert!((m - (3.0 * 2.0 + 2.0 * 2.0 + 10.0) / 6.0).abs() < 1e-12);
    }
}
