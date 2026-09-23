use std::time::Instant;

use phx_exec::Clock;

/// Wall time since the run began, which reaches the run's records and never the world.
#[derive(Debug)]
pub struct WallClock {
    start: Instant,
}

impl WallClock {
    pub fn new() -> WallClock {
        WallClock { start: Instant::now() }
    }
}

impl Clock for WallClock {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}
