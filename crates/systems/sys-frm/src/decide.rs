//! The firms' decisions on the agenda: on each firm's production schedule, the attention it gives its price, and on
//! each of its reviews, its markup and the point it posts. Each reads the firm's own state; a firm that holds no sales
//! outlook, price or unit cost yet takes no decision, which waits for its opening accounts.

use if_firm::facts::{
    ExpectedSales, Markup, Price, PriceAttention, SalesSince, SalesWidth, Stock, UnitCost, WagePerHour,
};
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads, Writes};
use phx_core::{Declarations, Prim, declare_handler, declare_prim};
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{Count, Fixed, Missing, PointTable};
use phx_rand::float::from_i64;

use crate::rules;

declare_prim! {
    /// Days between a firm's production decisions.
    pub PRODUCTION_DAYS = "FRM.production_days" { kind: Preference, value: Count, clause: "FRM.22", scope: Shared }
}

declare_prim! {
    /// The stock a firm aims to hold, in days of expected sales.
    pub STOCK_COVER_DAYS = "FRM.stock_cover_days" { kind: Preference, value: Count, clause: "FRM.4", scope: Shared }
}

declare_prim! {
    /// Days over which a firm closes the gap to the stock it aims to hold.
    pub ADJUSTMENT_DAYS = "FRM.adjustment_days" { kind: Preference, value: Count, clause: "FRM.4", scope: Shared }
}

declare_prim! {
    /// How far a firm's markup moves with its sales against what it expected, on each review.
    pub MARKUP_SALES_SPEED = "FRM.markup_sales_speed" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// How far a firm's markup moves with the prices it sees competitors post against its own, on each review.
    pub MARKUP_SEEN_SPEED = "FRM.markup_seen_speed" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// How strongly a firm's desired price follows the pressure of its demand.
    pub PRESSURE_CURVATURE = "FRM.pressure_curvature" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// The hours of its staff a firm spends reviewing its price.
    pub REVIEW_HOURS = "FRM.review_hours" { kind: Technology, value: Fixed { exp: 2 }, clause: "REP.21", scope: Shared }
}

declare_prim! {
    /// The hours of its staff a firm spends changing its price.
    pub MENU_HOURS = "FRM.menu_hours" { kind: Technology, value: Fixed { exp: 2 }, clause: "FRM.5", scope: Shared }
}

declare_prim! {
    /// The price points of a trade within a decade, as parts of its top: a posted price is one of them times a power
    /// of ten.
    pub PRICE_POINTS = "FRM.price_points" {
        kind: Policy, decided_by: "the trade's convention", value: PointTable { exp: 0 }, clause: "REP.34", scope: Shared
    }
}

/// The handles the decisions read.
#[derive(Clone, Copy, Debug)]
pub struct DecidePrims {
    pub production_days: Prim<Count>,
    pub cover_days: Prim<Count>,
    pub adjustment_days: Prim<Count>,
    pub sales_speed: Prim<Fixed<4>>,
    pub seen_speed: Prim<Fixed<4>>,
    pub curvature: Prim<Fixed<4>>,
    pub review_hours: Prim<Fixed<2>>,
    pub menu_hours: Prim<Fixed<2>>,
    pub points: Prim<PointTable>,
}

impl DecidePrims {
    pub fn declare(d: &mut Declarations) -> DecidePrims {
        DecidePrims {
            production_days: d.prim(&PRODUCTION_DAYS),
            cover_days: d.prim(&STOCK_COVER_DAYS),
            adjustment_days: d.prim(&ADJUSTMENT_DAYS),
            sales_speed: d.prim(&MARKUP_SALES_SPEED),
            seen_speed: d.prim(&MARKUP_SEEN_SPEED),
            curvature: d.prim(&PRESSURE_CURVATURE),
            review_hours: d.prim(&REVIEW_HOURS),
            menu_hours: d.prim(&MENU_HOURS),
            points: d.prim(&PRICE_POINTS),
        }
    }
}

/// What the firms' decisions read of the register, compiled once: the management's speeds and the trade's points.
#[derive(Clone, Debug, PartialEq)]
pub struct Management {
    pub production_days: f64,
    pub cover_days: f64,
    pub sales_speed: f64,
    pub seen_speed: f64,
    pub curvature: f64,
    pub review_hours: f64,
    pub menu_hours: f64,
    /// The points within a decade, each over the decade's top.
    pub points: Vec<i64>,
}

