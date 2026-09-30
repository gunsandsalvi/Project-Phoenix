//! GDS, goods and commodities: the grades a commodity is classed by, extraction from the deposits whose rights a firm
//! holds, weighing today's price against its outlook, the spoilage of what is held, and the merchants' primitives.

mod consts;
pub mod points;
pub mod rules;

use phx_core::register::values::{Table1, Table2};
use phx_core::{Declarations, Register, StreamDef, System, declare_prim, declare_stream};
use phx_num::Count;

declare_stream! { pub VisitStream = "GDS.visits" { family: World, purpose: Occasion, keyed: false, clause: "REP.21" } }
declare_stream! { pub OpeningStream = "GDS.opening" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_prim! {
    /// Each product's world price at the opening, a unit's in a currency's smallest units, by the product's place: what a
    /// country's snapshot prints it at over its price level there.
    pub OPENING_PRICE = "GDS.opening_price" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 2 }, clause: "GEN.5", scope: Shared
    }
}

declare_prim! {
    /// Each product's price at the opening over its world price in a country, by the product's place: its price level
    /// over the country's.
    pub PRICE_LEVEL = "GDS.price_level" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.5", scope: PerCountry
    }
}

declare_prim! {
    /// The upper bounds of each extracted product's grade classes (rows, the products' places; columns, the classes),
    /// in the deposit's grade index; a product with no row has one class.
    pub GRADE_BOUNDS = "GDS.grade_bounds" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 3 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// How fast a deposit's grade falls as it is worked, the richest part first: its log falls by this times the share
    /// of the deposit taken, by the extracted product's place.
    pub GRADE_FALL = "GDS.grade_fall" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// Each storable product's yearly rate of loss in stock, by its place.
    pub SPOILAGE_RATE = "GDS.spoilage_rate" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 4 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// The room each storable product is kept in, by its place: an open yard, a dry warehouse, a cold store, a tank or
    /// a silo, the storage bought for it from whoever owns such room.
    pub STORAGE = "GDS.storage" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// Whether each product is standardised and meets in a call at each place, by its place; the others meet at posted
    /// prices between firms.
    pub STANDARDISED = "GDS.standardised" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GDS.7", scope: Shared
    }
}

declare_prim! {
    /// Days between an extractor's decisions on its deposits.
    pub EXTRACTION_DAYS = "GDS.extraction_days" { kind: Preference, value: Count, clause: "GDS.4", scope: Shared }
}

declare_prim! {
    /// The product merchants sell, by its place among the products: the firms that make it carry the goods of their
    /// place.
    pub MERCHANT_PRODUCT = "GDS.merchant_product" { kind: Technology, value: Count, clause: "GDS.6", scope: Shared }
}

declare_prim! {
    /// Days between a merchant's decisions to carry the goods of its place.
    pub MERCHANT_DAYS = "GDS.merchant_days" { kind: Preference, value: Count, clause: "GDS.6", scope: Shared }
}

declare_prim! {
    /// Days between the realisations of a holder's spoilage, each over the days since the last.
    pub SPOILAGE_DAYS = "GDS.spoilage_days" { kind: Resolution, value: Count, clause: "GDS.8", scope: Shared }
}

/// A table primitive's values in its decimals.
fn decimals(values: &[i64], exp: u8) -> Vec<f64> {
    let scale = libm::pow(consts::TEN, f64::from(exp));
    values.iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect()
}

/// The places a table primitive's integers carry, as its declaration states them.
fn places(p: &phx_core::PrimDecl) -> u8 {
    match p.value {
        phx_core::ValueType::Table1 { exp, .. } | phx_core::ValueType::Table2 { exp, .. } => exp,
        _ => phx_num::violation!(clause = "GDS.13", "a goods table declared as another type"),
    }
}

/// Each product's yearly rate of loss in stock, by its place, none past the list: what the kernel realises at the
/// stock visits.
///
/// # Errors
/// When the table is not in the register or its axis is not the products' places in order.
pub fn spoilage_rates(register: &Register) -> Result<Vec<f64>, String> {
    let t = register.table1(SPOILAGE_RATE.id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err("the spoilage rates are not by the products' places in order".to_owned());
    }
    Ok(decimals(t.values(), places(&SPOILAGE_RATE)))
}

/// Goods and commodities.
#[derive(Debug)]
pub struct Gds;

impl System for Gds {
    const CODE: &'static str = "GDS";

    fn declare(d: &mut Declarations) {
        let _ = d.prim::<Table2>(&GRADE_BOUNDS);
        for table in [&GRADE_FALL, &STANDARDISED, &SPOILAGE_RATE, &STORAGE] {
            let _ = d.prim::<Table1>(table);
        }
        for count in [&EXTRACTION_DAYS, &MERCHANT_PRODUCT, &MERCHANT_DAYS] {
            let _ = d.prim::<Count>(count);
        }
        let _ = d.prim::<Count>(&SPOILAGE_DAYS);
        d.stream(VisitStream::DECL);
        d.stream(OpeningStream::DECL);
        let _ = d.prim::<Table1>(&OPENING_PRICE);
        let _ = d.prim::<Table1>(&PRICE_LEVEL);
        d.decision(&points::EXTRACT);
    }
}
