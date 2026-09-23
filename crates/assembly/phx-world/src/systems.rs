use phx_core::{ItemDecl, SystemEntry};

/// Every system of the world, one line each; the order carries no meaning.
pub const SYSTEMS: &[fn() -> SystemEntry] = &[];

/// Every interface crate's items, one line each.
pub const INTERFACES: &[&[ItemDecl]] = &[];
