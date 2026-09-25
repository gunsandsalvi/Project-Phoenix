//! Registered series: outlooks of instrument prices are formed only for the (method, series) pairs some holder or
//! candidate list registers, and an individual's own outlook is its method's plus a deviation it catches up on in
//! one step however many prints it missed.

use std::collections::BTreeMap;

use libm::pow;
use phx_macros::clause;
use phx_num::violation;

use crate::method::Method;
use crate::outlook::VarId;

/// The registrations per (method, series): how many holders and candidate lists read each pair.
#[clause("VAL.23")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registered {
    pairs: BTreeMap<(Method, VarId), u64>,
}

impl Registered {
    /// An apply's changes, summed per pair before they land, so the order the changes were made in decides nothing.
    /// A pair released more often than registered is an impossible state.
    pub fn apply(&mut self, changes: &[((Method, VarId), i64)]) {
        let mut summed: BTreeMap<(Method, VarId), i128> = BTreeMap::new();
        for (pair, delta) in changes {
            *summed.entry(*pair).or_insert(0) += i128::from(*delta);
        }
        // A pair no one registers is kept as no entry, so its count is none registered.
        for (pair, delta) in summed {
            let now = i128::from(self.pairs.get(&pair).copied().unwrap_or(0)) + delta;
            let Ok(now) = u64::try_from(now) else {
                violation!(clause = "VAL.23", "a series pair released more often than registered", var = pair.1.get());
            };
            if now == 0 {
                self.pairs.remove(&pair);
            } else {
                self.pairs.insert(pair, now);
            }
        }
    }

    /// The methods registered on a series, whose outlooks a new print of it updates.
    pub fn methods_of(&self, var: VarId) -> impl Iterator<Item = Method> + '_ {
        self.pairs.keys().filter(move |(_, v)| *v == var).map(|(m, _)| *m)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// An individual's own deviation from its method's adaptive outlook after `prints` prints it did not read: each
/// print closes the same fraction of it, so it decays as `(1 − lambda)^prints`.
#[clause("VAL.23")]
#[must_use]
pub fn deviation_after(deviation: f64, lambda: f64, prints: u32) -> f64 {
    deviation * pow(1.0 - lambda, f64::from(prints))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::HeuristicId;
    use crate::heuristics::adaptive;
    use crate::method::{AgeWindow, MemoryType};

    #[test]
    fn registered_deviation_catch_up_exact() {
        let lambda = 0.3;
        let prints = [1.0, 1.4, 0.8, 2.0, 1.1, 0.9];
        let (mut method, mut own) = (1.0, 1.6);
        for x in prints {
            method = adaptive(method, x, lambda);
            own = adaptive(own, x, lambda);
        }
        let caught_up = method + deviation_after(1.6 - 1.0, lambda, 6);
        assert!((own - caught_up).abs() < 1e-12);
    }

    #[test]
    fn registrations_sum_per_pair() {
        let m = Method { heuristic: HeuristicId::new(0), memory: MemoryType(1), window: AgeWindow(40) };
        let (a, b) = (VarId::new(7), VarId::new(8));
        let mut r = Registered::default();
        r.apply(&[((m, a), 2), ((m, b), 1), ((m, a), -1)]);
        assert_eq!(r.methods_of(a).count(), 1);
        r.apply(&[((m, a), -1)]);
        assert_eq!(r.methods_of(a).count(), 0);
        assert_eq!(r.len(), 1);
        assert_eq!(crate::testing::refused(move || r.clone().apply(&[((m, b), -2)])), Some("VAL.23"));
    }
}
