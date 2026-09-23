use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

use phx_num::violation;

use crate::consts::SPIN_ROUNDS;
use crate::os;
use crate::spec::PoolSpec;

/// Why a pool could not be started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolError(pub String);

#[derive(Debug)]
struct Shared {
    tids: Vec<AtomicI32>,
    unpinned: AtomicUsize,
    /// Advanced at every dispatch; a spinning worker stops when it changes.
    epoch: AtomicU64,
    spun: AtomicU64,
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

impl Pool {
    /// Starts one worker per core given, pinning each as it starts; a refused pin leaves that worker unpinned
    /// and is counted.
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
            spun: AtomicU64::new(0),
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

    /// Spin rounds idle workers have spent waiting for the next dispatch, a cost the counters report.
    #[must_use]
    pub fn spun(&self) -> u64 {
        self.shared.spun.load(Ordering::Relaxed)
    }

    /// Runs `f` once per item, on whichever worker is free; returns when every item is done. Nothing it computes may
    /// depend on which worker ran which item or in what order.
    pub fn for_each<I: Send, F: Fn(I) + Sync>(&self, items: impl IntoIterator<Item = I>, f: F) {
        let items: Vec<I> = items.into_iter().collect();
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        let f = &f;
        self.threads.scope(move |s| {
            for item in items {
                s.spawn(move |_| f(item));
            }
        });
        self.keep_warm();
    }

    /// `f` of every index below `n`, each stored at its index whatever order they ran in.
    pub fn map<T: Send>(&self, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
        let mut out: Vec<Option<T>> = std::iter::repeat_with(|| None).take(n).collect();
        self.for_each(out.iter_mut().enumerate(), |(i, slot)| *slot = Some(f(i)));
        let Some(all) = out.into_iter().collect::<Option<Vec<T>>>() else {
            violation!(clause = "TIME.6", "a dispatched task that never ran", n = n);
        };
        all
    }

    /// `f` run once on every worker at the same time, results in worker order: for measuring the workers
    /// themselves, never for the world's work, whose results must not depend on which worker ran it.
    pub fn on_every_worker<T: Send>(&self, f: impl Fn() -> T + Sync) -> Vec<T> {
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        let out = self.threads.broadcast(|_| f());
        self.keep_warm();
        out
    }

    /// Keeps the workers awake for a while after a dispatch, so the next one does not wait for them to wake.
    fn keep_warm(&self) {
        let shared = Arc::clone(&self.shared);
        let epoch = shared.epoch.load(Ordering::Acquire);
        self.threads.spawn_broadcast(move |_| {
            let mut rounds = 0;
            while rounds < SPIN_ROUNDS && shared.epoch.load(Ordering::Relaxed) == epoch {
                std::hint::spin_loop();
                rounds += 1;
            }
            shared.spun.fetch_add(rounds, Ordering::Relaxed);
        });
    }
}

/// Runs `f` on each item, on the pool when one is given and on the calling thread otherwise.
pub fn each<I: Send>(pool: Option<&Pool>, items: impl IntoIterator<Item = I>, f: impl Fn(I) + Sync) {
    match pool {
        Some(p) => p.for_each(items, f),
        None => items.into_iter().for_each(f),
    }
}

/// `f` of every index below `n` in index order, on the pool when one is given and on the calling thread otherwise.
pub fn map<T: Send>(pool: Option<&Pool>, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    match pool {
        Some(p) => p.map(n, f),
        None => (0..n).map(f).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn map_places_results_by_index() {
        for workers in [1, 2, 3, 8] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            assert_eq!(pool.workers(), workers);
            assert!(pool.worker_tids().iter().all(|t| *t != 0));
            let squares = pool.map(1000, |i| i * i);
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
            pool.for_each(0..4, |i| {
                if i == 3 {
                    phx_num::violation!(clause = "TIME.6", "test");
                }
            });
        }));
        let payload = caught.expect_err("the violation reaches the caller");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("TIME.6"));
    }
}
