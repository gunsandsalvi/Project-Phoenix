use phx_id::{Slot, TableId};
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::{Column, chunk_start};
use crate::convert::to_usize;
use crate::pod::Pod;
use crate::region::Region;

/// A table's slots: released slots return to use only after the day closes, lowest first, so which slot a row gets
/// depends only on the order of allocations.
#[derive(Debug)]
pub struct SlotAlloc<B: Backing = SystemBacking> {
    live: Region<u64, B>,
    /// Free slots in descending order, so the lowest is taken from the end.
    free: Region<u32, B>,
    n_free: usize,
    released: Region<u32, B>,
    n_released: usize,
    high: u32,
    max: u32,
}

const BITS: u32 = u64::BITS;

fn bit(slot: Slot) -> (usize, u64) {
    (to_usize(slot.get() / BITS), 1 << (slot.get() % BITS))
}

impl<B: Backing> SlotAlloc<B> {
    pub fn new(space: &mut AddressSpace, max: u32) -> SlotAlloc<B> {
        let words = to_usize(max.div_ceil(BITS));
        SlotAlloc {
            live: Region::reserve(space, words),
            free: Region::reserve(space, to_usize(max)),
            n_free: 0,
            released: Region::reserve(space, to_usize(max)),
            n_released: 0,
            high: 0,
            max,
        }
    }

    /// Slots ever handed out: every live slot lies below.
    #[must_use]
    pub fn high_water(&self) -> u32 {
        self.high
    }

    #[must_use]
    pub fn is_live(&self, slot: Slot) -> bool {
        let (word, mask) = bit(slot);
        slot.get() < self.high
            && self.live.slice(to_usize(self.high.div_ceil(BITS))).get(word).is_some_and(|w| w & mask != 0)
    }

    fn set_live(&mut self, slot: Slot, live: bool) {
        let words = to_usize(self.high.div_ceil(BITS));
        self.live.ensure(words);
        let (word, mask) = bit(slot);
        if let Some(w) = self.live.slice_mut(words).get_mut(word) {
            *w = if live { *w | mask } else { *w & !mask };
        }
    }

    /// The lowest free slot, or a new one above every slot handed out.
    pub fn alloc(&mut self) -> Slot {
        let slot = if let Some(last) = self.n_free.checked_sub(1) {
            let Some(free) = self.free.slice(self.n_free).get(last).copied() else {
                violation!(clause = "SET.12", "a free list shorter than its count", count = self.n_free);
            };
            self.n_free = last;
            Slot::new(free)
        } else {
            if self.high == self.max {
                capacity_exceeded!("table slots", self.max, u64::from(self.max) + 1);
            }
            self.high += 1;
            Slot::new(self.high - 1)
        };
        self.set_live(slot, true);
        slot
    }

    /// Frees a live slot; it is handed out again only after the day closes.
    pub fn release(&mut self, slot: Slot) {
        if !self.is_live(slot) {
            violation!(clause = "SET.12", "a slot released that is not in use", slot = slot.get());
        }
        self.set_live(slot, false);
        let n = self.n_released + 1;
        self.released.ensure(n);
        if let Some(cell) = self.released.slice_mut(n).last_mut() {
            *cell = slot.get();
        }
        self.n_released = n;
    }

    /// Merges the day's released slots into the free list, which stays in descending order.
    pub fn close_day(&mut self) {
        let total = self.n_free + self.n_released;
        self.free.ensure(total);
        let released = self.released.slice_mut(self.n_released);
        released.sort_unstable_by(|x, y| y.cmp(x));
        let free = self.free.slice_mut(total);
        // Merged from the back, the smallest first: the write position never passes an unread free slot.
        let (mut from_free, mut from_released) = (self.n_free, self.n_released);
        for at in (0..total).rev() {
            let take_free = match (from_free.checked_sub(1), from_released.checked_sub(1)) {
                (Some(f), Some(r)) => free.get(f) < released.get(r),
                (Some(_), None) => true,
                (None, _) => false,
            };
            let value = if take_free {
                from_free -= 1;
                free.get(from_free).copied()
            } else {
                from_released -= 1;
                released.get(from_released).copied()
            };
            if let (Some(cell), Some(v)) = (free.get_mut(at), value) {
                *cell = v;
            }
        }
        self.n_free = total;
        self.n_released = 0;
    }

    /// Live slots in ascending order, a word of the live bits at a time.
    pub fn live_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        live_in(self.live_words())
    }

    /// The live bits, a slot to a bit, as far as the high water.
    #[must_use]
    pub fn live_words(&self) -> &[u64] {
        self.live.slice(to_usize(self.high.div_ceil(BITS)))
    }
}

