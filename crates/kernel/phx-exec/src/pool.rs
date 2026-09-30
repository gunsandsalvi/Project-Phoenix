use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

use crate::partition::Cursor;

use phx_num::violation;

use crate::consts::{NS_PER_S, SPIN_CALIBRATION_ROUNDS, SPIN_US, US_PER_S};
use crate::spec::PoolSpec;
use crate::{os, site, trace};

/// What every pool of the process has run, as the counters read it; never read by the world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolUsage {
    pub chunks: u64,
    pub dispatches: u64,
    /// Nanoseconds idle workers have spun waiting for the next dispatch.
    pub spun: u64,
}

/// Every pool's work so far.
#[must_use]
pub fn usage() -> PoolUsage {
    let (c, read) = (trace::counted(), |a: &AtomicU64| a.load(Ordering::Relaxed));
    PoolUsage { chunks: read(&c.chunks), dispatches: read(&c.dispatches), spun: read(&c.spun) }
}

/// `f` run as one counted chunk. A chunk's CPU is not read here: a span reads the whole process's at its ends, which
/// sums every worker's at no cost a chunk. The pool wraps every chunk in it; the counters' own measure calls it bare.
pub fn counted_chunk(f: impl FnOnce()) {
    f();
    trace::counted().chunks.fetch_add(1, Ordering::Relaxed);
}

/// `f` run as a chunk of a dispatch: a dispatch from inside it is refused, and its site names the chunk.
pub(crate) fn as_chunk<R>(chunk: usize, parent: Option<site::Site>, f: impl FnOnce() -> R) -> R {
    /// Restores what the thread was running when the chunk ends, whether it returns or stops the run.
    struct Leave(Option<site::Site>);
    impl Drop for Leave {
        fn drop(&mut self) {
            site::enter_or_leave(self.0);
            site::set_in_chunk(false);
        }
    }
    let _leave = Leave(site::current());
    site::set_in_chunk(true);
    if let Some(at) = parent {
        site::enter(site::Site { chunk: crate::convert::to_u32(chunk), ..at });
    }
    let mut out = None;
    counted_chunk(|| out = Some(f()));
    match out {
        Some(r) => r,
        None => violation!(clause = "TIME.6", "a chunk that returned nothing", chunk = chunk),
    }
}

/// Refuses a dispatch from inside a chunk: it would wait on the workers its own dispatch holds.
pub(crate) fn refuse_nested() {
    if site::in_chunk() {
        violation!(clause = "TIME.6", "a dispatch from inside a chunk");
    }
}

/// `f` on each item, each a chunk: on the pool when one is given, inline in order otherwise.
pub(crate) fn each<I: Send>(pool: Option<&Pool>, items: impl IntoIterator<Item = I>, f: impl Fn(I) + Sync) {
    if let Some(p) = pool {
        p.for_each(items, f);
        return;
    }
    refuse_nested();
    let parent = site::current();
    for (i, item) in items.into_iter().enumerate() {
        as_chunk(i, parent, || f(item));
    }
}

/// `f` of every index below `n` in index order, each a chunk: on the pool when one is given, inline otherwise.
pub(crate) fn map<T: Send>(pool: Option<&Pool>, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    if let Some(p) = pool {
        return p.map(n, f);
    }
    refuse_nested();
    let parent = site::current();
    (0..n).map(|i| as_chunk(i, parent, || f(i))).collect()
}

/// `f` of every item in the items' order, each a chunk: on the pool when one is given, inline otherwise.
pub(crate) fn map_items<I: Send, T: Send>(pool: Option<&Pool>, items: Vec<I>, f: impl Fn(I) -> T + Sync) -> Vec<T> {
    if let Some(p) = pool {
        return p.map_items(items, f);
    }
    refuse_nested();
    let parent = site::current();
    items.into_iter().enumerate().map(|(i, item)| as_chunk(i, parent, || f(item))).collect()
}

/// Why a pool could not be started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolError(pub String);

#[derive(Debug)]
struct Shared {
    tids: Vec<AtomicI32>,
    unpinned: AtomicUsize,
    /// Advanced at every dispatch; a spinning worker stops when it changes.
    epoch: AtomicU64,
    /// The rounds of spin that make `SPIN_US` on these cores, and the nanoseconds the calibration's rounds took,
    /// learnt at the start; none where the system gives no CPU clock, and then workers park at once.
    spin: Option<(u64, u64)>,
}

/// The engine's workers, pinned one per core; every parallel primitive of the crate runs on it, and nothing else in
/// the engine starts a thread.
pub struct Pool {
    threads: rayon_core::ThreadPool,
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool").field("shared", &self.shared).finish_non_exhaustive()
    }
}

