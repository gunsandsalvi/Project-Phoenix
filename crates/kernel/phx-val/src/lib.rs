//! Each party's own view: outlooks of public series by method and of its own variables, the heuristics that form
//! them and the switching among them, surprises and confidence, attention to its decisions, and values by its own
//! simple models. Pure functions only: nothing here reads the world, so nothing here can run it forward.

pub mod attention;
pub mod experience;
pub mod heuristic;
pub mod heuristics;
pub mod method;
pub mod outlook;
pub mod registered;
pub mod schedule;
pub mod surprise;
pub mod switching;
pub mod value;

#[cfg(test)]
mod testing {
    /// The clause a call stops the run with, or none when it returns.
    pub fn refused<R>(f: impl FnOnce() -> R + std::panic::UnwindSafe) -> Option<&'static str> {
        let payload = std::panic::catch_unwind(f).err()?;
        payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause)
    }
}
