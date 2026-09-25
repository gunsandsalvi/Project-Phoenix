//! HSG, housing: here its opening alone — each household that does not own its home rents it, paying a rent that is
//! the country's median rent burden of its income, on the nearest rent point, to landlords apportioned once every
//! household is drawn. Its decisions, the dwelling stock and its owners arrive with its own step.

mod consts;
mod tenancies;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_prim};

pub use tenancies::{Declared, TENANCY, Tenancies, TenancyStream};

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
        kind: Shape, value: Fixed { exp: 6 }, clause: "REP.34", scope: Shared, shape: placeholder("HSG")
    }
}

/// Housing.
#[derive(Debug)]
pub struct Hsg;

impl System for Hsg {
    const CODE: &'static str = "HSG";

    fn declare(d: &mut Declarations) {
        d.stream(TenancyStream::DECL);
        let tenancies = Tenancies { tenure: d.prim(&TENURE), ratio: d.prim(&RENT_POINT_RATIO) };
        d.contribution(Box::new(Declared));
        let draw: Box<dyn phx_ledger::attachments::AttachmentDraw> = Box::new(tenancies);
        d.attachment(Box::new(draw));
    }

    fn handlers(_: &mut HandlerTable) {}
}
