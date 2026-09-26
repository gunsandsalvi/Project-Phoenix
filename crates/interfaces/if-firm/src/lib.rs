//! The firm, the vocabulary every system that reads or writes a firm shares: what it makes and the ways it knows.
//! A large firm keeps them as facts of its row; a small firm as its agent's attributes.

pub mod consts;
pub mod known;

use phx_core::ItemDecl;

/// Every item the crate exports.
pub const ITEMS: &[ItemDecl] =
    &[<known::Industry as phx_core::FactDef>::ITEM, <known::Known as phx_core::FactDef>::ITEM];
