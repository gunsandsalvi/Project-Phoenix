use phx_num::violation;

use crate::consts::{KEYED_SHARD_BITS, KEYED_SHARDS};
use crate::convert::{index, to_usize};
use crate::mix::mix64;
use crate::pool::{Pool, map};
use crate::radix::{RadixKey, radix_sort};

fn shard_of(key: impl RadixKey) -> usize {
    index(mix64(key.word()) >> (u64::BITS - KEYED_SHARD_BITS))
}

/// Where each shard's pairs start in a partitioned chunk; the last entry is the chunk's length.
type Starts = [usize; KEYED_SHARDS + 1];

fn span(starts: &Starts, shard: usize) -> std::ops::Range<usize> {
    let (Some(from), Some(to)) = (starts.get(shard), starts.get(shard + 1)) else {
        violation!(clause = "TIME.6", "a shard outside the reduction", shard = shard);
    };
    *from..*to
}

/// One chunk's pairs regrouped by shard, keeping their order within each shard, with where each shard starts.
fn partition<K: RadixKey, V: Copy>(pairs: &[(K, V)]) -> (Vec<(K, V)>, Starts) {
    let mut counts = [0_usize; KEYED_SHARDS];
    let shards: Vec<usize> = pairs.iter().map(|(k, _)| shard_of(*k)).collect();
    for s in &shards {
        if let Some(c) = counts.get_mut(*s) {
            *c += 1;
        }
    }
    let mut starts: Starts = [0; KEYED_SHARDS + 1];
    let mut total = 0;
    for (start, c) in starts.iter_mut().skip(1).zip(counts) {
        total += c;
        *start = total;
    }
    let mut out = pairs.to_vec();
    let mut next = starts;
    for (pair, s) in pairs.iter().zip(shards) {
        let Some(at) = next.get_mut(s) else {
            violation!(clause = "TIME.6", "a shard outside the reduction", shard = s);
        };
        let Some(slot) = out.get_mut(*at) else {
            violation!(clause = "TIME.6", "a shard fuller than its count", shard = s);
        };
        *slot = *pair;
        *at += 1;
    }
    (out, starts)
}

/// Sums over rows a pass does not own: pairs from every chunk, sharded by a fixed mix of the key, each shard sorted by
/// key and its equal keys folded in (chunk, position) order, so the result depends on neither the workers nor the
/// order they finish in, and each shard can be applied by one worker.
#[derive(Debug)]
pub struct KeyedReduce<K, V> {
    pub shards: Vec<Vec<(K, V)>>,
}

impl<K: RadixKey, V: Copy + Send + Sync> KeyedReduce<K, V> {
    /// On the pool when one is given, on the calling thread otherwise; the result is the same.
    pub fn run(pool: Option<&Pool>, chunks: &[Vec<(K, V)>], fold: impl Fn(&mut V, V) + Sync) -> KeyedReduce<K, V> {
        let parts: Vec<(Vec<(K, V)>, Starts)> = map(pool, chunks.len(), |c| {
            let Some(pairs) = chunks.get(c) else {
                violation!(clause = "TIME.6", "a chunk outside the reduction", chunk = c);
            };
            partition(pairs)
        });
        let shards = map(pool, KEYED_SHARDS, |s| {
            let pairs: Vec<(K, V)> =
                parts.iter().flat_map(|(p, starts)| p.get(span(starts, s)).into_iter().flatten().copied()).collect();
            let mut order: Vec<(K, u32)> = pairs.iter().zip(0_u32..).map(|((k, _), i)| (*k, i)).collect();
            radix_sort(None, &mut order, &mut Vec::new());
            let mut folded: Vec<(K, V)> = Vec::new();
            for (k, i) in order {
                let Some(&(_, v)) = pairs.get(to_usize(i)) else {
                    violation!(clause = "TIME.6", "a sorted index outside its shard", index = i);
                };
                match folded.last_mut() {
                    Some((last, acc)) if *last == k => fold(acc, v),
                    _ => folded.push((k, v)),
                }
            }
            folded
        });
        KeyedReduce { shards }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::KeyedReduce;
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn keyed_reduce_matches_sequential() {
        let mut d = Draws::new(stream_key(Seed::new(2), "keyed"), Subject::new(SubjectTag::World, 0), 0, 0);
        let chunks: Vec<Vec<(u64, u64)>> = (0..37)
            .map(|_| {
                (0..below_u64(&mut d, 5000)).map(|_| (below_u64(&mut d, 20_000), below_u64(&mut d, 1000))).collect()
            })
            .collect();
        // Not commutative: the fold remembers the order values arrived in.
        let fold = |acc: &mut u64, v: u64| *acc = acc.wrapping_mul(1_000_003).wrapping_add(v);
        let mut expected: BTreeMap<u64, u64> = BTreeMap::new();
        for (k, v) in chunks.iter().flatten() {
            expected.entry(*k).and_modify(|acc| fold(acc, *v)).or_insert(*v);
        }
        for workers in [1, 8] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let out = KeyedReduce::run(Some(&pool), &chunks, fold);
            let mut got: Vec<(u64, u64)> = out.shards.iter().flatten().copied().collect();
            assert!(out.shards.iter().all(|s| s.windows(2).all(|w| w[0].0 < w[1].0)), "each shard sorted by key");
            got.sort_unstable();
            assert_eq!(got, expected.clone().into_iter().collect::<Vec<_>>(), "workers={workers}");
        }
    }
}
