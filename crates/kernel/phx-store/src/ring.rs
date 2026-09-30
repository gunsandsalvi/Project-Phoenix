//! Horizon rings: every kept history — marks, events, filed statements, weather, credit records — an append-only dated
//! ring with its declared horizon, pruned by whole chunks, never by a pass over rows.

use phx_macros::{Pod, clause, opening};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::convert::{to_u32, to_u64, to_usize};
use crate::daybuf::DayBuf;
use crate::index::Index;
use crate::pod::Pod;
use crate::region::Region;
use crate::stats::StoreStats;

/// A live chunk: the days it holds, its first row's ordinal, where its rows lie, and how many it holds.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct ChunkHeader {
    first_day: u32,
    last_day: u32,
    first_ordinal: u64,
    at: u32,
    len: u32,
}

/// A ring's rows: each row's day and payload in chunks of a declared row count; the live chunks' headers in order, as
/// a ring of headers from `head`; the chunks pruned, to be taken again; the ordinals live, `[base, next)`; its horizon
/// and the floors its owner sets.
///
/// A subject index, where a ring's kind declares one, is its owner's lazy index from a subject to row ordinals, left
/// out of the save and rebuilt from the live rows: `index_since` fills it, `of_subject` walks it, and an entry below
/// the ring's base is dead by construction.
#[derive(Debug)]
pub struct HorizonRing<T: Pod, B: Backing = SystemBacking> {
    chunk_rows: u32,
    days: Region<u32, B>,
    rows: Region<T, B>,
    headers: Column<ChunkHeader, B>,
    head: usize,
    live: usize,
    free: Column<u32, B>,
    made: u32,
    max_chunks: u32,
    base: u64,
    next: u64,
    horizon: u32,
    floors: Column<[u32; 2], B>,
}

/// A row's place within its chunk, which a chunk's row count bounds.
fn offset(n: u64) -> usize {
    match usize::try_from(n) {
        Ok(n) => n,
        Err(_) => violation!(clause = "SET.12", "a ring's row past its chunk", offset = n),
    }
}

impl<T: Pod, B: Backing> HorizonRing<T, B> {
    /// A ring of chunks of `chunk_rows` rows, at most `max_chunks` of them, keeping `horizon` days.
    ///
    /// # Errors
    /// A ring of no rows, or of more rows than a slot counts.
    #[opening]
    pub fn new(
        space: &mut AddressSpace,
        (chunk_rows, max_chunks): (u32, u32),
        horizon: u32,
    ) -> Result<HorizonRing<T, B>, String> {
        if chunk_rows == 0 || max_chunks == 0 {
            return Err("a ring of no rows".to_owned());
        }
        let Some(rows) = chunk_rows.checked_mul(max_chunks) else {
            return Err(format!("a ring of {max_chunks} chunks of {chunk_rows} rows, past a slot's width"));
        };
        let chunk = crate::consts::SUMTREE_ROWS_PER_CHUNK;
        let mut headers = Column::new(space, max_chunks, chunk);
        for _ in 0..max_chunks {
            headers.push(ChunkHeader { first_day: 0, last_day: 0, first_ordinal: 0, at: 0, len: 0 });
        }
        Ok(HorizonRing {
            chunk_rows,
            days: Region::reserve(space, to_usize(rows)),
            rows: Region::reserve(space, to_usize(rows)),
            headers,
            head: 0,
            live: 0,
            free: Column::new(space, max_chunks, chunk),
            made: 0,
            max_chunks,
            base: 0,
            next: 0,
            horizon,
            floors: Column::new(space, crate::consts::RING_FLOORS, chunk),
        })
    }

    fn header(&self, i: usize) -> ChunkHeader {
        match self.headers.slice().get((self.head + i) % to_usize(self.max_chunks)) {
            Some(h) => *h,
            None => violation!(clause = "SET.12", "a ring's chunk past its headers", chunk = i),
        }
    }

    fn put_header(&mut self, i: usize, h: ChunkHeader) {
        let at = (self.head + i) % to_usize(self.max_chunks);
        match self.headers.slice_mut().get_mut(at) {
            Some(cell) => *cell = h,
            None => violation!(clause = "SET.12", "a ring's chunk past its headers", chunk = i),
        }
    }

    /// A chunk to write: one pruned, else a new one.
    fn take_chunk(&mut self) -> u32 {
        if let Some(&at) = self.free.slice().last() {
            self.free.truncate(self.free.len() - 1);
            return at;
        }
        if self.made == self.max_chunks {
            capacity_exceeded!("a ring's chunks", self.max_chunks, u64::from(self.made) + 1);
        }
        self.made += 1;
        let end = to_usize(self.made * self.chunk_rows);
        self.days.ensure(end);
        self.rows.ensure(end);
        self.made - 1
    }

