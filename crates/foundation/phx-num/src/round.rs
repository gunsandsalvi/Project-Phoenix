use std::cmp::Ordering;

use phx_macros::clause;

use crate::qty::Count;
use crate::violation;

/// The side of a payment a rounding may favour.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Payer,
    Payee,
}

/// A rounding convention, always read from a contract, a law or a unit's declaration.
#[clause("MON.16")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    HalfEven,
    HalfAwayFromZero,
    TowardZero,
    Floor,
    Ceil,
    /// For an amount owed by payer to payee: `Payee` rounds up in magnitude, `Payer` down.
    InFavourOf(Side),
}

/// `n / d` rounded once by `r`, for every sign of `n`.
#[clause("Law 7")]
#[must_use]
pub fn div_round(n: i128, d: i128, r: Round) -> i128 {
    rounded(n, d, r)
}

/// The signed integers a rounded division runs in: a word where the product fits one, two otherwise.
trait Signed: Copy + Ord + crate::violation::Key + core::ops::Sub<Output = Self> {
    const ZERO: Self;
    const ONE: Self;
    const TWO: Self;
    fn div_euclid(self, d: Self) -> Self;
    fn rem_euclid(self, d: Self) -> Self;
    fn checked_add(self, d: Self) -> Option<Self>;
}

macro_rules! signed {
    ($t:ty) => {
        impl Signed for $t {
            const ZERO: $t = 0;
            const ONE: $t = 1;
            const TWO: $t = 2;
            fn div_euclid(self, d: $t) -> $t {
                <$t>::div_euclid(self, d)
            }
            fn rem_euclid(self, d: $t) -> $t {
                <$t>::rem_euclid(self, d)
            }
            fn checked_add(self, d: $t) -> Option<$t> {
                <$t>::checked_add(self, d)
            }
        }
    };
}
signed!(i64);
signed!(i128);

fn rounded<T: Signed>(n: T, d: T, r: Round) -> T {
    if d <= T::ZERO {
        violation!(clause = "Law 7", "a rounded division needs a positive divisor", n = n, d = d);
    }
    let floor = n.div_euclid(d);
    let rem = n.rem_euclid(d);
    if rem == T::ZERO {
        return floor;
    }
    let up = match r {
        Round::Floor => false,
        Round::Ceil => true,
        Round::TowardZero | Round::InFavourOf(Side::Payer) => n < T::ZERO,
        Round::InFavourOf(Side::Payee) => n > T::ZERO,
        Round::HalfAwayFromZero | Round::HalfEven => match rem.cmp(&(d - rem)) {
            Ordering::Less => false,
            Ordering::Greater => true,
            Ordering::Equal if r == Round::HalfAwayFromZero => n > T::ZERO,
            Ordering::Equal => floor.rem_euclid(T::TWO) != T::ZERO,
        },
    };
    if !up {
        return floor;
    }
    floor.checked_add(T::ONE).unwrap_or_else(|| violation!(clause = "Law 7", "a rounded quotient overflows", n = n))
}

/// A total split between those leaving and those staying, `leaving = round(k·total/w)`; the two always sum to the
/// total, so no unit is lost or made.
#[clause("REP.9")]
#[must_use]
pub fn split_total(total: i64, k: u64, w: u64, r: Round) -> (i64, i64) {
    if w == 0 || k > w {
        violation!(clause = "REP.9", "a split takes a share of a positive whole", k = k, w = w);
    }
    // In a word when the product fits one, which is the same division.
    let product = i64::try_from(k).ok().and_then(|k| total.checked_mul(k));
    let leaving = match (product, i64::try_from(w)) {
        (Some(p), Ok(w)) => i128::from(rounded(p, w, r)),
        _ => div_round(i128::from(total) * i128::from(k), i128::from(w), r),
    };
    let Ok(leaving) = i64::try_from(leaving) else {
        violation!(clause = "Law 7", "a share of a total exceeds the total", total = total);
    };
    let Some(staying) = total.checked_sub(leaving) else {
        violation!(clause = "Law 7", "a split's remainder overflows", total = total, leaving = leaving);
    };
    (leaving, staying)
}

/// A per-member amount times a count of members, exact or stopping the run.
#[clause("REP.9")]
#[must_use]
pub fn per_member_times_count(per_member: i64, count: Count) -> i64 {
    let product = i128::from(per_member) * i128::from(count.get());
    let Ok(total) = i64::try_from(product) else {
        violation!(clause = "Law 7", "a per-member amount times its count overflows", per_member = per_member);
    };
    total
}

