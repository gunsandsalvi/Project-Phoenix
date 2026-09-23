use std::hint::black_box;

use phx_rand::philox;
use phx_store::{AddressSpace, Region};

use crate::clock::Clock;
use crate::consts::{NS_PER_S, PREFETCH_DISTANCE, PROBE_PIECE_BYTES, PROBE_UNIT_BLOCKS};
use crate::convert::{index, to_u64};
use crate::mix::mix64;
use crate::os;
use crate::pool::{Pool, PoolError};
use crate::spec::{PoolSpec, read_capacity};

/// One core's compute rate, in units of fixed work per second, alone and with every allowed core busy at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreRate {
    pub core: usize,
    pub capacity: Option<u32>,
    pub alone_per_s: u64,
    pub loaded_per_s: u64,
}

/// Random reads of 8-byte rows over a region, every worker reading at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatherRate {
    pub bytes: u64,
    pub prefetch: bool,
    pub rows: u64,
    /// Wall time over all rows: what a row costs the pool.
    pub ns_per_row: u64,
    /// Wall time times workers over all rows: what a row costs the core that reads it.
    pub core_ns_per_row: u64,
    pub page_size: Option<u64>,
}

/// A time span measured on a clock that may run backwards, which yields none rather than a guess.
fn since(clock: &dyn Clock, start: u64) -> Option<u64> {
    clock.now_ns().checked_sub(start)
}

/// A count over a span, per second; none for an empty or unmeasured span.
fn per_second(count: u64, ns: Option<u64>) -> Option<u64> {
    let ns = ns.filter(|n| *n > 0)?;
    u64::try_from(u128::from(count) * u128::from(NS_PER_S) / u128::from(ns)).ok()
}

/// Units of fixed work the calling thread does in about `window_ns`, per second.
fn units_per_second(clock: &dyn Clock, window_ns: u64) -> Option<u64> {
    let start = clock.now_ns();
    let (mut units, mut ctr) = (0_u64, [0_u32; 4]);
    loop {
        for _ in 0..PROBE_UNIT_BLOCKS {
            ctr = black_box(philox(ctr, [1, 2]));
        }
        units += 1;
        let spent = since(clock, start)?;
        if spent >= window_ns {
            return per_second(units, Some(spent));
        }
    }
}

/// Each allowed core's compute rate alone, then with every allowed core busy, so the report can say how many
/// fast-core-seconds the phone sustains per second and how that changes with heat.
///
/// # Errors
/// When the system will not say which cores the process may use, or will not start a worker on them.
pub fn core_rates(clock: &dyn Clock, window_ns: u64) -> Result<Vec<CoreRate>, PoolError> {
    let cores = os::allowed_cores().ok_or_else(|| PoolError("no affinity mask".to_owned()))?;
    let mut alone = Vec::with_capacity(cores.len());
    for core in &cores {
        let pool = Pool::new(&PoolSpec { cores: vec![*core], pin: true })?;
        alone.push(pool.on_every_worker(|| units_per_second(clock, window_ns)).into_iter().flatten().next());
    }
    let loaded = Pool::new(&PoolSpec { cores: cores.clone(), pin: true })?
        .on_every_worker(|| units_per_second(clock, window_ns));
    Ok(cores
        .iter()
        .zip(alone)
        .zip(loaded)
        .filter_map(|((core, alone), loaded)| {
            Some(CoreRate { core: *core, capacity: read_capacity(*core), alone_per_s: alone?, loaded_per_s: loaded? })
        })
        .collect())
}

/// A region of `bytes` committed and filled, as the probe's gathers and sweeps read it; each word holds its index.
#[must_use]
pub fn filled_region(pool: &Pool, bytes: u64) -> Region<u64> {
    let words = index(bytes / to_u64(size_of::<u64>()));
    let mut space = AddressSpace::empty();
    let mut region: Region<u64> = Region::reserve(&mut space, words);
    region.ensure(words);
    let piece_words = PROBE_PIECE_BYTES / size_of::<u64>();
    let pieces: Vec<(u64, &mut [u64])> =
        (0_u64..).step_by(piece_words).zip(region.slice_mut(words).chunks_mut(piece_words)).collect();
    pool.for_each(pieces, |(base, piece)| {
        for (i, w) in (base..).zip(piece.iter_mut()) {
            *w = i;
        }
    });
    region
}

