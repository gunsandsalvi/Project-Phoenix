//! Heuristic switching (Brock and Hommes, 1997): each heuristic's recent squared error for the party, and the shares
//! a discrete choice over them gives at the party's switching intensity.

use libm::exp;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::float::{from_u64, len_u64};

/// A heuristic's performance after an error, in the method's widths so the switching intensity reads alike for every
/// series: the exponentially weighted squared error at the party's memory speed; the first error, with no record yet,
/// is the record.
#[clause("VAL.3", "VAL.7")]
#[must_use]
pub fn performance(previous: Missing<f64>, error: f64, lambda: f64) -> f64 {
    let squared = error * error;
    match previous {
        Missing::Present(p) => p + lambda * (squared - p),
        Missing::Absent => squared,
    }
}

/// The share choosing each heuristic, `exp(−beta·perf_h) ÷ Σ exp(−beta·perf_j)`, written into `out`. With no record
/// yet for any heuristic nothing favours one, so the shares are equal and the members' tastes alone decide; records
/// kept for some heuristics of a method and not others are an impossible state, since every heuristic of a method
/// is scored on the same prints.
#[clause("VAL.7", "REP.22")]
pub fn shares(perf: &[Missing<f64>], beta: f64, out: &mut [f64]) {
    if perf.len() != out.len() || perf.is_empty() {
        violation!(clause = "VAL.7", "shares asked for a different number of heuristics than scored", n = perf.len());
    }
    let present = perf.iter().filter(|p| matches!(p, Missing::Present(_))).count();
    if present == 0 {
        let equal = 1.0 / from_u64(len_u64(out.len()));
        for s in out.iter_mut() {
            *s = equal;
        }
        return;
    }
    if present != perf.len() {
        violation!(clause = "VAL.7", "a method scored on some of its heuristics only", scored = present);
    }
    // The least error scales every weight, so the best heuristic's weight is one and none underflows to nothing.
    let mut best = f64::INFINITY;
    for p in perf {
        if let Missing::Present(v) = p
            && *v < best
        {
            best = *v;
        }
    }
    let mut total = 0.0;
    for (s, p) in out.iter_mut().zip(perf) {
        if let Missing::Present(v) = p {
            *s = exp(-beta * (v - best));
            total += *s;
        }
    }
    for s in out.iter_mut() {
        *s /= total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_shares_logit() {
        let perf = [Missing::Present(1.0), Missing::Present(2.0), Missing::Present(1.0)];
        let mut out = [0.0; 3];
        shares(&perf, 1.0, &mut out);
        let e = (-1.0_f64).exp();
        let z = 2.0 + e;
        for (got, want) in out.iter().zip([1.0 / z, e / z, 1.0 / z]) {
            assert!((got - want).abs() < 1e-12);
        }
        // No intensity, no switching toward the better; a high one puts nearly all on the best.
        shares(&perf, 0.0, &mut out);
        assert!(out.iter().all(|s| (s - 1.0 / 3.0).abs() < 1e-12));
        shares(&[Missing::Present(0.0), Missing::Present(100.0)], 1.0, &mut out[..2]);
        assert!(out[0] > 0.999_999);
        // Errors far from zero lose nothing to underflow.
        shares(&[Missing::Present(1e6), Missing::Present(1e6 + 1.0)], 1.0, &mut out[..2]);
        assert!((out[0] - 1.0 / (1.0 + e)).abs() < 1e-12);
    }

    #[test]
    fn no_record_leaves_the_choice_to_taste() {
        let mut out = [0.0; 4];
        shares(&[Missing::Absent; 4], 5.0, &mut out);
        assert!(out.iter().all(|s| (s - 0.25).abs() < f64::EPSILON));
        assert_eq!(
            crate::testing::refused(|| shares(&[Missing::Absent, Missing::Present(1.0)], 1.0, &mut [0.0; 2])),
            Some("VAL.7")
        );
    }

    #[test]
    fn performance_ewma_of_squared_error() {
        assert!((performance(Missing::Absent, -2.0, 0.5) - 4.0).abs() < f64::EPSILON);
        assert!((performance(Missing::Present(4.0), 0.0, 0.25) - 3.0).abs() < f64::EPSILON);
    }
}
