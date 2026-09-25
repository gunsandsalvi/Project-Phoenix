use phx_id::PartyId;
use phx_num::capacity_exceeded;

use crate::consts::ID_PAGE_BITS;

/// A key issued in order from one counter, so consecutive keys share a page.
pub trait PageKey: Copy + Ord {
    fn index(self) -> u64;
    fn from_index(index: u64) -> Self;
}

impl PageKey for PartyId {
    fn index(self) -> u64 {
        self.get()
    }

    fn from_index(index: u64) -> PartyId {
        PartyId::new(index)
    }
}

struct Page<V> {
    held: usize,
    entries: Box<[Option<V>]>,
}

/// A map from keys issued in order to their values, as pages of consecutive keys: a read is two array reads and no
/// hash, and a page is freed when its last key goes, so what the map holds follows the keys still held.
pub struct PagedMap<K, V> {
    pages: Vec<Option<Page<V>>>,
    len: usize,
    key: std::marker::PhantomData<K>,
}

impl<K, V> std::fmt::Debug for PagedMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PagedMap").field("len", &self.len).field("pages", &self.pages.len()).finish()
    }
}

const PAGE: usize = 1 << ID_PAGE_BITS;
const MASK: u64 = (1 << ID_PAGE_BITS) - 1;

fn split<K: PageKey>(key: K) -> (usize, usize) {
    let i = key.index();
    let (Ok(page), Ok(at)) = (usize::try_from(i >> ID_PAGE_BITS), usize::try_from(i & MASK)) else {
        capacity_exceeded!("id map pages", usize::MAX, i);
    };
    (page, at)
}

impl<K: PageKey, V> Default for PagedMap<K, V> {
    fn default() -> Self {
        PagedMap::new()
    }
}

impl<K: PageKey, V> PagedMap<K, V> {
    #[must_use]
    pub fn new() -> PagedMap<K, V> {
        PagedMap { pages: Vec::new(), len: 0, key: std::marker::PhantomData }
    }

    #[must_use]
    pub fn get(&self, key: K) -> Option<&V> {
        let (page, at) = split(key);
        self.pages.get(page)?.as_ref()?.entries.get(at)?.as_ref()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let (page, at) = split(key);
        self.pages.get_mut(page)?.as_mut()?.entries.get_mut(at)?.as_mut()
    }

    /// Inserts or replaces, returning the value replaced.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (page, at) = split(key);
        if self.pages.len() <= page {
            self.pages.resize_with(page + 1, || None);
        }
        let Some(slot) = self.pages.get_mut(page) else {
            capacity_exceeded!("id map pages", self.pages.len(), page);
        };
        let p = slot.get_or_insert_with(|| Page { held: 0, entries: (0..PAGE).map(|_| None).collect() });
        let Some(entry) = p.entries.get_mut(at) else {
            capacity_exceeded!("id map page", PAGE, at);
        };
        let old = entry.replace(value);
        if old.is_none() {
            p.held += 1;
            self.len += 1;
        }
        old
    }

    /// Removes a key, freeing its page when it held the page's last.
    pub fn remove(&mut self, key: K) -> Option<V> {
        let (page, at) = split(key);
        let slot = self.pages.get_mut(page)?;
        let p = slot.as_mut()?;
        let old = p.entries.get_mut(at)?.take()?;
        p.held -= 1;
        self.len -= 1;
        if p.held == 0 {
            *slot = None;
        }
        Some(old)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Every entry in key order.
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> + '_ {
        self.pages.iter().enumerate().filter_map(|(n, p)| p.as_ref().map(|p| (n, p))).flat_map(|(n, p)| {
            p.entries.iter().enumerate().filter_map(move |(at, v)| {
                let index = (u64::try_from(n).ok()? << ID_PAGE_BITS) | u64::try_from(at).ok()?;
                v.as_ref().map(|v| (K::from_index(index), v))
            })
        })
    }

    /// The bytes the map takes: its page table and the pages it holds.
    #[must_use]
    pub fn bytes(&self) -> usize {
        let held = self.pages.iter().filter(|p| p.is_some()).count();
        self.pages.len() * size_of::<Option<Page<V>>>() + held * PAGE * size_of::<Option<V>>()
    }
}

/// Saved as a count and its entries in key order, as the kernel map is, so its pages are never part of a save.
impl<K: PageKey + phx_store::Saved, V: phx_store::Saved> phx_store::Saved for PagedMap<K, V> {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        w.count(self.len);
        for (k, v) in self.iter() {
            k.save(w);
            v.save(w);
        }
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<PagedMap<K, V>, phx_store::LoadError> {
        let n = r.count()?;
        let mut map = PagedMap::new();
        for _ in 0..n {
            let k = K::load(r)?;
            if map.insert(k, V::load(r)?).is_some() {
                return Err(phx_store::LoadError::Invalid("a key saved twice in a map".to_owned()));
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;

    use super::PagedMap;
    use crate::consts::ID_PAGE_BITS;

    #[test]
    fn paged_map_keeps_in_order_and_frees_an_empty_page() {
        let mut m: PagedMap<PartyId, u32> = PagedMap::new();
        let far = 3 << ID_PAGE_BITS;
        for k in [far + 5, 7, far + 1, 1] {
            assert_eq!(m.insert(PartyId::new(k), u32::try_from(k).unwrap()), None);
        }
        assert_eq!(m.insert(PartyId::new(7), 70), Some(7));
        let keys: Vec<u64> = m.iter().map(|(k, _)| k.get()).collect();
        assert_eq!(keys, vec![1, 7, far + 1, far + 5]);
        assert_eq!((m.len(), m.get(PartyId::new(7)), m.get(PartyId::new(8))), (4, Some(&70), None));
        let full = m.bytes();
        assert_eq!(m.remove(PartyId::new(far + 1)), Some(u32::try_from(far + 1).unwrap()));
        assert_eq!(m.bytes(), full, "a page with a key left is kept");
        assert_eq!(m.remove(PartyId::new(far + 5)), Some(u32::try_from(far + 5).unwrap()));
        assert!(m.bytes() < full, "a page whose last key went is freed");
        assert_eq!((m.remove(PartyId::new(far + 5)), m.len()), (None, 2));
    }
}
