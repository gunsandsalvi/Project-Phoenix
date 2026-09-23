use libm::{exp, floor, lgamma, log, sqrt};
use phx_macros::clause;
use phx_num::violation;

use crate::binomial::exact;
use crate::consts::{HALF, HIN_SWITCH, HRUA_D1, HRUA_D2, HRUA_FOUR, HRUA_SPAN, HRUA_THREE};
use crate::draws::Draws;
use crate::float::{floor_to_u64, from_u64};
use crate::uniform::open_unit;

/// ln C(a, b), from log-gamma.
fn ln_choose(a: f64, b: f64) -> f64 {
    lgamma(a + 1.0) - lgamma(b + 1.0) - lgamma(a - b + 1.0)
}

/// Successes in `draws` taken without replacement from `total`, of which `successes` are successes. A sample larger
/// than half the population is drawn as its complement, so the work is bounded by the smaller.
#[clause("CHN.7")]
pub fn hypergeometric(d: &mut Draws, total: u64, successes: u64, draws: u64) -> u64 {
    if successes > total || draws > total {
        violation!(clause = "CHN.2", "a sample or successes beyond the population", total = total);
    }
    if draws == 0 || successes == 0 {
        return 0;
    }
    if successes == total {
        return draws;
    }
    if draws == total {
        return successes;
    }
    let complement = draws > total - draws;
    let m = if complement { total - draws } else { draws };
    let x = if m <= HIN_SWITCH { hin(d, total, successes, m) } else { hrua(d, total, successes, m) };
    if complement { successes - x } else { x }
}

/// Inversion from the lowest possible count, with the pmf's ratio recurrence; the rounding leftover past the highest
/// is rejected, not truncated.
fn hin(d: &mut Draws, total: u64, good: u64, m: u64) -> u64 {
    let failures = total - good;
    // The sample less as many failures as it can hold: the fewest successes it can contain.
    let lo = m - if failures < m { failures } else { m };
    let hi = if m < good { m } else { good };
    let (nf, sf, mf) = (exact(total), exact(good), exact(m));
    let lof = from_u64(lo);
    let p_lo = exp(ln_choose(sf, lof) + ln_choose(nf - sf, mf - lof) - ln_choose(nf, mf));
    loop {
        let mut target = open_unit(d);
        let mut count = lo;
        let mut px = p_lo;
        loop {
            if target <= px {
                return count;
            }
            target -= px;
            if count == hi {
                break;
            }
            let cf = from_u64(count);
            px *= (sf - cf) * (mf - cf) / ((cf + 1.0) * (nf - sf - mf + cf + 1.0));
            count += 1;
            if px <= 0.0 {
                break;
            }
        }
    }
}

/// Stadlober's HRUA (1990): ratio of uniforms around the mode for the smaller of successes and failures, with the
/// sample at most half the population.
fn hrua(d: &mut Draws, total: u64, good: u64, m: u64) -> u64 {
    let bad = total - good;
    let flip = good > bad;
    let (small, large) = if flip { (bad, good) } else { (good, bad) };
    let (pop, smallf, largef, mf) = (exact(total), exact(small), exact(large), exact(m));
    let d4 = smallf / pop;
    let d5 = 1.0 - d4;
    let d6 = mf * d4 + HALF;
    let d7 = sqrt((pop - mf) * mf * d4 * d5 / (pop - 1.0) + HALF);
    let d8 = HRUA_D1 * d7 + HRUA_D2;
    let d9 = floor((mf + 1.0) * (smallf + 1.0) / (pop + 2.0));
    let log_mass =
        |k: f64| lgamma(k + 1.0) + lgamma(smallf - k + 1.0) + lgamma(mf - k + 1.0) + lgamma(largef - mf + k + 1.0);
    let d10 = log_mass(d9);
    let top = if mf < smallf { mf } else { smallf };
    let tail = floor(d6 + HRUA_SPAN * d7);
    let d11 = if top + 1.0 < tail { top + 1.0 } else { tail };
    let candidate = loop {
        let u1 = open_unit(d);
        let u2 = open_unit(d);
        let point = d6 + d8 * (u2 - HALF) / u1;
        if point < 0.0 || point >= d11 {
            continue;
        }
        let k = floor(point);
        let gap = d10 - log_mass(k);
        if u1 * (HRUA_FOUR - u1) - HRUA_THREE <= gap {
            break k;
        }
        if u1 * (u1 - gap) >= 1.0 {
            continue;
        }
        if 2.0 * log(u1) <= gap {
            break k;
        }
    };
    let Some(count) = floor_to_u64(candidate) else {
        violation!(clause = "CHN.2", "a hypergeometric count outside its support");
    };
    if flip { m - count } else { count }
}

