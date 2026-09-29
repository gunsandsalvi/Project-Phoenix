//! HSG, housing: here its primitives alone, which the data declares; its tenancies, decisions, the dwelling stock and
//! its owners arrive with its own step.

use phx_core::register::values::Table1;
use phx_core::{Declarations, System, declare_prim};
use phx_num::Fixed;

declare_prim! {
    /// Owners with a mortgage among owners, subsidised tenants among tenants, and the median mortgage and rent
    /// burdens: the rent burden read.
    pub TENURE = "HSG.tenure_and_costs" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The ratio between neighbouring rent points, the round numbers rents are paid at, until landlords set their own.
    pub RENT_POINT_RATIO = "HSG.rent_point_ratio" {
        kind: Resolution, value: Fixed { exp: 6 }, clause: "REP.34", scope: Shared
    }
}

/// Housing.
#[derive(Debug)]
pub struct Hsg;

impl System for Hsg {
    const CODE: &'static str = "HSG";

    fn declare(d: &mut Declarations) {
        let _: phx_core::Prim<Table1> = d.prim(&TENURE);
        let _: phx_core::Prim<Fixed<6>> = d.prim(&RENT_POINT_RATIO);
    }
}
