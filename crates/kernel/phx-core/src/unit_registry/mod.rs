//! The units registry: every unit a holding, flow, offer or print is denominated in — a good, a capital class at a
//! zone, an instrument, a special unit, a deposit's right — issued one 24-bit id the first time something names it, its
//! 16-byte row saying what it is. Ids are never reused: a retired unit's row still reads. A unit is looked up only when
//! a party's units change, and the id it resolves to is kept where it is used.

pub mod grades;
pub mod keys;

use phx_id::Day;
use phx_macros::{Pod, clause, opening};
use phx_num::{Missing, UnitId, capacity_exceeded, violation};
use phx_store::StoreStats;

pub use grades::{Content, GradeContents};
pub use keys::{CapitalClass, KeyKind, UnitKey};

use crate::capacity::UNIT_ID_ROWS;
use crate::consts::{
    UNIT_AT_ZONE, UNIT_HASH, UNIT_HASH_FOLD, UNIT_INDEX_FREE, UNIT_PERISHABLE, UNIT_RETIRED, UNIT_STORABLE,
};

/// What a unit is beyond its key, fixed when it is issued: its price exponent, whether it keeps and whether it spoils,
/// and its spoilage class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitTraits {
    pub price_exp: u8,
    pub storable: bool,
    pub perishable: bool,
    pub spoilage: u8,
}

/// A unit's row: its key's word, the day it retired, its zone or place, its kind's code, its flags, its price exponent
/// and its spoilage class.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct UnitRow {
    word: u32,
    retired: u32,
    zone: u16,
    kind: u8,
    flags: u8,
    price_exp: u8,
    spoilage: u8,
    pad: [u8; 2],
}

const _: () = assert!(size_of::<UnitRow>() == size_of::<u128>(), "a unit row is sixteen bytes, its store's");

impl UnitRow {
    fn flag(self, bit: u8) -> bool {
        self.flags & bit != 0
    }

    #[must_use]
    pub fn key(self) -> UnitKey {
        let zone = if self.flag(UNIT_AT_ZONE) { Missing::Present(self.zone) } else { Missing::Absent };
        UnitKey::of((KeyKind::of(self.kind), self.word, zone))
    }

    #[must_use]
    pub fn traits(self) -> UnitTraits {
        UnitTraits {
            price_exp: self.price_exp,
            storable: self.flag(UNIT_STORABLE),
            perishable: self.flag(UNIT_PERISHABLE),
            spoilage: self.spoilage,
        }
    }

    /// The day the unit retired, if it has.
    pub fn retired(self) -> Missing<Day> {
        if self.flag(UNIT_RETIRED) { Missing::Present(Day::new(self.retired)) } else { Missing::Absent }
    }
}

/// The index's empty slot: no id reaches it, ids being below 2^24.
const EMPTY: u32 = u32::MAX;

fn wide(n: usize) -> u64 {
    match u64::try_from(n) {
        Ok(w) => w,
        Err(_) => capacity_exceeded!("units the registry issues", u64::MAX, n),
    }
}

fn index(n: u64) -> usize {
    match usize::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("units the registry issues", usize::MAX, n),
    }
}

/// The index's slots for a capacity: the power of two that keeps a quarter of them free, and one at least, so every
/// probe ends at an empty slot.
fn slots_for(capacity: u32) -> usize {
    let rows = index(u64::from(capacity));
    (rows + rows / (UNIT_INDEX_FREE - 1) + 1).next_power_of_two()
}

/// Every unit issued, by id, and the index from a key to its newest id: open addressing over a power of two of slots,
/// a quarter kept free, sized once from the capacity table and never grown, so an issue moves nothing.
#[clause("GDS.1", "CAP.1", "REP.24")]
#[derive(Debug, phx_macros::Saved)]
pub struct UnitRegistry {
    rows: Vec<UnitRow>,
    capacity: u32,
    #[saved(skip, rebuild = UnitRegistry::reindex)]
    slots: Vec<u32>,
}

impl UnitRegistry {
    /// The registry at the capacity table's rows.
    #[opening]
    #[must_use]
    pub fn new() -> UnitRegistry {
        UnitRegistry::with_capacity(UNIT_ID_ROWS)
    }

    /// A registry of `capacity` rows, its index at the power of two that keeps a quarter of it free.
    #[opening]
    #[must_use]
    pub fn with_capacity(capacity: u32) -> UnitRegistry {
        let rows = index(u64::from(capacity));
        UnitRegistry { rows: Vec::with_capacity(rows), capacity, slots: vec![EMPTY; slots_for(capacity)] }
    }

    /// The slot a key's probe starts at.
    fn home(&self, (kind, word, zone): (KeyKind, u32, Missing<u16>)) -> usize {
        let zone = match zone {
            Missing::Present(z) => u64::from(z) + 1,
            Missing::Absent => 0,
        };
        let [a, b, c] = UNIT_HASH;
        let sum = u64::from(word) * a + zone * b + u64::from(kind.code()) * c;
        let folded = sum ^ (sum >> UNIT_HASH_FOLD);
        index(folded & wide(self.slots.len() - 1))
    }

