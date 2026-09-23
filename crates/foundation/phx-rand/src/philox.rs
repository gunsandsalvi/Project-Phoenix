use phx_macros::clause;

use crate::consts::{BLOCK_WORDS, PHILOX_M0, PHILOX_M1, PHILOX_ROUNDS, PHILOX_W0, PHILOX_W1};

/// The high and low words of a 32×32-bit product, split from its bytes so neither half can be truncated.
#[inline]
fn mul_hi_lo(a: u32, b: u32) -> (u32, u32) {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = (u64::from(a) * u64::from(b)).to_le_bytes();
    (u32::from_le_bytes([b4, b5, b6, b7]), u32::from_le_bytes([b0, b1, b2, b3]))
}

/// Philox's key schedule: each word advances by its Weyl constant, modulo 2^32 by design.
#[inline]
fn bump(key: [u32; 2]) -> [u32; 2] {
    let [k0, k1] = key;
    [k0.wrapping_add(PHILOX_W0), k1.wrapping_add(PHILOX_W1)]
}

#[inline]
fn round(c: [u32; 4], key: [u32; 2]) -> [u32; 4] {
    let [c0, c1, c2, c3] = c;
    let [k0, k1] = key;
    let (hi0, lo0) = mul_hi_lo(PHILOX_M0, c0);
    let (hi1, lo1) = mul_hi_lo(PHILOX_M1, c2);
    [hi1 ^ c1 ^ k0, lo1, hi0 ^ c3 ^ k1, lo0]
}

/// Philox4x32-10: four pseudo-random words that depend only on the counter and the key.
#[clause("CHN.1", "CHN.6")]
#[must_use]
pub fn philox(ctr: [u32; 4], key: [u32; 2]) -> [u32; 4] {
    let mut c = ctr;
    let mut k = key;
    for r in 0..PHILOX_ROUNDS {
        if r > 0 {
            k = bump(k);
        }
        c = round(c, k);
    }
    c
}

/// Applies `f` lane by lane over four-lane arrays.
#[inline]
fn lanes<A: Copy, B: Copy>(
    a: [A; BLOCK_WORDS],
    b: [A; BLOCK_WORDS],
    f: impl Fn(A, A) -> B,
    zero: B,
) -> [B; BLOCK_WORDS] {
    let mut out = [zero; BLOCK_WORDS];
    for ((o, x), y) in out.iter_mut().zip(a).zip(b) {
        *o = f(x, y);
    }
    out
}

/// Four Philox blocks at once, held word by word across the four counters, so each round is the same operation on
/// four lanes and the compiler can keep each word in one vector register.
#[must_use]
pub fn philox_x4(ctrs: [[u32; 4]; BLOCK_WORDS], key: [u32; 2]) -> [[u32; 4]; BLOCK_WORDS] {
    let mut c0 = ctrs.map(|[w, _, _, _]| w);
    let mut c1 = ctrs.map(|[_, w, _, _]| w);
    let mut c2 = ctrs.map(|[_, _, w, _]| w);
    let mut c3 = ctrs.map(|[_, _, _, w]| w);
    let mut k = key;
    for r in 0..PHILOX_ROUNDS {
        if r > 0 {
            k = bump(k);
        }
        let [k0, k1] = k;
        let p0 = c0.map(|x| mul_hi_lo(PHILOX_M0, x));
        let p2 = c2.map(|x| mul_hi_lo(PHILOX_M1, x));
        let next0 = lanes(p2.map(|(hi, _)| hi), c1, |hi1, w| hi1 ^ w ^ k0, 0);
        let next2 = lanes(p0.map(|(hi, _)| hi), c3, |hi0, w| hi0 ^ w ^ k1, 0);
        c1 = p2.map(|(_, lo)| lo);
        c3 = p0.map(|(_, lo)| lo);
        c0 = next0;
        c2 = next2;
    }
    let mut out = [[0_u32; 4]; BLOCK_WORDS];
    for ((((block, w0), w1), w2), w3) in out.iter_mut().zip(c0).zip(c1).zip(c2).zip(c3) {
        *block = [w0, w1, w2, w3];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{philox, philox_x4};

    #[test]
    fn philox_known_answer() {
        assert_eq!(philox([0; 4], [0; 2]), [0x6627_e8d5, 0xe169_c58d, 0xbc57_ac4c, 0x9b00_dbd8]);
        assert_eq!(philox([u32::MAX; 4], [u32::MAX; 2]), [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]);
        assert_eq!(
            philox([0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344], [0xa409_3822, 0x299f_31d0]),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }

    #[test]
    fn philox_x4_equals_scalar() {
        let key = [0x1234_5678, 0x9abc_def0];
        for i in 0..2_500_u32 {
            let ctrs = [[i, 1, 2, 3], [i, i, 7, 9], [3, i, 0, u32::MAX], [i ^ 0xffff, 5, i, 1]];
            let batch = philox_x4(ctrs, key);
            for (ctr, out) in ctrs.iter().zip(batch) {
                assert_eq!(out, philox(*ctr, key));
            }
        }
    }
}
