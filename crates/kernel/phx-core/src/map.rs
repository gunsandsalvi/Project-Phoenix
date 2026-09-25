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

/// The kernel's one map: sharded as a keyed reduction shards, hashed with a fixed seed, and read whole only sorted by
/// key, so no outcome can depend on the order of its entries.
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

    /// The value at a key, made by `make` where there is none: one look-up either way.
    pub fn get_or_insert_with(&mut self, key: K, make: impl FnOnce() -> V) -> &mut V {
        let i = shard(key);
        let Some(s) = self.shards.get_mut(i) else {
            capacity_exceeded!("map shards", KEYED_SHARDS, i);
        };
        match s.entry(key) {
            hashbrown::hash_map::Entry::Occupied(o) => o.into_mut(),
            hashbrown::hash_map::Entry::Vacant(v) => {
                self.len += 1;
                v.insert(make())
            }
        }
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

    /// The entries the map holds room for, grown or not, so what it takes in memory can be counted.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.shards.iter().map(HashMap::capacity).sum()
    }

    /// Every entry, sorted by key, for a read that must not depend on the entries' order.
    #[must_use]
    pub fn sorted(&self) -> Vec<(K, &V)> {
        let mut all: Vec<(K, &V)> = self.shards.iter().flat_map(|s| s.iter().map(|(k, v)| (*k, v))).collect();
        all.sort_unstable_by_key(|entry| entry.0);
        all
    }

    /// Empties the map, keeping the room it holds, so a map filled again each day maps no new pages.
    pub fn clear(&mut self) {
        for s in &mut self.shards {
            s.clear();
        }
        self.len = 0;
    }

    /// Every entry, sorted by key, leaving the map empty, for a save.
    pub fn drain_sorted(&mut self) -> Vec<(K, V)> {
        let mut all: Vec<(K, V)> = self.shards.iter_mut().flat_map(HashMap::drain).collect();
        all.sort_unstable_by_key(|entry| entry.0);
        self.len = 0;
        all
    }
}

/// A map saved sorted by key and read back entry by entry, so its layout is never part of a save.
impl<K: MapKey + phx_store::Saved, V: phx_store::Saved> phx_store::Saved for KernelMap<K, V> {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        w.count(self.len);
        for (k, v) in self.sorted() {
            k.save(w);
            v.save(w);
        }
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<KernelMap<K, V>, phx_store::LoadError> {
        let n = r.count()?;
        let mut map = KernelMap::new();
        for _ in 0..n {
            let k = K::load(r)?;
            if map.insert(k, V::load(r)?).is_some() {
                return Err(phx_store::LoadError::Invalid("a key saved twice in a map".to_owned()));
            }
        }
        Ok(map)
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
