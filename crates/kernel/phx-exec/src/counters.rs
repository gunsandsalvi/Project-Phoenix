use phx_num::capacity_exceeded;
use phx_store::ColumnDescriptor;

use crate::clock::Clock;
use crate::consts::KINDS;

/// What one sub-step cost: rows and bytes touched, chunks and barriers, the rows visited of each kind, the day
/// buffers' bytes, the workers' spin and the wall time an application's clock measured, which only counters and
/// reports ever see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecCounters {
    pub rows: u64,
    pub bytes: u64,
    pub chunks: u64,
    pub barriers: u64,
    pub wall_ns: u64,
    pub visited: [u64; KINDS],
    pub buffer_bytes: u64,
    pub spun: u64,
}

/// `n` added to a counter; a counter past `u64` stops the run rather than wrapping into a small number.
fn add(counter: &mut u64, n: u64) {
    match counter.checked_add(n) {
        Some(sum) => *counter = sum,
        None => capacity_exceeded!("a measurement counter", u64::MAX, i128::from(*counter) + i128::from(n)),
    }
}

impl ExecCounters {
    pub const ZERO: ExecCounters = ExecCounters {
        rows: 0,
        bytes: 0,
        chunks: 0,
        barriers: 0,
        wall_ns: 0,
        visited: [0; KINDS],
        buffer_bytes: 0,
        spun: 0,
    };

    /// One traversal: its rows, the bytes of the columns it touched per row, its chunks and its barrier.
    pub fn pass(&mut self, rows: u64, columns: &[&ColumnDescriptor], chunks: u64) {
        let per_row: u64 = columns.iter().map(|c| u64::from(c.elem_bytes)).sum();
        let Some(bytes) = rows.checked_mul(per_row) else {
            capacity_exceeded!("a measurement counter", u64::MAX, i128::from(rows) * i128::from(per_row));
        };
        add(&mut self.rows, rows);
        add(&mut self.bytes, bytes);
        add(&mut self.chunks, chunks);
        add(&mut self.barriers, 1);
    }

    /// Rows of one kind a traversal visited.
    pub fn visited(&mut self, kind: usize, rows: u64) {
        let Some(count) = self.visited.get_mut(kind) else {
            capacity_exceeded!("the kinds a counter holds", KINDS, kind);
        };
        add(count, rows);
    }

    /// Bytes a day buffer holds.
    pub fn buffered(&mut self, bytes: u64) {
        add(&mut self.buffer_bytes, bytes);
    }

    /// Rounds the workers spun waiting between dispatches.
    pub fn spin(&mut self, rounds: u64) {
        add(&mut self.spun, rounds);
    }

    /// Runs `f`, adding its wall time; a clock that runs backwards adds nothing.
    pub fn timed<T>(&mut self, clock: &dyn Clock, f: impl FnOnce() -> T) -> T {
        let start = clock.now_ns();
        let out = f();
        if let Some(elapsed) = clock.now_ns().checked_sub(start) {
            add(&mut self.wall_ns, elapsed);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use phx_store::{ColumnDescriptor, FieldDescriptor, FieldTag, Transform};

    use super::ExecCounters;
    use crate::clock::Clock;

    /// Each read advances ten nanoseconds.
    struct Ticks(AtomicU64);

    impl Clock for Ticks {
        fn now_ns(&self) -> u64 {
            self.0.fetch_add(10, Ordering::Relaxed) + 10
        }
    }

    #[test]
    fn counters_add_rows_bytes_and_time() {
        const F: &[FieldDescriptor] =
            &[FieldDescriptor { name: "w", offset: 0, width: 4, transform: Transform::Plain, tag: FieldTag::Plain }];
        let weights = ColumnDescriptor::checked::<u32>("w", 4096, F).unwrap();
        let mut c = ExecCounters::ZERO;
        c.pass(1000, &[&weights, &weights], 1);
        assert_eq!((c.rows, c.bytes, c.chunks, c.barriers), (1000, 8000, 1, 1));
        let clock = Ticks(AtomicU64::new(0));
        assert_eq!(c.timed(&clock, || 7), 7);
        assert_eq!(c.wall_ns, 10);
        c.visited(3, 40);
        c.visited(3, 2);
        assert_eq!(c.visited[3], 42);
    }

    #[test]
    fn counter_overflow_stops() {
        let mut c = ExecCounters::ZERO;
        c.visited(0, u64::MAX);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.visited(0, 1)));
        let payload = caught.expect_err("a counter past u64 stops");
        let stopped = payload.downcast_ref::<phx_num::violation::CapacityExceeded>();
        assert_eq!(stopped.map(|s| s.needed), Some(i128::from(u64::MAX) + 1));
    }
}
