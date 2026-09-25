uniffi::setup_scaffolding!();

pub mod bench;
mod json;
pub mod world;

pub use bench::{BenchHost, BenchLine, DeviceInfo, run_bench};
pub use world::run_world;