/// A table's geometry and slot space, shared by every column its owner keeps.
#[derive(Debug)]
pub struct Table<B: Backing = SystemBacking> {
    pub id: TableId,
    max_rows: u32,
    rows_per_chunk: u32,
    pub slots: SlotAlloc<B>,
}

impl<B: Backing> Table<B> {
    pub fn new(space: &mut AddressSpace, id: TableId, max_rows: u32, rows_per_chunk: u32) -> Table<B> {
        if !rows_per_chunk.is_power_of_two() {
            violation!(clause = "SET.12", "rows per chunk not a power of two", rows_per_chunk = rows_per_chunk);
        }
        Table { id, max_rows, rows_per_chunk, slots: SlotAlloc::new(space, max_rows) }
    }

    /// A column in this table's slot space.
    pub fn column<T: Pod>(&self, space: &mut AddressSpace) -> Column<T, B> {
        Column::new(space, self.max_rows, self.rows_per_chunk)
    }

    #[must_use]
    pub fn rows_per_chunk(&self) -> u32 {
        self.rows_per_chunk
    }
}

/// One chunk of several columns of a table at once.
#[derive(Debug)]
pub struct TableChunk<R> {
    pub chunk: u32,
    pub first: Slot,
    pub rows: R,
}

/// Several columns of one table viewed chunk by chunk together, each chunk owned by one handler.
pub trait TableChunks {
    type Rows;
    fn chunks(self) -> impl Iterator<Item = TableChunk<Self::Rows>>;
}

fn same_geometry(columns: &[(usize, usize)]) -> (usize, usize) {
    let Some(first) = columns.first().copied() else {
        violation!(clause = "SET.12", "a table view of no columns");
    };
    if columns.iter().any(|c| *c != first) {
        violation!(clause = "SET.12", "columns of one table with different lengths or chunks");
    }
    first
}

macro_rules! table_chunks {
    ($($c:ident: $t:ident),+) => {
        impl<'a, $($t: Pod,)+ B: Backing> TableChunks for ($(&'a mut Column<$t, B>,)+) {
            type Rows = ($(&'a mut [$t],)+);

            fn chunks(self) -> impl Iterator<Item = TableChunk<Self::Rows>> {
                let ($($c,)+) = self;
                let (len, rpc) = same_geometry(&[$(($c.len(), $c.rows_per_chunk())),+]);
                $(let mut $c = $c.slice_mut().chunks_mut(rpc);)+
                let Ok(count) = u32::try_from(len.div_ceil(rpc)) else {
                    capacity_exceeded!("chunks", u32::MAX, len.div_ceil(rpc));
                };
                (0..count).map_while(move |chunk| {
                    Some(TableChunk { chunk, first: chunk_start(chunk, rpc), rows: ($($c.next()?,)+) })
                })
            }
        }
    };
}

table_chunks!(a: A);
table_chunks!(a: A, b: C);
table_chunks!(a: A, b: C, c: D);
table_chunks!(a: A, b: C, c: D, d: E);
table_chunks!(a: A, b: C, c: D, d: E, e: F);
table_chunks!(a: A, b: C, c: D, d: E, e: F, f: G);

impl<B: Backing> crate::save::Saved for SlotAlloc<B> {
    fn save(&self, w: &mut crate::save::Writer<'_>) {
        self.high.save(w);
        self.max.save(w);
        self.live.save_prefix(to_usize(self.high.div_ceil(BITS)), w, crate::Transform::Plain);
        self.free.save_prefix(self.n_free, w, crate::Transform::Plain);
        self.released.save_prefix(self.n_released, w, crate::Transform::Plain);
    }

    fn load(r: &mut crate::save::Reader<'_>) -> Result<SlotAlloc<B>, crate::save::LoadError> {
        let (high, max) = (u32::load(r)?, u32::load(r)?);
        let (live, words) = Region::load_prefix(r, crate::Transform::Plain)?;
        let (free, n_free) = Region::load_prefix(r, crate::Transform::Plain)?;
        let (released, n_released) = Region::load_prefix(r, crate::Transform::Plain)?;
        let wrong = high > max
            || words != to_usize(high.div_ceil(BITS))
            || free.capacity() != to_usize(max)
            || released.capacity() != to_usize(max);
        if wrong {
            return Err(crate::save::LoadError::Invalid("a slot allocator whose parts disagree".to_owned()));
        }
        Ok(SlotAlloc { live, free, n_free, released, n_released, high, max })
    }
}

