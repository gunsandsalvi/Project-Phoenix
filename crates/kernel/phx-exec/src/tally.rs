//! A count several workers add to at once. Integer sums do not depend on the order they are added in, so its value at
//! the end of a pass is the same on any pool.

use std::sync::atomic::{AtomicU64, Ordering};

/// A count added to from any worker.
#[derive(Debug, Default)]
pub struct Tally(AtomicU64);

impl Tally {
    /// Adds to the count.
    pub fn add(&self, n: u64) {
        self.0.fetch_add(n, Ordering::Relaxed);
    }

    /// The count, read once every worker adding to it has joined.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::Tally;

    #[test]
    fn workers_add_to_one_count() {
        let t = Tally::default();
        std::thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| (0..1_000).for_each(|_| t.add(1)));
            }
        });
        assert_eq!(t.get(), 4_000);
    }
}
