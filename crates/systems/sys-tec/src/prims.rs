//! What the data declares of products and ways: the products shared by every country, and each country's opening
//! ways, one per product, as tables over the products.

use phx_core::register::values::{Table1, Table2};
use phx_core::{Declarations, Prim, ProductEntry, declare_prim};

declare_prim! {
    /// The products, each with its unit, industry, and how it is held and delivered.
    pub PRODUCTS = "TEC.products" { kind: Technology, value: Products, clause: "TEC.1", scope: Shared }
}

declare_prim! {
    /// Tonnes an extracted product takes from its deposit per unit made, by product.
    pub DEPOSIT_DRAW = "TEC.deposit_draw" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 9 }, clause: "TEC.2", scope: Shared
    }
}

declare_prim! {
    /// What a country's way for each product (columns) uses of each product (rows) per unit made.
    pub INPUTS = "TEC.inputs" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "TEC.13", scope: PerCountry
    }
}

declare_prim! {
    /// Hours of each occupation family (rows) a country's way for each product (columns) takes per unit.
    pub LABOUR = "TEC.labour" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "TEC.13", scope: PerCountry
    }
}

declare_prim! {
    /// Stock of each kind of plant (rows) a unit a year of each product (columns) needs, in a country's ways.
    pub CAPITAL = "TEC.capital" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "TEC.13", scope: PerCountry
    }
}

declare_prim! {
    /// Hectares of land a unit of each product takes a year, in a country's ways.
    pub LAND = "TEC.land" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 9 }, clause: "TEC.13", scope: PerCountry
    }
}

declare_prim! {
    /// Days from starting a unit of each product to finishing it.
    pub LEAD_TIME = "TEC.lead_time" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "TEC.2", scope: Shared
    }
}

declare_prim! {
    /// The share of what is started that is finished, by product, in parts per million.
    pub YIELD = "TEC.yield" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "TEC.2", scope: Shared
    }
}

declare_prim! {
    /// The least quantity of each product started at once, in its units.
    pub BATCH = "TEC.batch" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "TEC.2", scope: Shared
    }
}

/// The handles the technology is compiled from.
#[derive(Debug, Clone, Copy)]
pub struct TecPrims {
    pub products: Prim<Vec<ProductEntry>>,
    pub deposit_draw: Prim<Table1>,
    pub inputs: Prim<Table2>,
    pub labour: Prim<Table2>,
    pub capital: Prim<Table2>,
    pub land: Prim<Table1>,
    pub lead_time: Prim<Table1>,
    pub yield_ppm: Prim<Table1>,
    pub batch: Prim<Table1>,
}

impl TecPrims {
    pub fn declare(d: &mut Declarations) -> TecPrims {
        TecPrims {
            products: d.prim(&PRODUCTS),
            deposit_draw: d.prim(&DEPOSIT_DRAW),
            inputs: d.prim(&INPUTS),
            labour: d.prim(&LABOUR),
            capital: d.prim(&CAPITAL),
            land: d.prim(&LAND),
            lead_time: d.prim(&LEAD_TIME),
            yield_ppm: d.prim(&YIELD),
            batch: d.prim(&BATCH),
        }
    }
}
