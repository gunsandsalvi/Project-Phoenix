use libm::{log, log1p};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::binomial::check_probability;
use crate::draws::Draws;
use crate::float::floor_to_u64;
use crate::uniform::open_unit;

/// The number of failures before the first success, floor(ln U / ln(1 − p)); `Absent` when that lies beyond what a
/// `u64` counts, which is no success within the world's representable time.
#[clause("CHN.7")]
pub fn geometric(d: &mut Draws, p: f64) -> Missing<u64> {
    check_probability(p);
    if p <= 0.0 {
        violation!(clause = "CHN.2", "a geometric wait with no chance of success");
    }
    if p >= 1.0 {
        return Missing::Present(0);
    }
    match floor_to_u64(log(open_unit(d)) / log1p(-p)) {
        Some(g) => Missing::Present(g),
        None => Missing::Absent,
    }
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::geometric;
    use crate::testing::{draws, mean_within, violation_clause};

    #[test]
    fn geometric_mean_and_absent_beyond_range() {
        for (i, p) in [0.5, 0.1, 0.003].into_iter().enumerate() {
            let mut d = draws("geometric", i);
            let xs: Vec<u64> = (0..100_000)
                .map(|_| match geometric(&mut d, p) {
                    Missing::Present(g) => g,
                    Missing::Absent => panic!("a wait of ordinary length is absent"),
                })
                .collect();
            assert!(mean_within(&xs, (1.0 - p) / p, (1.0 - p) / (p * p)), "p={p}");
        }
        let mut d = draws("geometric", 7);
        // At p = 1e-30 the wait is about 1e30 failures, beyond 2^64 ≈ 1.8e19 unless U > 1 − 1e-11.
        assert!((0..1_000).all(|_| geometric(&mut d, 1e-30) == Missing::Absent));
        assert_eq!(geometric(&mut d, 1.0), Missing::Present(0));
        assert_eq!(violation_clause(move || geometric(&mut draws("geometric", 8), 0.0)), "CHN.2");
    }
}
