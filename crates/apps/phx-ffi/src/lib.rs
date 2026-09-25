uniffi::setup_scaffolding!();

mod agents;
pub mod bench;
mod json;
pub mod load;
pub mod programme;
pub mod stopped;
pub mod world;

pub use bench::{BenchHost, BenchLine, DeviceInfo, run_bench};
pub use load::run_load;
pub use programme::run_programme;
pub use world::run_world;
