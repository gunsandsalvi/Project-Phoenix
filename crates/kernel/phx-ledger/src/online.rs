//! A choice among counterparties in proportion to their sizes, made one at a time: how the opening gives each
//! household its bank.

use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;

/// A choice among counterparties in proportion to their drawn sizes, made online: the n-th choice takes the one
/// furthest below its share of n, ties by lot, so every prefix of the choices is apportioned within one of exact.
#[clause("GEN.4", "REP.23")]
#[derive(Clone, Debug)]
pub struct Online {
    weights: Vec<u64>,
    taken: Vec<u64>,
    made: u64,
}

impl Online {
    #[must_use]
    pub fn new(weights: Vec<u64>) -> Online {
        if weights.iter().all(|w| *w == 0) {
            violation!(clause = "GEN.4", "a choice among counterparties of no size");
        }
        let n = weights.len();
        Online { weights, taken: vec![0; n], made: 0 }
    }

    /// The next choice, by its place among the counterparties.
    pub fn next(&mut self, d: &mut Draws) -> usize {
        let total: u128 = self.weights.iter().map(|w| u128::from(*w)).sum();
        let n = u128::from(self.made + 1);
        // Each one's deficit, n·w − taken·W, compared exactly in whole numbers.
        let deficits: Vec<i128> = self
            .weights
            .iter()
            .zip(&self.taken)
            .map(|(w, t)| (n * u128::from(*w)).cast_signed() - (u128::from(*t) * total).cast_signed())
            .collect();
        let Some(first) = deficits.first().copied() else {
            violation!(clause = "GEN.4", "a choice among no counterparties");
        };
        let best = deficits.iter().fold(first, |b, x| if *x > b { *x } else { b });
        let tied: Vec<usize> = deficits.iter().enumerate().filter(|(_, x)| **x == best).map(|(i, _)| i).collect();
        let at = if let [one] = tied.as_slice() {
            *one
        } else {
            let k = phx_rand::below_u64(d, phx_rand::float::len_u64(tied.len()));
            let Some(i) = usize::try_from(k).ok().and_then(|k| tied.get(k)) else {
                violation!(clause = "CHN.2", "a lot beyond its ties");
            };
            *i
        };
        if let Some(t) = self.taken.get_mut(at) {
            *t += 1;
        }
        self.made += 1;
        at
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::Online;

    fn draws(seed: u64) -> Draws {
        Draws::new(stream_key(Seed::new(seed), "GEN.test"), Subject::new(SubjectTag::World, 0), 0, 0)
    }

    #[test]
    fn every_prefix_is_within_one_of_its_share() {
        let weights: Vec<u64> = vec![5, 3, 2];
        let mut o = Online::new(weights.clone());
        let mut d = draws(1);
        let mut taken = [0_i64; 3];
        for n in 1..=200_i64 {
            taken[o.next(&mut d)] += 1;
            for (t, w) in taken.iter().zip(&weights) {
                let exact = n * i64::try_from(*w).unwrap();
                assert!((t * 10 - exact).abs() <= 10, "{taken:?} after {n}");
            }
        }
        assert_eq!(taken, [100, 60, 40]);
    }
}