impl<B: Backing> crate::save::Saved for Table<B> {
    fn save(&self, w: &mut crate::save::Writer<'_>) {
        self.id.save(w);
        self.max_rows.save(w);
        self.rows_per_chunk.save(w);
        self.slots.save(w);
    }

    fn load(r: &mut crate::save::Reader<'_>) -> Result<Table<B>, crate::save::LoadError> {
        let (id, max_rows, rows_per_chunk) = (TableId::load(r)?, u32::load(r)?, u32::load(r)?);
        let slots = SlotAlloc::load(r)?;
        if !rows_per_chunk.is_power_of_two() || slots.max != max_rows {
            return Err(crate::save::LoadError::Invalid("a table whose geometry disagrees".to_owned()));
        }
        Ok(Table { id, max_rows, rows_per_chunk, slots })
    }
}

/// Whether a table's live bits mark a slot live.
#[must_use]
pub fn live_at(words: &[u64], slot: Slot) -> bool {
    let (word, mask) = bit(slot);
    words.get(word).is_some_and(|w| w & mask != 0)
}

/// The live slots a table's live bits mark, in ascending order.
pub fn live_in(words: &[u64]) -> impl Iterator<Item = Slot> + '_ {
    (0_u32..).zip(words.iter()).flat_map(|(w, bits)| {
        let mut rest = *bits;
        std::iter::from_fn(move || {
            (rest != 0).then(|| {
                let bit = rest.trailing_zeros();
                rest &= rest - 1;
                Slot::new(w * BITS + bit)
            })
        })
    })
}

/// The live slots among `first`, `first + step`, `first + 2·step`…, in ascending order: a rolling slice read without
/// walking the slots between.
pub fn live_every(words: &[u64], first: u32, step: u32) -> impl Iterator<Item = Slot> + '_ {
    (first..)
        .step_by(to_usize(step))
        .map_while(move |s| (to_usize(s / BITS) < words.len()).then_some(Slot::new(s)))
        .filter(move |s| live_at(words, *s))
}

#[cfg(test)]
mod tests {
    use phx_id::{Slot, TableId};

    use super::{SlotAlloc, Table, TableChunks};
    use crate::backing::{AddressSpace, HeapBacking};

    type Heap = HeapBacking<4096>;

    #[test]
    fn live_every_reads_only_the_slice() {
        let words = [0b1011_u64, 1 << 1];
        let every: Vec<u32> = super::live_every(&words, 1, 2).map(Slot::get).collect();
        assert_eq!(every, vec![1, 3, 65]);
        assert_eq!(super::live_every(&words, 0, 2).count(), 1);
    }

    #[test]
    fn slot_recycling_is_ordered() {
        let mut space = AddressSpace::empty();
        let mut s: SlotAlloc<Heap> = SlotAlloc::new(&mut space, 100);
        let first: Vec<u32> = (0..10).map(|_| s.alloc().get()).collect();
        assert_eq!(first, (0..10).collect::<Vec<_>>());
        for n in [7, 3, 5] {
            s.release(Slot::new(n));
        }
        assert_eq!(s.alloc().get(), 10, "released slots wait for the day's close");
        s.close_day();
        s.release(Slot::new(1));
        s.close_day();
        let next: Vec<u32> = (0..6).map(|_| s.alloc().get()).collect();
        assert_eq!(next, vec![1, 3, 5, 7, 11, 12]);
        assert_eq!(s.live_slots().count(), 13);
    }

    #[test]
    fn releasing_a_free_slot_stops_the_run() {
        let mut space = AddressSpace::empty();
        let mut s: SlotAlloc<Heap> = SlotAlloc::new(&mut space, 10);
        let slot = s.alloc();
        s.release(slot);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.release(slot)));
        let payload = caught.expect_err("a double release stops the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("SET.12"));
    }

    #[test]
    fn table_chunks_view_columns_together() {
        let mut space = AddressSpace::empty();
        let table: Table<Heap> = Table::new(&mut space, TableId::new(1), 64, 8);
        let mut weights = table.column::<u32>(&mut space);
        let mut totals = table.column::<i64>(&mut space);
        weights.extend(&[1; 20]);
        totals.extend(&[0; 20]);
        for k in (&mut weights, &mut totals).chunks() {
            let (w, t) = k.rows;
            for (wi, ti) in w.iter().zip(t.iter_mut()) {
                *ti = i64::from(*wi) * i64::from(k.first.get());
            }
        }
        assert_eq!(totals.get(Slot::new(17)), Some(16));
        assert_eq!((&mut weights, &mut totals).chunks().count(), 3);
    }
}
