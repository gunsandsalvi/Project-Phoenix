//! Plans over hand-given row counts and costs: bounds from the rows alone, whole table chunks, enough chunks for
//! every worker count, and one result whichever workers run them.
#![cfg(test)]

use phx_id::Slot;

use super::ChunkPlan;
use crate::consts::{CHUNK_COST, CHUNKS_PER_MAX_WORKER, INLINE_BELOW, POOL_MAX_WORKERS};
use crate::pool::Pool;
use crate::spec::PoolSpec;
use crate::traverse::for_plan;

fn chunks(plan: &ChunkPlan) -> Vec<std::ops::Range<usize>> {
    (0..plan.len()).map(|k| plan.chunk(k)).collect()
}

/// A hash of every row's value, a chunk at a time, into the chunk's result.
fn hashed(pool: Option<&Pool>, plan: &ChunkPlan, rows: &[u64]) -> Vec<u64> {
    let mut out = vec![0; plan.len()];
    for_plan(pool, plan, &mut out, |r, o| *o = rows[r].iter().fold(0_u64, |h, v| h.rotate_left(7) ^ v));
    out
}

#[test]
fn bounds_independent_of_workers() {
    // A plan takes rows, cost and the table's chunking; no worker count reaches it, so it is one value.
    let a = ChunkPlan::new(1_000_000, 90, 4096);
    assert_eq!(a, ChunkPlan::new(1_000_000, 90, 4096));
    assert_eq!(chunks(&a).iter().map(ExactSizeIterator::len).sum::<usize>(), 1_000_000);
}

#[test]
fn bounds_snap_to_table_chunks() {
    let plan = ChunkPlan::new(100_000, 700, 4096);
    for r in chunks(&plan) {
        assert_eq!(r.start % 4096, 0, "{r:?} begins a table chunk");
        assert!(r.end % 4096 == 0 || r.end == 100_000, "{r:?} ends one");
    }
    let slots: Vec<Slot> = (0..3000).map(|i| Slot::new(i * 37)).collect();
    let over = ChunkPlan::over_slots(&slots, 40_000, 1024);
    for r in chunks(&over) {
        let (first, last) = (slots[r.start].get() / 1024, slots[r.end - 1].get() / 1024);
        assert!(r.start == 0 || slots[r.start - 1].get() / 1024 < first, "{r:?} begins where a table chunk does");
        assert!(r.end == slots.len() || slots[r.end].get() / 1024 > last, "{r:?} ends where one does");
    }
}

#[test]
fn at_least_four_chunks_per_max_worker() {
    let most = usize::try_from(POOL_MAX_WORKERS * CHUNKS_PER_MAX_WORKER).unwrap();
    // Rows costing less than a chunk's cost each way still give every worker four chunks.
    assert!(ChunkPlan::new(1 << 20, 1, 1024).len() >= most);
    // A heavy table is cut by its cost, far past four a worker.
    let heavy = ChunkPlan::new(1 << 20, 400, 1024);
    assert!(heavy.len() > most);
    let per_chunk = CHUNK_COST / 400;
    assert!(chunks(&heavy).iter().all(|r| u64::try_from(r.len()).unwrap() <= per_chunk + 1024));
    // Fewer table chunks than that: one chunk each.
    assert_eq!(ChunkPlan::new(3 * 1024, 1_000, 1024).len(), 3);
    assert!(ChunkPlan::new(0, 1_000, 1024).is_empty());
}

#[test]
fn plan_from_heavy_rows() {
    // A payday's dues at the design point: every row planned, no chunk past its cost by more than a table chunk.
    let dues = 22_500_000;
    let plan = ChunkPlan::new(dues, 15, 4096);
    assert_eq!(plan.chunk(plan.len() - 1).end, 22_500_000);
    assert_eq!(plan.cost(), 22_500_000 * 15);
    assert!(chunks(&plan).iter().all(|r| u64::try_from(r.len()).unwrap() * 15 < CHUNK_COST + 4096 * 15));
}

#[test]
fn inline_equals_pooled() {
    let rows: Vec<u64> = (0..50_000).map(|i| i * 2_654_435_761 % 1_000_003).collect();
    let pool = Pool::new(&PoolSpec::unpinned(3)).unwrap();
    let light = ChunkPlan::new(50_000, 1, 1024);
    assert!(light.cost() < INLINE_BELOW, "this plan runs inline");
    let heavy = ChunkPlan::new(50_000, 100, 1024);
    assert_eq!(hashed(Some(&pool), &light, &rows), hashed(None, &light, &rows));
    assert_eq!(hashed(Some(&pool), &heavy, &rows), hashed(None, &heavy, &rows));
}

#[test]
fn plan_results_same_for_1_to_8_workers() {
    let rows: Vec<u64> = (0..300_000).map(|i| i * 7_919 % 1_000_003).collect();
    let plan = ChunkPlan::new(300_000, 50, 4096);
    let one = hashed(Some(&Pool::new(&PoolSpec::unpinned(1)).unwrap()), &plan, &rows);
    for workers in [2, 3, 8] {
        let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
        assert_eq!(hashed(Some(&pool), &plan, &rows), one, "{workers} workers");
    }
}

#[test]
fn rows_of_no_table_cut_evenly_by_cost() {
    let plan = ChunkPlan::of_rows(1_000_000, 25);
    let most = usize::try_from(POOL_MAX_WORKERS * CHUNKS_PER_MAX_WORKER).unwrap();
    assert!(plan.len() >= most);
    assert!(chunks(&plan).iter().all(|r| u64::try_from(r.len()).unwrap() * 25 <= CHUNK_COST + 25));
    assert_eq!(plan.chunk(plan.len() - 1).end, 1_000_000);
    assert_eq!(ChunkPlan::of_rows(10, 0).len(), 1, "rows declaring no cost are one chunk");
    assert!(ChunkPlan::of_rows(0, 25).is_empty());
}

#[test]
fn span_closes_at_table_chunks() {
    // A slice from inside one table chunk to inside another: its chunks close only where a table chunk ends, and its
    // places count from the slice's start.
    let mut plan = ChunkPlan::default();
    plan.cut_span(5_000..70_000, 40_000, 4096);
    let got = chunks(&plan);
    assert_eq!(got.iter().map(ExactSizeIterator::len).sum::<usize>(), 65_000);
    for r in &got {
        assert!((r.end + 5_000) % 4096 == 0 || r.end == 65_000, "{r:?} ends a table chunk");
    }
    plan.cut_span(9..9, 40_000, 4096);
    assert!(plan.is_empty());
}
