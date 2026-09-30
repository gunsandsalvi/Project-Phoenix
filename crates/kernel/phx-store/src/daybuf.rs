//! The day's buffers: reserved address space a day fills and the next day clears by its length, so after the first
//! heaviest day of each kind a day commits no page and allocates nothing.

use phx_macros::opening;
use phx_num::capacity_exceeded;
use phx_num::violation::{CapacityExceeded, Key, raise_capacity};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::consts::DAY_KINDS;
use crate::pod::Pod;
use crate::region::Region;
use crate::stats::StoreStats;

/// A buffer a day fills: its name, its reservation at its declared capacity, the length the day has written, and the
/// longest it has been on each kind of day. Pages are committed as it first grows past them and never released; the
/// kernel never zeroes them, so what a day reads is only what it wrote.
#[derive(Debug)]
pub struct DayBuf<T: Pod, B: Backing = SystemBacking> {
    name: &'static str,
    region: Region<T, B>,
    len: usize,
    highs: [usize; DAY_KINDS],
}

impl<T: Pod, B: Backing> DayBuf<T, B> {
    /// A buffer of at most `capacity` elements, reserved and not yet committed.
    pub fn new(space: &mut AddressSpace, name: &'static str, capacity: usize) -> DayBuf<T, B> {
        DayBuf { name, region: Region::reserve(space, capacity), len: 0, highs: [0; DAY_KINDS] }
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.region.capacity()
    }

    /// Empties the buffer, keeping its pages.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Commits the pages `n` elements need; more than the reservation stops the run naming the buffer.
    fn grow(&mut self, n: usize) {
        if n > self.region.capacity() {
            raise_capacity(CapacityExceeded {
                what: self.name,
                declared: Key::key(self.region.capacity()),
                needed: Key::key(n),
            });
        }
        self.region.ensure(n);
    }

    pub fn push(&mut self, value: T) {
        let n = self.len + 1;
        // Past the committed pages, or past the capacity a page's tail may hold beyond, the slow path commits or stops.
        if n > self.region.committed_len() || n > self.region.capacity() {
            self.grow(n);
        }
        if let Some(cell) = self.region.slice_mut(n).last_mut() {
            *cell = value;
        }
        self.len = n;
    }

    pub fn extend(&mut self, values: &[T]) {
        let n = self.len + values.len();
        self.grow(n);
        if let Some(cells) = self.region.slice_mut(n).get_mut(self.len..) {
            cells.copy_from_slice(values);
        }
        self.len = n;
    }

    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.region.slice(self.len)
    }

    /// The buffer at `len` elements to be written in place: those beyond its old length hold what an earlier day left,
    /// and the caller writes every one it reads.
    pub fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        self.grow(len);
        self.len = len;
        self.region.slice_mut(len)
    }

    /// Records the day's length as the longest on its kind of day, when it is.
    pub fn mark(&mut self, day_kind: usize) {
        if let Some(high) = self.highs.get_mut(day_kind)
            && self.len > *high
        {
            *high = self.len;
        }
    }

    /// The longest the buffer has been on each kind of day.
    #[must_use]
    pub fn highs(&self) -> [usize; DAY_KINDS] {
        self.highs
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.region.bytes_committed()
    }
}

/// A length as a count.
fn wide(n: usize) -> u64 {
    u64::try_from(n).unwrap_or_else(|_| capacity_exceeded!("a buffer's length", u64::MAX, n))
}

impl<T: Pod, B: Backing> StoreStats for DayBuf<T, B> {
    fn rows_live(&self) -> u64 {
        wide(self.len)
    }

    fn rows_ever(&self) -> u64 {
        self.highs.iter().fold(wide(self.len), |a, h| if wide(*h) > a { wide(*h) } else { a })
    }

    fn bytes(&self) -> u64 {
        wide(self.region.bytes_committed())
    }
}

/// A traversal's buffers, one to each chunk's handler: each chunk's are filled by the worker that holds the chunk,
/// and read or joined in (chunk, handler) order, so what they hold does not depend on which worker filled which.
#[derive(Debug)]
pub struct DayBufs<T: Pod, B: Backing = SystemBacking> {
    handlers: usize,
    bufs: Vec<DayBuf<T, B>>,
}

impl<T: Pod, B: Backing> DayBufs<T, B> {
    /// `chunks × handlers` buffers of `capacity` elements each.
    #[opening]
    pub fn new(
        space: &mut AddressSpace,
        name: &'static str,
        (chunks, handlers): (usize, usize),
        capacity: usize,
    ) -> DayBufs<T, B> {
        DayBufs { handlers, bufs: (0..chunks * handlers).map(|_| DayBuf::new(space, name, capacity)).collect() }
    }

    /// Empties every buffer, keeping their pages.
    pub fn clear(&mut self) {
        self.bufs.iter_mut().for_each(DayBuf::clear);
    }

    /// Each chunk's handlers' buffers, for the worker that holds the chunk.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = &mut [DayBuf<T, B>]> {
        self.bufs.chunks_mut(self.handlers)
    }

    /// What a chunk's handler wrote, read in place.
    #[must_use]
    pub fn get(&self, chunk: usize, handler: usize) -> &[T] {
        self.bufs.get(chunk * self.handlers + handler).map_or(&[], DayBuf::as_slice)
    }

    /// Every buffer's elements in (chunk, handler) order, appended to `into`.
    pub fn join(&self, into: &mut DayBuf<T, B>) {
        let total: usize = self.bufs.iter().map(DayBuf::len).sum();
        let at = into.len();
        let out = into.as_mut_slice(at + total);
        let mut next = at;
        for b in &self.bufs {
            let end = next + b.len();
            if let Some(cells) = out.get_mut(next..end) {
                cells.copy_from_slice(b.as_slice());
            }
            next = end;
        }
    }

    /// Records each buffer's day's length as the longest on its kind of day, when it is.
    pub fn mark(&mut self, day_kind: usize) {
        self.bufs.iter_mut().for_each(|b| b.mark(day_kind));
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.bufs.iter().map(DayBuf::bytes_committed).sum()
    }
}

#[cfg(test)]
#[path = "daybuf_tests.rs"]
mod tests;
