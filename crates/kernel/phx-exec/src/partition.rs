#![expect(unsafe_code, reason = "each piece writes its disjoint positions of one output in parallel")]

//! A stable partition of many buffers' items into buckets: the day's flows grouped by the range of parties that owns
//! each, so each range's worker reads only its own. Pieces are fixed by the inputs' lengths, counted and scattered in
//! parallel into disjoint positions, so the result is the same whatever the workers.

use std::mem::MaybeUninit;

use phx_num::violation;

use crate::consts::RADIX_CHUNK;
use crate::pool::{Pool, map};

/// Items grouped by bucket, each bucket's items in input order, with where each bucket starts; kept across days so
/// a day partitions without allocating once the heaviest day has sized it.
#[derive(Debug)]
pub struct Partitioned<T> {
    pub items: Vec<T>,
    pub starts: Vec<usize>,
}

impl<T> Default for Partitioned<T> {
    fn default() -> Partitioned<T> {
        Partitioned { items: Vec::new(), starts: Vec::new() }
    }
}

impl<T> Partitioned<T> {
    /// The items of one bucket.
    #[must_use]
    pub fn bucket(&self, b: usize) -> &[T] {
        match (self.starts.get(b), self.starts.get(b + 1)) {
            (Some(from), Some(to)) => self.items.get(*from..*to).unwrap_or(&[]),
            _ => &[],
        }
    }
}

/// A pointer shared by the pieces, each writing only the positions its counts gave it.
struct Out<T>(*mut MaybeUninit<T>);

// SAFETY: the pieces write disjoint positions of one allocation that outlives the scatter.
unsafe impl<T: Send> Send for Out<T> {}
// SAFETY: as above; no position is read during the scatter.
unsafe impl<T: Send> Sync for Out<T> {}

/// Partitions every input's items into `buckets` by `key`, stably: bucket by bucket, and within a bucket in the order
/// of the inputs and of their items.
pub fn partition_into<T: Copy + Send + Sync>(
    pool: Option<&Pool>,
    inputs: &[&[T]],
    buckets: usize,
    key: impl Fn(&T) -> usize + Sync,
    out: &mut Partitioned<T>,
) {
    partition_map_into(pool, inputs, buckets, key, |t| *t, out);
}

/// As `partition_into`, each item stored as what `map` makes of it: a narrower record where a bucket's reader needs
/// only part of the item.
pub fn partition_map_into<T: Sync, U: Copy + Send + Sync>(
    pool: Option<&Pool>,
    inputs: &[&[T]],
    buckets: usize,
    key: impl Fn(&T) -> usize + Sync,
    map_item: impl Fn(&T) -> U + Sync,
    out: &mut Partitioned<U>,
) {
    let pieces: Vec<&[T]> = inputs.iter().flat_map(|i| i.chunks(RADIX_CHUNK)).collect();
    let counts: Vec<Vec<usize>> = map(pool, pieces.len(), |p| {
        let mut c = vec![0_usize; buckets];
        for item in pieces.get(p).copied().unwrap_or(&[]) {
            let Some(n) = c.get_mut(key(item)) else {
                violation!(clause = "TIME.6", "an item keyed outside the buckets", buckets = buckets);
            };
            *n += 1;
        }
        c
    });
    // Each piece's first position in each bucket: the bucket's start plus the earlier pieces' counts in it.
    out.starts.clear();
    out.starts.push(0);
    let mut offsets: Vec<Vec<usize>> = vec![vec![0; buckets]; pieces.len()];
    let mut at = 0_usize;
    for b in 0..buckets {
        for (p, c) in counts.iter().enumerate() {
            if let Some(o) = offsets.get_mut(p).and_then(|o| o.get_mut(b)) {
                *o = at;
            }
            at += c.get(b).copied().unwrap_or(0);
        }
        out.starts.push(at);
    }
    out.items.clear();
    out.items.reserve(at);
    let dst = Out(out.items.spare_capacity_mut().as_mut_ptr());
    let dst = &dst;
    map(pool, pieces.len(), |p| {
        let (Some(piece), Some(offs)) = (pieces.get(p), offsets.get(p)) else { return };
        let mut next = offs.clone();
        for item in *piece {
            let Some(pos) = next.get_mut(key(item)) else { return };
            // SAFETY: `pos` lies below `at`, the reserved length, and no other piece is given it: the offsets split
            // each bucket's positions among the pieces by their counts.
            unsafe { dst.0.add(*pos).write(MaybeUninit::new(map_item(item))) };
            *pos += 1;
        }
    });
    // SAFETY: every position below `at` was written once by the scatter, the counts summing to `at`.
    unsafe { out.items.set_len(at) };
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{Partitioned, partition_into};
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn flows_partition_is_order_free() {
        let mut d = Draws::new(stream_key(Seed::new(3), "partition"), Subject::new(SubjectTag::World, 0), 0, 0);
        let inputs: Vec<Vec<(u64, u32)>> = (0..5)
            .map(|b| {
                (0..below_u64(&mut d, 150_000))
                    .map(|i| (below_u64(&mut d, 1 << 20), b * 1_000_000 + u32::try_from(i).unwrap()))
                    .collect()
            })
            .collect();
        let slices: Vec<&[(u64, u32)]> = inputs.iter().map(Vec::as_slice).collect();
        let buckets = 37;
        let key = |x: &(u64, u32)| usize::try_from(x.0 % 37).unwrap();
        let mut expected: Vec<(u64, u32)> = Vec::new();
        for b in 0..buckets {
            expected.extend(inputs.iter().flatten().filter(|x| key(x) == b));
        }
        for workers in [1, 4] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let mut out = Partitioned::default();
            partition_into(Some(&pool), &slices, buckets, key, &mut out);
            assert_eq!(out.items, expected, "stable and the same for {workers} workers");
            assert_eq!(out.bucket(3).len(), expected.iter().filter(|x| key(x) == 3).count());
        }
        let mut serial = Partitioned::default();
        partition_into(None, &slices, buckets, key, &mut serial);
        assert_eq!(serial.items, expected);
    }
}
