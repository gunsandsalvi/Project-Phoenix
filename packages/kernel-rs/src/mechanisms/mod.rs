//! The modules: one spec system, instrument family or seed each. A module declares its kinds,
//! units, params, phases, participants, audit contributions and seed contribution, and it reaches
//! the kernel ONLY through `module.rs`'s doors — never by importing another module (Law 15).

pub mod capital_programme;
pub mod estate;
pub mod expectations;
pub mod forced_sale;
pub mod lending;
pub mod loss;
pub mod money;
pub mod mortality;
pub mod redeemable;
pub mod sovereign;
