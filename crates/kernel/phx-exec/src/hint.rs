/// A performance-hint session: the turn's target and its actual duration, so the system can raise or lower the
/// cores' frequency for the next.
pub trait PerfHint {
    fn begin_turn(&self, target_ns: u64);
    fn end_turn(&self, actual_ns: u64);
}

/// No session, where the system offers none.
#[derive(Clone, Copy, Debug)]
pub struct NoHint;

impl PerfHint for NoHint {
    fn begin_turn(&self, _: u64) {}
    fn end_turn(&self, _: u64) {}
}
