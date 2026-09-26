//! TRS, the treasury's first cut: the order its parliament declares it pays in when its cash runs short. Its outlays
//! are the lines it owes on; its funding plan arrives with its own step.

use if_state::kinds::{PaymentOrder, TreasuryKind};
use phx_core::{Declarations, HandlerTable, OpeningCountry, Register, System, declare_prim};
use phx_macros::clause;

declare_prim! {
    /// The rank of the treasury's debt service, its pensions and its benefits in its payment order, the first paid
    /// first.
    pub PAYMENT_ORDER = "TRS.payment_order" {
        kind: Policy, decided_by: "parliament", value: Table1 { axis_exp: 0, exp: 0 }, clause: "TRS.10", scope: PerCountry
    }
}

/// A country's payment order.
///
/// # Errors
/// A primitive missing, of another shape, or not three ranks.
#[clause("TRS.10")]
pub fn order(register: &Register, c: &OpeningCountry) -> Result<PaymentOrder, String> {
    let t = register.table1_in(PAYMENT_ORDER.id, c.id)?;
    let rank = |v: &i64| u8::try_from(*v).map_err(|_| format!("`{}` holds {v}, no rank", PAYMENT_ORDER.id));
    let [debt_service, pensions, benefits] = t.values() else {
        return Err(format!("`{}` ranks other than three kinds of payment", PAYMENT_ORDER.id));
    };
    Ok(PaymentOrder { debt_service: rank(debt_service)?, pensions: rank(pensions)?, benefits: rank(benefits)? })
}

/// The treasury, which the kernel binds.
pub const TREASURY: TreasuryKind = TreasuryKind { order };

/// The treasury.
#[derive(Debug)]
pub struct Trs;

impl System for Trs {
    const CODE: &'static str = "TRS";

    fn declare(d: &mut Declarations) {
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&PAYMENT_ORDER);
        d.market(Box::new(TREASURY));
    }

    fn handlers(_: &mut HandlerTable) {}
}
