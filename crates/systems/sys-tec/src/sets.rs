//! The distinct sets of ways, each kept once and sorted, so a set's membership is a binary search and a firm's or
//! an industry's known ways are four bytes wherever they are kept.

use std::collections::BTreeMap;

use if_base::{WayId, WaySetId};
use phx_num::violation;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaySets {
    sets: Vec<Box<[WayId]>>,
    index: BTreeMap<Box<[WayId]>, WaySetId>,
}

impl WaySets {
    /// The set of these ways, the same identity however they are listed.
    pub fn intern(&mut self, ways: &[WayId]) -> WaySetId {
        let mut key: Vec<WayId> = ways.to_vec();
        key.sort_unstable();
        key.dedup();
        let key = key.into_boxed_slice();
        if let Some(id) = self.index.get(&key) {
            return *id;
        }
        let Ok(raw) = u32::try_from(self.sets.len()) else {
            violation!(clause = "NUM.6", "more sets of ways than a set's identity holds", sets = self.sets.len());
        };
        let id = WaySetId::new(raw);
        self.sets.push(key.clone());
        self.index.insert(key, id);
        id
    }

    /// The ways of a set.
    #[must_use]
    pub fn ways(&self, id: WaySetId) -> Option<&[WayId]> {
        self.sets.get(usize::try_from(id.index()).ok()?).map(|s| &**s)
    }

    /// Whether a set holds a way.
    #[must_use]
    pub fn contains(&self, id: WaySetId, way: WayId) -> bool {
        self.ways(id).is_some_and(|s| s.binary_search(&way).is_ok())
    }

    /// The distinct sets kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_is_interned_once() {
        let mut sets = WaySets::default();
        let a = sets.intern(&[WayId::new(3), WayId::new(1), WayId::new(3)]);
        let b = sets.intern(&[WayId::new(1), WayId::new(3)]);
        assert_eq!(a, b);
        assert_eq!(sets.len(), 1);
        assert!(sets.contains(a, WayId::new(3)));
        assert!(!sets.contains(a, WayId::new(2)));
        let empty = sets.intern(&[]);
        assert_ne!(empty, a);
        assert_eq!(sets.ways(empty), Some(&[][..]));
    }
}
