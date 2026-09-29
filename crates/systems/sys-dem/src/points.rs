//! Demography's decision point: whether a household tries for a child, at its head's birthday.

use phx_core::decisions::DecisionPointDecl;
use phx_core::schedule::WakeKind;
use phx_macros::clause;
use phx_num::Missing;

use crate::fertility::{ChildIn, Scale, tries};

/// What the decision reads: the household as it stands, and its country's equivalence scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TryIn {
    pub child: ChildIn,
    pub scale: Scale,
}

/// Whether it tries.
#[clause("POP.16")]
#[must_use]
pub fn try_for(i: &TryIn) -> bool {
    tries(&i.child, i.scale)
}

/// A household's decision to try for a child.
pub const TRY_FOR_CHILD: DecisionPointDecl<TryIn, bool> = DecisionPointDecl {
    name: "DEM.try_for_child",
    system: "DEM",
    rule: try_for,
    schedule: Missing::Absent,
    wakes: &[WakeKind::KinkDay],
    runs_on_non_business: true,
    clause: "POP.16",
};
