//! The population's cells: the tables households and small firms live in, their keys, positions, steps and
//! profiles.

pub mod attach;
pub mod audit;
pub mod check;
pub mod consts;
pub mod envelope;
pub mod explicit;
#[cfg(test)]
mod fixture;
pub mod group_demand;
pub mod holder;
pub mod hot;
pub mod index;
pub mod individual;
pub mod join;
pub mod key;
pub mod kind;
pub mod landing;
#[cfg(test)]
mod landing_tests;
pub mod measure;
pub mod occasion;
pub mod overlap;
pub mod pairing;
pub mod part;
pub mod pick;
pub mod pooled;
pub mod population;
pub mod prims;
pub mod profile;
pub mod promote;
pub mod rekey;
pub mod renumber;
pub mod review;
pub mod sample;
pub mod screen;
pub mod seller_spread;
pub mod sig;
pub mod split;
pub mod steps;
pub mod synthetic;
pub mod table;
pub mod tiles;
pub mod tolerance;
