use std::ops::Range;

use phx_id::Slot;
use phx_num::violation;

use crate::consts::INLINE_BELOW;
use crate::convert::{to_u32, to_usize};
use crate::plan::ChunkPlan;
use crate::pool::{self, Pool, as_chunk, refuse_nested};
use crate::site;

/// `f` on every slot of `out` with its index, each a chunk: on the pool when one is given and inline on the calling
/// thread otherwise, in chunk order, so either way each result lies at its chunk's index.
fn run<T: Send>(pool: Option<&Pool>, out: &mut [T], f: impl Fn(usize, &mut T) + Sync) {
    if let Some(p) = pool {
        p.run_into(out, f);
    } else {
        refuse_nested();
        let parent = site::current();
        for (i, slot) in out.iter_mut().enumerate() {
            as_chunk(i, parent, || f(i, slot));
        }
    }
}

/// `f` on each of the caller's items with its index, each item a chunk that owns what it holds: nothing is
/// allocated.
pub fn for_each_chunk<T: Send>(pool: Option<&Pool>, items: &mut [T], f: impl Fn(usize, &mut T) + Sync) {
    run(pool, items, f);
}

/// `f` on the items of two of the caller's slices at the same index, each pair a chunk: a chunk's inputs in one and
/// the part of the output it alone writes in the other, with nothing allocated.
pub fn for_each_pair<A: Send, B: Send>(
    pool: Option<&Pool>,
    (a, b): (&mut [A], &mut [B]),
    f: impl Fn(usize, &mut A, &mut B) + Sync,
) {
    if a.len() != b.len() {
        violation!(clause = "TIME.6", "a paired traversal over slices of two lengths", left = a.len(), right = b.len());
    }
    if let Some(p) = pool {
        p.run_pairs((a, b), f);
    } else {
        refuse_nested();
        let parent = site::current();
        for (i, (x, y)) in a.iter_mut().zip(b.iter_mut()).enumerate() {
            as_chunk(i, parent, || f(i, x, y));
        }
    }
}

/// `f` on each chunk of a plan, with its rows' places and its slot of `out`: inline when no pool is given or the
/// plan's declared cost is below a dispatch's worth, over the same chunks in order.
pub fn for_plan<T: Send>(
    pool: Option<&Pool>,
    plan: &ChunkPlan,
    out: &mut [T],
    f: impl Fn(Range<usize>, &mut T) + Sync,
) {
    if out.len() != plan.len() {
        violation!(clause = "TIME.6", "a plan's results not one a chunk", chunks = plan.len());
    }
    let pool = pool.filter(|_| plan.cost() >= INLINE_BELOW);
    run(pool, out, |k, slot| f(plan.chunk(k), slot));
}

/// `f` of every chunk index below `n`, each output at its chunk's index.
pub fn for_chunks<T: Send>(pool: Option<&Pool>, n: u32, f: impl Fn(u32) -> T + Sync) -> Vec<T> {
    pool::map(pool, to_usize(n), |i| f(to_u32(i)))
}

/// `f` of each owned item, each a chunk, its result at the item's place.
pub fn map_chunks<I: Send, T: Send>(pool: Option<&Pool>, items: Vec<I>, f: impl Fn(I) -> T + Sync) -> Vec<T> {
    pool::map_items(pool, items, f)
}

/// `f` of each owned item — a chunk's inputs and the parts of the output it alone writes — each a chunk.
pub fn each_chunk<I: Send>(pool: Option<&Pool>, items: impl IntoIterator<Item = I>, f: impl Fn(I) + Sync) {
    pool::each(pool, items, f);
}

/// The agenda's units: runs of its sorted rows that close only where a table chunk ends, once their declared cost
/// reaches the chunk cost, so a unit is one or more whole chunks' rows and its bounds depend on the rows alone.
#[must_use]
pub fn agenda_units(agenda: &[Slot], rows_per_chunk: u32, cost_per_row: u32) -> ChunkPlan {
    ChunkPlan::over_slots(agenda, u64::from(cost_per_row), rows_per_chunk)
}

/// `f` on each agenda unit's rows, each output at its unit's index.
pub fn for_agenda<T: Send>(
    pool: Option<&Pool>,
    agenda: &[Slot],
    rows_per_chunk: u32,
    cost_per_row: u32,
    f: impl Fn(&[Slot]) -> T + Sync,
) -> Vec<T> {
    let plan = agenda_units(agenda, rows_per_chunk, cost_per_row);
    for_chunks(pool, to_u32(plan.len()), |u| match agenda.get(plan.chunk(to_usize(u))) {
        Some(rows) => f(rows),
        None => violation!(clause = "TIME.6", "an agenda unit outside the agenda", unit = u),
    })
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;

    use super::{agenda_units, for_agenda, for_chunks};
    use crate::consts::CHUNK_COST;
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn traverse_is_worker_independent() {
        let rows: Vec<u64> = (0..1_000_000).map(|i| i * 7 % 1_000_003).collect();
        let chunk_rows = 4096;
        let run = |workers: usize| {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let n = u32::try_from(rows.len().div_ceil(chunk_rows)).unwrap();
            for_chunks(Some(&pool), n, |c| {
                let start = usize::try_from(c).unwrap() * chunk_rows;
                rows[start..].iter().take(chunk_rows).fold(0_u64, |h, r| h.rotate_left(5) ^ r)
            })
        };
        let one = run(1);
        for workers in [2, 3, 8] {
            assert_eq!(run(workers), one, "workers={workers}");
        }
    }

    #[test]
    fn agenda_units_snap_to_table_chunks() {
        let per_row = u32::try_from(CHUNK_COST / 10).unwrap();
        // Chunks of 64 rows: 25 agenda rows in chunk 0, 3 in chunk 1, 30 in chunk 2.
        let agenda: Vec<Slot> = (0..25).chain(64..67).chain(128..158).map(Slot::new).collect();
        let plan = agenda_units(&agenda, 64, per_row);
        let units: Vec<_> = (0..plan.len()).map(|k| plan.chunk(k)).collect();
        for u in &units {
            let first = agenda[u.end - 1].get() / 64;
            let next = agenda.get(u.end).map(|s| s.get() / 64);
            assert_ne!(next, Some(first), "a unit ends where a table chunk ends: {u:?}");
        }
        assert_eq!(units.first().map(|u| u.start), Some(0));
        assert_eq!(units.last().map(|u| u.end), Some(agenda.len()));
        assert!(units.windows(2).all(|w| w[0].end == w[1].start));
        // So few rows give each of the most workers four units only by closing a unit at every boundary.
        assert_eq!(units.len(), 3);
        assert!(agenda_units(&[], 64, per_row).is_empty());
    }

    #[test]
    fn for_agenda_covers_every_row_once() {
        let pool = Pool::new(&PoolSpec::unpinned(3)).unwrap();
        let agenda: Vec<Slot> = (0..50_000).map(|i| Slot::new(i * 3)).collect();
        let seen = for_agenda(Some(&pool), &agenda, 4096, 100, <[Slot]>::to_vec);
        assert_eq!(seen.concat(), agenda);
    }
}
