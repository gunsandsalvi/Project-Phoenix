use phx_store::ColumnDescriptor;

use crate::clock::Clock;

/// What one sub-step cost: rows and bytes touched, chunks and barriers, and the wall time an application's clock
/// measured, which only counters and reports ever see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecCounters {
    pub rows: u64,
    pub bytes: u64,
    pub chunks: u64,
    pub barriers: u64,
    pub wall_ns: u64,
}

impl ExecCounters {
    pub const ZERO: ExecCounters = ExecCounters { rows: 0, bytes: 0, chunks: 0, barriers: 0, wall_ns: 0 };

    /// One traversal: its rows, the bytes of the columns it touched per row, its chunks and its barrier.
    pub fn pass(&mut self, rows: u64, columns: &[&ColumnDescriptor], chunks: u64) {
        let per_row: u64 = columns.iter().map(|c| u64::from(c.elem_bytes)).sum();
        self.rows += rows;
        self.bytes += rows * per_row;
        self.chunks += chunks;
        self.barriers += 1;
    }

    /// Runs `f`, adding its wall time; a clock that runs backwards adds nothing.
    pub fn timed<T>(&mut self, clock: &dyn Clock, f: impl FnOnce() -> T) -> T {
        let start = clock.now_ns();
        let out = f();
        if let Some(elapsed) = clock.now_ns().checked_sub(start) {
            self.wall_ns += elapsed;
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
    }
}
