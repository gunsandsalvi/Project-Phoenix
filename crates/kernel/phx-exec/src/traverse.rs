use std::ops::Range;

use phx_id::Slot;
use phx_num::violation;

use crate::consts::CHUNK_COST;
use crate::convert::{to_u32, to_usize};
use crate::pool::Pool;
use crate::site::{self, Site};

fn chunk_of(slot: Slot, rows_per_chunk: u32) -> u32 {
    slot.get() / rows_per_chunk
}

/// Runs `f` on every chunk index below `n`, each output at its chunk's index; the site records the chunk each
/// worker is on.
pub fn for_chunks<T: Send>(pool: &Pool, site: Site, n: u32, f: impl Fn(u32) -> T + Sync) -> Vec<T> {
    pool.map(to_usize(n), |i| {
        let chunk = to_u32(i);
        site::enter(Site { chunk, ..site });
        let result = f(chunk);
        site::leave();
        result
    })
}

/// The agenda's units: runs of its sorted rows that close only where a table chunk ends, once their declared cost
/// reaches `CHUNK_COST`, so a unit is one or more whole chunks' rows and its bounds depend on the rows alone.
#[must_use]
pub fn agenda_units(agenda: &[Slot], rows_per_chunk: u32, cost_per_row: u32) -> Vec<Range<usize>> {
    let mut units = Vec::new();
    let (mut start, mut cost) = (0, 0_u64);
    for (i, pair) in agenda.windows(2).enumerate() {
        cost += u64::from(cost_per_row);
        let [row, next] = [pair.first(), pair.last()].map(|s| s.copied().map(|s| chunk_of(s, rows_per_chunk)));
        if row != next && cost >= CHUNK_COST {
            units.push(start..i + 1);
            (start, cost) = (i + 1, 0);
        }
    }
    if start < agenda.len() {
        units.push(start..agenda.len());
    }
    units
}

/// Runs `f` on each agenda unit's rows, each output at its unit's index.
pub fn for_agenda<T: Send>(
    pool: &Pool,
    site: Site,
    agenda: &[Slot],
    rows_per_chunk: u32,
    cost_per_row: u32,
    f: impl Fn(&[Slot]) -> T + Sync,
) -> Vec<T> {
    let units = agenda_units(agenda, rows_per_chunk, cost_per_row);
    for_chunks(pool, site, to_u32(units.len()), |u| {
        let Some(rows) = units.get(to_usize(u)).and_then(|r| agenda.get(r.clone())) else {
            violation!(clause = "TIME.6", "an agenda unit outside the agenda", unit = u);
        };
        f(rows)
    })
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;

    use super::{agenda_units, for_agenda, for_chunks};
    use crate::consts::CHUNK_COST;
    use crate::pool::Pool;
    use crate::site::Site;
    use crate::spec::PoolSpec;

    const SITE: Site = Site { day: 1, substep: 0, handler: 0, chunk: 0 };

    #[test]
    fn traverse_is_worker_independent() {
        let rows: Vec<u64> = (0..1_000_000).map(|i| i * 7 % 1_000_003).collect();
        let chunk_rows = 4096;
        let run = |workers: usize| {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let n = u32::try_from(rows.len().div_ceil(chunk_rows)).unwrap();
            for_chunks(&pool, SITE, n, |c| {
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
        let units = agenda_units(&agenda, 64, per_row);
        for u in &units {
            let first = agenda[u.end - 1].get() / 64;
            let next = agenda.get(u.end).map(|s| s.get() / 64);
            assert_ne!(next, Some(first), "a unit ends where a table chunk ends: {u:?}");
        }
        assert_eq!(units.first().map(|u| u.start), Some(0));
        assert_eq!(units.last().map(|u| u.end), Some(agenda.len()));
        assert!(units.windows(2).all(|w| w[0].end == w[1].start));
        // A unit closes once it passes the cost at a boundary: chunk 0 alone is past it; chunks 1 and 2 share one.
        assert_eq!(units.len(), 2);
        assert_eq!(agenda_units(&[], 64, per_row), vec![]);
    }

    #[test]
    fn for_agenda_covers_every_row_once() {
        let pool = Pool::new(&PoolSpec::unpinned(3)).unwrap();
        let agenda: Vec<Slot> = (0..50_000).map(|i| Slot::new(i * 3)).collect();
        let seen = for_agenda(&pool, SITE, &agenda, 4096, 100, <[Slot]>::to_vec);
        assert_eq!(seen.concat(), agenda);
    }
}