impl Management {
    /// # Errors
    /// A schedule of no days, or points that are no decade's.
    pub fn compile(p: &DecidePrims, register: &phx_core::Register) -> Result<Management, String> {
        let days = p.production_days.shared(register).get();
        if days == 0 {
            return Err("a production schedule of no days".to_owned());
        }
        let points = p.points.shared(register).points().to_vec();
        if points.is_empty() {
            return Err("a trade with no price points".to_owned());
        }
        Ok(Management {
            production_days: phx_rand::float::from_u64(days),
            cover_days: phx_rand::float::from_u64(p.cover_days.shared(register).get()),
            sales_speed: p.sales_speed.shared(register).to_f64(),
            seen_speed: p.seen_speed.shared(register).to_f64(),
            curvature: p.curvature.shared(register).to_f64(),
            review_hours: p.review_hours.shared(register).to_f64(),
            menu_hours: p.menu_hours.shared(register).to_f64(),
            points,
        })
    }

    /// Whether a price is a point of the trade's table in some decade.
    #[clause("REP.34")]
    #[must_use]
    pub fn is_point(&self, price: i64) -> bool {
        self.points_near(from_i64(price)).contains(&price)
    }

    /// The posted prices near a price: the points of the decade it lies in and of the decades either side, each point
    /// the table's part of its decade's top.
    #[clause("REP.34")]
    #[must_use]
    pub fn points_near(&self, price: f64) -> Vec<i64> {
        let Some(top) = self.points.last().copied().map(from_i64) else { return Vec::new() };
        if price <= 0.0 || top <= 0.0 {
            return Vec::new();
        }
        let decade = libm::floor(libm::log10(price)) - libm::floor(libm::log10(top));
        let mut out = Vec::new();
        for k in [decade - 1.0, decade, decade + 1.0] {
            let scale = libm::pow(crate::consts::DECADE, k);
            for m in &self.points {
                if let Some(p) = whole(from_i64(*m) * scale)
                    && p > 0
                    && !out.contains(&p)
                {
                    out.push(p);
                }
            }
        }
        out.sort_unstable();
        out
    }
}

declare_handler! {
    /// A small firm's price review.
    pub ReviewSmall = "FRM.review_small" {
        substep: S5c,
        table: "small_firm",
        reads: [ExpectedSales, SalesSince, Stock, UnitCost, Markup, Price, WagePerHour],
        writes: [Markup, Price],
        clause: "FRM.5",
        body: review,
    }
}

declare_handler! {
    /// A large firm's price review.
    pub ReviewLarge = "FRM.review_large" {
        substep: S5c,
        table: "firm",
        reads: [ExpectedSales, SalesSince, Stock, UnitCost, Markup, Price, WagePerHour],
        writes: [Markup, Price],
        clause: "FRM.5",
        body: review,
    }
}

declare_handler! {
    /// A small firm's attention to its price, on its production schedule.
    pub AttendSmall = "FRM.attend_small" {
        substep: S5b,
        table: "small_firm",
        reads: [ExpectedSales, SalesWidth, Markup, Price, WagePerHour],
        writes: [PriceAttention],
        clause: "REP.38",
        body: attend,
    }
}

declare_handler! {
    /// A large firm's attention to its price, on its production schedule.
    pub AttendLarge = "FRM.attend_large" {
        substep: S5b,
        table: "firm",
        reads: [ExpectedSales, SalesWidth, Markup, Price, WagePerHour],
        writes: [PriceAttention],
        clause: "REP.38",
        body: attend,
    }
}

/// A value in whole units of its fact, to the nearest; none beyond a fact's width.
fn whole(x: f64) -> Option<i64> {
    Fixed::<0>::from_f64(x, phx_num::round::Round::HalfEven).ok().map(Fixed::raw)
}

fn read<F, H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot) -> Option<f64>
where
    F: phx_core::FactDef,
    H: HandlerDecl + Reads<F>,
    S: FactStore + ?Sized,
{
    match ctx.read::<F>(row) {
        Missing::Present(v) => Some(from_i64(v)),
        Missing::Absent => None,
    }
}

