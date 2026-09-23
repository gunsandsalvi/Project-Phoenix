use phx_macros::clause;
use phx_num::violation;

use crate::consts::{HALF, UNIT_BITS, UNIT_SCALE};
use crate::draws::Draws;
use crate::float::from_u64;

/// A 64-bit word as (k + 1/2)·2^-52 for its top 52 bits: never 0 and never 1, so every logarithm is finite.
#[must_use]
pub fn unit_from_bits(word: u64) -> f64 {
    (from_u64(word >> (u64::BITS - UNIT_BITS)) + HALF) * UNIT_SCALE
}

/// A uniform on the open interval (0, 1).
#[clause("CHN.7")]
pub fn open_unit(d: &mut Draws) -> f64 {
    unit_from_bits(d.next_u64())
}

/// The high and low words of a 128-bit product, from its bytes.
fn split_u128(v: u128) -> (u64, u64) {
    let [l0, l1, l2, l3, l4, l5, l6, l7, h0, h1, h2, h3, h4, h5, h6, h7] = v.to_le_bytes();
    (u64::from_le_bytes([h0, h1, h2, h3, h4, h5, h6, h7]), u64::from_le_bytes([l0, l1, l2, l3, l4, l5, l6, l7]))
}

/// A uniform integer in [0, n) by Lemire's multiply-shift, rejecting the low products that would bias it; the
/// threshold is 2^64 mod n, computed without wrapping.
#[clause("CHN.7")]
pub fn below_u64(d: &mut Draws, n: u64) -> u64 {
    if n == 0 {
        violation!(clause = "CHN.2", "a uniform draw from an empty range");
    }
    let (mut high, mut low) = split_u128(u128::from(d.next_u64()) * u128::from(n));
    if low < n {
        let threshold = (u64::MAX % n + 1) % n;
        while low < threshold {
            (high, low) = split_u128(u128::from(d.next_u64()) * u128::from(n));
        }
    }
    high
}

/// The 32-bit form of `below_u64`.
pub fn below_u32(d: &mut Draws, n: u32) -> u32 {
    if n == 0 {
        violation!(clause = "CHN.2", "a uniform draw from an empty range");
    }
    let split = |w: u32| {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = (u64::from(w) * u64::from(n)).to_le_bytes();
        (u32::from_le_bytes([b4, b5, b6, b7]), u32::from_le_bytes([b0, b1, b2, b3]))
    };
    let (mut high, mut low) = split(d.next_u32());
    if low < n {
        let threshold = (u32::MAX % n + 1) % n;
        while low < threshold {
            (high, low) = split(d.next_u32());
        }
    }
    high
}

#[cfg(test)]
mod tests {
    use super::{below_u32, below_u64, split_u128, unit_from_bits};
    use crate::draws::Draws;
    use crate::key::{Seed, Subject, SubjectTag, stream_key};

    #[test]
    fn open_unit_never_zero_or_one() {
        for w in [0, 1, 1 << 12, u64::MAX >> 1, u64::MAX - 1, u64::MAX] {
            let u = unit_from_bits(w);
            assert!(u > 0.0 && u < 1.0, "{w}: {u}");
        }
        assert!(unit_from_bits(u64::MAX) < 1.0);
    }

    #[test]
    fn below_exhaustive_u8() {
        for n in 1..=255_u16 {
            let threshold = (255 % n + 1) % n;
            let mut counts = vec![0_u16; usize::from(n)];
            for x in 0..=255_u16 {
                let m = x * n;
                if m & 0xff >= threshold {
                    counts[usize::from(m >> 8)] += 1;
                }
            }
            assert!(counts.iter().all(|c| *c == 256 / n), "n={n}");
        }
    }

    #[test]
    fn below_stays_in_range_and_split_is_exact() {
        let mut d = Draws::new(stream_key(Seed::new(5), "test"), Subject::new(SubjectTag::World, 0), 0, 0);
        for n in [1_u64, 2, 3, 7, 1 << 40, u64::MAX] {
            for _ in 0..200 {
                assert!(below_u64(&mut d, n) < n);
            }
        }
        for n in [1_u32, 3, u32::MAX] {
            assert!(below_u32(&mut d, n) < n);
        }
        assert_eq!(split_u128((7_u128 << 64) | 9), (7, 9));
    }
}
