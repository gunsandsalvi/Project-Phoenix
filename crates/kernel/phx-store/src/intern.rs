//! The interner: a value many rows share — a contract's terms shape, a way set, a market or route key — stored once
//! and named by a 32-bit id found from the value's bytes; each id counts the rows that hold it and retires at zero.

use phx_id::Slot;
use phx_macros::{Pod, clause, opening};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::consts::{
    INTERN_CELLS_PER_IDS, INTERN_DEAD_SHARE, INTERN_GROUP, INTERN_KEY, INTERN_SLOT_BITS, SUMTREE_ROWS_PER_CHUNK,
};
use crate::convert::{to_u32, to_u64, to_usize};
use crate::hash::Sip128;
use crate::region::Region;
use crate::stats::StoreStats;
use crate::table::SlotAlloc;

/// Whether a retired id's slot is taken again: after the close behind its generation, where only counted holders keep
/// the id; or never, where records name it for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reuse {
    AfterClose,
    Never,
}

/// An interned value's name: its slot in the low 24 bits, the slot's generation above.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct InternId(u32);

impl InternId {
    fn new(slot: u32, generation: u8) -> InternId {
        InternId(slot | (u32::from(generation) << INTERN_SLOT_BITS))
    }

    pub fn slot(self) -> Slot {
        Slot::new(self.0 & ((1 << INTERN_SLOT_BITS) - 1))
    }

    #[must_use]
    pub fn generation(self) -> u8 {
        match u8::try_from(self.0 >> INTERN_SLOT_BITS) {
            Ok(g) => g,
            Err(_) => violation!(clause = "SET.12", "an interned id's generation past its byte", id = self.0),
        }
    }

    /// The id as the word its holders store.
    #[must_use]
    pub fn word(self) -> u32 {
        self.0
    }
}

/// An id's state: held; held by none since today, to retire at the close unless held again; its slot free for reuse;
/// retired for good, its value's bytes released.
const LIVE: u8 = 0;
const LISTED: u8 = 1;
const FREE: u8 = 2;
const RETIRED: u8 = FREE + 1;

/// An id's row: where its value lies, its holders, its slot's generation and its state.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct Row {
    offset: u32,
    len: u32,
    count: u32,
    generation: u8,
    state: u8,
    pad: [u8; 2],
}

/// An index cell's slot word: no id yet, or an id's entry tombstoned at its retirement.
const EMPTY: u32 = 0;
const TOMB: u32 = u32::MAX;

/// A value's hash under the interner's fixed key: the index's content depends on the values alone.
fn hash_of(bytes: &[u8]) -> u64 {
    let mut sip = Sip128::new(INTERN_KEY);
    sip.write(bytes);
    // The low half of the hash: every bit of it is as well mixed as the rest.
    match u64::try_from(sip.finish() & u128::from(u64::MAX)) {
        Ok(h) => h,
        Err(_) => violation!(clause = "SET.12", "a hash's low half past a word"),
    }
}

/// A hash's tag, the part a cell keeps to pass over other values before their bytes are compared.
fn tag(hash: u64) -> u32 {
    match u32::try_from(hash & u64::from(u32::MAX)) {
        Ok(t) => t,
        Err(_) => violation!(clause = "SET.12", "a hash's tag past a word", hash = hash),
    }
}

/// The index: open addressing with linear probing over cells of (tag, slot + 1), at most two-thirds full; probed, never
/// iterated, so its order is never read.
#[derive(Debug)]
pub(crate) struct Probe<B: Backing> {
    cells: Region<[u32; 2], B>,
    cap: u32,
    occupied: u32,
    tombs: u32,
}

impl<B: Backing> Probe<B> {
    fn new(space: &mut AddressSpace, max_ids: u32) -> Probe<B> {
        let (cells, ids) = INTERN_CELLS_PER_IDS;
        let cap = to_u32((to_usize(max_ids) * to_usize(cells)).div_ceil(to_usize(ids)));
        let mut region = Region::reserve(space, to_usize(cap));
        region.ensure(to_usize(cap));
        Probe { cells: region, cap, occupied: 0, tombs: 0 }
    }

    /// The first cell a hash probes: its high bits scaled to the cells, so the cells need not be a power of two.
    fn home(&self, hash: u64) -> usize {
        let at = (u128::from(hash) * u128::from(self.cap)) >> u64::BITS;
        match usize::try_from(at) {
            Ok(at) => at,
            Err(_) => violation!(clause = "SET.12", "an index cell past its cells", cap = self.cap),
        }
    }

