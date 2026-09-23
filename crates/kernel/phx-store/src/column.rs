use phx_id::Slot;
use phx_num::violation::Key;
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::convert::to_usize;
use crate::pod::{Pod, as_bytes_mut};
use crate::region::Region;

/// One field of every row of a table, in reserved address space: it grows by committing pages and never moves.
#[derive(Debug)]
pub struct Column<T: Pod, B: Backing = SystemBacking> {
    region: Region<T, B>,
    len: usize,
    rows_per_chunk: usize,
}

/// One chunk's rows of a column, which a parallel handler owns alone.
#[derive(Debug)]
pub struct ChunkMut<'a, T> {
    pub chunk: u32,
    pub first: Slot,
    pub rows: &'a mut [T],
}

pub(crate) fn chunk_start(chunk: u32, rows_per_chunk: usize) -> Slot {
    let Some(first) = u32::try_from(rows_per_chunk).ok().and_then(|r| r.checked_mul(chunk)) else {
        capacity_exceeded!("slots", u32::MAX, u64::from(chunk) + 1);
    };
    Slot::new(first)
}

impl<T: Pod, B: Backing> Column<T, B> {
    /// A column of at most `max_rows`, chunked by `rows_per_chunk`, a power of two.
    pub fn new(space: &mut AddressSpace, max_rows: u32, rows_per_chunk: u32) -> Column<T, B> {
        if !rows_per_chunk.is_power_of_two() {
            violation!(clause = "SET.12", "rows per chunk not a power of two", rows_per_chunk = rows_per_chunk);
        }
        let (Ok(max), Ok(rpc)) = (usize::try_from(max_rows), usize::try_from(rows_per_chunk)) else {
            capacity_exceeded!("column rows", usize::MAX, max_rows);
        };
        Column { region: Region::reserve(space, max), len: 0, rows_per_chunk: rpc }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Rows the column's reservation can ever hold.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.region.capacity()
    }

    #[must_use]
    pub fn rows_per_chunk(&self) -> usize {
        self.rows_per_chunk
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.region.bytes_committed()
    }

    fn grow(&mut self, n: usize) -> usize {
        let Some(end) = self.len.checked_add(n).filter(|e| *e <= self.region.capacity()) else {
            capacity_exceeded!("column rows", self.region.capacity(), Key::key(self.len) + Key::key(n));
        };
        self.region.ensure(end);
        end
    }

    /// Appends a row, at the slot one past the last.
    pub fn push(&mut self, value: T) {
        let end = self.grow(1);
        if let Some(cell) = self.region.slice_mut(end).last_mut() {
            *cell = value;
        }
        self.len = end;
    }

    pub fn extend(&mut self, values: &[T]) {
        let start = self.len;
        let end = self.grow(values.len());
        if let Some(rows) = self.region.slice_mut(end).get_mut(start..) {
            rows.copy_from_slice(values);
        }
        self.len = end;
    }

    /// Appends `n` rows of zero bytes for a caller to fill, as decoding does; zero is a value of every stored type.
    pub(crate) fn append_zeroed(&mut self, n: usize) -> &mut [T] {
        let start = self.len;
        self.len = self.grow(n);
        let Some(rows) = self.region.slice_mut(self.len).get_mut(start..) else {
            violation!(clause = "SET.12", "a column shorter than it just grew", len = start);
        };
        as_bytes_mut(rows).fill(0);
        rows
    }

    /// Drops rows past `len`, as a failed decode does with the rows it appended.
    pub(crate) fn truncate(&mut self, len: usize) {
        if len < self.len {
            self.len = len;
        }
    }

    /// The row at a slot, or none past the column's end.
    #[must_use]
    pub fn get(&self, slot: Slot) -> Option<T> {
        self.slice().get(to_usize(slot.get())).copied()
    }

    /// Overwrites the row at a slot, which must exist.
    pub fn set(&mut self, slot: Slot, value: T) {
        let len = self.len;
        let Some(cell) = self.slice_mut().get_mut(to_usize(slot.get())) else {
            violation!(clause = "SET.12", "a write past a column's end", slot = slot.get(), len = len);
        };
        *cell = value;
    }

    #[must_use]
    pub fn slice(&self) -> &[T] {
        self.region.slice(self.len)
    }

    pub fn slice_mut(&mut self) -> &mut [T] {
        self.region.slice_mut(self.len)
    }

    /// Disjoint mutable views of each chunk, in chunk order.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = ChunkMut<'_, T>> {
        let rpc = self.rows_per_chunk;
        (0_u32..).zip(self.region.slice_mut(self.len).chunks_mut(rpc)).map(move |(chunk, rows)| ChunkMut {
            chunk,
            first: chunk_start(chunk, rpc),
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;

    use super::Column;
    use crate::backing::{AddressSpace, HeapBacking, SystemBacking};

    #[test]
    fn column_never_moves() {
        let n: u32 = if cfg!(miri) { 10_000 } else { 1_000_000 };
        let mut space = AddressSpace::empty();
        let mut c: Column<u64, SystemBacking> = Column::new(&mut space, n, 4096);
        c.push(0);
        let first = c.slice().as_ptr();
        for i in 1..u64::from(n) {
            c.push(i);
        }
        assert_eq!(c.slice().as_ptr(), first);
        assert_eq!(c.len(), usize::try_from(n).unwrap());
        assert_eq!(c.get(Slot::new(n - 1)), Some(u64::from(n) - 1));
        assert_eq!(c.get(Slot::new(n)), None);
    }

    #[test]
    fn chunks_mut_are_disjoint() {
        let mut space = AddressSpace::empty();
        let mut c: Column<u32, HeapBacking<4096>> = Column::new(&mut space, 100, 16);
        c.extend(&[0; 70]);
        let mut chunks: Vec<_> = c.chunks_mut().collect();
        assert_eq!(
            chunks.iter().map(|k| (k.chunk, k.first.get(), k.rows.len())).collect::<Vec<_>>(),
            vec![(0, 0, 16), (1, 16, 16), (2, 32, 16), (3, 48, 16), (4, 64, 6)]
        );
        // Each view is written while all are alive: they cannot overlap.
        for k in &mut chunks {
            let chunk = k.chunk;
            k.rows.iter_mut().for_each(|r| *r = chunk);
        }
        assert_eq!(c.get(Slot::new(15)), Some(0));
        assert_eq!(c.get(Slot::new(16)), Some(1));
        assert_eq!(c.get(Slot::new(69)), Some(4));
    }
}
