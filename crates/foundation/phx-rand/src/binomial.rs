use libm::{exp, expm1, fabs, floor, log, log1p, sqrt};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{
    BINV_SWITCH, BTPE_C0, BTPE_C1, BTPE_C2, BTPE_FAR, BTPE_P1_Q, BTPE_P1_SQRT, BTPE_SQUEEZE_A, BTPE_SQUEEZE_B, HALF,
    MAX_EXACT_COUNT, ONE_THIRD, STIRLING, STIRLING_DENOMINATOR,
};
use crate::draws::Draws;
use crate::float::{floor_to_u64, from_u64};
use crate::uniform::open_unit;

/// A probability must lie in [0, 1]; NaN fails both comparisons.
pub(crate) fn check_probability(p: f64) {
    if !(0.0..=1.0).contains(&p) {
        violation!(clause = "CHN.2", "a probability outside [0, 1]");
    }
}

/// A count exact as `f64`, since every sampler computes with it.
pub(crate) fn exact(n: u64) -> f64 {
    if n >= MAX_EXACT_COUNT {
        capacity_exceeded!("a count exact as f64", MAX_EXACT_COUNT, n);
    }
    from_u64(n)
}

/// Binomial(n, p), exactly: inversion while n·p and n·(1 − p) are both small, BTPE otherwise.
#[clause("CHN.7")]
pub fn binomial(d: &mut Draws, n: u64, p: f64) -> u64 {
    check_probability(p);
    if n == 0 || p <= 0.0 {
        return 0;
    }
    if p >= 1.0 {
        return n;
    }
    let flip = p > HALF;
    let r = if flip { 1.0 - p } else { p };
    let count = if exact(n) * r < BINV_SWITCH { inversion(d, n, r) } else { btpe(d, n, r) };
    if flip { n - count } else { count }
}

/// Sequential search from 0 with the pmf's ratio recurrence; the rounding leftover past n is rejected, not truncated.
fn inversion(d: &mut Draws, n: u64, p: f64) -> u64 {
    let ratio = p / (1.0 - p);
    let p0 = exp(exact(n) * log1p(-p));
    loop {
        let mut target = open_unit(d);
        let mut count = 0_u64;
        let mut px = p0;
        loop {
            if target <= px {
                return count;
            }
            target -= px;
            if count == n {
                break;
            }
            count += 1;
            px *= ratio * from_u64(n - count + 1) / from_u64(count);
            if px <= 0.0 {
                break;
            }
        }
    }
}

/// Stirling's remainder of ln x!, for BTPE's final test.
fn stirling(x: f64) -> f64 {
    let [s0, s1, s2, s3, s4] = STIRLING;
    let x2 = x * x;
    (s0 - (s1 - (s2 - (s3 - s4 / x2) / x2) / x2) / x2) / x / STIRLING_DENOMINATOR
}

/// Kachitvichyanukul and Schmeiser's BTPE (1988) for p ≤ 1/2: a triangle, a parallelogram and two exponential tails
/// over the pmf, with an exact acceptance test. The names are the paper's, so the code can be checked against it.
#[derive(Debug)]
struct Btpe {
    nf: f64,
    r: f64,
    q: f64,
    mode: f64,
    npq: f64,
    xm: f64,
    xl: f64,
    xr: f64,
    height: f64,
    lam_l: f64,
    lam_r: f64,
    p1: f64,
    p2: f64,
    p3: f64,
    p4: f64,
}