    fn cells(&self) -> &[[u32; 2]] {
        self.cells.slice(to_usize(self.cap))
    }

    fn cells_mut(&mut self) -> &mut [[u32; 2]] {
        self.cells.slice_mut(to_usize(self.cap))
    }

    /// Every cell emptied, for a rebuild from the rows.
    fn clear(&mut self) {
        self.cells_mut().fill([0, EMPTY]);
        (self.occupied, self.tombs) = (0, 0);
    }

    /// A new value's cell: the first empty or tombstoned one from its home.
    fn put(&mut self, hash: u64, slot: u32) {
        let (mut at, cap) = (self.home(hash), to_usize(self.cap));
        loop {
            let Some(cell) = self.cells_mut().get_mut(at) else {
                violation!(clause = "SET.12", "an index cell past its cells", cell = at);
            };
            let was = cell[1];
            if was == EMPTY || was == TOMB {
                *cell = [tag(hash), slot + 1];
                if was == TOMB {
                    self.tombs -= 1;
                } else {
                    self.occupied += 1;
                }
                return;
            }
            at = if at + 1 == cap { 0 } else { at + 1 };
        }
    }

    /// The cell naming a slot, found from its value's hash, tombstoned.
    fn remove(&mut self, hash: u64, slot: u32) {
        let (mut at, cap) = (self.home(hash), to_usize(self.cap));
        loop {
            let Some(cell) = self.cells_mut().get_mut(at) else {
                violation!(clause = "SET.12", "an index cell past its cells", cell = at);
            };
            let was = cell[1];
            if was == slot + 1 {
                cell[1] = TOMB;
                self.tombs += 1;
                return;
            }
            if was == EMPTY {
                violation!(clause = "SET.12", "an interned id missing from its index", slot = slot);
            }
            at = if at + 1 == cap { 0 } else { at + 1 };
        }
    }
}

/// Values stored once each in a byte arena, a 16-byte row an id, the index from a value's bytes to its id, the ids'
/// slots reused first in first out after the close, and the ids whose holders fell to none today.
#[derive(Debug)]
pub struct Interner<B: Backing = SystemBacking> {
    reuse: Reuse,
    max_ids: u32,
    rows: Column<Row, B>,
    bytes: Region<u8, B>,
    used: usize,
    dead: usize,
    scratch: Region<u8, B>,
    slots: SlotAlloc<B>,
    listed: Column<u32, B>,
    index: Probe<B>,
    live: u64,
    ever: u64,
}

impl<B: Backing> Interner<B> {
    /// An interner of at most `max_ids` ids at once and `max_bytes` bytes of values, reusing retired slots as `reuse`
    /// declares.
    ///
    /// # Errors
    /// More ids than an id's slot bits name, or none.
    #[opening]
    pub fn new(
        space: &mut AddressSpace,
        (max_ids, max_bytes): (u32, usize),
        reuse: Reuse,
    ) -> Result<Interner<B>, String> {
        if max_ids == 0 || max_ids > 1 << INTERN_SLOT_BITS {
            return Err(format!("an interner of {max_ids} ids, beyond an id's {INTERN_SLOT_BITS} slot bits"));
        }
        Ok(Interner {
            reuse,
            max_ids,
            rows: Column::new(space, max_ids, SUMTREE_ROWS_PER_CHUNK),
            bytes: Region::reserve(space, max_bytes),
            used: 0,
            dead: 0,
            scratch: Region::reserve(space, max_bytes),
            slots: SlotAlloc::new(space, max_ids),
            listed: Column::new(space, max_ids, SUMTREE_ROWS_PER_CHUNK),
            index: Probe::new(space, max_ids),
            live: 0,
            ever: 0,
        })
    }

    fn row(&self, slot: u32) -> Row {
        match self.rows.get(Slot::new(slot)) {
            Some(r) => r,
            None => violation!(clause = "SET.12", "an interned id past its rows", slot = slot),
        }
    }

    fn value(&self, row: Row) -> &[u8] {
        let at = to_usize(row.offset);
        match self.bytes.slice(self.used).get(at..at + to_usize(row.len)) {
            Some(v) => v,
            None => violation!(clause = "SET.12", "an interned value past its bytes", offset = row.offset),
        }
    }

