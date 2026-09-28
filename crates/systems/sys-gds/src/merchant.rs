//! The merchant's visit: a firm of the trade carries the standardised goods of its place when the price it expects
//! over the days to its next decision, less what spoils and what its money costs it meanwhile, beats today's; it bids
//! for them with the money it holds beyond what its own planned sales cost, at no more than they are worth to it
//! carried; and when carrying no longer pays, it offers what it holds at no less.

use if_firm::facts::{ExpectedSales, Method, RequiredReturn, UnitCost};
use if_firm::known::Product;
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads};
use phx_core::{Emits, declare_handler};
use phx_id::Slot;
use phx_macros::clause;
use phx_market::intents::OrderIntent;
use phx_market::order::{Side, Step};
use phx_num::{Missing, PriceRaw};
use phx_rand::float::{floor_to_i64, from_i64};

use crate::consts::DAYS_A_YEAR;
use crate::extract::Own;
use crate::rules::stockist::{carries, carry_value};

declare_handler! {
    /// A small firm's trade in the goods of its place.
    pub MerchantSmall = "GDS.merchant_small" {
        substep: S5c,
        table: "small_firm",
        reads: [Product, UnitCost, ExpectedSales, RequiredReturn, Method],
        writes: [],
        intents: [OrderIntent],
        clause: "GDS.6",
        body: trade,
    }
}

declare_handler! {
    /// A large firm's trade in the goods of its place.
    pub MerchantLarge = "GDS.merchant_large" {
        substep: S5c,
        table: "firm",
        reads: [Product, UnitCost, ExpectedSales, RequiredReturn, Method],
        writes: [],
        intents: [OrderIntent],
        clause: "GDS.6",
        body: trade,
    }
}

fn read<F, H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot) -> Option<i64>
where
    F: phx_core::FactDef,
    H: HandlerDecl + Reads<F>,
    S: FactStore + ?Sized,
{
    match ctx.read::<F>(row) {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

/// The visit, a merchant's alone: for each standardised good its place marks, by each grade class, the carry
/// weighed against today's price, a bid when it pays and an offer of what it holds when it does not.
#[clause("GDS.6", "GDS.8", "MKT.16")]
fn trade<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<Product>
        + Reads<UnitCost>
        + Reads<ExpectedSales>
        + Reads<RequiredReturn>
        + Reads<Method>
        + Emits<OrderIntent>,
    S: FactStore + ?Sized,
{
    let own: &Own = ctx.own::<Own>();
    let Some(sells) = read::<Product, H, S>(ctx, row).and_then(|p| u16::try_from(p).ok()) else { return };
    if sells != own.merchant {
        return;
    }
    let (Some(cost), Some(expected), Some(required), Some(method), Missing::Present(money)) = (
        read::<UnitCost, H, S>(ctx, row),
        read::<ExpectedSales, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
        read::<Method, H, S>(ctx, row).and_then(|m| u16::try_from(m).ok()),
        ctx.money(row),
    ) else {
        return;
    };
    let Some(own_lot) = own.products.get(usize::from(sells)).map(|p| p.lot) else { return };
    let rate = from_i64(required) / crate::consts::RATE_SCALE;
    let horizon = from_i64(own.merchant_days) / DAYS_A_YEAR;
    // What its own planned sales will cost it is kept back from its bids.
    let mut free = from_i64(money) - from_i64(cost) * from_i64(expected) / from_i64(own_lot);
    let goods: Vec<(u16, u8, i64, f64)> = (0_u16..)
        .zip(&own.products)
        .filter(|(_, p)| p.standardised)
        .filter_map(|(i, p)| {
            // A product's classes lie between its bounds: one more than its bounds, one where it has none.
            let classes = u8::try_from(p.bounds.len() + 1).ok()?;
            let spoil = own.spoilage.get(usize::from(i)).copied()?;
            Some((0..classes).map(move |g| (i, g, p.lot, spoil)))
        })
        .flatten()
        .collect();
    for (product, grade, lot, spoil) in goods {
        let (Missing::Present(price), Missing::Present(outlook)) =
            (ctx.mark(row, product, grade), ctx.outlook(row, product, grade, method))
        else {
            continue;
        };
        let worth = carry_value(from_i64(outlook), (spoil, 0.0), (rate, horizon));
        let kind = own.market(product);
        if carries(from_i64(price), worth) {
            let lots = floor_to_i64(free / from_i64(price)).filter(|l| *l > 0);
            let (Some(lots), Some(limit)) = (lots, floor_to_i64(worth)) else { continue };
            let Some(qty) = lots.checked_mul(lot) else { continue };
            free -= from_i64(lots) * from_i64(price);
            let steps = vec![Step { limit: PriceRaw::from_raw(limit), qty }];
            ctx.emit(&OrderIntent { row, kind, product, grade, at: Missing::Absent, side: Side::Buy, steps });
        } else {
            let held = ctx.held(row, product, grade);
            let offered = held - held % lot;
            let Some(limit) = floor_to_i64(-worth).map(|f| -f) else { continue };
            if offered > 0 {
                let steps = vec![Step { limit: PriceRaw::from_raw(limit), qty: offered }];
                ctx.emit(&OrderIntent { row, kind, product, grade, at: Missing::Absent, side: Side::Sell, steps });
            }
        }
    }
}
