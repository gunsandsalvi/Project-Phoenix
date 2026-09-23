pub mod names;
pub mod runner;
pub mod stream;
pub mod time;

pub use names::{NAMES, Names};
pub use runner::{Audit, CloseInputs, CloseRecord};
pub use stream::StreamAudit;
pub use time::{TIME, Time};

use phx_core::AuditFamily;

/// The families the kernel owns, which every world runs beside its systems' own.
#[must_use]
pub fn kernel_families() -> Vec<Box<dyn AuditFamily>> {
    vec![Box::new(Names), Box::new(Time)]
}

#[cfg(test)]
mod tests;