impl Btpe {
    fn new(n: u64, r: f64) -> Btpe {
        let nf = exact(n);
        let q = 1.0 - r;
        let fm = nf * r + r;
        let mode = floor(fm);
        let npq = nf * r * q;
        let p1 = floor(BTPE_P1_SQRT * sqrt(npq) - BTPE_P1_Q * q) + HALF;
        let xm = mode + HALF;
        let (xl, xr) = (xm - p1, xm + p1);
        let height = BTPE_C0 + BTPE_C1 / (BTPE_C2 + mode);
        let left = (fm - xl) / (fm - xl * r);
        let right = (xr - fm) / (xr * q);
        let lam_l = left * (1.0 + left / 2.0);
        let lam_r = right * (1.0 + right / 2.0);
        let p2 = p1 * (1.0 + 2.0 * height);
        let p3 = p2 + height / lam_l;
        let p4 = p3 + height / lam_r;
        Btpe { nf, r, q, mode, npq, xm, xl, xr, height, lam_l, lam_r, p1, p2, p3, p4 }
    }

    /// A candidate and its acceptance variate, or `None` when the region's own test rejects it.
    fn candidate(&self, d: &mut Draws) -> Option<(f64, f64, bool)> {
        let u = open_unit(d) * self.p4;
        let v = open_unit(d);
        if u <= self.p1 {
            return Some((floor(self.xm - self.p1 * v + u), v, true));
        }
        if u <= self.p2 {
            let x = self.xl + (u - self.p1) / self.height;
            let v = v * self.height + 1.0 - fabs(self.mode - x + HALF) / self.p1;
            return (v <= 1.0).then_some((floor(x), v, false));
        }
        if u <= self.p3 {
            let y = floor(self.xl + log(v) / self.lam_l);
            return (y >= 0.0).then_some((y, v * (u - self.p2) * self.lam_l, false));
        }
        let y = floor(self.xr - log(v) / self.lam_r);
        (y <= self.nf).then_some((y, v * (u - self.p3) * self.lam_r, false))
    }

    /// The exact test: the pmf's ratio to the mode's by products near the mode, by the squeeze and Stirling's series
    /// far from it.
    fn accept(&self, y: f64, v: f64) -> bool {
        let dist = fabs(y - self.mode);
        if dist <= BTPE_FAR || dist >= self.npq / 2.0 - 1.0 {
            let odds = self.r / self.q;
            let scale = odds * (self.nf + 1.0);
            let mut ratio = 1.0;
            let mut step = if self.mode < y { self.mode + 1.0 } else { y + 1.0 };
            let (upper, up) = if self.mode < y { (y, true) } else { (self.mode, false) };
            while step <= upper {
                if up {
                    ratio *= scale / step - odds;
                } else {
                    ratio /= scale / step - odds;
                }
                step += 1.0;
            }
            return v <= ratio;
        }
        let rho = (dist / self.npq) * ((dist * (dist * ONE_THIRD + BTPE_SQUEEZE_A) + BTPE_SQUEEZE_B) / self.npq + HALF);
        let centre = -dist * dist / (2.0 * self.npq);
        let log_v = log(v);
        if log_v < centre - rho {
            return true;
        }
        if log_v > centre + rho {
            return false;
        }
        let x1 = y + 1.0;
        let f1 = self.mode + 1.0;
        let z1 = self.nf + 1.0 - self.mode;
        let w1 = self.nf - y + 1.0;
        let bound = self.xm * log(f1 / x1)
            + (self.nf - self.mode + HALF) * log(z1 / w1)
            + (y - self.mode) * log(w1 * self.r / (x1 * self.q))
            + stirling(f1)
            + stirling(z1)
            + stirling(x1)
            + stirling(w1);
        log_v <= bound
    }
}

fn btpe(d: &mut Draws, n: u64, r: f64) -> u64 {
    let sampler = Btpe::new(n, r);
    loop {
        let Some((candidate, variate, certain)) = sampler.candidate(d) else {
            continue;
        };
        if (certain || sampler.accept(candidate, variate))
            && let Some(count) = floor_to_u64(candidate).filter(|count| *count <= n)
        {
            return count;
        }
    }
}

