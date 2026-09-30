use std::cell::Cell;

/// What a thread is running: the day, the sub-step, the handler and the chunk, so a violation report can say where
/// the run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub day: u32,
    pub substep: u8,
    pub handler: u16,
    pub chunk: u32,
}

thread_local! {
    static CURRENT: Cell<Option<Site>> = const { Cell::new(None) };
    /// Whether this thread is running a chunk, when a dispatch from it would wait on the workers it occupies.
    static IN_CHUNK: Cell<bool> = const { Cell::new(false) };
}

/// Marks this thread as running a chunk, or no longer.
pub(crate) fn set_in_chunk(running: bool) {
    IN_CHUNK.with(|c| c.set(running));
}

/// Whether this thread is running a chunk.
pub(crate) fn in_chunk() -> bool {
    IN_CHUNK.with(Cell::get)
}

/// Records what this thread is about to run.
pub fn enter(site: Site) {
    CURRENT.with(|c| c.set(Some(site)));
}

pub fn leave() {
    CURRENT.with(|c| c.set(None));
}

/// Restores what this thread was running before a chunk, or nothing.
pub(crate) fn enter_or_leave(site: Option<Site>) {
    CURRENT.with(|c| c.set(site));
}

/// What this thread was running, read by the application's panic hook and nothing else.
#[must_use]
pub fn current() -> Option<Site> {
    CURRENT.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::{Site, current, enter, leave};

    #[test]
    fn site_is_per_thread() {
        let here = Site { day: 3, substep: 7, handler: 2, chunk: 9 };
        enter(here);
        assert_eq!(current(), Some(here));
        assert_eq!(std::thread::scope(|s| s.spawn(current).join().unwrap()), None);
        leave();
        assert_eq!(current(), None);
    }
}
