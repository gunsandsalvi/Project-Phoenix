//! Partitioned apply: every per-item effect on party state — a due's two sides, a sale's payee credit, an intent's
//! target — scattered by its target's range and applied by one job a range, sweeping the range's shard in a declared
//! order with the range's columns in cache, so writes need no lock and the result is the same for any workers.

use phx_num::violation;

use crate::consts::{APPLY_ITEM_COST, INLINE_BELOW};
use crate::convert::to_u64;
use crate::hooks::SweepHooks;
use crate::partition::{Partitioned, partition_into};
use crate::pool::Pool;
use crate::traverse::for_each_chunk;

/// An effect on one target: the target's dense index, a tag its rule reads, and an amount.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub target: u32,
    pub tag: u32,
    pub amount: i64,
}

/// The ranges a store of `count` rows falls into at `shift`: each `2^shift` rows, the last the rest, so a store of any
/// size has as many ranges as its count needs.
#[must_use]
pub fn ranges_of(count: u32, shift: u32) -> usize {
    crate::convert::to_usize(count.div_ceil(1 << shift))
}

/// An apply's shards, kept across days so a day scatters without allocating once its heaviest wave has sized them,
/// and the waves the last apply ran.
#[derive(Debug, Default)]
pub struct Shards {
    parts: Partitioned<Item>,
    waves: u64,
}

impl Shards {
    /// The waves the last apply ran.
    #[must_use]
    pub fn waves(&self) -> u64 {
        self.waves
    }

    /// The bytes the shards hold: the items of the largest wave and the scatter's counts.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        to_u64(self.parts.bytes())
    }

    /// The bytes of the scatter's own counts and places, beside the wave's items in their lane.
    #[must_use]
    pub fn scatter_bytes(&self) -> u64 {
        to_u64(self.parts.scatter_bytes())
    }
}

/// Every item applied: the producing chunks' items, in chunk order, cut into waves of whole chunks whose items fill
/// `lane`; each wave scattered by `target >> shift` into its ranges' shards, stably, and swept one job a range, each
/// range's items in (producing chunk, emission) order — a declared order, never arrival — calling `f` on the range's
/// own state and then its hooks. A range's state and hooks are the caller's `ranges[range]`, disjoint mutable views,
/// so no two jobs write one word. A target past the ranges stops the run. A light wave is swept inline.
pub fn apply_by_range<R: Send, H: SweepHooks>(
    pool: Option<&Pool>,
    (inputs, shift, lane): (&[&[Item]], u32, usize),
    ranges: &mut [(R, H)],
    shards: &mut Shards,
    f: impl Fn(&mut R, &Item) + Sync,
) {
    shards.waves = 0;
    let buckets = ranges.len();
    let key = |item: &Item| match usize::try_from(item.target >> shift) {
        Ok(range) => range,
        Err(_) => violation!(clause = "TIME.6", "an item's range past a word", target = item.target),
    };
    let mut start = 0;
    while start < inputs.len() {
        // A wave takes whole chunks while their items fit the lane, and at least one.
        let mut end = start;
        let mut items = 0;
        while let Some(chunk) = inputs.get(end) {
            if end > start && items + chunk.len() > lane {
                break;
            }
            items += chunk.len();
            end += 1;
        }
        let Some(wave) = inputs.get(start..end) else {
            violation!(clause = "TIME.6", "a wave past the apply's chunks", chunks = inputs.len());
        };
        if buckets == 0 && items > 0 {
            violation!(clause = "TIME.6", "items applied to a store of no ranges", items = items);
        }
        if buckets > 0 {
            partition_into(pool, wave, buckets, key, &mut shards.parts);
            let parts = &shards.parts;
            let sweep_pool = pool.filter(|_| to_u64(items) * APPLY_ITEM_COST >= INLINE_BELOW);
            for_each_chunk(sweep_pool, ranges, |range, (state, hooks)| {
                hooks.open_range(range);
                for item in parts.bucket(range) {
                    f(state, item);
                    hooks.item(item);
                }
                hooks.close_range();
            });
        }
        shards.waves += 1;
        start = end;
    }
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;
