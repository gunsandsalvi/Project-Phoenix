//! HSG, housing: here its primitives alone, which the data declares; its tenancies, decisions, the dwelling stock and
//! its owners arrive with its own step.

use phx_core::register::values::Table1;
use phx_core::{Declarations, System, declare_prim};

declare_prim! {
    /// Owners with a mortgage among owners, subsidised tenants among tenants, and the median mortgage and rent
    /// burdens: the rent burden read.
    pub TENURE = "HSG.tenure_and_costs" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

/// Housing.
#[derive(Debug)]
pub struct Hsg;

impl System for Hsg {
    const CODE: &'static str = "HSG";

    fn declare(d: &mut Declarations) {
        let _: phx_core::Prim<Table1> = d.prim(&TENURE);
    }
}
