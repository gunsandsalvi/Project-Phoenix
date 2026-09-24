//! The population's cells: the tables households and small firms live in, their keys, positions, steps and
//! profiles.

pub mod audit;
pub mod check;
pub mod consts;
pub mod envelope;
#[cfg(test)]
mod fixture;
pub mod group_demand;
pub mod holder;
pub mod hot;
pub mod individual;
pub mod join;
pub mod key;
pub mod kind;
pub mod landing;
#[cfg(test)]
mod landing_tests;
pub mod occasion;
pub mod overlap;
pub mod pairing;
pub mod part;
pub mod pick;
pub mod pooled;
pub mod profile;
pub mod rekey;
pub mod review;
pub mod screen;
pub mod seller_spread;
pub mod sig;
pub mod split;
pub mod steps;
pub mod table;
pub mod tiles;
