//! What each operation of a base costs on a measured day: its items, the CPU its threads spent, the wall it took, and
//! the faults, allocations and bytes the counters give.

use std::collections::BTreeMap;

/// One operation's measure over a day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measure {
    pub items: u64,
    /// CPU time summed over every thread that worked on it.
    pub cpu_ns: u64,
    pub wall_ns: u64,
    pub faults: u64,
    pub allocations: u64,
}

impl Measure {
    /// Nanoseconds an item: CPU summed over threads over items, the same for any number of workers.
    #[must_use]
    pub fn ns_per_op(&self) -> Option<u64> {
        self.cpu_ns.checked_div(self.items)
    }
}

/// The measures of a day, by base and operation.
#[derive(Debug, Clone, Default)]
pub struct Measures {
    ops: BTreeMap<(String, String), Measure>,
}

impl Measures {
    /// Adds an operation's work to its measure.
    pub fn record(&mut self, base: &str, op: &str, m: &Measure) {
        let at = self.ops.entry((base.to_owned(), op.to_owned())).or_default();
        at.items += m.items;
        at.cpu_ns += m.cpu_ns;
        at.wall_ns += m.wall_ns;
        at.faults += m.faults;
        at.allocations += m.allocations;
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
