//! The modules: one spec system, instrument family or seed each. A module declares its kinds,
//! units, params, phases, participants, audit contributions and seed contribution, and it reaches
//! the kernel ONLY through `module.rs`'s doors — never by importing another module (Law 15).

pub mod bank_capital;
pub mod benchmarks;
pub mod capital_programme;
pub mod cds;
pub mod commodities;
pub mod control;
pub mod cost_of_capital;
pub mod currency;
pub mod dealing;
pub mod derivative_layer;
pub mod employment;
pub mod equity;
pub mod estate;
pub mod expectations;
pub mod firms;
pub mod forced_sale;
pub mod fx_forwards;
pub mod funds;
pub mod goods;
pub mod households;
pub mod housing;
pub mod insurers;
pub mod irs;
pub mod lending;
pub mod loss;
pub mod money;
pub mod money_market;
pub mod mortality;
pub mod polity;
pub mod ratings;
pub mod recipe;
pub mod redeemable;
pub mod reporting;
pub mod second_opinion;
pub mod spot_fx;
pub mod trade_credit;
pub mod securities_lending;
pub mod securitisation;
pub mod sovereign;