    /// A day's rows appended in the order the caller's stage fixes: they begin a new chunk unless they fit the last
    /// one's room, and a day of more rows than a chunk fills whole chunks. Returns the first row's ordinal. A day
    /// before the last appended stops the run.
    #[clause("TIME.9", "TIME.10")]
    pub fn append(&mut self, day: u32, rows: &[T]) -> u64 {
        if self.live > 0 && self.header(self.live - 1).last_day > day {
            violation!(clause = "TIME.10", "a ring's rows appended before its last day", day = day);
        }
        let first = self.next;
        let per = to_usize(self.chunk_rows);
        let mut rest = rows;
        while !rest.is_empty() {
            let tail = if self.live > 0 { Missing::Present(self.header(self.live - 1)) } else { Missing::Absent };
            let room = match tail {
                Missing::Present(t) => per - to_usize(t.len),
                Missing::Absent => 0,
            };
            let fits = rest.len() <= room || (room > 0 && rest.len() > per);
            let (i, mut h) = match tail {
                Missing::Present(t) if fits => (self.live - 1, t),
                _ => {
                    if self.live == to_usize(self.max_chunks) {
                        capacity_exceeded!("a ring's live chunks", self.max_chunks, self.live + 1);
                    }
                    let at = self.take_chunk();
                    self.live += 1;
                    (self.live - 1, ChunkHeader { first_day: day, last_day: day, first_ordinal: self.next, at, len: 0 })
                }
            };
            let room = per - to_usize(h.len);
            let (now, later) = rest.split_at(if rest.len() < room { rest.len() } else { room });
            let start = to_usize(h.at) * per + to_usize(h.len);
            let end = to_usize(self.made) * per;
            let (Some(d), Some(r)) = (
                self.days.slice_mut(end).get_mut(start..start + now.len()),
                self.rows.slice_mut(end).get_mut(start..start + now.len()),
            ) else {
                violation!(clause = "SET.12", "a ring's chunk past its rows", chunk = h.at);
            };
            d.fill(day);
            r.copy_from_slice(now);
            h.len += to_u32(now.len());
            h.last_day = day;
            self.put_header(i, h);
            self.next += to_u64(now.len());
            rest = later;
        }
        first
    }