/// Binomial(n, p) conditioned on at least one success: inversion from 1 when zero is likely, rejection of zeros
/// otherwise; the normaliser 1 − P(0) is computed without cancellation.
#[clause("CHN.7")]
pub fn binomial_at_least_one(d: &mut Draws, n: u64, p: f64) -> u64 {
    check_probability(p);
    if n == 0 || p <= 0.0 {
        violation!(clause = "CHN.2", "conditioning on an impossible event", n = n);
    }
    if p >= 1.0 {
        return n;
    }
    let nf = exact(n);
    let log_q = log1p(-p);
    let log_p0 = nf * log_q;
    if exp(log_p0) <= HALF {
        loop {
            let count = binomial(d, n, p);
            if count > 0 {
                return count;
            }
        }
    }
    let norm = -expm1(log_p0);
    let ratio = p / (1.0 - p);
    let p1 = nf * p * exp((nf - 1.0) * log_q);
    loop {
        let mut target = open_unit(d) * norm;
        let mut count = 1_u64;
        let mut px = p1;
        loop {
            if target <= px {
                return count;
            }
            target -= px;
            if count == n {
                break;
            }
            count += 1;
            px *= ratio * from_u64(n - count + 1) / from_u64(count);
            if px <= 0.0 {
                break;
            }
        }
    }
}

