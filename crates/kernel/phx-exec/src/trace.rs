//! The bench's trace: what the run is doing, told as it does it, so a slow or stuck run shows where it is and how far
//! it got. The run's host sets where the marks go and the clock that times them; with none set, nothing is told. No
//! outcome ever reads it.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::Clock;
use crate::os;
use crate::pool::{self, PoolUsage};

/// A mark of the run, at the depth of the spans open around it: a span begun, a span ended with its own time where
/// the clock gave one and what else it spent, or a note of named counts.
#[derive(Clone, Copy, Debug)]
pub enum Mark<'a> {
    Begun(&'static str),
    Ended(&'static str, Option<u64>, &'a [(&'static str, i64)]),
    Note(&'static str, &'a [(&'static str, i64)]),
}

/// Where the marks go, and the clock that times the spans.
pub trait Tracer: Clock {
    fn mark(&self, mark: Mark<'_>, depth: usize);
}

/// Where the marks go, the spans open now for the marks' depth, and what every thread counts for the spans to read.
struct Trace {
    tracer: OnceLock<&'static dyn Tracer>,
    depth: AtomicUsize,
    counted: Counted,
}

/// What any thread adds to and a span reads: the pools' chunks, dispatches and spin, and the allocations the bench's
/// allocator counts.
pub(crate) struct Counted {
    pub(crate) chunks: AtomicU64,
    pub(crate) dispatches: AtomicU64,
    pub(crate) spun: AtomicU64,
    #[cfg(feature = "bench")]
    pub(crate) alloc_calls: AtomicU64,
    #[cfg(feature = "bench")]
    pub(crate) alloc_bytes: AtomicU64,
}

static TRACE: Trace = Trace {
    tracer: OnceLock::new(),
    depth: AtomicUsize::new(0),
    counted: Counted {
        chunks: AtomicU64::new(0),
        dispatches: AtomicU64::new(0),
        spun: AtomicU64::new(0),
        #[cfg(feature = "bench")]
        alloc_calls: AtomicU64::new(0),
        #[cfg(feature = "bench")]
        alloc_bytes: AtomicU64::new(0),
    },
};

/// The process's counts, for the pool and the allocator to add to.
pub(crate) fn counted() -> &'static Counted {
    &TRACE.counted
}

/// Sends every mark from now on to `tracer`; a second setting is ignored.
pub fn set(tracer: &'static dyn Tracer) {
    let _ = TRACE.tracer.set(tracer);
}

/// Whether anyone is told, so a note's counts are gathered only when they will be read.
#[must_use]
pub fn on() -> bool {
    TRACE.tracer.get().is_some()
}

/// `work` run between its span's two marks, the second with the time it took and what else it spent.
pub fn span<T>(name: &'static str, work: impl FnOnce() -> T) -> T {
    traced(name, None, work)
}

/// `work` run as a span of `items`, so its costs read per item.
pub fn span_items<T>(name: &'static str, items: u64, work: impl FnOnce() -> T) -> T {
    traced(name, Some(items), work)
}

fn traced<T>(name: &'static str, items: Option<u64>, work: impl FnOnce() -> T) -> T {
    let (Some(t), open) = (TRACE.tracer.get(), &TRACE.depth) else { return work() };
    let depth = open.fetch_add(1, Ordering::Relaxed);
    t.mark(Mark::Begun(name), depth);
    let before = Reading::now();
    let start = t.now_ns();
    let out = work();
    let ns = t.now_ns().checked_sub(start);
    let counts = Spent::between(items, &before, &Reading::now()).counts();
    open.fetch_sub(1, Ordering::Relaxed);
    t.mark(Mark::Ended(name, ns, &counts), depth);
    out
}

/// What the process has spent so far, as a span reads it at its start and end: every thread's CPU and faults, the
/// pools' chunks, and the allocations where the bench's allocator counts them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    pub cpu_ns: Option<u64>,
    pub faults: Option<u64>,
    pub pool: PoolUsage,
    /// Calls and bytes.
    pub allocated: Option<(u64, u64)>,
}

impl Reading {
    #[must_use]
    pub fn now() -> Reading {
        #[cfg(feature = "bench")]
        let allocated = Some(crate::alloc::allocated());
        #[cfg(not(feature = "bench"))]
        let allocated = None;
        Reading { cpu_ns: os::process_cpu_ns(), faults: os::process_faults(), pool: pool::usage(), allocated }
    }
}

/// What a span spent between two readings: its items where it has them, the CPU and faults of every thread — the
/// caller's, the workers' in their chunks and in their spin between dispatches — so the sums do not depend on how
/// many workers shared the work, and the chunks, barriers, spin and allocations. A count the system did not report
/// is missing, never zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    pub items: Option<u64>,
    pub cpu_ns: Option<u64>,
    pub faults: Option<u64>,
    pub chunks: Option<u64>,
    pub barriers: Option<u64>,
    pub spun: Option<u64>,
    pub allocs: Option<u64>,
    pub alloc_bytes: Option<u64>,
}

impl Spent {
    #[must_use]
    pub fn between(items: Option<u64>, before: &Reading, after: &Reading) -> Spent {
        let delta = |b: Option<u64>, a: Option<u64>| a?.checked_sub(b?);
        let (pb, pa) = (&before.pool, &after.pool);
        let (ab, aa) = (before.allocated, after.allocated);
        Spent {
            items,
            cpu_ns: delta(before.cpu_ns, after.cpu_ns),
            faults: delta(before.faults, after.faults),
            chunks: delta(Some(pb.chunks), Some(pa.chunks)),
            barriers: delta(Some(pb.dispatches), Some(pa.dispatches)),
            spun: delta(Some(pb.spun), Some(pa.spun)),
            allocs: delta(ab.map(|(calls, _)| calls), aa.map(|(calls, _)| calls)),
            alloc_bytes: delta(ab.map(|(_, bytes)| bytes), aa.map(|(_, bytes)| bytes)),
        }
    }

    /// The counts a mark carries, the missing ones left out.
    #[must_use]
    pub fn counts(&self) -> Vec<(&'static str, i64)> {
        let fields = [
            ("items", self.items),
            ("cpu_ns", self.cpu_ns),
            ("faults", self.faults),
            ("chunks", self.chunks),
            ("barriers", self.barriers),
            ("spun", self.spun),
            ("allocs", self.allocs),
            ("alloc_bytes", self.alloc_bytes),
        ];
        fields.into_iter().filter_map(|(k, v)| Some((k, i64::try_from(v?).ok()?))).collect()
    }
}

pub fn note(name: &'static str, counts: &[(&'static str, i64)]) {
    if let Some(tracer) = TRACE.tracer.get() {
        tracer.mark(Mark::Note(name, counts), TRACE.depth.load(Ordering::Relaxed));
    }
}

/// Whether a loop at its `n`th pass tells how far it is: at each power of two, so a loop that runs away still tells
/// often enough to be seen, and a long one never floods.
#[must_use]
pub fn doubling(n: u64) -> bool {
    n.is_power_of_two()
}

/// A count as a note carries it.
#[must_use]
pub fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{Reading, Spent, doubling};
    use crate::pool::PoolUsage;

    #[test]
    fn a_loop_tells_at_powers_of_two() {
        let told: Vec<u64> = (0..20).filter(|n| doubling(*n)).collect();
        assert_eq!(told, vec![1, 2, 4, 8, 16]);
    }

    #[test]
    fn span_cpu_sums_workers() {
        // The same four chunks' CPU, spread over one worker or four, reads the same in the process's sum.
        let chunks = [700_u64, 300, 450, 550];
        let reading = |caller: u64, done: &[u64], dispatches: u64| Reading {
            cpu_ns: Some(caller + done.iter().sum::<u64>()),
            faults: Some(3),
            pool: PoolUsage { chunks: done.len().try_into().unwrap(), dispatches, spun: 0 },
            allocated: None,
        };
        let one = Spent::between(Some(4), &reading(1_000, &[], 0), &reading(1_200, &chunks, 4));
        let four = Spent::between(Some(4), &reading(1_000, &[], 0), &reading(1_200, &chunks, 1));
        assert_eq!(one.cpu_ns, Some(200 + 2_000), "the caller's CPU and every chunk's, whoever ran them");
        assert_eq!(one.cpu_ns, four.cpu_ns);
        assert_eq!((one.chunks, one.items, one.allocs, one.faults), (Some(4), Some(4), None, Some(0)));
        assert!(!one.counts().iter().any(|(k, _)| *k == "allocs"), "no allocations told where none were counted");
        let blind = Reading { cpu_ns: None, ..reading(1_200, &chunks, 1) };
        assert_eq!(Spent::between(None, &reading(1_000, &[], 0), &blind).cpu_ns, None, "an unread clock is missing");
    }
}