/// The rounds of spin that make `SPIN_US`, and the nanoseconds a fixed count of rounds took on the CPU clock.
fn calibrate() -> Option<(u64, u64)> {
    let start = os::process_cpu_ns()?;
    for _ in 0..SPIN_CALIBRATION_ROUNDS {
        std::hint::spin_loop();
    }
    let spent = os::process_cpu_ns()?.checked_sub(start).filter(|ns| *ns > 0)?;
    let ns_per_us = NS_PER_S / US_PER_S;
    Some(((SPIN_CALIBRATION_ROUNDS * SPIN_US * ns_per_us).checked_div(spent)?, spent))
}

impl Pool {
    /// Starts one worker per core given, pinning each as it starts; a refused pin leaves that worker unpinned
    /// and is counted. The spin a worker keeps up between dispatches is timed once here.
    ///
    /// # Errors
    /// When no core is given or the system will not start the threads.
    pub fn new(spec: &PoolSpec) -> Result<Pool, PoolError> {
        let workers = spec.workers();
        if workers == 0 {
            return Err(PoolError("a pool of no workers".to_owned()));
        }
        let shared = Arc::new(Shared {
            tids: (0..workers).map(|_| AtomicI32::new(0)).collect(),
            unpinned: AtomicUsize::new(0),
            epoch: AtomicU64::new(0),
            spin: calibrate(),
        });
        let (cores, pin, started) = (spec.cores.clone(), spec.pin, Arc::clone(&shared));
        let threads = rayon_core::ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|i| format!("phx-worker-{i}"))
            .start_handler(move |i| {
                if let Some(tid) = started.tids.get(i) {
                    tid.store(os::current_tid(), Ordering::Relaxed);
                }
                if pin && !cores.get(i).is_some_and(|c| os::pin_current(*c)) {
                    started.unpinned.fetch_add(1, Ordering::Relaxed);
                }
            })
            .build()
            .map_err(|e| PoolError(e.to_string()))?;
        // Every worker has run its start handler before its first job, so after this the ids are all known.
        threads.broadcast(|_| ());
        Ok(Pool { threads, shared })
    }

    #[must_use]
    pub fn workers(&self) -> usize {
        self.shared.tids.len()
    }

    /// The workers' kernel thread ids, for performance-hint sessions.
    #[must_use]
    pub fn worker_tids(&self) -> Vec<i32> {
        self.shared.tids.iter().map(|t| t.load(Ordering::Relaxed)).collect()
    }

    /// Workers the system would not pin.
    #[must_use]
    pub fn unpinned(&self) -> usize {
        self.shared.unpinned.load(Ordering::Relaxed)
    }

    /// The rounds of spin a worker keeps up after a dispatch, none where the system gives no CPU clock.
    #[must_use]
    pub fn spin_rounds(&self) -> Option<u64> {
        self.shared.spin.map(|(rounds, _)| rounds)
    }

    /// One dispatch: `f` on every slot of `out` with its index, each a chunk. One job a worker takes the slots from a
    /// shared cursor until none is left, so no task is made a chunk and nothing is allocated; each result lies at its
    /// chunk's index whichever worker made it.
    pub(crate) fn run_into<T: Send>(&self, out: &mut [T], f: impl Fn(usize, &mut T) + Sync) {
        self.run(&Cursor::one(out), |i, slot, ()| f(i, slot));
    }

    /// One dispatch over two slices of one length: `f` on the items at each index, each pair a chunk, handed out as
    /// `run_into` hands out its slots.
    pub(crate) fn run_pairs<A: Send, B: Send>(
        &self,
        (left, right): (&mut [A], &mut [B]),
        each: impl Fn(usize, &mut A, &mut B) + Sync,
    ) {
        self.run(&Cursor::new(left, right), each);
    }

    /// One dispatch: one job a worker, each taking the cursor's next index until none is left, so no task is made a
    /// chunk and nothing is allocated.
    fn run<A: Send, B: Send>(&self, cursor: &Cursor<'_, A, B>, each: impl Fn(usize, &mut A, &mut B) + Sync) {
        refuse_nested();
        let parent = site::current();
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        trace::counted().dispatches.fetch_add(1, Ordering::Relaxed);
        crate::partition::run_everywhere(
            &self.threads,
            &|| {
                while let Some((at, x, y)) = cursor.next() {
                    as_chunk(at, parent, || each(at, x, y));
                }
            },
            self.spin(),
        );
    }

    /// `f` once for each item, each a chunk, on whichever worker takes it: for the crate's kernels whose items own
    /// what they write.
    pub(crate) fn for_each<I: Send>(&self, items: impl IntoIterator<Item = I>, f: impl Fn(I) + Sync) {
        let mut items: Vec<Option<I>> = items.into_iter().map(Some).collect();
        self.run_into(&mut items, |_, item| {
            if let Some(i) = item.take() {
                f(i);
            }
        });
    }

    /// `f` of every index below `n`, each a chunk, each stored at its index whatever order they ran in.
    pub(crate) fn map<T: Send>(&self, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
        let mut out: Vec<Option<T>> = std::iter::repeat_with(|| None).take(n).collect();
        self.run_into(&mut out, |i, slot| *slot = Some(f(i)));
        let Some(all) = out.into_iter().collect::<Option<Vec<T>>>() else {
            violation!(clause = "TIME.6", "a dispatched chunk that never ran", n = n);
        };
        all
    }

    /// `f` of every item, each a chunk, each result stored at its item's place, for work that owns what each item
    /// hands it.
    pub(crate) fn map_items<I: Send, T: Send>(&self, items: Vec<I>, f: impl Fn(I) -> T + Sync) -> Vec<T> {
        let n = items.len();
        let mut cells: Vec<(Option<I>, Option<T>)> = items.into_iter().map(|i| (Some(i), None)).collect();
        self.run_into(&mut cells, |_, (item, out)| *out = item.take().map(&f));
        let Some(all) = cells.into_iter().map(|(_, out)| out).collect::<Option<Vec<T>>>() else {
            violation!(clause = "TIME.6", "a dispatched chunk that never ran", n = n);
        };
        all
    }

    /// `f` run once on every worker at the same time, results in worker order: for measuring the workers
    /// themselves, never for the world's work, whose results must not depend on which worker ran it.
    pub fn on_every_worker<T: Send>(&self, f: impl Fn() -> T + Sync) -> Vec<T> {
        refuse_nested();
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        let out = self.threads.broadcast(|_| f());
        self.keep_warm();
        out
    }

    /// Keeps the workers awake for `SPIN_US` after a dispatch, so the next one does not wait for them to wake, then
    /// lets them park; the spin is counted in nanoseconds.
    fn keep_warm(&self) {
        self.threads.spawn_broadcast({
            let spin = self.spin();
            move |_| spin()
        });
    }

    /// A worker's spin after a dispatch: at most `SPIN_US`, ended early by the next dispatch; none where the system
    /// gives no CPU clock.
    fn spin(&self) -> impl Fn() + Send + Sync + 'static {
        let shared = Arc::clone(&self.shared);
        let epoch = shared.epoch.load(Ordering::Acquire);
        move || {
            let Some((most, spent)) = shared.spin else { return };
            let mut rounds = 0;
            while rounds < most && shared.epoch.load(Ordering::Relaxed) == epoch {
                std::hint::spin_loop();
                rounds += 1;
            }
            trace::counted().spun.fetch_add(rounds * spent / SPIN_CALIBRATION_ROUNDS, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn results_placed_by_index() {
        for workers in [1, 2, 3, 8] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            assert_eq!(pool.workers(), workers);
            assert!(pool.worker_tids().iter().all(|t| *t != 0));
            let mut squares = vec![0; 1000];
            pool.run_into(&mut squares, |i, s| *s = i * i);
            assert!(squares.iter().enumerate().all(|(i, s)| *s == i * i));
        }
        assert!(Pool::new(&PoolSpec::unpinned(0)).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn pinning_counts_refusals() {
        let pool = Pool::new(&PoolSpec { cores: vec![0, 100_000], pin: true }).unwrap();
        assert_eq!(pool.unpinned(), 1, "a core that does not exist is refused and counted");
        let detected = Pool::new(&PoolSpec::detect()).unwrap();
        assert_eq!(detected.unpinned(), 0, "the cores the process may use accept their workers");
    }

    #[test]
    fn a_violation_in_a_worker_stops_the_caller() {
        let pool = Pool::new(&PoolSpec::unpinned(2)).unwrap();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.run_into(&mut [0; 4], |i, _| {
                if i == 3 {
                    phx_num::violation!(clause = "TIME.6", "test");
                }
            });
        }));
        let payload = caught.expect_err("the violation reaches the caller");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("TIME.6"));
        // The pool still runs after a chunk stopped.
        let mut out = [0; 3];
        pool.run_into(&mut out, |i, o| *o = i);
        assert_eq!(out, [0, 1, 2]);
    }

    #[test]
    fn nested_dispatch_refused() {
        let pool = Pool::new(&PoolSpec::unpinned(2)).unwrap();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.run_into(&mut [0; 2], |_, _| pool.run_into(&mut [0; 2], |_, _| ()));
        }));
        let payload = caught.expect_err("a dispatch from a chunk stops the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("TIME.6"));
    }

    #[test]
    fn spin_is_bounded() {
        let pool = Pool::new(&PoolSpec::unpinned(2)).unwrap();
        let Some((rounds, spent)) = pool.shared.spin else { return };
        // The rounds a worker spins after a dispatch cost what the calibration's rounds cost at most `SPIN_US` of.
        let ns = rounds * spent / crate::consts::SPIN_CALIBRATION_ROUNDS;
        assert!(ns <= crate::consts::SPIN_US * 1_000, "{rounds} rounds spin {ns} ns");
        assert!(ns * 2 > crate::consts::SPIN_US * 1_000, "and most of it");
    }
}