/// Counts per value, jointly conditioned on a total of at least one: while no success has been drawn, value `v` is
/// zero with probability `z_v·(1 − r_v)/(1 − z_v·r_v)`, where `z_v` is the chance value `v` draws none and `r_v` the
/// chance every later value draws none; after the first success, plain binomials. `out` holds the suffix sums of
/// `ln z` while they are needed, so nothing is allocated.
#[clause("CHN.7")]
pub fn binomials_joint_at_least_one(d: &mut Draws, ns: &[u64], ps: &[f64], out: &mut [u64]) {
    if ns.len() != ps.len() || ns.len() != out.len() {
        violation!(clause = "CHN.2", "counts, probabilities and outputs of different lengths");
    }
    let log_z = |n: u64, p: f64| if n == 0 || p <= 0.0 { 0.0 } else { exact(n) * log1p(-p) };
    let mut suffix = 0.0_f64;
    let mut possible = false;
    for ((slot, n), p) in out.iter_mut().zip(ns).zip(ps).rev() {
        check_probability(*p);
        *slot = suffix.to_bits();
        suffix += log_z(*n, *p);
        possible |= *n > 0 && *p > 0.0;
    }
    if !possible {
        violation!(clause = "CHN.2", "a joint draw of at least one where every value has no chance");
    }
    let mut succeeded = false;
    for ((slot, n), p) in out.iter_mut().zip(ns).zip(ps) {
        if succeeded {
            *slot = binomial(d, *n, *p);
            continue;
        }
        let lz = log_z(*n, *p);
        let lr = f64::from_bits(*slot);
        let zero = exp(lz) * -expm1(lr) / -expm1(lz + lr);
        if open_unit(d) < zero {
            *slot = 0;
        } else {
            *slot = binomial_at_least_one(d, *n, *p);
            succeeded = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{binomial, binomial_at_least_one, binomials_joint_at_least_one};
    use crate::float::from_u64;
    use crate::testing::{
        chi_square_counts, chi_square_passes, draws, ln_choose, mean_within, variance_within, violation_clause,
    };

    fn pmf(n: u64, p: f64, k: u64) -> f64 {
        libm::exp(ln_choose(n, k) + from_u64(k) * libm::log(p) + from_u64(n - k) * libm::log1p(-p))
    }

    #[test]
    fn binomial_exact() {
        let cases = [(0, 0.3), (1, 0.5), (7, 0.01), (40, 0.2), (1000, 0.5), (1_000_000, 1e-5), (1_000_000_000, 0.5)];
        for (i, (n, p)) in cases.into_iter().enumerate() {
            let mut d = draws("binomial", i);
            let xs: Vec<u64> = (0..100_000).map(|_| binomial(&mut d, n, p)).collect();
            let var = from_u64(n) * p * (1.0 - p);
            // The binomial's fourth central moment, npq(1 + 3(n − 2)pq), sets the sample variance's spread.
            let mu4 = var * (1.0 + 3.0 * (from_u64(n) - 2.0) * p * (1.0 - p));
            assert!(mean_within(&xs, from_u64(n) * p, var), "mean n={n} p={p}");
            if n > 1 {
                assert!(variance_within(&xs, var, mu4), "variance n={n} p={p}");
            }
            if n <= 1000 {
                let probs: Vec<f64> = (0..=n).map(|k| pmf(n, p, k)).collect();
                assert!(chi_square_passes(&xs, &probs), "chi-square n={n} p={p}");
            }
        }
    }

    #[test]
    fn binomial_refuses_bad_probabilities() {
        let d = draws("binomial", 99);
        assert_eq!(violation_clause(|| binomial(&mut d.clone(), 3, 1.5)), "CHN.2");
        assert_eq!(violation_clause(|| binomial(&mut d.clone(), 3, f64::NAN)), "CHN.2");
        assert_eq!((binomial(&mut d.clone(), 5, 0.0), binomial(&mut d.clone(), 5, 1.0)), (0, 5));
    }

    #[test]
    fn binomial_at_least_one_never_zero_and_exact() {
        // (5, .01) and (40, .02) invert from one; (40, .2) and (1000, .5) reject zeros.
        for (i, (n, p)) in [(5_u64, 0.01), (40, 0.02), (40, 0.2), (1000, 0.5)].into_iter().enumerate() {
            let mut d = draws("at_least_one", i);
            let xs: Vec<u64> = (0..100_000).map(|_| binomial_at_least_one(&mut d, n, p)).collect();
            assert!(xs.iter().all(|x| *x >= 1));
            let p0 = pmf(n, p, 0);
            let probs: Vec<f64> = (0..=n).map(|k| if k == 0 { 0.0 } else { pmf(n, p, k) / (1.0 - p0) }).collect();
            assert!(chi_square_passes(&xs, &probs), "n={n} p={p}");
        }
        let mut d = draws("at_least_one", 9);
        // P(X ≥ 2 | X ≥ 1) is about 4.5e-12 at n = 10, p = 1e-12: every one of 10⁴ draws is one.
        assert!((0..10_000).all(|_| binomial_at_least_one(&mut d, 10, 1e-12) == 1));
    }

    #[test]
    fn joint_at_least_one_matches_conditioned_product() {
        let (ns, ps) = ([3_u64, 5, 2], [0.01, 0.2, 0.05]);
        let cell = |a: u64, b: u64, c: u64| usize::try_from(a * 18 + b * 3 + c).unwrap();
        let mut d = draws("joint", 0);
        let mut counts = vec![0_u64; 72];
        let mut out = [0_u64; 3];
        for _ in 0..1_000_000 {
            binomials_joint_at_least_one(&mut d, &ns, &ps, &mut out);
            assert!(out.iter().sum::<u64>() >= 1);
            counts[cell(out[0], out[1], out[2])] += 1;
        }
        let p_none: f64 = ns.iter().zip(ps).map(|(n, p)| pmf(*n, p, 0)).product();
        let mut probs = vec![0.0; 72];
        for a in 0..=3 {
            for b in 0..=5 {
                for c in 0..=2 {
                    let joint = pmf(3, 0.01, a) * pmf(5, 0.2, b) * pmf(2, 0.05, c);
                    probs[cell(a, b, c)] = if a + b + c == 0 { 0.0 } else { joint / (1.0 - p_none) };
                }
            }
        }
        assert!(chi_square_counts(&counts, &probs));
    }

    #[test]
    fn joint_at_least_one_refuses_impossible() {
        let mut d = draws("joint", 1);
        let mut out = [0_u64; 2];
        let clause = violation_clause(move || binomials_joint_at_least_one(&mut d, &[0, 4], &[0.5, 0.0], &mut out));
        assert_eq!(clause, "CHN.2");
    }
}
