use std::hash::Hash;

use foldhash::fast::FixedState;
use hashbrown::HashMap;
use phx_exec::consts::{KEYED_SHARD_BITS, KEYED_SHARDS};
use phx_exec::mix64;
use phx_id::{InstrumentId, LineId, PartyId};
use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::consts::MAP_SEED;

/// A key of the kernel map: its 64 bits choose the shard, as a keyed reduction's do.
pub trait MapKey: Copy + Eq + Hash + Ord {
    fn key64(self) -> u64;
}

impl MapKey for PartyId {
    fn key64(self) -> u64 {
        self.get()
    }
}

impl MapKey for LineId {
    fn key64(self) -> u64 {
        u64::from(self.get())
    }
}

impl MapKey for InstrumentId {
    fn key64(self) -> u64 {
        u64::from(self.get())
    }
}

impl MapKey for u64 {
    fn key64(self) -> u64 {
        self
    }
}

/// The kernel's one map: sharded as a keyed reduction shards, hashed with a fixed seed, and never iterated, so no
/// outcome can depend on the order of its entries; a save takes them out sorted by key.
#[clause("CHN.6")]
#[derive(Debug)]
pub struct KernelMap<K, V> {
    shards: Vec<HashMap<K, V, FixedState>>,
    len: usize,
}

fn shard<K: MapKey>(key: K) -> usize {
    let top = mix64(key.key64()) >> (u64::BITS - KEYED_SHARD_BITS);
    let Ok(i) = usize::try_from(top) else {
        capacity_exceeded!("map shards", KEYED_SHARDS, top);
    };
    i
}

impl<K: MapKey, V> KernelMap<K, V> {
    #[must_use]
    pub fn new() -> KernelMap<K, V> {
        let shards = (0..KEYED_SHARDS).map(|_| HashMap::with_hasher(FixedState::with_seed(MAP_SEED))).collect();
        KernelMap { shards, len: 0 }
    }

    fn at(&self, key: K) -> &HashMap<K, V, FixedState> {
        let Some(s) = self.shards.get(shard(key)) else {
            capacity_exceeded!("map shards", KEYED_SHARDS, shard(key));
        };
        s
    }

    fn at_mut(&mut self, key: K) -> &mut HashMap<K, V, FixedState> {
        let Some(s) = self.shards.get_mut(shard(key)) else {
            capacity_exceeded!("map shards", KEYED_SHARDS, shard(key));
        };
        s
    }

    #[must_use]
    pub fn get(&self, key: K) -> Option<&V> {
        self.at(key).get(&key)
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.at_mut(key).get_mut(&key)
    }

    /// Inserts or replaces, returning the value replaced.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let old = self.at_mut(key).insert(key, value);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let old = self.at_mut(key).remove(&key);
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Every entry, sorted by key, leaving the map empty: the one way to see them all, for a save.
    pub fn drain_sorted(&mut self) -> Vec<(K, V)> {
        let mut all: Vec<(K, V)> = self.shards.iter_mut().flat_map(HashMap::drain).collect();
        all.sort_unstable_by_key(|entry| entry.0);
        self.len = 0;
        all
    }
}

impl<K: MapKey, V> Default for KernelMap<K, V> {
    fn default() -> Self {
        KernelMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::KernelMap;

    #[test]
    fn kernel_map_keeps_and_drains_sorted() {
        let mut m: KernelMap<u64, u32> = KernelMap::new();
        for k in (0..1000_u64).rev() {
            assert_eq!(m.insert(k * 7, u32::try_from(k).unwrap()), None);
        }
        assert_eq!(m.insert(14, 99), Some(2));
        assert_eq!((m.len(), m.get(14), m.get(15)), (1000, Some(&99), None));
        assert_eq!(m.remove(0), Some(0));
        *m.get_mut(7).unwrap() += 1;
        let all = m.drain_sorted();
        assert!(all.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!((all.len(), all[0], m.is_empty()), (999, (7, 2), true));
    }
}
