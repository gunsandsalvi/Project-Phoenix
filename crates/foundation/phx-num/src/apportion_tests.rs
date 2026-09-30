//! Apportionment over hand-built weights: shares sum to the total under every rule, remainders and ties fall as
//! declared, and splits that name no holder stop.
#![cfg(test)]

use super::{Residue, Ties, apportion};
use crate::round::{Round, Side};
use crate::violation::testing::violated_clause;

fn split(total: i64, weights: &[u64], rule: Residue<'_>) -> Vec<i64> {
    let mut out = vec![0; weights.len()];
    apportion(total, weights, rule, &mut out);
    out
}

const ORDER: Residue<'static> = Residue::LargestRemainder { ties: Ties::Order };

/// The largest-remainder shares by sorting, the reference the selection must equal: floors, then a unit to each of
/// the first `left` by (remainder descending, key ascending, index ascending).
fn by_sorting(total: i64, weights: &[u64], keys: Option<&[u64]>) -> Vec<i64> {
    let t = i128::from(total.unsigned_abs());
    let whole: i128 = weights.iter().map(|w| i128::from(*w)).sum();
    let mut out: Vec<i128> = weights.iter().map(|w| t * i128::from(*w) / whole).collect();
    let left = usize::try_from(t - out.iter().sum::<i128>()).unwrap();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| {
        let rem = t * i128::from(weights[i]) % whole;
        (std::cmp::Reverse(rem), keys.map_or(0, |k| k[i]), i)
    });
    for &i in &order[..left] {
        out[i] += 1;
    }
    out.into_iter().map(|s| i64::try_from(if total < 0 { -s } else { s }).unwrap()).collect()
}

/// Every weight set of up to four claimants of weights 0–3 with a weight somewhere.
fn weight_sets() -> Vec<Vec<u64>> {
    let mut sets = Vec::new();
    for n in 1..=4_u32 {
        for code in 0..4_u64.pow(n) {
            let set: Vec<u64> = (0..n).map(|i| code / 4_u64.pow(i) % 4).collect();
            if set.iter().any(|w| *w > 0) {
                sets.push(set);
            }
        }
    }
    sets
}

#[test]
fn shares_sum_to_total() {
    let rounds = [
        Round::HalfEven,
        Round::HalfAwayFromZero,
        Round::TowardZero,
        Round::Floor,
        Round::Ceil,
        Round::InFavourOf(Side::Payer),
        Round::InFavourOf(Side::Payee),
    ];
    for weights in weight_sets() {
        let keys: Vec<u64> = (0_u64..).take(weights.len()).map(|i| (i * 7 + 3) % 5).collect();
        for total in -25..=25 {
            let ordered = split(total, &weights, ORDER);
            assert_eq!(ordered.iter().sum::<i64>(), total, "{weights:?} {total}");
            assert_eq!(ordered, by_sorting(total, &weights, None), "{weights:?} {total}");
            let keyed = split(total, &weights, Residue::LargestRemainder { ties: Ties::Keys(&keys) });
            assert_eq!(keyed, by_sorting(total, &weights, Some(&keys)), "{weights:?} {total} {keys:?}");
            for index in 0..weights.len() {
                for round in rounds {
                    let named = split(total, &weights, Residue::To { index, round });
                    assert_eq!(named.iter().sum::<i64>(), total, "{weights:?} {total} {index} {round:?}");
                }
            }
        }
    }
}

#[test]
fn many_claimants_match_sorting() {
    // Weights and keys from a fixed recurrence, over claimants enough that the selection takes several digits.
    let mut x = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for (n, span) in [(10_usize, 1_000_u64), (1_000, 1 << 40), (100_000, 97), (5_000, 3)] {
        let weights: Vec<u64> = (0..n).map(|_| next() % span).collect();
        let keys: Vec<u64> = (0..n).map(|_| next() % 16).collect();
        for total in [1_i64, 999_999_937, -123_456_789_012, i64::MAX] {
            assert_eq!(split(total, &weights, ORDER), by_sorting(total, &weights, None), "{n} {total}");
            let keyed = split(total, &weights, Residue::LargestRemainder { ties: Ties::Keys(&keys) });
            assert_eq!(keyed, by_sorting(total, &weights, Some(&keys)), "{n} {total}");
        }
    }
}

#[test]
fn largest_remainder_matches_hand_example() {
    // 7 over 5:3:2 is 3.5, 2.1 and 1.4: floors 3, 2, 1, and the one unit left to the largest remainder, the first.
    assert_eq!(split(7, &[5, 3, 2], ORDER), vec![4, 2, 1]);
    // 100 over 1:1:1 is 33⅓ each: the one unit left goes by the declared order.
    assert_eq!(split(100, &[1, 1, 1], ORDER), vec![34, 33, 33]);
}

#[test]
fn ties_by_order() {
    assert_eq!(split(5, &[1, 1, 1, 1], ORDER), vec![2, 1, 1, 1]);
    assert_eq!(split(6, &[1, 1, 1, 1], ORDER), vec![2, 2, 1, 1]);
}

#[test]
fn ties_by_keys() {
    let keys = [9, 2, 5, 2];
    let rule = Residue::LargestRemainder { ties: Ties::Keys(&keys) };
    // Two units among four equal remainders: the lowest keys, 2 and 2, the second and fourth claimants.
    assert_eq!(split(6, &[1, 1, 1, 1], rule), vec![1, 2, 1, 2]);
    // Three: then the key 5.
    assert_eq!(split(7, &[1, 1, 1, 1], rule), vec![1, 2, 2, 2]);
    // A larger remainder still comes before any key.
    assert_eq!(split(7, &[3, 1, 1, 1], rule), vec![4, 1, 1, 1]);
}

#[test]
fn residue_to_named_index() {
    // Each share rounded down and the residue on the third claimant, whatever its own weight.
    assert_eq!(split(10, &[1, 1, 1], Residue::To { index: 2, round: Round::Floor }), vec![3, 3, 4]);
    assert_eq!(split(10, &[1, 1, 0], Residue::To { index: 2, round: Round::Floor }), vec![5, 5, 0]);
    assert_eq!(split(11, &[1, 1, 0], Residue::To { index: 2, round: Round::Floor }), vec![5, 5, 1]);
}

#[test]
fn negative_total_mirrors() {
    for weights in weight_sets() {
        for total in 1..=25 {
            let up = split(total, &weights, ORDER);
            let down: Vec<i64> = split(-total, &weights, ORDER).into_iter().map(|s| -s).collect();
            assert_eq!(up, down, "{weights:?} {total}");
        }
    }
}

#[test]
fn empty_split_refused() {
    assert_eq!(violated_clause(|| split(10, &[], ORDER)), "MON.16");
    assert_eq!(violated_clause(|| split(10, &[0, 0], ORDER)), "MON.16");
    assert_eq!(violated_clause(|| split(10, &[1, 1], Residue::To { index: 2, round: Round::Floor })), "MON.16");
    assert_eq!(violated_clause(|| split(10, &[1, 1], Residue::LargestRemainder { ties: Ties::Keys(&[1]) })), "MON.16");
}

#[test]
fn weights_overflow_stops() {
    assert_eq!(violated_clause(|| split(10, &[u64::MAX, 1], ORDER)), "MON.16");
    assert_eq!(violated_clause(|| split(i64::MIN, &[1], ORDER)), "MON.16");
}
