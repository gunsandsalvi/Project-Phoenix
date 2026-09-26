//! The identifiers and declared data every system shares: products and their industries, occupation families,
//! kinds of plant, and the ways products are made, with the interned sets of ways a firm knows. What differs between
//! products or ways is data these types carry, never a kind a mechanism branches on.

pub mod consts;
pub mod products;
pub mod ways;

pub use products::{IndustryId, ProductDecl, ProductId, Products};
pub use ways::{CapKind, OccFamily, PerUnit, Way, WayId, WaySetId};