/// The row of a region of `words` a worker reads at step `i`: a fixed mix, spread over the region by
/// multiply-and-shift so no division is paid per row.
fn row_at(seed: u64, i: u64, words: u64) -> usize {
    let [_, high] = split((u128::from(mix64(seed ^ i)) * u128::from(words)).to_le_bytes());
    index(high)
}

fn split(bytes: [u8; 16]) -> [u64; 2] {
    let [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15] = bytes;
    [u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]), u64::from_le_bytes([b8, b9, b10, b11, b12, b13, b14, b15])]
}

/// `rows_per_worker` random reads by every worker at once over the whole region, with or without asking for each
/// row `PREFETCH_DISTANCE` reads ahead; none when nothing was read or the clock ran backwards.
#[must_use]
pub fn gather(
    pool: &Pool,
    clock: &dyn Clock,
    region: &Region<u64>,
    rows_per_worker: u64,
    prefetch: bool,
) -> Option<GatherRate> {
    let words = region.committed_len();
    let data = region.slice(words);
    let n = to_u64(words);
    let rows = rows_per_worker * to_u64(pool.workers());
    if rows == 0 || n == 0 {
        return None;
    }
    let start = clock.now_ns();
    let sums = pool.on_every_worker(|| {
        // Each worker reads its own rows, seeded by its thread, so no two share a cache line by design.
        let seed = mix64(u64::from(os::current_tid().unsigned_abs()));
        let mut sum = 0_u64;
        for i in 0..rows_per_worker {
            if prefetch && let Some(ahead) = data.get(row_at(seed, i + PREFETCH_DISTANCE, n)) {
                os::prefetch(ahead);
            }
            if let Some(v) = data.get(row_at(seed, i, n)) {
                sum = sum.wrapping_add(*v);
            }
        }
        black_box(sum)
    });
    let spent = since(clock, start)?;
    black_box(sums);
    let workers = to_u64(pool.workers());
    Some(GatherRate {
        bytes: n * to_u64(size_of::<u64>()),
        prefetch,
        rows,
        ns_per_row: spent / rows,
        core_ns_per_row: spent * workers / rows,
        page_size: os::page_size(),
    })
}

/// Bytes per second every worker together reads sweeping the region in pieces, once over.
#[must_use]
pub fn sweep(pool: &Pool, clock: &dyn Clock, region: &Region<u64>) -> Option<u64> {
    let words = region.committed_len();
    let data = region.slice(words);
    let start = clock.now_ns();
    let sums = pool.map(words.div_ceil(PROBE_PIECE_BYTES / size_of::<u64>()), |p| {
        let piece_words = PROBE_PIECE_BYTES / size_of::<u64>();
        data.iter().skip(p * piece_words).take(piece_words).fold(0_u64, |a, v| a.wrapping_add(*v))
    });
    black_box(sums);
    per_second(to_u64(words) * to_u64(size_of::<u64>()), since(clock, start))
}

/// Wall nanoseconds of one empty dispatch and join of every worker, hot, over `rounds` in a row.
#[must_use]
pub fn barrier(pool: &Pool, clock: &dyn Clock, rounds: u64) -> Option<u64> {
    let items: Vec<usize> = (0..pool.workers()).collect();
    let start = clock.now_ns();
    for _ in 0..rounds {
        pool.for_each(items.iter().copied(), |i| {
            black_box(i);
        });
    }
    since(clock, start)?.checked_div(rounds)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{barrier, core_rates, filled_region, gather, sweep};
    use crate::clock::Clock;
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    /// A clock that advances one microsecond a read, so every measured span is positive and short.
    struct Steps(AtomicU64);

    impl Clock for Steps {
        fn now_ns(&self) -> u64 {
            self.0.fetch_add(1000, Ordering::Relaxed)
        }
    }

    #[test]
    fn probes_measure_small_regions() {
        let pool = Pool::new(&PoolSpec::unpinned(2)).unwrap();
        let clock = Steps(AtomicU64::new(0));
        let region = filled_region(&pool, 1 << 20);
        assert_eq!(region.slice(region.committed_len())[12_345], 12_345);
        for prefetch in [false, true] {
            let g = gather(&pool, &clock, &region, 10_000, prefetch).unwrap();
            assert_eq!((g.rows, g.bytes, g.prefetch), (20_000, 1 << 20, prefetch));
        }
        assert!(sweep(&pool, &clock, &region).is_some());
        assert!(barrier(&pool, &clock, 100).is_some());
        let rates = core_rates(&clock, 5_000).unwrap();
        assert!(!rates.is_empty() && rates.iter().all(|r| r.alone_per_s > 0 && r.loaded_per_s > 0));
    }
}
