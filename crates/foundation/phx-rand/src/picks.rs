use phx_macros::clause;
use phx_num::violation;

use crate::draws::Draws;
use crate::float::len_u64;
use crate::uniform::below_u64;

/// A Fenwick tree over counts: prefix sums and single decrements in O(log e); rebuilt in place, so a caller that keeps
/// one allocates only when its categories outgrow it.
#[derive(Clone, Debug, Default)]
pub struct Fenwick {
    tree: Vec<u64>,
}

fn lowbit(i: usize) -> usize {
    i.isolate_lowest_one()
}

impl Fenwick {
    #[must_use]
    pub fn new(counts: &[u64]) -> Fenwick {
        let mut t = Fenwick::default();
        t.rebuild(counts);
        t
    }

    /// An empty tree with room for `categories`, for a caller to keep and rebuild.
    #[must_use]
    pub fn with_room(categories: usize) -> Fenwick {
        Fenwick { tree: Vec::with_capacity(categories + 1) }
    }

    /// The tree made again over `counts`, in the room it already holds.
    pub fn rebuild(&mut self, counts: &[u64]) {
        let tree = &mut self.tree;
        tree.clear();
        tree.reserve(counts.len() + 1);
        tree.push(0);
        tree.extend_from_slice(counts);
        for i in 1..tree.len() {
            let parent = i + lowbit(i);
            if let (Some(own), true) = (tree.get(i).copied(), parent < tree.len())
                && let Some(p) = tree.get_mut(parent)
            {
                *p += own;
            }
        }
    }

    /// The index whose cumulative count first exceeds `target`.
    #[must_use]
    pub fn find(&self, target: u64) -> usize {
        let e = self.tree.len() - 1;
        let mut step = if e == 0 { 0 } else { 1 << (usize::BITS - 1 - e.leading_zeros()) };
        let (mut pos, mut rest) = (0, target);
        while step > 0 {
            if let Some(v) = self.tree.get(pos + step).filter(|v| **v <= rest) {
                pos += step;
                rest -= v;
            }
            step >>= 1;
        }
        pos
    }

    pub fn decrement(&mut self, index: usize) {
        let mut i = index + 1;
        while let Some(v) = self.tree.get_mut(i) {
            if *v == 0 {
                violation!(clause = "CHN.2", "a pick from an empty category", index = index);
            }
            *v -= 1;
            i += lowbit(i);
        }
    }
}

/// `k` picks without replacement from categories holding `counts`, as picks per category: each pick a uniform over
/// what remains and a descent of the tree, O(e + k log e). The tree is built in the caller's scratch.
#[clause("CHN.7")]
pub fn pick_without_replacement(d: &mut Draws, counts: &[u64], k: u64, (out, tree): (&mut [u64], &mut Fenwick)) {
    if counts.len() != out.len() {
        violation!(clause = "CHN.2", "counts and outputs of different lengths");
    }
    let Some(mut remaining) = counts.iter().try_fold(0_u64, |t, c| t.checked_add(*c)) else {
        violation!(clause = "Law 7", "a population overflows");
    };
    if k > remaining {
        violation!(clause = "CHN.2", "more picks than things to pick", k = k, total = remaining);
    }
    tree.rebuild(counts);
    out.fill(0);
    for _ in 0..k {
        let i = tree.find(below_u64(d, remaining));
        let Some(slot) = out.get_mut(i) else {
            violation!(clause = "CHN.2", "a pick beyond its categories", index = len_u64(i));
        };
        *slot += 1;
        tree.decrement(i);
        remaining -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{Fenwick, pick_without_replacement};
    use crate::float::from_u64;
    use crate::hypergeometric::multivariate_hypergeometric;
    use crate::testing::{Z, chi_square_counts, draws, ln_choose};

    #[test]
    fn fenwick_finds_by_cumulative_count() {
        let mut t = Fenwick::new(&[2, 0, 3, 1]);
        assert_eq!([0, 1, 2, 4, 5].map(|x| t.find(x)), [0, 0, 2, 2, 3]);
        t.decrement(2);
        assert_eq!([3, 4].map(|x| t.find(x)), [2, 3]);
    }

    #[test]
    fn picks_match_mvh() {
        let counts = [7_u64, 0, 20, 3, 10];
        let (k, rounds) = (9_u64, 40_000);
        let (mut dp, mut dm) = (draws("picks", 0), draws("picks", 1));
        let (mut picks, mut mvh) = ([0_u64; 5], [0_u64; 5]);
        let mut first_picks = vec![0_u64; 8];
        let mut first_mvh = vec![0_u64; 8];
        let (mut out, mut out2) = ([0_u64; 5], [0_u64; 5]);
        for _ in 0..rounds {
            pick_without_replacement(&mut dp, &counts, k, (&mut out, &mut Fenwick::default()));
            multivariate_hypergeometric(&mut dm, &counts, k, &mut out2);
            assert_eq!(out.iter().sum::<u64>(), k);
            assert!(out.iter().zip(counts).all(|(o, c)| *o <= c));
            for i in 0..5 {
                picks[i] += out[i];
                mvh[i] += out2[i];
            }
            first_picks[usize::try_from(out[0]).unwrap()] += 1;
            first_mvh[usize::try_from(out2[0]).unwrap()] += 1;
        }
        // Both are the same law, with category means k·c_i/N. Per round a count's variance is at most k/4, so its
        // total over the rounds has sd at most √(rounds·k).
        let total = from_u64(counts.iter().sum());
        for (i, c) in counts.iter().enumerate() {
            let expected = from_u64(rounds) * from_u64(k) * from_u64(*c) / total;
            let bound = Z * (from_u64(rounds) * from_u64(k)).sqrt() + 1e-9;
            assert!((from_u64(picks[i]) - expected).abs() <= bound, "picks {i}");
            assert!((from_u64(mvh[i]) - expected).abs() <= bound, "mvh {i}");
        }
        // The first category's count is exactly Hypergeometric(40, 7, 9) under both.
        let exact: Vec<f64> =
            (0..8_u64).map(|x| libm::exp(ln_choose(7, x) + ln_choose(33, k - x) - ln_choose(40, k))).collect();
        assert!(chi_square_counts(&first_picks, &exact));
        assert!(chi_square_counts(&first_mvh, &exact));
    }

    #[test]
    fn one_pick_is_proportional_to_counts() {
        let counts = [7_u64, 0, 20, 3, 10];
        let mut d = draws("picks.one", 0);
        let mut picked = Vec::with_capacity(100_000);
        let mut out = [0_u64; 5];
        for _ in 0..100_000 {
            pick_without_replacement(&mut d, &counts, 1, (&mut out, &mut Fenwick::default()));
            assert_eq!(out.iter().sum::<u64>(), 1);
            picked.push(out.iter().position(|x| *x == 1).map_or(u64::MAX, |i| u64::try_from(i).unwrap()));
        }
        let probs: Vec<f64> = counts.iter().map(|c| from_u64(*c) / 40.0).collect();
        assert!(crate::testing::chi_square_passes(&picked, &probs));
    }
}