#[cfg(test)]
mod tests {
    use super::{Round, Side, div_round, per_member_times_count, split_total};
    use crate::qty::Count;
    use crate::violation::testing::violated_clause;

    const ALL: [Round; 7] = [
        Round::HalfEven,
        Round::HalfAwayFromZero,
        Round::TowardZero,
        Round::Floor,
        Round::Ceil,
        Round::InFavourOf(Side::Payer),
        Round::InFavourOf(Side::Payee),
    ];

    /// The definition, by search: the two integers around n/d, and the rule's choice between them.
    fn reference(n: i128, d: i128, r: Round) -> i128 {
        let lo = (-60..=60).rfind(|k| k * d <= n).unwrap();
        let hi = if lo * d == n { lo } else { lo + 1 };
        if lo == hi {
            return lo;
        }
        let (below, above) = (n - lo * d, hi * d - n);
        let nearest = if below < above { lo } else { hi };
        match r {
            Round::Floor => lo,
            Round::Ceil => hi,
            Round::TowardZero | Round::InFavourOf(Side::Payer) => {
                if n > 0 {
                    lo
                } else {
                    hi
                }
            }
            Round::InFavourOf(Side::Payee) => {
                if n > 0 {
                    hi
                } else {
                    lo
                }
            }
            Round::HalfAwayFromZero if below == above => {
                if n > 0 {
                    hi
                } else {
                    lo
                }
            }
            Round::HalfEven if below == above => {
                if lo % 2 == 0 {
                    lo
                } else {
                    hi
                }
            }
            Round::HalfAwayFromZero | Round::HalfEven => nearest,
        }
    }

    #[test]
    fn div_round_matches_definition() {
        for n in -50..=50 {
            for d in 1..=7 {
                for r in ALL {
                    assert_eq!(div_round(n, d, r), reference(n, d, r), "n={n} d={d} {r:?}");
                }
            }
        }
    }

    #[test]
    fn div_round_refuses_non_positive_divisor() {
        assert_eq!(violated_clause(|| div_round(1, 0, Round::Floor)), "Law 7");
        assert_eq!(violated_clause(|| div_round(1, -3, Round::Floor)), "Law 7");
    }

    #[test]
    fn round_in_favour_of_payee_rounds_up_magnitude() {
        assert_eq!(div_round(7, 2, Round::InFavourOf(Side::Payee)), 4);
        assert_eq!(div_round(-7, 2, Round::InFavourOf(Side::Payee)), -4);
        assert_eq!(div_round(7, 2, Round::InFavourOf(Side::Payer)), 3);
        assert_eq!(div_round(-7, 2, Round::InFavourOf(Side::Payer)), -3);
    }

    #[test]
    fn split_total_conserves() {
        for total in [-1_000_003_i64, -97, -1, 0, 1, 5, 96, 1_000_003] {
            for w in 1..=97_u64 {
                for k in 0..=w {
                    for r in ALL {
                        let (leaving, staying) = split_total(total, k, w, r);
                        assert_eq!(leaving + staying, total);
                        let exact = i128::from(total) * i128::from(k);
                        let gap = i128::from(leaving) * i128::from(w) - exact;
                        assert!(gap.abs() < i128::from(w), "total={total} k={k} w={w} {r:?}");
                    }
                }
            }
        }
        assert_eq!(violated_clause(|| split_total(10, 3, 2, Round::Floor)), "REP.9");
        assert_eq!(violated_clause(|| split_total(10, 0, 0, Round::Floor)), "REP.9");
    }

    #[test]
    fn per_member_times_count_is_checked() {
        assert_eq!(per_member_times_count(-12, Count::new(5)), -60);
        assert_eq!(violated_clause(|| per_member_times_count(i64::MAX, Count::new(2))), "Law 7");
    }

    #[test]
    fn split_total_in_a_word_matches_wide_arithmetic() {
        // Totals and counts spread over many magnitudes, from small to where the product needs two words.
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..50_000 {
            let bits = u32::try_from(next() % 62 + 1).unwrap();
            let total = i64::try_from(next() >> (64 - bits)).unwrap() * if next() % 2 == 0 { 1 } else { -1 };
            let w = next() % 1_000_000 + 1;
            let k = next() % (w + 1);
            for r in ALL {
                let wide = div_round(i128::from(total) * i128::from(k), i128::from(w), r);
                let (leaving, staying) = split_total(total, k, w, r);
                assert_eq!(i128::from(leaving), wide, "total={total} k={k} w={w} {r:?}");
                assert_eq!(leaving + staying, total);
            }
        }
    }
}
