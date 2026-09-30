//! What each operation of a base costs on a measured day: its items, the CPU its threads spent, the wall it took, and
//! the faults and allocations the counters give; a count the machine did not give is missing, never zero.

use std::collections::BTreeMap;

use phx_exec::Clock;
use phx_exec::trace::{Reading, Spent};

/// One operation's measure over a day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measure {
    pub items: u64,
    /// CPU time summed over every thread that worked on it.
    pub cpu_ns: Option<u64>,
    pub wall_ns: Option<u64>,
    pub faults: Option<u64>,
    pub allocations: Option<u64>,
}

/// A sum that stays missing once any part of it was.
fn sum(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    a?.checked_add(b?)
}

impl Measure {
    /// Nanoseconds an item: CPU summed over threads over items, the same for any number of workers.
    #[must_use]
    pub fn ns_per_op(&self) -> Option<u64> {
        self.cpu_ns?.checked_div(self.items)
    }

    fn add(&mut self, m: &Measure) {
        self.items += m.items;
        self.cpu_ns = sum(self.cpu_ns, m.cpu_ns);
        self.wall_ns = sum(self.wall_ns, m.wall_ns);
        self.faults = sum(self.faults, m.faults);
        self.allocations = sum(self.allocations, m.allocations);
    }
}

/// The measures of a day, by base and operation, timed on the application's clock.
pub struct Measures<'c> {
    clock: &'c dyn Clock,
    ops: BTreeMap<(String, String), Measure>,
}

impl std::fmt::Debug for Measures<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Measures").field("ops", &self.ops).finish_non_exhaustive()
    }
}

impl<'c> Measures<'c> {
    #[must_use]
    pub fn new(clock: &'c dyn Clock) -> Measures<'c> {
        Measures { clock, ops: BTreeMap::new() }
    }

    /// Adds an operation's work to its measure.
    pub fn record(&mut self, base: &str, op: &str, m: &Measure) {
        self.ops.entry((base.to_owned(), op.to_owned())).and_modify(|at| at.add(m)).or_insert_with(|| m.clone());
    }

    /// `work` over `items` run and recorded as the operation's: its CPU over every thread, wall, faults and
    /// allocations.
    pub fn read<T>(&mut self, base: &str, op: &str, items: u64, work: impl FnOnce() -> T) -> T {
        let before = Reading::now();
        let start = self.clock.now_ns();
        let out = work();
        let wall_ns = self.clock.now_ns().checked_sub(start);
        let spent = Spent::between(Some(items), &before, &Reading::now());
        let m = Measure { items, cpu_ns: spent.cpu_ns, wall_ns, faults: spent.faults, allocations: spent.allocs };
        self.record(base, op, &m);
        out
    }

    /// Every operation measured, by base and name.
    pub fn iter(&self) -> impl Iterator<Item = (&(String, String), &Measure)> {
        self.ops.iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}
