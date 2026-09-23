use phx_macros::clause;
use phx_num::violation;

use crate::binomial::binomial;
use crate::draws::Draws;
use crate::float::{from_u64, index, len_u64};
use crate::uniform::{below_u64, open_unit};

/// Weights must be finite and not negative, and not all zero.
fn check_weights(weights: &[f64]) -> f64 {
    let mut total = 0.0;
    for w in weights {
        if !(w.is_finite() && *w >= 0.0) {
            violation!(clause = "CHN.2", "a weight that is negative or not finite");
        }
        total += w;
    }
    if total <= 0.0 {
        violation!(clause = "CHN.2", "weights with no mass");
    }
    total
}

/// Counts per category summing to `n`, by conditional binomials in the given order. The suffix sums of the weights
/// are held in `out` until each is read, so the conditional probability `w_i / Σ_{j≥i} w_j` never exceeds one and
/// nothing is allocated.
#[clause("CHN.7")]
pub fn multinomial(d: &mut Draws, n: u64, probs: &[f64], out: &mut [u64]) {
    if probs.len() != out.len() {
        violation!(clause = "CHN.2", "probabilities and outputs of different lengths");
    }
    check_weights(probs);
    let mut suffix = 0.0_f64;
    for (slot, p) in out.iter_mut().zip(probs).rev() {
        suffix += p;
        *slot = suffix.to_bits();
    }
    let mut remaining = n;
    for (slot, p) in out.iter_mut().zip(probs) {
        let rest = f64::from_bits(*slot);
        let x = if remaining == 0 || rest <= 0.0 { 0 } else { binomial(d, remaining, p / rest) };
        *slot = x;
        remaining -= x;
    }
}

/// Walker's alias table by Vose's method: one uniform index and one uniform per draw, whatever the categories.
#[derive(Clone, Debug)]
pub struct AliasTable {
    prob: Box<[f64]>,
    alias: Box<[usize]>,
}

impl AliasTable {
    #[must_use]
    pub fn new(weights: &[f64]) -> AliasTable {
        let total = check_weights(weights);
        let k = from_u64(len_u64(weights.len()));
        let mut scaled: Vec<f64> = weights.iter().map(|w| w * k / total).collect();
        let mut prob = vec![1.0; weights.len()];
        let mut alias: Vec<usize> = (0..weights.len()).collect();
        let (mut small, mut large): (Vec<usize>, Vec<usize>) =
            (0..weights.len()).partition(|i| scaled.get(*i).is_some_and(|s| *s < 1.0));
        while let (Some(l), Some(g)) = (small.pop(), large.pop()) {
            let (Some(sl), Some(pl), Some(al)) = (scaled.get(l).copied(), prob.get_mut(l), alias.get_mut(l)) else {
                violation!(clause = "CHN.2", "an alias index beyond its table");
            };
            *pl = sl;
            *al = g;
            let Some(sg) = scaled.get_mut(g) else {
                violation!(clause = "CHN.2", "an alias index beyond its table");
            };
            *sg = (*sg + sl) - 1.0;
            if *sg < 1.0 { small.push(g) } else { large.push(g) }
        }
        AliasTable { prob: prob.into_boxed_slice(), alias: alias.into_boxed_slice() }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.prob.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prob.is_empty()
    }

    /// One category, drawn with probability proportional to its weight.
    pub fn draw(&self, d: &mut Draws) -> usize {
        let i = index(below_u64(d, len_u64(self.prob.len())));
        match (self.prob.get(i), self.alias.get(i)) {
            (Some(p), Some(a)) => {
                if open_unit(d) < *p {
                    i
                } else {
                    *a
                }
            }
            _ => violation!(clause = "CHN.2", "an alias index beyond its table"),
        }
    }
}

/// `n` alias draws counted per category, for when draws are fewer than categories.
#[clause("CHN.7")]
pub fn multinomial_alias(d: &mut Draws, n: u64, table: &AliasTable, out: &mut [u64]) {
    if table.len() != out.len() {
        violation!(clause = "CHN.2", "an alias table and outputs of different lengths");
    }
    out.fill(0);
    for _ in 0..n {
        let Some(slot) = out.get_mut(table.draw(d)) else {
            violation!(clause = "CHN.2", "an alias index beyond its table");
        };
        *slot += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{AliasTable, multinomial, multinomial_alias};
    use crate::float::from_u64;
    use crate::testing::{Z, chi_square_counts, draws};

    #[test]
    fn multinomial_sums_and_marginals() {
        let probs = [0.1, 0.0, 0.25, 0.4, 0.25];
        let mut d = draws("multinomial", 0);
        let mut out = [0_u64; 5];
        let mut totals = [0_u64; 5];
        let (n, rounds) = (37_u64, 20_000_u64);
        for _ in 0..rounds {
            multinomial(&mut d, n, &probs, &mut out);
            assert_eq!(out.iter().sum::<u64>(), n);
            assert_eq!(out[1], 0);
            for (t, x) in totals.iter_mut().zip(out) {
                *t += x;
            }
        }
        // Each marginal is Binomial(n, p_i) per round; the total over the rounds has sd √(rounds·n·p(1 − p)).
        let draws_total = from_u64(n * rounds);
        for (t, p) in totals.iter().zip(probs) {
            let sd = (draws_total * p * (1.0 - p)).sqrt();
            assert!((from_u64(*t) - draws_total * p).abs() <= Z * sd + 1e-9);
        }
    }

    #[test]
    fn alias_matches_weights() {
        let weights = [3.0, 0.0, 1.0, 6.0, 0.5, 9.5];
        let table = AliasTable::new(&weights);
        let mut d = draws("alias", 0);
        let mut out = [0_u64; 6];
        multinomial_alias(&mut d, 200_000, &table, &mut out);
        let total: f64 = weights.iter().sum();
        let probs: Vec<f64> = weights.iter().map(|w| w / total).collect();
        assert_eq!(out[1], 0);
        assert!(chi_square_counts(&out, &probs));
    }
}
