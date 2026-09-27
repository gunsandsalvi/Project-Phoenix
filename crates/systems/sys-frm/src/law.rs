//! The firm-size law: a Pareto law of a sourced exponent whose scale the country's firms per employed set, so the
//! sizes its firms are drawn at sum, in expectation, to the persons employed.

use phx_num::violation;
use phx_rand::float::{floor_to_u64, from_u64};

/// A Pareto law: its scale, the least size, and its exponent.
#[derive(Clone, Copy, Debug)]
pub struct Law {
    pub scale: f64,
    pub alpha: f64,
}

impl Law {
    /// The share of firms whose size is above `x`.
    fn above(self, x: f64) -> f64 {
        if x <= self.scale { 1.0 } else { (self.scale / x).powf(self.alpha) }
    }

    /// The share of firms whose whole size is `k`: a firm employs whole persons, so a size above `k - 1` and up to
    /// `k` employs `k`.
    #[must_use]
    pub fn at_size(self, k: u64) -> f64 {
        let Some(below) = k.checked_sub(1) else {
            violation!(clause = "GEN.2", "a firm of no persons");
        };
        self.above(from_u64(below)) - self.above(from_u64(k))
    }

    /// A firm's whole size from its size on the law of scale one, rounded up to whole persons.
    #[must_use]
    pub fn whole(self, standard: f64) -> u64 {
        let Some(n) = floor_to_u64((self.scale * standard).ceil()) else {
            violation!(clause = "GEN.2", "a firm's size beyond counting");
        };
        n
    }

    /// The mean whole size of the firms no larger than `most`.
    #[must_use]
    pub fn mean_to(self, most: u64) -> f64 {
        let (mut firms, mut persons) = (0.0, 0.0);
        for k in 1..=most {
            let share = self.at_size(k);
            firms += share;
            persons += share * from_u64(k);
        }
        if firms <= 0.0 {
            violation!(clause = "GEN.2", "a firm-size law with no firm up to its cut", most = most);
        }
        persons / firms
    }
}

/// The least `s` at which `short(s)`, rising in `s` and below nought at nought, stops being below it, found by halving:
/// the bracket's top doubled from one until it is reached, then halved until no float lies between its ends.
fn root(short: impl Fn(f64) -> f64) -> f64 {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    while short(hi) < 0.0 {
        lo = hi;
        hi *= 2.0;
        if !hi.is_finite() {
            violation!(clause = "GEN.2", "a firm-size law no scale fits");
        }
    }
    loop {
        let mid = lo + (hi - lo) / 2.0;
        if mid <= lo || mid >= hi {
            return hi;
        }
        if short(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
}

/// The law's scale at which the largest firms, drawn at `standard` sizes on the law of scale one, and `small` firms
/// drawn up to the smallest of them employ `employed` persons in expectation.
#[must_use]
pub fn scale(employed: f64, standard: &[f64], small: u64, alpha: f64) -> f64 {
    let Some(&least) = standard.last() else {
        violation!(clause = "REP.2", "a country with no firm within the promotion rank");
    };
    if from_u64(phx_rand::float::len_u64(standard.len()) + small) >= employed {
        violation!(clause = "GEN.2", "a country with no fewer firms than persons employed");
    }
    root(|s| {
        let law = Law { scale: s, alpha };
        let large: u64 = standard.iter().map(|y| law.whole(*y)).sum();
        from_u64(large) + from_u64(small) * law.mean_to(law.whole(least)) - employed
    })
}

/// The law's scale at which `small` firms drawn up to `most` persons employ `persons` in expectation.
#[must_use]
pub fn scale_to(persons: f64, small: u64, most: u64, alpha: f64) -> f64 {
    let mean = persons / from_u64(small);
    if !(1.0..from_u64(most)).contains(&mean) {
        violation!(clause = "GEN.2", "small firms whose persons no size up to the cut fits", most = most);
    }
    root(|s| Law { scale: s, alpha }.mean_to(most) - mean)
}

#[cfg(test)]
mod tests {
    use phx_rand::float::from_u64;

    use super::{Law, scale, scale_to};

    #[test]
    fn the_law_shares_sum_to_the_firms_from_its_scale() {
        let law = Law { scale: 0.3, alpha: 1.059 };
        let within: f64 = (1..100_000_u64).map(|k| law.at_size(k)).sum();
        assert!((within - (1.0 - (0.3_f64 / 99_999.0).powf(1.059))).abs() < 1e-12);
        assert!((law.at_size(1) - (1.0 - 0.3_f64.powf(1.059))).abs() < 1e-12, "sizes up to one employ one");
    }

    #[test]
    fn the_scale_makes_the_sizes_sum_to_the_employed() {
        let standard = [5_000.0, 900.0, 400.0];
        let s = scale(40_000.0, &standard, 10_000, 1.059);
        let law = Law { scale: s, alpha: 1.059 };
        let large = from_u64(standard.iter().map(|y| law.whole(*y)).sum());
        let total = large + 10_000.0 * law.mean_to(law.whole(400.0));
        assert!((total - 40_000.0).abs() < 1_000.0, "{total}: the sizes step by a whole person at the largest");
        let t = scale_to(40_000.0 - large, 10_000, law.whole(400.0), 1.059);
        let small = 10_000.0 * Law { scale: t, alpha: 1.059 }.mean_to(law.whole(400.0));
        assert!((small - (40_000.0 - large)).abs() < 1e-6, "the small firms alone take the rest");
    }
}