    /// The row an id names while it is held or listed today; a stale id, a free or retired one, reads none.
    fn held(&self, id: InternId) -> Missing<Row> {
        let slot = id.slot().get();
        match self.rows.get(Slot::new(slot)) {
            Some(r) if r.generation == id.generation() && (r.state == LIVE || r.state == LISTED) => Missing::Present(r),
            _ => Missing::Absent,
        }
    }

    /// The id a value is interned under, found without holding it: the read an apply makes before its range's new
    /// values are staged.
    pub fn find(&self, bytes: &[u8]) -> Missing<InternId> {
        self.find_hashed(bytes, hash_of(bytes))
    }

    fn find_hashed(&self, bytes: &[u8], hash: u64) -> Missing<InternId> {
        let (cells, cap, want) = (self.index.cells(), to_usize(self.index.cap), tag(hash));
        let mut at = self.index.home(hash);
        loop {
            let Some(&[t, s]) = cells.get(at) else {
                violation!(clause = "SET.12", "an index cell past its cells", cell = at);
            };
            if s == EMPTY {
                return Missing::Absent;
            }
            if s != TOMB && t == want {
                let row = self.row(s - 1);
                if self.value(row) == bytes {
                    return Missing::Present(InternId::new(s - 1, row.generation));
                }
            }
            at = if at + 1 == cap { 0 } else { at + 1 };
        }
    }

    /// One group's lookups, each phase read across the group so its reads of memory overlap: the values' hashes,
    /// their first cells, the rows those cells name, then the values compared; a first cell that is not the value's
    /// is walked in full. The hashes are left for the misses' inserts.
    fn look(&self, value: &[&[u8]], hash: &mut [u64; INTERN_GROUP], out: &mut [Missing<InternId>]) {
        let cells = self.index.cells();
        for (h, v) in hash.iter_mut().zip(value) {
            *h = hash_of(v);
        }
        let mut cell = [[0_u32; 2]; INTERN_GROUP];
        for (c, h) in cell.iter_mut().zip(hash.iter()).take(value.len()) {
            let home = self.index.home(*h);
            let Some(first) = cells.get(home) else {
                violation!(clause = "SET.12", "an index cell past its cells", cell = home);
            };
            *c = *first;
        }
        let mut row = [Missing::Absent; INTERN_GROUP];
        for ((r, [t, s]), h) in row.iter_mut().zip(&cell).zip(hash.iter()).take(value.len()) {
            if *s != EMPTY && *s != TOMB && *t == tag(*h) {
                *r = Missing::Present((*s - 1, self.row(*s - 1)));
            }
        }
        for ((((o, v), h), r), [_, s]) in out.iter_mut().zip(value).zip(hash.iter()).zip(&row).zip(&cell) {
            *o = match *r {
                Missing::Present((slot, r)) if self.value(r) == *v => {
                    Missing::Present(InternId::new(slot, r.generation))
                }
                _ if *s == EMPTY => Missing::Absent,
                _ => self.find_hashed(v, *h),
            };
        }
    }