    fn row_of(&self, id: u32) -> UnitRow {
        match self.rows.get(index(u64::from(id))) {
            Some(r) => *r,
            None => violation!(clause = "GDS.1", "a unit id the registry never issued", id = id),
        }
    }

    /// The slot holding a key, or the empty slot its probe ends at.
    fn probe(&self, parts: (KeyKind, u32, Missing<u16>)) -> (usize, Missing<u32>) {
        let mask = self.slots.len() - 1;
        let mut at = self.home(parts);
        loop {
            let id = match self.slots.get(at) {
                Some(id) => *id,
                None => violation!(clause = "GDS.1", "a probe past the units' index"),
            };
            if id == EMPTY {
                return (at, Missing::Absent);
            }
            let row = self.row_of(id);
            let zone = if row.flag(UNIT_AT_ZONE) { Missing::Present(row.zone) } else { Missing::Absent };
            if (KeyKind::of(row.kind), row.word, zone) == parts {
                return (at, Missing::Present(id));
            }
            at = (at + 1) & mask;
        }
    }

    /// A key's newest id, if it has been issued.
    pub fn find(&self, key: UnitKey) -> Missing<UnitId> {
        match self.probe(key.parts()) {
            (_, Missing::Present(id)) if !self.row_of(id).flag(UNIT_RETIRED) => Missing::Present(UnitId::new(id)),
            _ => Missing::Absent,
        }
    }

    /// A key's id, issued now with its traits if it has none, or anew if its last retired. A key issued again with
    /// other traits is a fact with two writers and stops the run.
    pub fn issue(&mut self, key: UnitKey, traits: UnitTraits) -> UnitId {
        let parts = key.parts();
        let (slot, found) = self.probe(parts);
        if let Missing::Present(id) = found {
            let row = self.row_of(id);
            if !row.flag(UNIT_RETIRED) {
                if row.traits() != traits {
                    violation!(clause = "GDS.1", "a unit issued again with other traits", id = id);
                }
                return UnitId::new(id);
            }
        }
        let id = match u32::try_from(self.rows.len()) {
            Ok(id) if id < self.capacity => id,
            _ => capacity_exceeded!("units the registry issues", self.capacity, self.rows.len()),
        };
        let (kind, word, zone) = parts;
        let (zone, at_zone) = match zone {
            Missing::Present(z) => (z, UNIT_AT_ZONE),
            Missing::Absent => (0, 0),
        };
        let flags = at_zone
            | if traits.storable { UNIT_STORABLE } else { 0 }
            | if traits.perishable { UNIT_PERISHABLE } else { 0 };
        let unit = UnitId::new(id);
        self.rows.push(UnitRow {
            word,
            retired: 0,
            zone,
            kind: kind.code(),
            flags,
            price_exp: traits.price_exp,
            spoilage: traits.spoilage,
            pad: [0; 2],
        });
        if let Some(s) = self.slots.get_mut(slot) {
            *s = id;
        }
        unit
    }

    /// A unit's row, retired or not.
    #[must_use]
    pub fn row(&self, unit: UnitId) -> UnitRow {
        self.row_of(unit.index())
    }

    /// A unit retired on a day: its row kept, its key issued anew if named again.
    pub fn retire(&mut self, unit: UnitId, day: Day) {
        match self.rows.get_mut(index(u64::from(unit.index()))) {
            Some(r) if !r.flag(UNIT_RETIRED) => {
                r.flags |= UNIT_RETIRED;
                r.retired = day.get();
            }
            Some(_) => violation!(clause = "GDS.1", "a unit retired twice", id = unit.index()),
            None => violation!(clause = "GDS.1", "a unit id the registry never issued", id = unit.index()),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The index rebuilt from the rows in id order after a load, each key's slot its newest id's as when issued.
    #[opening]
    fn reindex(&mut self) -> u64 {
        self.slots = vec![EMPTY; slots_for(self.capacity)];
        for id in 0..self.rows.len() {
            let Ok(id) = u32::try_from(id) else { break };
            let row = self.row_of(id);
            let zone = if row.flag(UNIT_AT_ZONE) { Missing::Present(row.zone) } else { Missing::Absent };
            let (slot, _) = self.probe((KeyKind::of(row.kind), row.word, zone));
            if let Some(s) = self.slots.get_mut(slot) {
                *s = id;
            }
        }
        wide(self.rows.len())
    }
}

impl Default for UnitRegistry {
    fn default() -> UnitRegistry {
        UnitRegistry::new()
    }
}

impl StoreStats for UnitRegistry {
    fn rows_live(&self) -> u64 {
        wide(self.rows.iter().filter(|r| !r.flag(UNIT_RETIRED)).count())
    }

    fn rows_ever(&self) -> u64 {
        wide(self.rows.len())
    }

    fn bytes(&self) -> u64 {
        wide(self.rows.capacity() * size_of::<UnitRow>() + self.slots.len() * size_of::<u32>())
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
