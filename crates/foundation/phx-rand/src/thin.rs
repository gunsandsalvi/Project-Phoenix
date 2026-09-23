use phx_macros::clause;

use crate::binomial::check_probability;
use crate::draws::Draws;
use crate::uniform::open_unit;

/// Thinning: keeps a candidate with probability `prob`.
#[clause("CHN.7")]
pub fn accept(d: &mut Draws, prob: f64) -> bool {
    check_probability(prob);
    open_unit(d) < prob
}

#[cfg(test)]
mod tests {
    use super::accept;
    use crate::float::from_u64;
    use crate::testing::{Z, draws};

    #[test]
    fn accept_keeps_its_share() {
        let mut d = draws("accept", 0);
        let n = 100_000_u64;
        let kept = (0..n).filter(|_| accept(&mut d, 0.3)).count();
        let (kept, nf) = (from_u64(u64::try_from(kept).unwrap()), from_u64(n));
        assert!((kept - 0.3 * nf).abs() <= Z * (nf * 0.3 * 0.7).sqrt());
    }
}
