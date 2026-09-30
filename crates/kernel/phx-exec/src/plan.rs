//! Chunk plans: a traversal's chunks, whose bounds depend only on its rows and their declared cost, never on the
//! workers that run them.

use std::ops::Range;

use phx_id::Slot;

use crate::consts::{CHUNK_COST, CHUNKS_PER_MAX_WORKER, POOL_MAX_WORKERS};
use crate::convert::to_usize;

/// A traversal's chunks, as the bounds of runs of its rows, and the cost its rows declare.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChunkPlan {
    bounds: Vec<usize>,
    cost: u64,
}

impl ChunkPlan {
    /// A plan over `rows` rows of a table chunked by `rows_per_chunk`, each declaring `cost_per_row`: its chunks are
    /// runs of whole table chunks, so writes stay chunk-local.
    #[must_use]
    pub fn new(rows: u32, cost_per_row: u64, rows_per_chunk: u32) -> ChunkPlan {
        let mut plan = ChunkPlan::default();
        plan.cut(rows, cost_per_row, rows_per_chunk);
        plan
    }

    /// The plan cut again over `rows` rows of a table chunked by `rows_per_chunk`, keeping its bounds' room.
    pub fn cut(&mut self, rows: u32, cost_per_row: u64, rows_per_chunk: u32) {
        let (full, tail) = (rows / rows_per_chunk, rows % rows_per_chunk);
        let groups = std::iter::repeat_n(rows_per_chunk, to_usize(full)).chain((tail > 0).then_some(tail));
        self.cut_groups(groups, u64::from(rows), cost_per_row);
    }

    /// The plan cut again over the slots `span` of a table chunked by `rows_per_chunk` — a sweep's slice — its places
    /// counted from the span's start: its chunks close only where a table chunk ends, so writes stay chunk-local.
    pub fn cut_span(&mut self, span: Range<u32>, cost_per_row: u64, rows_per_chunk: u32) {
        let runs = SpanRuns { at: span.start, end: span.end, rows_per_chunk };
        self.cut_groups(runs, u64::from(span.end - span.start), cost_per_row);
    }

    /// A plan over `rows` rows of no table — a buffer the day filled — each declaring `cost_per_row`: its chunks are
    /// even runs of rows, each at the cost a chunk closes at.
    #[must_use]
    pub fn of_rows(rows: usize, cost_per_row: u64) -> ChunkPlan {
        let mut plan = ChunkPlan::default();
        plan.cut_rows(rows, cost_per_row);
        plan
    }

    /// The plan cut again over `rows` rows of no table, keeping its bounds' room.
    pub fn cut_rows(&mut self, rows: usize, cost_per_row: u64) {
        self.cost = crate::convert::to_u64(rows) * cost_per_row;
        self.bounds.clear();
        self.bounds.push(0);
        if rows == 0 {
            return;
        }
        // Rows declaring no cost, or too few to share, are one chunk.
        let per = match (Self::target(self.cost), cost_per_row) {
            (0, _) | (_, 0) => rows,
            (target, each) => match usize::try_from(target.div_ceil(each)) {
                Ok(per) => per,
                Err(_) => rows,
            },
        };
        let mut at = per;
        while at < rows {
            self.bounds.push(at);
            at += per;
        }
        self.bounds.push(rows);
    }

    /// The cost a chunk closes at: the chunk cost, or the share of the whole that gives every one of the most workers
    /// four chunks where that is smaller.
    fn target(cost: u64) -> u64 {
        let share = cost / u64::from(POOL_MAX_WORKERS * CHUNKS_PER_MAX_WORKER);
        if share < CHUNK_COST { share } else { CHUNK_COST }
    }

    /// A plan over sorted slots — an agenda's rows — each declaring `cost_per_row`: its chunks close only where the
    /// slots pass from one table chunk to the next.
    #[must_use]
    pub fn over_slots(slots: &[Slot], cost_per_row: u64, rows_per_chunk: u32) -> ChunkPlan {
        let mut plan = ChunkPlan::default();
        plan.cut_slots(slots, cost_per_row, rows_per_chunk);
        plan
    }

    /// The plan cut again over sorted slots, keeping its bounds' room.
    pub fn cut_slots(&mut self, slots: &[Slot], cost_per_row: u64, rows_per_chunk: u32) {
        let runs = SlotRuns { slots, rows_per_chunk, at: 0 };
        self.cut_groups(runs, crate::convert::to_u64(slots.len()), cost_per_row);
    }

    /// Chunks of whole groups, each closing once its cost reaches the cost a chunk closes at.
    fn cut_groups(&mut self, groups: impl Iterator<Item = u32>, rows: u64, cost_per_row: u64) {
        self.cost = rows * cost_per_row;
        let target = Self::target(self.cost);
        self.bounds.clear();
        self.bounds.push(0);
        let (mut at, mut open) = (0, 0_u64);
        for rows in groups {
            at += to_usize(rows);
            open += u64::from(rows) * cost_per_row;
            if open >= target {
                self.bounds.push(at);
                open = 0;
            }
        }
        if self.bounds.last() != Some(&at) {
            self.bounds.push(at);
        }
    }

    /// The plan's chunks.
    #[must_use]
    pub fn len(&self) -> usize {
        match self.bounds.len() {
            0 => 0,
            n => n - 1,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A chunk's rows, by their places among the plan's rows.
    #[must_use]
    pub fn chunk(&self, k: usize) -> Range<usize> {
        match (self.bounds.get(k), self.bounds.get(k + 1)) {
            (Some(&from), Some(&to)) => from..to,
            _ => phx_num::violation!(clause = "TIME.6", "a chunk past its plan", chunk = k),
        }
    }

    /// The cost the plan's rows declare.
    #[must_use]
    pub fn cost(&self) -> u64 {
        self.cost
    }
}

/// Sorted slots' runs within one table chunk each, as their counts.
struct SlotRuns<'a> {
    slots: &'a [Slot],
    rows_per_chunk: u32,
    at: usize,
}

impl Iterator for SlotRuns<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let rest = self.slots.get(self.at..)?;
        let chunk = rest.first()?.get() / self.rows_per_chunk;
        let run = rest.partition_point(|s| s.get() / self.rows_per_chunk == chunk);
        self.at += run;
        Some(crate::convert::to_u32(run))
    }
}

/// A span of slots' runs within one table chunk each, as their counts.
struct SpanRuns {
    at: u32,
    end: u32,
    rows_per_chunk: u32,
}

impl Iterator for SpanRuns {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.at >= self.end {
            return None;
        }
        let to = match (self.at / self.rows_per_chunk + 1).checked_mul(self.rows_per_chunk) {
            Some(chunk_end) if chunk_end < self.end => chunk_end,
            _ => self.end,
        };
        let run = to - self.at;
        self.at = to;
        Some(run)
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
