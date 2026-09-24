use phx_core::{KernelMap, KeyAttrDecl, MapKey};
use phx_exec::consts::RADIX_BITS;
use phx_exec::{KeyedReduce, RadixKey};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

use crate::consts::{KEY_BITS, KEY_WORDS};

/// A cell's key: its key attributes bit-packed into a fixed record, the first word most significant in its order.
#[clause("REP.19")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Pod)]
pub struct KeyRecord {
    pub words: [u64; KEY_WORDS],
}

/// One key attribute's place in the record: its word, its first bit there, its width, and how many values it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyField {
    pub name: &'static str,
    pub word: usize,
    pub shift: u32,
    pub bits: u32,
    pub values: u32,
}

/// Where each of a kind's key attributes lies in its record, in declared order; an attribute never straddles two
/// words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyLayout {
    fields: Vec<KeyField>,
}

/// The bits a value below `values` needs, `values` being at least one.
fn bits_for(values: u32) -> u32 {
    u32::BITS - (values - 1).leading_zeros()
}

impl KeyLayout {
    /// The layout of a kind's key attributes.
    ///
    /// # Errors
    /// An attribute of no values, or attributes that do not fit the record.
    pub fn new(attrs: &[KeyAttrDecl]) -> Result<KeyLayout, String> {
        let mut fields = Vec::with_capacity(attrs.len());
        let (mut word, mut used) = (0_usize, 0_u32);
        for a in attrs {
            if a.values == 0 {
                return Err(format!("key attribute `{}` takes no value", a.name));
            }
            let bits = bits_for(a.values);
            if used + bits > u64::BITS {
                (word, used) = (word + 1, 0);
            }
            if word >= KEY_WORDS {
                return Err(format!("key attribute `{}` does not fit a key of {KEY_WORDS} words", a.name));
            }
            fields.push(KeyField { name: a.name, word, shift: used, bits, values: a.values });
            used += bits;
        }
        Ok(KeyLayout { fields })
    }

    #[must_use]
    pub fn fields(&self) -> &[KeyField] {
        &self.fields
    }

    fn field(&self, attr: usize) -> KeyField {
        let Some(f) = self.fields.get(attr) else {
            violation!(clause = "REP.19", "a key attribute the kind does not declare", attr = attr);
        };
        *f
    }

    /// An attribute's value in a record.
    #[must_use]
    pub fn get(&self, key: &KeyRecord, attr: usize) -> u32 {
        read(&self.field(attr), key)
    }

    /// An attribute's field by its name.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<KeyField> {
        self.fields.iter().find(|f| f.name == name).copied()
    }

    /// An attribute's value written into a record; a value outside the attribute's declared values stops the run.
    pub fn set(&self, key: &mut KeyRecord, attr: usize, value: u32) {
        let f = self.field(attr);
        if value >= f.values {
            capacity_exceeded!("values of a key attribute", f.values, value);
        }
        let Some(w) = key.words.get_mut(f.word) else {
            violation!(clause = "REP.19", "a key field beyond its record", word = f.word);
        };
        let mask = mask(f.bits) << f.shift;
        *w = (*w & !mask) | (u64::from(value) << f.shift);
    }
}

/// A field's value in a record.
#[must_use]
pub fn read(f: &KeyField, key: &KeyRecord) -> u32 {
    let Some(w) = key.words.get(f.word) else {
        violation!(clause = "REP.19", "a key field beyond its record", word = f.word);
    };
    let Ok(v) = u32::try_from((w >> f.shift) & mask(f.bits)) else {
        violation!(clause = "REP.19", "a key field wider than its values", word = f.word);
    };
    v
}

fn mask(bits: u32) -> u64 {
    match 1_u64.checked_shl(bits) {
        Some(top) => top - 1,
        None => u64::MAX,
    }
}

/// A key's identity in its kind's interner.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Pod)]
pub struct KeyId(u32);