    /// Where a row lies, by its ordinal, while it is live.
    fn place(&self, ordinal: u64) -> Missing<usize> {
        if ordinal < self.base || ordinal >= self.next {
            return Missing::Absent;
        }
        let (mut lo, mut hi) = (0, self.live);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.header(mid).first_ordinal <= ordinal {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let h = self.header(lo - 1);
        Missing::Present(to_usize(h.at) * to_usize(self.chunk_rows) + offset(ordinal - h.first_ordinal))
    }

    /// A live row and its day, by its ordinal; none once pruned.
    pub fn row(&self, ordinal: u64) -> Missing<(u32, T)> {
        let Missing::Present(at) = self.place(ordinal) else { return Missing::Absent };
        let end = to_usize(self.made) * to_usize(self.chunk_rows);
        match (self.days.slice(end).get(at), self.rows.slice(end).get(at)) {
            (Some(d), Some(r)) => Missing::Present((*d, *r)),
            _ => violation!(clause = "SET.12", "a live ordinal with no row", ordinal = ordinal),
        }
    }

    /// The live chunks in order from the `from`th: each header, its days and its rows.
    fn chunks(&self, from: usize) -> impl Iterator<Item = (ChunkHeader, &[u32], &[T])> {
        let per = to_usize(self.chunk_rows);
        let end = to_usize(self.made) * per;
        let (days, rows) = (self.days.slice(end), self.rows.slice(end));
        (from..self.live).map(move |i| {
            let h = self.header(i);
            let start = to_usize(h.at) * per;
            let (Some(d), Some(r)) =
                (days.get(start..start + to_usize(h.len)), rows.get(start..start + to_usize(h.len)))
            else {
                violation!(clause = "SET.12", "a ring's chunk past its rows", chunk = h.at);
            };
            (h, d, r)
        })
    }

    /// Calls `each` on the live rows whose days lie in `[from, to]`, a chunk's run at a time in ordinal order: its
    /// first ordinal, its rows' days and its rows. The first chunk is found by binary search on the chunks' last days,
    /// each run's ends by binary search on its days, so the ring reads no row outside the days and the reader's own
    /// loop is the only one over rows.
    pub fn range(&self, (from, to): (u32, u32), mut each: impl FnMut(u64, &[u32], &[T])) {
        let (mut lo, mut hi) = (0, self.live);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.header(mid).last_day < from {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        for (h, d, r) in self.chunks(lo) {
            if h.first_day > to {
                break;
            }
            let (start, end) = (d.partition_point(|day| *day < from), d.partition_point(|day| *day <= to));
            if let (Some(d), Some(r)) = (d.get(start..end), r.get(start..end))
                && !d.is_empty()
            {
                each(h.first_ordinal + to_u64(start), d, r);
            }
        }
    }

    /// The owner's floor on the horizon from a day on: the policy in force from then, which pruning never goes below.
    pub fn set_floor(&mut self, day: u32, days: u32) {
        if self.floors.slice().last().is_some_and(|&[from, _]| from > day) {
            violation!(clause = "TIME.10", "a ring's floor set before its last", day = day);
        }
        self.floors.push([day, days]);
    }

    /// The horizon on a day: the ring's own, or the floor in force that day where it is longer.
    #[must_use]
    pub fn horizon_on(&self, day: u32) -> u32 {
        match self.floors.slice().iter().take_while(|&&[from, _]| from <= day).last() {
            Some(&[_, floor]) if floor > self.horizon => floor,
            _ => self.horizon,
        }
    }

    /// Every leading chunk whose last day lies before today less the horizon dropped whole, its rows never visited,
    /// its space taken again by later appends. Returns the rows dropped.
    #[clause("SET.13")]
    pub fn prune(&mut self, today: u32) -> u64 {
        let horizon = u64::from(self.horizon_on(today));
        let mut dropped = 0;
        while self.live > 0 {
            let h = self.header(0);
            if u64::from(h.last_day) + horizon >= u64::from(today) {
                break;
            }
            self.free.push(h.at);
            self.head = (self.head + 1) % to_usize(self.max_chunks);
            self.live -= 1;
            self.base = h.first_ordinal + u64::from(h.len);
            dropped += u64::from(h.len);
        }
        dropped
    }

    /// Enters every live row from ordinal `from` on into a subject index, keyed by the subject its row names: after an
    /// append, from its first ordinal; at a load, from the base. Returns the rows entered.
    pub fn index_since(&self, from: u64, index: &mut Index<B>, of: impl Fn(&T) -> u64) -> u64 {
        let mut entered = 0;
        for (h, _, r) in self.chunks(0) {
            let end = h.first_ordinal + u64::from(h.len);
            if end <= from {
                continue;
            }
            let skip = if from > h.first_ordinal { offset(from - h.first_ordinal) } else { 0 };
            for (k, row) in (h.first_ordinal..).zip(r).skip(skip) {
                let Ok(member) = u32::try_from(k) else {
                    capacity_exceeded!("a ring's indexed ordinals", u32::MAX, k);
                };
                index.insert(of(row), member);
                entered += 1;
            }
        }
        entered
    }

    /// Whether an index's member is a live row naming the subject.
    fn names(&self, ordinal: u32, subject: u64, of: &impl Fn(&T) -> u64) -> bool {
        matches!(self.row(u64::from(ordinal)), Missing::Present((_, row)) if of(&row) == subject)
    }

    /// A subject's live rows' ordinals, appended to `out` in ordinal order: an entry below the ring's base is dropped
    /// by the walk. Returns the entries dropped; a key whose walk drops as many as it keeps is compacted by
    /// `compact_subject`.
    pub fn of_subject(&self, index: &Index<B>, subject: u64, of: impl Fn(&T) -> u64, out: &mut DayBuf<u32, B>) -> u32 {
        index.members(subject, |ordinal| self.names(ordinal, subject, &of), out)
    }

    /// A subject's entries in its index rewritten to its live rows alone.
    pub fn compact_subject(
        &self,
        index: &mut Index<B>,
        subject: u64,
        of: impl Fn(&T) -> u64,
        scratch: &mut DayBuf<u32, B>,
    ) {
        index.compact(subject, |ordinal| self.names(ordinal, subject, &of), scratch);
    }

    /// The live ordinals, `[base, next)`.
    #[must_use]
    pub fn ordinals(&self) -> (u64, u64) {
        (self.base, self.next)
    }

    /// The chunks live.
    #[must_use]
    pub fn chunks_live(&self) -> usize {
        self.live
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.days.bytes_committed()
            + self.rows.bytes_committed()
            + self.headers.bytes_committed()
            + self.free.bytes_committed()
            + self.floors.bytes_committed()
    }
}

impl<T: Pod, B: Backing> StoreStats for HorizonRing<T, B> {
    fn rows_live(&self) -> u64 {
        self.next - self.base
    }

    fn rows_ever(&self) -> u64 {
        self.next
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

// Saving, loading and comparing a ring, which no day runs.
#[path = "ring_save.rs"]
mod save;

#[cfg(test)]
#[path = "ring_tests.rs"]
mod tests;
