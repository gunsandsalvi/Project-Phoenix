use phx_core::{ItemDecl, SystemEntry};

/// Every system of the world, one line each; the order carries no meaning.
pub const SYSTEMS: &[fn() -> SystemEntry] = &[SystemEntry::of::<phx_geo::Geo>];

/// Every interface crate's items, one line each.
pub const INTERFACES: &[&[ItemDecl]] = &[phx_geo::ITEMS];