/// A review: the markup moved by the sales since the last against those expected, and the point nearest the desired
/// price posted when the move gains more over the next period than its staff's hours to make it cost. The
/// competitors' prices the firm sees arrive with the goods markets; until then it sees none.
#[clause("FRM.5", "REP.34")]
fn review<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<ExpectedSales>
        + Reads<SalesSince>
        + Reads<Stock>
        + Reads<UnitCost>
        + Reads<Markup>
        + Reads<Price>
        + Reads<WagePerHour>
        + Writes<Markup>
        + Writes<Price>,
    S: FactStore + ?Sized,
{
    let m: &Management = ctx.own::<crate::Own>().management();
    let (Some(expected), Some(sold), Some(stock), Some(cost), Some(markup), Some(price), Some(wage)) = (
        read::<ExpectedSales, H, S>(ctx, row),
        read::<SalesSince, H, S>(ctx, row),
        read::<Stock, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<Markup, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<WagePerHour, H, S>(ctx, row),
    ) else {
        return;
    };
    let scale = crate::consts::MARKUP_SCALE;
    let Missing::Present(next) =
        rules::markup::update(markup / scale, (m.sales_speed, m.seen_speed), (sold, expected), Missing::Absent, price)
    else {
        return;
    };
    let target_stock = expected * m.cover_days / m.production_days;
    let Missing::Present(pressure) = rules::price::pressure_stocked(sold, expected, target_stock, stock) else {
        return;
    };
    let wanted = rules::price::desired(next, cost, pressure, m.curvature);
    if let Some(markup) = whole(next * scale) {
        ctx.write::<Markup>(row, markup);
    }
    let Some(current) = whole(price) else { return };
    let revenue = expected * price;
    let menu_cost = m.menu_hours * wage;
    if let Some(point) = rules::price::reprice(&m.points_near(wanted), current, wanted, revenue, next, menu_cost) {
        ctx.write::<Price>(row, point);
    }
}

/// The attention a firm gives its price: its daily chance of a review from the loss a gap costs it, read from its
/// revenue and markup, against the variance of its sales outlook, a review costing its staff's hours at their wage.
#[clause("REP.38")]
fn attend<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<ExpectedSales>
        + Reads<SalesWidth>
        + Reads<Markup>
        + Reads<Price>
        + Reads<WagePerHour>
        + Writes<PriceAttention>,
    S: FactStore + ?Sized,
{
    let m: &Management = ctx.own::<crate::Own>().management();
    let (Some(expected), Some(width), Some(markup), Some(price), Some(wage)) = (
        read::<ExpectedSales, H, S>(ctx, row),
        read::<SalesWidth, H, S>(ctx, row),
        read::<Markup, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<WagePerHour, H, S>(ctx, row),
    ) else {
        return;
    };
    if expected <= 0.0 {
        return;
    }
    let revenue_per_day = expected * price / m.production_days;
    let relative = width / expected;
    let var_own = relative * relative / m.production_days;
    let cost = m.review_hours * wage;
    if cost <= 0.0 {
        return;
    }
    let scale = crate::consts::MARKUP_SCALE;
    if let Missing::Present(chance) =
        rules::attention::review_chance(revenue_per_day, markup / scale, (var_own, 0.0), cost)
        && let Some(billionths) = whole(chance * crate::consts::BILLIONTHS)
    {
        ctx.write::<PriceAttention>(row, billionths);
    }
}

#[cfg(test)]
mod tests {
    use super::Management;

    #[test]
    fn points_near_span_the_decades_around_a_price() {
        let m = Management {
            production_days: 7.0,
            cover_days: 42.0,
            sales_speed: 0.05,
            seen_speed: 0.05,
            curvature: 0.5,
            review_hours: 8.0,
            menu_hours: 2.0,
            points: vec![100, 199, 499, 999],
        };
        assert_eq!(m.points_near(250.0), vec![10, 20, 50, 100, 199, 499, 999, 1000, 1990, 4990, 9990]);
        assert_eq!(m.points_near(2500.0), vec![100, 199, 499, 999, 1000, 1990, 4990, 9990, 10000, 19900, 49900, 99900]);
        assert!(m.points_near(0.0).is_empty());
        assert!(m.is_point(1990) && m.is_point(4990) && !m.is_point(2000));
    }
}
