//! A household's fertility, as its attributes hold it: its own preference for children, drawn once, and whether its
//! last decision was to try for a child, which the conception hazard reads.

use phx_core::AttrDecl;

use crate::consts::IDEAL_VALUES;

/// Whether the household's last decision was to try for a child.
pub const TRYING: AttrDecl = AttrDecl { name: "DEM.trying", values: 2, clause: "POP.10" };
pub const NOT_TRYING: u32 = 0;
pub const TRYING_FOR_CHILD: u32 = 1;

/// The household's ideal number of children, one more than the number, held from its first decision on; nought before
/// it is drawn.
pub const IDEAL: AttrDecl = AttrDecl { name: "DEM.ideal_children", values: IDEAL_VALUES, clause: "POP.2" };
pub const IDEAL_UNDRAWN: u32 = 0;
