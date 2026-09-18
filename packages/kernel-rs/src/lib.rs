//! Project Phoenix's kernel, columnar.
//!
//! The TypeScript kernel is 32,514 lines and this replaces it store by store, against it as the
//! ORACLE: the Rust engine is right when it reproduces `events 178604`, `audit 356268` and the op
//! count on the same seed (`docs/IMPLEMENTATION.md` 0g.41).
//!
//! Why at all: measured on the real world, a record fetched by id costs **47.80 ns** in TypeScript
//! and **0.77 ns** here; a traversal of the 544,104 holdings **95.70 ms** against **2.23 ms**; and
//! one real module ported whole came out **10.9×** faster with the same algorithm (0g.40). The
//! collector, which is **22.4% of every period**, does not exist here.
//!
//! The laws are not relaxed by the move. What was a TypeScript type or an eslint rule becomes a
//! type, a `debug_assert` or a checker here, and 0g.43 is the step that says each one's new home.

pub mod chronicle;
pub mod clearing;
pub mod draw;
pub mod ids;
pub mod instruments;
pub mod journal;
pub mod ledger;
pub mod mechanisms;
pub mod module;
pub mod nouns;
pub mod num;
pub mod params;
pub mod parties;
pub mod prices;
pub mod register;
pub mod session;
pub mod systems;
pub mod world;
pub mod assembly;
pub mod audit;
pub mod calendar;
