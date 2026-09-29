//! TAX, the tax system's first cut: income tax withheld from wages by marginal bands over a member's yearly wage, and
//! a consumption tax at the till, each country's schedule as its parliament declares it. The kernel withholds and
//! charges; the annual return and the full system arrive with their own steps.

use if_state::kinds::{TaxKind, TaxLaw};
use phx_core::{Declarations, HandlerTable, OpeningCountry, Register, System, declare_prim};
use phx_macros::clause;
use phx_num::Fixed;

declare_prim! {
    /// Each income tax band's lower edge, as a multiple of the mean wage, the first at nothing.
    pub BAND_EDGES = "TAX.income_band_edges" {
        kind: Policy, decided_by: "parliament", value: Table1 { axis_exp: 0, exp: 2 }, clause: "TAX.1", scope: PerCountry
    }
}

declare_prim! {
    /// Each income tax band's marginal rate.
    pub BAND_RATES = "TAX.income_band_rates" {
        kind: Policy, decided_by: "parliament", value: Table1 { axis_exp: 0, exp: 3 }, clause: "TAX.1", scope: PerCountry
    }
}

declare_prim! {
    /// The consumption tax's rate on the price before it.
    pub CONSUMPTION_RATE = "TAX.consumption_rate" {
        kind: Policy, decided_by: "parliament", value: Fixed { exp: 3 }, clause: "TAX.1", scope: PerCountry
    }
}

declare_prim! {
    /// The day of the month after a tax is collected by which its collector remits it to the treasury.
    pub REMIT_DAY = "TAX.remit_day" { kind: Policy, decided_by: "parliament", value: Count, clause: "TAX.8", scope: Shared }
}

/// A table of one axis's values in its declared decimals.
fn table(register: &Register, id: &str, country: phx_id::CountryId) -> Result<Vec<f64>, String> {
    let t = register.table1_in(id, country)?;
    let phx_core::ValueType::Table1 { exp, .. } = register.decl_by_id(id)?.value else {
        return Err(format!("`{id}` is no table of one axis"));
    };
    let scale = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
    Ok(t.values().iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect())
}

/// A country's taxes.
///
/// # Errors
/// A primitive missing or of another shape, or bands whose edges and rates differ in number.
#[clause("TAX.1")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<TaxLaw, String> {
    let edges = table(register, BAND_EDGES.id, c.id)?;
    let rates = table(register, BAND_RATES.id, c.id)?;
    if edges.len() != rates.len() {
        return Err("the income tax's band edges and rates differ in number".to_owned());
    }
    Ok(TaxLaw {
        bands: edges.into_iter().zip(rates).collect(),
        consumption_rate: register.fixed_in(CONSUMPTION_RATE.id, c.id)?,
        remit_day: u32::try_from(register.count(REMIT_DAY.id)?).map_err(|e| e.to_string())?,
    })
}

/// The consumption tax a price paid includes: the rate's share of the price with the tax, since the price posted
/// is the price paid.
#[clause("TAX.1", "TAX.7")]
#[must_use]
pub fn included(price: f64, rate: f64) -> f64 {
    price * rate / (1.0 + rate)
}

/// The tax system, which the kernel binds.
pub const TAXES: TaxKind = TaxKind { withheld_from: "employment", law, included };

#[cfg(test)]
mod tests {
    use super::included;

    #[test]
    fn consumption_tax_ad_valorem_per_unit() {
        assert!((included(120.0, 0.2) - 20.0).abs() < 1e-9, "a fifth on 100 is 20 in a price of 120");
        assert!(included(50.0, 0.0).abs() < 1e-12, "no rate, no tax");
    }
}

/// The tax system.
#[derive(Debug)]
pub struct Tax;

impl System for Tax {
    const CODE: &'static str = "TAX";

    fn declare(d: &mut Declarations) {
        for p in [&BAND_EDGES, &BAND_RATES] {
            let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<3>> = d.prim(&CONSUMPTION_RATE);
        let _: phx_core::Prim<phx_num::Count> = d.prim(&REMIT_DAY);
        d.market(Box::new(TAXES));
    }

    fn handlers(_: &mut HandlerTable) {}
}
