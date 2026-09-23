use phx_num::violation;

use crate::consts::{RADIX_BITS, RADIX_BUCKETS, RADIX_CHUNK, RADIX_MIN};
use crate::convert::to_usize;
use crate::pool::{Pool, each, map};

const DIGIT_MASK: u16 = (1 << RADIX_BITS) - 1;

/// A key a radix sort orders by and a keyed reduction shards by.
pub trait RadixKey: Copy + Ord + Send + Sync {
    const BITS: u32;
    /// The `RADIX_BITS` bits of the key from bit `shift` up.
    fn digit(self, shift: u32) -> usize;
    /// The key folded to one word, which the shard function mixes.
    fn word(self) -> u64;
}

fn low_digit(bytes: [u8; 2]) -> usize {
    usize::from(u16::from_le_bytes(bytes) & DIGIT_MASK)
}

impl RadixKey for u32 {
    const BITS: u32 = u32::BITS;
    fn digit(self, shift: u32) -> usize {
        let [a, b, ..] = (self >> shift).to_le_bytes();
        low_digit([a, b])
    }
    fn word(self) -> u64 {
        u64::from(self)
    }
}

impl RadixKey for u64 {
    const BITS: u32 = u64::BITS;
    fn digit(self, shift: u32) -> usize {
        let [a, b, ..] = (self >> shift).to_le_bytes();
        low_digit([a, b])
    }
    fn word(self) -> u64 {
        self
    }
}

impl RadixKey for u128 {
    const BITS: u32 = u128::BITS;
    fn digit(self, shift: u32) -> usize {
        let [a, b, ..] = (self >> shift).to_le_bytes();
        low_digit([a, b])
    }
    fn word(self) -> u64 {
        let [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15] = self.to_le_bytes();
        u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7])
            ^ u64::from_le_bytes([b8, b9, b10, b11, b12, b13, b14, b15])
    }
}

fn count(items: &[(impl RadixKey, u32)], shift: u32) -> [u32; RADIX_BUCKETS] {
    let mut counts = [0_u32; RADIX_BUCKETS];
    for (k, _) in items {
        if let Some(c) = counts.get_mut(k.digit(shift)) {
            *c += 1;
        }
    }
    counts
}

/// Sorts (key, index) pairs by key, stably, least significant digit first, with digits of `RADIX_BITS` bits; a digit
/// every key shares is skipped. Pieces of `RADIX_CHUNK` pairs are counted and scattered in parallel when a pool is
/// given, each into its own run of every bucket, so the result is the same with any number of workers.
pub fn radix_sort<K: RadixKey>(pool: Option<&Pool>, items: &mut Vec<(K, u32)>, scratch: &mut Vec<(K, u32)>) {
    let Some(&first) = items.first() else { return };
    if items.len() < RADIX_MIN {
        items.sort_by_key(|(k, _)| *k);
        return;
    }
    let passes = K::BITS.div_ceil(RADIX_BITS);
    let pieces = items.len().div_ceil(RADIX_CHUNK);
    for pass in 0..passes {
        let shift = pass * RADIX_BITS;
        let counts = map(pool, pieces, |p| {
            items.chunks(RADIX_CHUNK).nth(p).map_or([0; RADIX_BUCKETS], |piece| count(piece, shift))
        });
        // A digit every key shares leaves the order as it is.
        let first_digit = first.0.digit(shift);
        if counts.iter().all(|c| c.iter().enumerate().all(|(d, n)| d == first_digit || *n == 0)) {
            continue;
        }
        // Every slot is overwritten by the scatter, so the scratch is only sized, never cleared.
        if scratch.len() != items.len() {
            scratch.clear();
            scratch.resize(items.len(), first);
        }
        // Each piece's run of each bucket, bucket-major and piece-minor, so equal digits keep their order.
        let mut runs: Vec<Vec<&mut [(K, u32)]>> = (0..pieces).map(|_| Vec::with_capacity(RADIX_BUCKETS)).collect();
        let mut rest = scratch.as_mut_slice();
        for bucket in 0..RADIX_BUCKETS {
            for (piece, piece_runs) in counts.iter().zip(runs.iter_mut()) {
                let n = piece.get(bucket).map_or(0, |c| to_usize(*c));
                let (run, tail) = std::mem::take(&mut rest).split_at_mut(n);
                piece_runs.push(run);
                rest = tail;
            }
        }
        let work: Vec<_> = items.chunks(RADIX_CHUNK).zip(runs).collect();
        each(pool, work, |(piece, mut piece_runs)| {
            let mut at = [0_usize; RADIX_BUCKETS];
            for item in piece {
                let d = item.0.digit(shift);
                let (Some(run), Some(next)) = (piece_runs.get_mut(d), at.get_mut(d)) else {
                    violation!(clause = "TIME.6", "a radix digit outside its buckets", digit = d);
                };
                let Some(slot) = run.get_mut(*next) else {
                    violation!(clause = "TIME.6", "a radix bucket fuller than its count", digit = d);
                };
                *slot = *item;
                *next += 1;
            }
        });
        std::mem::swap(items, scratch);
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::radix_sort;
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    fn random_keys(n: usize, bound: u64) -> Vec<(u64, u32)> {
        let mut d = Draws::new(stream_key(Seed::new(1), "radix"), Subject::new(SubjectTag::World, 0), 0, 0);
        (0..n).map(|i| (below_u64(&mut d, bound), u32::try_from(i).unwrap())).collect()
    }

    #[test]
    fn radix_sorts_stably() {
        let pool = Pool::new(&PoolSpec::unpinned(4)).unwrap();
        for bound in [u64::MAX, 1 << 20, 7, 1] {
            let original = random_keys(300_000, bound);
            let mut expected = original.clone();
            expected.sort_by_key(|(k, _)| *k);
            for p in [None, Some(&pool)] {
                let (mut items, mut scratch) = (original.clone(), Vec::new());
                radix_sort(p, &mut items, &mut scratch);
                assert_eq!(items, expected, "bound={bound}");
            }
        }
        // Keys that differ only in their top digit, and 128-bit keys.
        let mut high: Vec<(u64, u32)> = (0..1000).map(|i| ((999 - u64::from(i)) << 60, i)).collect();
        radix_sort(Some(&pool), &mut high, &mut Vec::new());
        assert!(high.windows(2).all(|w| w[0].0 <= w[1].0));
        let mut wide: Vec<(u128, u32)> = (0..1000).map(|i| (u128::from(i % 7) << 100 | u128::from(i), i)).collect();
        radix_sort(Some(&pool), &mut wide, &mut Vec::new());
        assert!(wide.windows(2).all(|w| w[0].0 <= w[1].0));
    }
}
