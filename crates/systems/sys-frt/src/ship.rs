//! The shipper's visit: a firm holding more of its product than it expects to sell where it stands over its planning
//! days weighs carrying the rest to each other place its country marks the good at, by each mode carriage is sold in
//! where it stands, and asks to carry it to the place where the gap over the freight is widest, when there is one.

use if_firm::facts::ExpectedSales;
use if_firm::known::Product;
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads};
use phx_core::{Emits, declare_handler};
use phx_id::Slot;
use phx_macros::clause;
use phx_market::carriage::FreightTech;
use phx_market::intents::ShipIntent;
use phx_num::Missing;
use phx_rand::float::from_i64;

declare_handler! {
    /// A small firm's shipping.
    pub ShipSmall = "FRT.ship_small" {
        substep: S5c,
        table: "small_firm",
        reads: [Product, ExpectedSales],
        writes: [],
        intents: [ShipIntent],
        clause: "FRT.5",
        body: ship,
    }
}

declare_handler! {
    /// A large firm's shipping.
    pub ShipLarge = "FRT.ship_large" {
        substep: S5c,
        table: "firm",
        reads: [Product, ExpectedSales],
        writes: [],
        intents: [ShipIntent],
        clause: "FRT.5",
        body: ship,
    }
}

/// What the shipper reads, compiled once: freight's technology, each product's lot, and the carriage kind's code.
#[derive(Debug)]
pub struct Own {
    pub tech: FreightTech,
    pub lots: Vec<i64>,
    pub kind: u64,
}

/// The visit: the surplus over the planning days' expected sales, in whole lots, carried to the place with the widest
/// gap over its freight, when the gap is wider than the freight.
#[clause("FRT.5", "FRT.11")]
fn ship<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl + Reads<Product> + Reads<ExpectedSales> + Emits<ShipIntent>,
    S: FactStore + ?Sized,
{
    let (Missing::Present(product), Missing::Present(expected)) =
        (ctx.read::<Product>(row), ctx.read::<ExpectedSales>(row))
    else {
        return;
    };
    let Ok(product) = u16::try_from(product) else { return };
    let own: &Own = ctx.own::<Own>();
    // Only a product with a weight can be carried.
    if usize::from(product) >= own.tech.units_a_tonne.len() {
        return;
    }
    let Some(lot) = own.lots.get(usize::from(product)).copied() else { return };
    let surplus = ctx.held(row, product, 0) - expected;
    if surplus < lot {
        return;
    }
    let Missing::Present(here) = ctx.mark(row, product, 0) else { return };
    let mut best: Option<(f64, phx_core::Away)> = None;
    for a in ctx.away(row, product, 0) {
        let Missing::Present(cost) = a.outbound else { continue };
        let gap = from_i64(a.there - here - cost);
        if crate::books(a.there, here, cost) && best.is_none_or(|(b, _)| gap > b) {
            best = Some((gap, a));
        }
    }
    let Some((_, a)) = best else { return };
    let (kind, qty) = (own.kind, surplus - surplus % lot);
    ctx.emit(&ShipIntent { row, kind, product, grade: 0, from: Missing::Absent, qty, to: a.zone, mode: a.mode });
}