impl KeyId {
    pub const fn new(index: u32) -> KeyId {
        KeyId(index)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl MapKey for KeyRecord {
    fn key64(self) -> u64 {
        self.words.iter().fold(0, |acc, w| acc ^ w)
    }
}

impl RadixKey for KeyRecord {
    const BITS: u32 = KEY_BITS;

    /// The digit from bit `shift` of the record read as one number, the last word least significant, so the radix
    /// order is the record's own.
    fn digit(self, shift: u32) -> usize {
        let from_low = |bit: u32| -> u64 {
            let (word, off) = (bit / u64::BITS, bit % u64::BITS);
            let at = |w: u32| {
                usize::try_from(w).ok().and_then(|w| KEY_WORDS.checked_sub(w + 1)).and_then(|i| self.words.get(i))
            };
            let low = at(word).map_or(0, |w| w >> off);
            let high = match (off, at(word + 1)) {
                (0, _) | (_, None) => 0,
                (_, Some(w)) => w << (u64::BITS - off),
            };
            low | high
        };
        let Ok(d) = usize::try_from(from_low(shift) & mask(RADIX_BITS)) else {
            capacity_exceeded!("a radix digit", usize::MAX, shift);
        };
        d
    }

    fn word(self) -> u64 {
        self.key64()
    }
}

/// Every key of a kind, interned: two cells share a key id if and only if their keys are identical. Each key counts
/// the cells holding it; a key no cell holds frees its id, which the next new key takes, lowest first. The interner
/// changes only by a reduction keyed by record, applied shard by shard and in key order within each, so the ids it
/// hands out depend on neither the workers nor the order they finished in.
#[clause("REP.19")]
#[derive(Debug)]
pub struct KeyInterner {
    records: Vec<KeyRecord>,
    refs: Vec<u32>,
    free: std::collections::BTreeSet<u32>,
    index: KernelMap<KeyRecord, KeyId>,
}

impl Default for KeyInterner {
    fn default() -> KeyInterner {
        KeyInterner::new()
    }
}

impl KeyInterner {
    #[must_use]
    pub fn new() -> KeyInterner {
        KeyInterner {
            records: Vec::new(),
            refs: Vec::new(),
            free: std::collections::BTreeSet::new(),
            index: KernelMap::new(),
        }
    }

    /// A key's identity, if some cell holds it.
    pub fn id(&self, key: &KeyRecord) -> Missing<KeyId> {
        match self.index.get(*key) {
            Some(id) => Missing::Present(*id),
            None => Missing::Absent,
        }
    }

    /// The key an identity names; an identity no cell holds stops the run.
    #[must_use]
    pub fn record(&self, id: KeyId) -> KeyRecord {
        self.live(id);
        let Some(r) = self.records.get(slot(id)) else {
            violation!(clause = "REP.19", "a key identity never handed out", id = id.get());
        };
        *r
    }

    /// How many cells hold a key; an identity never handed out is held by none.
    #[must_use]
    pub fn refs(&self, id: KeyId) -> u32 {
        match self.refs.get(slot(id)) {
            Some(r) => *r,
            None => 0,
        }
    }

    /// Keys some cell holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    fn live(&self, id: KeyId) {
        if self.refs(id) == 0 {
            violation!(clause = "REP.19", "a key identity no cell holds", id = id.get());
        }
    }

    /// A day's changes in the cells holding each key, reduced by record: a key newly held takes an identity, and a key
    /// no longer held frees its own.
    pub fn apply(&mut self, changes: &KeyedReduce<KeyRecord, i64>) {
        for (key, delta) in changes.shards.iter().flatten() {
            self.hold(*key, *delta);
        }
    }

    /// One key held by `delta` more cells, or fewer, as one change of a reduction applies it.
    pub fn hold(&mut self, key: KeyRecord, delta: i64) {
        match self.index.get(key).copied() {
            Some(id) => self.change(id, key, delta),
            None if delta > 0 => {
                let id = self.take_id(key);
                self.change(id, key, delta);
            }
            None if delta == 0 => {}
            None => violation!(clause = "REP.19", "a key released that no cell holds", by = delta),
        }
    }

    fn take_id(&mut self, key: KeyRecord) -> KeyId {
        let id = if let Some(i) = self.free.pop_first() {
            KeyId(i)
        } else {
            let Ok(i) = u32::try_from(self.records.len()) else {
                capacity_exceeded!("key identities", u32::MAX, self.records.len());
            };
            self.records.push(KeyRecord::default());
            self.refs.push(0);
            KeyId(i)
        };
        if let Some(r) = self.records.get_mut(slot(id)) {
            *r = key;
        }
        self.index.insert(key, id);
        id
    }

    fn change(&mut self, id: KeyId, key: KeyRecord, delta: i64) {
        let Some(r) = self.refs.get_mut(slot(id)) else {
            violation!(clause = "REP.19", "a key identity never handed out", id = id.get());
        };
        let Some(next) = i64::from(*r).checked_add(delta).and_then(|n| u32::try_from(n).ok()) else {
            violation!(clause = "REP.19", "a key held by fewer than no cells", id = id.get(), by = delta);
        };
        *r = next;
        if next == 0 {
            self.index.remove(key);
            self.free.insert(id.0);
        }
    }
}

/// A save keeps each identity's record and count and the free identities; the index from record to identity is
/// rebuilt from them on reading.
impl phx_store::Saved for KeyInterner {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        self.records.save(w);
        self.refs.save(w);
        self.free.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<KeyInterner, phx_store::LoadError> {
        let records: Vec<KeyRecord> = Vec::load(r)?;
        let refs: Vec<u32> = Vec::load(r)?;
        let free: std::collections::BTreeSet<u32> = std::collections::BTreeSet::load(r)?;
        if records.len() != refs.len() {
            return Err(phx_store::LoadError::Invalid("key records and their counts differ in number".to_owned()));
        }
        let mut index = KernelMap::new();
        for (i, (record, n)) in (0_u32..).zip(records.iter().zip(&refs)) {
            if *n > 0 {
                index.insert(*record, KeyId(i));
            }
        }
        Ok(KeyInterner { records, refs, free, index })
    }
}

fn slot(id: KeyId) -> usize {
    let Ok(i) = usize::try_from(id.0) else {
        capacity_exceeded!("key identities", usize::MAX, id.0);
    };
    i
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use phx_core::KeyAttrDecl;
    use phx_exec::{KeyedReduce, radix_sort};
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{KeyInterner, KeyLayout, KeyRecord};

    const fn attr(name: &'static str, values: u32) -> KeyAttrDecl {
        KeyAttrDecl { name, values, clause: "REP.19" }
    }

    #[test]
    fn key_fields_pack_without_straddling() {
        let attrs =
            [attr("region", 40), attr("composition", 9), attr("tenure", 3), attr("stage", 1), attr("wide", 1 << 30)];
        let layout = KeyLayout::new(&attrs).unwrap();
        let places: Vec<(usize, u32, u32)> = layout.fields().iter().map(|f| (f.word, f.shift, f.bits)).collect();
        assert_eq!(places, [(0, 0, 6), (0, 6, 4), (0, 10, 2), (0, 12, 0), (0, 12, 30)]);
        let mut k = KeyRecord::default();
        for (i, v) in [39, 8, 2, 0, (1 << 30) - 1].into_iter().enumerate() {
            layout.set(&mut k, i, v);
        }
        layout.set(&mut k, 1, 5);
        let back: Vec<u32> = (0..attrs.len()).map(|i| layout.get(&k, i)).collect();
        assert_eq!(back, [39, 5, 2, 0, (1 << 30) - 1]);
        assert!(
            std::panic::catch_unwind(|| layout.set(&mut KeyRecord::default(), 2, 3)).is_err(),
            "a value past its set"
        );
        let too_many: Vec<KeyAttrDecl> = (0..9).map(|_| attr("x", u32::MAX)).collect();
        assert!(KeyLayout::new(&too_many).is_err());
        assert!(KeyLayout::new(&[attr("none", 0)]).is_err());
    }

    #[test]
    fn radix_order_is_the_records_order() {
        let mut d = Draws::new(stream_key(Seed::new(4), "key-radix"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut items: Vec<(KeyRecord, u32)> = (0..3_000_u32)
            .map(|i| {
                let mut w = [0_u64; super::KEY_WORDS];
                for x in &mut w {
                    *x = below_u64(&mut d, 4) << below_u64(&mut d, 64);
                }
                (KeyRecord { words: w }, i)
            })
            .collect();
        let mut expected = items.clone();
        expected.sort();
        radix_sort(None, &mut items, &mut Vec::new());
        assert_eq!(items, expected);
    }

    #[test]
    fn interner_refcounts_under_keyed_reduce() {
        let mut d = Draws::new(stream_key(Seed::new(9), "interner"), Subject::new(SubjectTag::World, 0), 0, 0);
        let key = |n: u64| KeyRecord { words: [n % 3, 0, n, n.rotate_left(17)] };
        let mut interner = KeyInterner::new();
        let mut model: BTreeMap<KeyRecord, i64> = BTreeMap::new();
        for _day in 0..40 {
            let chunks: Vec<Vec<(KeyRecord, i64)>> = (0..5)
                .map(|_| {
                    (0..below_u64(&mut d, 60))
                        .map(|_| {
                            let k = key(below_u64(&mut d, 50));
                            let held = model.get(&k).copied().unwrap_or(0);
                            let up = below_u64(&mut d, 3) > 0 || held == 0;
                            let delta = if up { 1 } else { -1 };
                            let e = model.entry(k).or_insert(0);
                            *e += delta;
                            (k, delta)
                        })
                        .collect()
                })
                .collect();
            interner.apply(&KeyedReduce::run(None, &chunks, |a, v| *a += v));
            for (k, n) in &model {
                match interner.id(k) {
                    Missing::Present(id) => {
                        assert_eq!(i64::from(interner.refs(id)), *n);
                        assert_eq!(interner.record(id), *k);
                    }
                    Missing::Absent => assert_eq!(*n, 0, "a key no cell holds has no identity"),
                }
            }
            assert_eq!(interner.len(), model.values().filter(|n| **n > 0).count());
        }
    }
}
