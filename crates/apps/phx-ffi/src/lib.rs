uniffi::setup_scaffolding!();

pub mod bench;
mod json;

pub use bench::{BenchHost, BenchLine, DeviceInfo, run_bench};
