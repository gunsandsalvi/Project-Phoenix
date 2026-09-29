//! SOV, the sovereign's debt, first cut: bills sold at uniform-price call auctions to the banks that bid, sized by the
//! treasury's placeholder plan, each a contract its holders are paid at maturity. Bonds, dealers and the funding plan
//! arrive with their own steps.

mod consts;
pub mod points;
mod rules;

use if_state::kinds::{BillKind, BillLaw};
use phx_core::{Declarations, OpeningCountry, Register, System, declare_prim};
use phx_macros::clause;
use phx_num::{Count, Fixed};

pub use rules::{bid, clear, size};

declare_prim! {
    /// A bill's face in dollars, the unit its holders' contracts are counted in.
    pub FACE = "SOV.bill_face" { kind: Policy, decided_by: "parliament", value: Count, clause: "SOV.1", scope: Shared }
}

declare_prim! {
    /// The weeks a bill runs.
    pub WEEKS = "SOV.bill_weeks" { kind: Policy, decided_by: "parliament", value: Count, clause: "SOV.1", scope: Shared }
}

declare_prim! {
    /// The weekday a country's bill auctions are held on, the first of the week its first.
    pub WEEKDAY = "SOV.auction_weekday" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "SOV.3", scope: Shared
    }
}

declare_prim! {
    /// The weeks of its outflow the treasury keeps as its cash buffer.
    pub BUFFER_WEEKS = "SOV.buffer_weeks" {
        kind: Policy, decided_by: "parliament", value: Fixed { exp: 1 }, clause: "TRS.9", scope: Shared
    }
}

/// A bill: the treasury's liability to the banks that hold it, each holding the contracts it bought.
/// The opening's units of a currency to the dollar.
const UNITS_PER_DOLLAR: &str = "GEN.units_per_dollar";

/// A country's bills.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("SOV.1", "SOV.3", "TRS.9")]
pub fn law(register: &Register, country: &OpeningCountry) -> Result<BillLaw, String> {
    Ok(BillLaw {
        face: face(register, country.id)?,
        weeks: u16::try_from(register.count(WEEKS.id)?).map_err(|e| e.to_string())?,
        weekday: u32::try_from(register.count(WEEKDAY.id)?).map_err(|e| e.to_string())?,
        buffer_weeks: register.fixed(BUFFER_WEEKS.id)?,
    })
}

/// A bill's face in the currency's smallest units: its face in dollars, at the country's units to the dollar.
///
/// # Errors
/// A primitive missing or of another shape, or a face beyond whole units.
pub fn face(register: &Register, country: phx_id::CountryId) -> Result<i64, String> {
    let dollars = register.count(FACE.id)?;
    let units = register.count_in(UNITS_PER_DOLLAR, country)?;
    dollars
        .checked_mul(units)
        .and_then(|f| i64::try_from(f).ok())
        .ok_or_else(|| format!("a bill's face of {dollars} dollars beyond whole units"))
}

/// The sovereign's bills, which the kernel runs.
pub const BILLS: BillKind = BillKind { law, size: &points::SIZE, bid: &points::BID, clear: rules::clear };

/// The sovereign's debt.
#[derive(Debug)]
pub struct Sov;

impl System for Sov {
    const CODE: &'static str = "SOV";

    fn declare(d: &mut Declarations) {
        for p in [&FACE, &WEEKS, &WEEKDAY] {
            let _: phx_core::Prim<Count> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<1>> = d.prim(&BUFFER_WEEKS);
        d.market(Box::new(BILLS));
        d.decision(&points::SIZE);
        d.decision(&points::BID);
    }
}