    /// Calls `each` on the staged values a group at a time — values end to end in `bytes`, each ending where `ends`
    /// says — with the group's values and its answers in `out`.
    fn groups<'v>(
        (bytes, ends): (&'v [u8], &[u32]),
        out: &mut [Missing<InternId>],
        mut each: impl FnMut(&[&'v [u8]], &mut [Missing<InternId>]),
    ) {
        if out.len() != ends.len() {
            violation!(clause = "SET.12", "a lookup's answers not one a value", values = ends.len());
        }
        let mut from = 0;
        for (out, ends) in out.chunks_mut(INTERN_GROUP).zip(ends.chunks(INTERN_GROUP)) {
            let mut value: [&[u8]; INTERN_GROUP] = [&[]; INTERN_GROUP];
            for (v, end) in value.iter_mut().zip(ends) {
                let end = to_usize(*end);
                let Some(bytes) = bytes.get(from..end) else {
                    violation!(clause = "SET.12", "a staged value past its bytes", end = end);
                };
                (*v, from) = (bytes, end);
            }
            let Some(value) = value.get(..ends.len()) else {
                violation!(clause = "SET.12", "a group past its values", values = ends.len());
            };
            each(value, out);
        }
    }

    /// The ids of many values, each found without holding it, into `out`: the read an apply makes of its range's
    /// values before it stages the new ones.
    pub fn find_many(&self, staged: (&[u8], &[u32]), out: &mut [Missing<InternId>]) {
        let mut hash = [0; INTERN_GROUP];
        Self::groups(staged, out, |value, out| self.look(value, &mut hash, out));
    }

    /// Many values' ids, each held once more, into `out`, a group at a time: the group found as `find_many` finds it,
    /// its holds applied while its rows are near, and each value not found stored in order, so a value staged twice
    /// has one id.
    #[clause("SET.12")]
    pub fn intern_many(&mut self, staged: (&[u8], &[u32]), out: &mut [Missing<InternId>]) {
        let mut hash = [0; INTERN_GROUP];
        Self::groups(staged, out, |value, out| {
            self.look(value, &mut hash, out);
            for ((o, v), h) in out.iter_mut().zip(value).zip(hash) {
                match *o {
                    Missing::Present(id) => self.hold(id),
                    Missing::Absent => *o = Missing::Present(self.intern_hashed(v, h)),
                }
            }
        });
    }

    /// A value's id, held once more: found by its bytes, or stored and given the slot free longest.
    #[clause("SET.12")]
    pub fn intern(&mut self, bytes: &[u8]) -> InternId {
        self.intern_hashed(bytes, hash_of(bytes))
    }

    fn intern_hashed(&mut self, bytes: &[u8], hash: u64) -> InternId {
        if let Missing::Present(id) = self.find_hashed(bytes, hash) {
            self.hold(id);
            return id;
        }
        let (cells, ids) = INTERN_CELLS_PER_IDS;
        // Tombstones fill the cells between closes; the index is laid again from the rows before it passes two-thirds.
        if u64::from(self.index.occupied + 1) * u64::from(cells) > u64::from(self.index.cap) * u64::from(ids)
            && self.index.tombs > 0
        {
            self.rebuild_index();
        }
        let slot = self.slots.alloc().get();
        let generation = if to_usize(slot) == self.rows.len() {
            self.rows.push(Row { offset: 0, len: 0, count: 0, generation: 0, state: FREE, pad: [0; 2] });
            0
        } else {
            let old = self.row(slot);
            if old.state != FREE {
                violation!(clause = "SET.12", "a slot handed out that its id still holds", slot = slot);
            }
            let Some(g) = old.generation.checked_add(1) else {
                capacity_exceeded!("an interned slot's generations", u8::MAX, u64::from(old.generation) + 1);
            };
            g
        };
        let end = self.used + bytes.len();
        self.bytes.ensure(end);
        if let Some(to) = self.bytes.slice_mut(end).get_mut(self.used..end) {
            to.copy_from_slice(bytes);
        }
        let row =
            Row { offset: to_u32(self.used), len: to_u32(bytes.len()), count: 1, generation, state: LIVE, pad: [0; 2] };
        self.rows.set(Slot::new(slot), row);
        self.used = end;
        self.index.put(hash, slot);
        (self.live, self.ever) = (self.live + 1, self.ever + 1);
        InternId::new(slot, generation)
    }

    /// A value by its id, one row read, while the id is held or listed today.
    pub fn get(&self, id: InternId) -> Missing<&[u8]> {
        match self.held(id) {
            Missing::Present(row) => Missing::Present(self.value(row)),
            Missing::Absent => Missing::Absent,
        }
    }

    /// The holders an id counts, while it is held or listed today.
    pub fn count(&self, id: InternId) -> Missing<u32> {
        match self.held(id) {
            Missing::Present(row) => Missing::Present(row.count),
            Missing::Absent => Missing::Absent,
        }
    }

    /// Whether an id names a row retired for good: under `Never`, a retired id still reads as retired.
    #[must_use]
    pub fn is_retired(&self, id: InternId) -> bool {
        matches!(self.rows.get(id.slot()), Some(r) if r.generation == id.generation() && r.state == RETIRED)
    }

    /// One more holder of a live id.
    pub fn hold(&mut self, id: InternId) {
        let Missing::Present(mut row) = self.held(id) else {
            violation!(clause = "SET.12", "a hold of an id no longer interned", id = id.0);
        };
        let Some(count) = row.count.checked_add(1) else {
            capacity_exceeded!("an interned id's holders", u32::MAX, u64::from(row.count) + 1);
        };
        row.count = count;
        self.rows.set(id.slot(), row);
    }

    /// One holder fewer; an id held by none is listed to retire at the close, unless held again first.
    pub fn release(&mut self, id: InternId) {
        let Missing::Present(mut row) = self.held(id) else {
            violation!(clause = "SET.12", "a release of an id no longer interned", id = id.0);
        };
        let Some(count) = row.count.checked_sub(1) else {
            violation!(clause = "SET.12", "a release of an id no row holds", id = id.0);
        };
        row.count = count;
        if count == 0 && row.state == LIVE {
            row.state = LISTED;
            self.listed.push(id.slot().get());
        }
        self.rows.set(id.slot(), row);
    }

    /// The close: each id still held by none retires — its index cell tombstoned, its bytes dead, its slot freed for
    /// the next day or its row kept retired for good — and the values compacted once their dead bytes pass an eighth.
    /// Returns the ids retired.
    #[clause("SET.7")]
    pub fn close_day(&mut self) -> u64 {
        let mut retired = 0;
        for i in 0..self.listed.len() {
            let Some(&slot) = self.listed.slice().get(i) else { break };
            let mut row = self.row(slot);
            if row.count > 0 {
                row.state = LIVE;
                self.rows.set(Slot::new(slot), row);
                continue;
            }
            let hash = hash_of(self.value(row));
            self.index.remove(hash, slot);
            self.dead += to_usize(row.len);
            row.state = match self.reuse {
                Reuse::AfterClose => {
                    self.slots.release(Slot::new(slot));
                    FREE
                }
                Reuse::Never => RETIRED,
            };
            (row.offset, row.len) = (0, 0);
            self.rows.set(Slot::new(slot), row);
            self.live -= 1;
            retired += 1;
        }
        self.listed.truncate(0);
        self.slots.close_day();
        if self.dead * INTERN_DEAD_SHARE > self.used {
            self.compact();
        }
        retired
    }

    /// The live values copied in slot order to the arena's start, their rows' offsets rewritten, and the pages above
    /// released.
    fn compact(&mut self) {
        self.scratch.ensure(self.used);
        let mut at = 0;
        for slot in 0..to_u32(self.rows.len()) {
            let mut row = self.row(slot);
            if row.state != LIVE && row.state != LISTED {
                continue;
            }
            let (from, len) = (to_usize(row.offset), to_usize(row.len));
            if let (Some(src), Some(dst)) = (
                self.bytes.slice(self.used).get(from..from + len),
                self.scratch.slice_mut(self.used).get_mut(at..at + len),
            ) {
                dst.copy_from_slice(src);
            }
            row.offset = to_u32(at);
            self.rows.set(Slot::new(slot), row);
            at += len;
        }
        if let (Some(src), Some(dst)) =
            (self.scratch.slice(self.used).get(..at), self.bytes.slice_mut(self.used).get_mut(..at))
        {
            dst.copy_from_slice(src);
        }
        (self.used, self.dead) = (at, 0);
        self.bytes.release_above(at);
        self.scratch.release_above(0);
    }

    /// The index laid again from the rows, every held or listed value hashed in slot order: at a load, and when
    /// tombstones would carry it past two-thirds full. Returns the values indexed.
    pub(crate) fn rebuild_index(&mut self) -> u64 {
        self.index.clear();
        let mut n = 0;
        for slot in 0..to_u32(self.rows.len()) {
            let row = self.row(slot);
            if row.state == LIVE || row.state == LISTED {
                let hash = hash_of(self.value(row));
                self.index.put(hash, slot);
                n += 1;
            }
        }
        n
    }

    /// The declared reuse.
    #[must_use]
    pub fn reuse(&self) -> Reuse {
        self.reuse
    }

    /// Ids held or listed now.
    #[must_use]
    pub fn live(&self) -> u64 {
        self.live
    }

    /// The bytes of values in use, dead ones included.
    #[must_use]
    pub fn value_bytes(&self) -> usize {
        self.used
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.rows.bytes_committed()
            + self.bytes.bytes_committed()
            + self.scratch.bytes_committed()
            + self.slots.bytes_committed()
            + self.listed.bytes_committed()
            + self.index.cells.bytes_committed()
    }
}

impl<B: Backing> StoreStats for Interner<B> {
    fn rows_live(&self) -> u64 {
        self.live
    }

    /// Ids ever issued: under `Never`, the rows' growth.
    fn rows_ever(&self) -> u64 {
        self.ever
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

// Saving, loading and comparing an interner, which no day runs.
#[path = "intern_save.rs"]
mod save;

#[cfg(test)]
#[path = "intern_tests.rs"]
mod tests;
