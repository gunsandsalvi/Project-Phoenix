//! Counts over fixed bins: each bin holds the values at or above its lower edge and below the next, the last all
//! values from its edge up, and a value below the first edge is counted apart, never folded into a bin.

use core::ops::Not;

use phx_macros::clause;
use phx_num::Missing;
use phx_rand::float::from_u64;

/// A fixed-bin histogram.
#[clause("OBS.6", "OBS.7")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Histogram {
    edges: Vec<i64>,
    counts: Vec<u64>,
    below: u64,
}

impl Histogram {
    /// Empty bins at strictly increasing lower edges.
    ///
    /// # Errors
    /// Edges that are none, or not strictly increasing.
    pub fn new(edges: Vec<i64>) -> Result<Histogram, String> {
        if edges.is_empty() || edges.is_sorted_by(|a, b| a < b).not() {
            return Err("a histogram's edges are one or more, strictly increasing".to_owned());
        }
        let counts = vec![0; edges.len()];
        Ok(Histogram { edges, counts, below: 0 })
    }

    /// Counts `n` values of `value`.
    pub fn add(&mut self, value: i64, n: u64) {
        let at = self.edges.partition_point(|e| *e <= value);
        match at.checked_sub(1).and_then(|i| self.counts.get_mut(i)) {
            Some(c) => *c += n,
            None => self.below += n,
        }
    }

    #[must_use]
    pub fn edges(&self) -> &[i64] {
        &self.edges
    }

    #[must_use]
    pub fn counts(&self) -> &[u64] {
        &self.counts
    }

    /// The values below the first edge.
    #[must_use]
    pub fn below(&self) -> u64 {
        self.below
    }

    /// How far apart two distributions over the same bins are: half the summed differences of each bin's share of its
    /// own total, the values below the first edge a bin of their own; nought when alike, one when disjoint. Absent
    /// for other bins or an empty histogram, which has no shares.
    #[clause("GEN.8")]
    pub fn distance(&self, other: &Histogram) -> Missing<f64> {
        if self.edges != other.edges {
            return Missing::Absent;
        }
        let (a, b) = (self.total(), other.total());
        if a == 0 || b == 0 {
            return Missing::Absent;
        }
        let (a, b) = (from_u64(a), from_u64(b));
        let bins = std::iter::once((self.below, other.below))
            .chain(self.counts.iter().copied().zip(other.counts.iter().copied()));
        let sum: f64 = bins.map(|(x, y)| (from_u64(x) / a - from_u64(y) / b).abs()).sum();
        Missing::Present(sum / 2.0)
    }

    fn total(&self) -> u64 {
        self.counts.iter().sum::<u64>() + self.below
    }
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::Histogram;

    #[test]
    fn values_fall_in_the_bin_at_or_below_them_and_none_is_lost() {
        let mut h = Histogram::new(vec![1, 10, 100]).unwrap();
        for (v, n) in [(0, 2), (1, 1), (9, 3), (10, 1), (5_000, 4)] {
            h.add(v, n);
        }
        assert_eq!((h.counts(), h.below()), ([4, 1, 4].as_slice(), 2));
        assert!(Histogram::new(vec![3, 3]).is_err() && Histogram::new(Vec::new()).is_err());
    }

    #[test]
    fn distance_is_half_the_shares_apart() {
        let of = |pairs: &[(i64, u64)]| {
            let mut h = Histogram::new(vec![0, 10]).unwrap();
            for (v, n) in pairs {
                h.add(*v, *n);
            }
            h
        };
        let a = of(&[(1, 3), (20, 1)]);
        assert_eq!(a.distance(&of(&[(2, 6), (30, 2)])), Missing::Present(0.0), "alike shares at other totals");
        assert_eq!(a.distance(&of(&[(-5, 4)])), Missing::Present(1.0), "disjoint, the below-edge bin its own");
        assert_eq!(a.distance(&of(&[(1, 1), (20, 1)])), Missing::Present(0.25));
        assert_eq!(a.distance(&of(&[])), Missing::Absent, "an empty histogram has no shares");
        assert_eq!(a.distance(&Histogram::new(vec![0, 5]).unwrap()), Missing::Absent, "other bins");
    }
}
