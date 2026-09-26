//! A firm's industry and the ways it knows: its industry's public ways at the opening and at its founding, and the
//! ways it discovers, licenses or imitates later, kept as one interned set.

use phx_core::{AttrDecl, declare_fact};

use crate::consts::{INDUSTRIES, WAY_SETS};

declare_fact! {
    /// The industry whose products the firm makes, an index of the industries the products declare.
    pub Industry = "FRM.industry" {
        value: Type, kinds: ["firm", "small_firm"], writer: "FRM", audience: Public, repr: Key, clause: "TEC.4",
    }
}

declare_fact! {
    /// The set of ways the firm knows, an identity of the technology's interned sets.
    pub Known = "TEC.known" {
        value: Count, kinds: ["firm", "small_firm"], writer: "TEC", audience: Party, repr: Key, clause: "TEC.4",
    }
}

/// A small firm's industry.
pub const INDUSTRY: AttrDecl = AttrDecl { name: "FRM.industry", values: INDUSTRIES, clause: "TEC.4" };
/// The ways a small firm knows.
pub const KNOWN: AttrDecl = AttrDecl { name: "TEC.known", values: WAY_SETS, clause: "TEC.4" };
