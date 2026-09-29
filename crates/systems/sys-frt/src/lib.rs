//! FRT, freight: the technology of vehicles by mode and of goods' weight, carriage's product, each carrier's mode at
//! the opening, and the shipper's rule on its schedule.

mod consts;
pub mod points;
pub use phx_core::register::values::Table1;
use phx_core::{Declarations, FactDef, Register, StreamDef, System, declare_prim, declare_stream};
use phx_macros::clause;
use phx_market::carriage::FreightTech;
use phx_num::Count;

declare_stream! { pub LotStream = "FRT.capacity_lot" { purpose: Meeting, keyed: false, clause: "FRT.7" } }
declare_stream! { pub OpeningStream = "FRT.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub VisitStream = "FRT.visits" { purpose: Occasion, keyed: false, clause: "FRT.5" } }

declare_prim! {
    /// Each mode's share of the people carriage employs, by the mode's place, which the carriers' modes are drawn by.
    pub MODE_SHARE = "FRT.mode_share" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 1 }, clause: "GEN.2", scope: Shared
    }
}

declare_prim! {
    /// The product carriage is sold as, by its place among the products.
    pub CARRIAGE_PRODUCT = "FRT.carriage_product" { kind: Technology, value: Count, clause: "FRT.1", scope: Shared }
}

declare_prim! {
    /// Days between a shipper's decisions to carry its goods elsewhere.
    pub SHIPPING_DAYS = "FRT.shipping_days" { kind: Preference, value: Count, clause: "FRT.5", scope: Shared }
}

declare_prim! {
    /// The chain vehicles are held in: the place of transport equipment among the kinds of plant.
    pub VEHICLES = "FRT.vehicles" { kind: Technology, value: Count, clause: "FRT.1", scope: Shared }
}

declare_prim! {
    /// The tonne-km a unit of vehicles carries a day, by the mode's place.
    pub TONNE_KM = "FRT.tonne_km" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 6 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The metres a vehicle runs a day, by the mode's place.
    pub METRES_A_DAY = "FRT.metres_a_day" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The days loading takes at each end of a trip, by the mode's place.
    pub LOADING_DAYS = "FRT.loading_days" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The units of carriage a tonne-km takes, by the mode's place: what carrying it is worth at the opening's
    /// prices, in the carriage product's units.
    pub CARRIAGE_A_TONNE_KM = "FRT.carriage_a_tonne_km" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 2 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The units of a good in a tonne, by its product's place, for the storable products.
    pub UNITS_A_TONNE = "FRT.units_a_tonne" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

/// A table of one axis over places in order, in the decimals its declaration gives.
fn places(register: &Register, decl: &phx_core::PrimDecl) -> Result<Vec<f64>, String> {
    let (id, phx_core::ValueType::Table1 { exp, .. }) = (decl.id, decl.value) else {
        return Err(format!("`{}` is declared as no table of one axis", decl.id));
    };
    let t = register.table1(id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err(format!("`{id}` is not by places in order"));
    }
    let scale = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
    Ok(t.values().iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect())
}

/// Whole values of a table of one axis over places in order.
fn whole<T: TryFrom<i64>>(register: &Register, id: &str) -> Result<Vec<T>, String> {
    let t = register.table1(id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err(format!("`{id}` is not by places in order"));
    }
    t.values().iter().map(|v| T::try_from(*v).map_err(|_| format!("`{id}` holds {v}, beyond its type"))).collect()
}

/// Freight's technology as the register holds it.
///
/// # Errors
/// When a table is missing or not by places in order, or a value is beyond its type.
pub fn tech(register: &Register) -> Result<FreightTech, String> {
    Ok(FreightTech {
        vehicles: u32::try_from(register.count(VEHICLES.id)?).map_err(|e| e.to_string())?,
        tonne_km: places(register, &TONNE_KM)?,
        metres_a_day: whole(register, METRES_A_DAY.id)?,
        loading_days: whole(register, LOADING_DAYS.id)?,
        carriage_a_tonne_km: places(register, &CARRIAGE_A_TONNE_KM)?,
        units_a_tonne: places(register, &UNITS_A_TONNE)?,
        carriage_product: u16::try_from(register.count(CARRIAGE_PRODUCT.id)?).map_err(|e| e.to_string())?,
    })
}

/// Whether a shipper books room: when what the goods fetch where they go, less what they fetch where they are,
/// exceeds what carrying them costs.
#[clause("FRT.5", "FRT.11")]
#[must_use]
pub fn books(there: i64, here: i64, freight: i64) -> bool {
    there - here > freight
}

/// Freight and logistics.
#[derive(Debug)]
pub struct Frt;

impl System for Frt {
    const CODE: &'static str = "FRT";

    fn declare(d: &mut Declarations) {
        let _ = d.prim::<Count>(&VEHICLES);
        for table in [&TONNE_KM, &METRES_A_DAY, &LOADING_DAYS, &CARRIAGE_A_TONNE_KM, &UNITS_A_TONNE] {
            let _ = d.prim::<Table1>(table);
        }
        d.stream(LotStream::DECL);
        let mode = <if_firm::freight::Mode as FactDef>::ITEM;
        d.claim(mode.name);
        let _ = d.prim::<Table1>(&MODE_SHARE);
        let _ = d.prim::<Count>(&CARRIAGE_PRODUCT);
        let _ = d.prim::<Count>(&SHIPPING_DAYS);
        d.stream(OpeningStream::DECL);
        d.stream(VisitStream::DECL);
        d.decision(&points::SHIP);
    }
}

#[cfg(test)]
mod tests {
    use super::books;

    #[test]
    fn shipper_books_only_above_freight() {
        assert!(books(1_300, 1_000, 250), "a gap of 300 pays a freight of 250");
        assert!(!books(1_250, 1_000, 250), "a gap equal to the freight does not");
        assert!(!books(900, 1_000, 50), "goods dearer here stay");
    }
}