/// Counts per category in a sample of `n` without replacement, by sequential conditional hypergeometrics: each
/// count at most its category's, summing to `n`.
#[clause("CHN.7")]
pub fn multivariate_hypergeometric(d: &mut Draws, counts: &[u64], n: u64, out: &mut [u64]) {
    if counts.len() != out.len() {
        violation!(clause = "CHN.2", "counts and outputs of different lengths");
    }
    let Some(mut rest) = counts.iter().try_fold(0_u64, |t, c| t.checked_add(*c)) else {
        violation!(clause = "Law 7", "a population overflows");
    };
    if n > rest {
        violation!(clause = "CHN.2", "a sample larger than its population", n = n, total = rest);
    }
    let mut remaining = n;
    for (slot, c) in out.iter_mut().zip(counts) {
        let x = hypergeometric(d, rest, *c, remaining);
        *slot = x;
        rest -= c;
        remaining -= x;
    }
}

#[cfg(test)]
mod tests {
    use super::{hypergeometric, multivariate_hypergeometric};
    use crate::float::from_u64;
    use crate::testing::{chi_square_passes, draws, ln_choose, mean_within, violation_clause};

    fn pmf(total: u64, s: u64, m: u64, k: u64) -> f64 {
        if k > s || m - k > total - s {
            return 0.0;
        }
        libm::exp(ln_choose(s, k) + ln_choose(total - s, m - k) - ln_choose(total, m))
    }

    #[test]
    fn hypergeometric_exact() {
        // Inversion (m ≤ 10), HRUA with successes the smaller and the larger, and a sample beyond half.
        let cases = [(50_u64, 20_u64, 10_u64), (1000, 300, 100), (1000, 700, 100), (200, 60, 150), (40, 39, 25)];
        for (i, (total, s, m)) in cases.into_iter().enumerate() {
            let mut d = draws("hypergeometric", i);
            let xs: Vec<u64> = (0..100_000).map(|_| hypergeometric(&mut d, total, s, m)).collect();
            let probs: Vec<f64> = (0..=m).map(|k| pmf(total, s, m, k)).collect();
            assert!(chi_square_passes(&xs, &probs), "total={total} s={s} m={m}");
            let (nf, sf, mf) = (from_u64(total), from_u64(s), from_u64(m));
            let mean = mf * sf / nf;
            let var = mean * (1.0 - sf / nf) * (nf - mf) / (nf - 1.0);
            assert!(mean_within(&xs, mean, var), "mean total={total} s={s} m={m}");
        }
        let mut d = draws("hypergeometric", 9);
        assert_eq!(violation_clause(move || hypergeometric(&mut d, 5, 6, 1)), "CHN.2");
    }

    #[test]
    fn mvh_bounds_and_sum() {
        let counts = [5_u64, 0, 17, 3, 40];
        let mut d = draws("mvh", 0);
        let mut out = [0_u64; 5];
        for n in [0_u64, 1, 12, 64, 65] {
            multivariate_hypergeometric(&mut d, &counts, n, &mut out);
            assert_eq!(out.iter().sum::<u64>(), n);
            assert!(out.iter().zip(counts).all(|(o, c)| *o <= c));
        }
        assert_eq!(out, counts);
    }
}
