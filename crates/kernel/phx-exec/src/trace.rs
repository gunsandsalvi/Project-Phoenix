//! The bench's trace: what the run is doing, told as it does it, so a slow or stuck run shows where it is and how far
//! it got. The run's host sets where the marks go and the clock that times them; with none set, nothing is told. No
//! outcome ever reads it.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::Clock;

/// A mark of the run, at the depth of the spans open around it: a span begun, a span ended with its own time where
/// the clock gave one, or a note of named counts.
#[derive(Clone, Copy, Debug)]
pub enum Mark<'a> {
    Begun(&'static str),
    Ended(&'static str, Option<u64>),
    Note(&'static str, &'a [(&'static str, i64)]),
}

/// Where the marks go, and the clock that times the spans.
pub trait Tracer: Clock {
    fn mark(&self, mark: Mark<'_>, depth: usize);
}

/// Where the marks go, and the spans open now, for the marks' depth.
struct Trace {
    tracer: &'static dyn Tracer,
    depth: AtomicUsize,
}

static TRACE: OnceLock<Trace> = OnceLock::new();

/// Sends every mark from now on to `tracer`; a second setting is ignored.
pub fn set(tracer: &'static dyn Tracer) {
    let _ = TRACE.set(Trace { tracer, depth: AtomicUsize::new(0) });
}

/// Whether anyone is told, so a note's counts are gathered only when they will be read.
#[must_use]
pub fn on() -> bool {
    TRACE.get().is_some()
}

/// `work` run between its span's two marks, the second with the time it took.
pub fn span<T>(name: &'static str, work: impl FnOnce() -> T) -> T {
    let Some(Trace { tracer: t, depth: open }) = TRACE.get() else { return work() };
    let depth = open.fetch_add(1, Ordering::Relaxed);
    t.mark(Mark::Begun(name), depth);
    let start = t.now_ns();
    let out = work();
    let ns = t.now_ns().checked_sub(start);
    open.fetch_sub(1, Ordering::Relaxed);
    t.mark(Mark::Ended(name, ns), depth);
    out
}

pub fn note(name: &'static str, counts: &[(&'static str, i64)]) {
    if let Some(Trace { tracer, depth }) = TRACE.get() {
        tracer.mark(Mark::Note(name, counts), depth.load(Ordering::Relaxed));
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
    use super::doubling;

    #[test]
    fn a_loop_tells_at_powers_of_two() {
        let told: Vec<u64> = (0..20).filter(|n| doubling(*n)).collect();
        assert_eq!(told, vec![1, 2, 4, 8, 16]);
    }
}
