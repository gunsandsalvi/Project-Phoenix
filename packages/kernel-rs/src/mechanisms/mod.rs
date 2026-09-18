//! The modules: one spec system, instrument family or seed each. A module declares its kinds,
//! units, params, phases, participants, audit contributions and seed contribution, and it reaches
//! the kernel ONLY through `module.rs`'s doors — never by importing another module (Law 15).

pub mod bank_capital;
pub mod benchmarks;
pub mod capital_programme;
pub mod cost_of_capital;
pub mod currency;
pub mod dealing;
pub mod equity;
pub mod estate;
pub mod expectations;
pub mod forced_sale;
pub mod lending;
pub mod loss;
pub mod money;
pub mod mortality;
pub mod redeemable;
pub mod sovereign;
