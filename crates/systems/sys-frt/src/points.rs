//! Freight's decision point: a shipper's whether to carry its goods elsewhere.

use phx_core::decisions::DecisionPointDecl;
use phx_macros::clause;
use phx_num::Missing;

/// What a shipper reads: what a lot fetches where it would go and where it is, and what carrying it there costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipIn {
    pub there: i64,
    pub here: i64,
    pub freight: i64,
}

/// Whether to carry it: when the gap exceeds the freight.
#[clause("FRT.5")]
#[must_use]
pub fn ship(i: &ShipIn) -> bool {
    crate::books(i.there, i.here, i.freight)
}

/// A shipper's decision, on its shipping schedule.
pub const SHIP: DecisionPointDecl<ShipIn, bool> = DecisionPointDecl {
    name: "FRT.ship",
    system: "FRT",
    rule: ship,
    schedule: Missing::Present("FRT.shipping_days"),
    wakes: &[],
    runs_on_non_business: true,
    clause: "FRT.5",
};
