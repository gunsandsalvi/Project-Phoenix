//! The firms' decisions on the agenda: on each firm's production schedule, the attention it gives its price, and on
//! each of its reviews, its markup and the point it posts. Each reads the firm's own state; a firm that holds no sales
//! outlook, price or unit cost yet takes no decision, which waits for its opening accounts.

use if_firm::facts::{
    DeliveredAtReview, DeliveredSeen, ExpectedSales, LastReview, Markup, Method, Price, PriceAttention, SalesWidth,
    UnitCost, WagePerHour,
};
use if_firm::known::Industry;
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
    /// Each memory type's speed of correction, by its place, and how many widths a surprise must pass to wake.
    pub gains: Vec<f64>,
    pub sensitivity: f64,
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
        let types = u16::try_from(register.count("VAL.memory_types")?).map_err(|e| e.to_string())?;
        let gain = register.distribution("VAL.adaptive_gain")?;
        let scale = libm::pow(crate::consts::DECADE, f64::from(gain.exp));
        let gains = phx_core::register::values::TypeSet::build(gain, types)?
            .types()
            .iter()
            .map(|t| from_i64(t.value) / scale)
            .collect();
        Ok(Management {
            gains,
            sensitivity: register.fixed("VAL.attention_sensitivity")?,
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

    /// A table point at a decade `k` from the table's own: the point times ten to the `k`, when that is a whole price.
    fn at_decade(point: i64, k: i32) -> Option<i64> {
        let ten = i64::from(crate::consts::TEN);
        if k >= 0 {
            (0..k).try_fold(point, |p, _| p.checked_mul(ten))
        } else {
            let down = (0..k.unsigned_abs()).try_fold(1_i64, |p, _| p.checked_mul(ten))?;
            (point % down == 0).then_some(point / down)
        }
    }

    /// The decade of a positive price from the table's own: how many powers of ten it lies above the table's top.
    fn decade_of(&self, price: f64) -> Option<i32> {
        let top = from_i64(*self.points.last()?);
        if price <= 0.0 || top <= 0.0 {
            return None;
        }
        let d = libm::floor(libm::log10(price)) - libm::floor(libm::log10(top));
        i32::try_from(phx_rand::float::floor_to_i64(d)?).ok()
    }

    /// Whether a price is a point of the trade's table in some decade.
    #[clause("REP.34")]
    #[must_use]
    pub fn is_point(&self, price: i64) -> bool {
        let Some(d) = self.decade_of(from_i64(price)) else { return false };
        self.points.iter().any(|m| Self::at_decade(*m, d) == Some(price))
    }

    /// The posted prices near a price: the points of the decade it lies in and of the decades either side, each point
    /// the table's part of its decade's top and whole at its scale.
    #[clause("REP.34")]
    #[must_use]
    pub fn points_near(&self, price: f64) -> Vec<i64> {
        let Some(d) = self.decade_of(price) else { return Vec::new() };
        let mut out: Vec<i64> =
            [d - 1, d, d + 1].iter().flat_map(|k| self.points.iter().filter_map(|m| Self::at_decade(*m, *k))).collect();
        out.retain(|p| *p > 0);
        out.sort_unstable();
        out.dedup();
        out
    }
}

declare_handler! {
    /// A small firm's price review.
    pub ReviewSmall = "FRM.review_small" {
        substep: S5c,
        table: "small_firm",
        reads: [Industry, ExpectedSales, DeliveredAtReview, LastReview, UnitCost, Markup, Price, PriceAttention, WagePerHour],
        writes: [Markup, Price, DeliveredAtReview, LastReview],
        clause: "FRM.5",
        body: review,
    }
}

declare_handler! {
    /// A large firm's price review.
    pub ReviewLarge = "FRM.review_large" {
        substep: S5c,
        table: "firm",
        reads: [Industry, ExpectedSales, DeliveredAtReview, LastReview, UnitCost, Markup, Price, PriceAttention, WagePerHour],
        writes: [Markup, Price, DeliveredAtReview, LastReview],
        clause: "FRM.5",
        body: review,
    }
}

declare_handler! {
    /// A small firm's attention to its price, on its production schedule.
    pub AttendSmall = "FRM.attend_small" {
        substep: S5b,
        table: "small_firm",
        reads: [Industry, ExpectedSales, SalesWidth, DeliveredSeen, Method, Markup, Price, WagePerHour],
        writes: [PriceAttention, ExpectedSales, SalesWidth, DeliveredSeen],
        clause: "REP.38",
        body: attend,
    }
}

declare_handler! {
    /// A large firm's attention to its price, on its production schedule.
    pub AttendLarge = "FRM.attend_large" {
        substep: S5b,
        table: "firm",
        reads: [Industry, ExpectedSales, SalesWidth, DeliveredSeen, Method, Markup, Price, WagePerHour],
        writes: [PriceAttention, ExpectedSales, SalesWidth, DeliveredSeen],
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

/// A value's scale in its fact: the powers of ten its fixed-point holding carries.
fn scale_of(item: phx_core::ItemDecl) -> f64 {
    match item.kind {
        phx_core::ItemKind::Fact(f) => match f.value {
            phx_core::FactType::Fixed { exp } => libm::pow(crate::consts::DECADE, f64::from(exp)),
            _ => phx_num::violation!(clause = "NUM.3", "a scaled read of a fact that holds no fixed point"),
        },
        _ => phx_num::violation!(clause = "NUM.3", "a scaled read of an item that is no fact"),
    }
}

/// A review: the markup moved by the sales since the last review — the units of its product delivered since then —
/// against those expected over the same days, and the point nearest the desired price posted when the move gains
/// more, over the days to its next review, than its staff's hours to make it cost; the pressure its stock of its
/// product puts on the price is read from what it holds. The competitors' prices the firm sees arrive with the posted
/// markets households buy in; until then it sees none.
#[clause("FRM.5", "REP.34")]
fn review<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<Industry>
        + Reads<ExpectedSales>
        + Reads<DeliveredAtReview>
        + Reads<LastReview>
        + Reads<UnitCost>
        + Reads<Markup>
        + Reads<Price>
        + Reads<PriceAttention>
        + Reads<WagePerHour>
        + Writes<Markup>
        + Writes<Price>
        + Writes<DeliveredAtReview>
        + Writes<LastReview>,
    S: FactStore + ?Sized,
{
    let m: &Management = ctx.own::<crate::Own>().management();
    let Some(product) = product_of(ctx, row) else { return };
    let delivered = ctx.delivered(row, product);
    let (Some(expected), Some(at_review), Some(last), Some(cost), Some(markup), Some(price), Some(chance)) = (
        read::<ExpectedSales, H, S>(ctx, row),
        read::<DeliveredAtReview, H, S>(ctx, row),
        read::<LastReview, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<Markup, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<PriceAttention, H, S>(ctx, row),
    ) else {
        return;
    };
    let sold = from_i64(delivered) - at_review;
    let stock = from_i64(ctx.goods(row).iter().filter(|(p, _, _)| *p == product).map(|(_, _, q)| q).sum::<i64>());
    let Some(wage) = read::<WagePerHour, H, S>(ctx, row) else { return };
    let today = from_i64(i64::from(ctx.day().get()));
    let days = today - last;
    let chance = chance / scale_of(<PriceAttention as phx_core::FactDef>::ITEM);
    if days <= 0.0 || chance <= 0.0 {
        return;
    }
    let expected_since = expected * days / m.production_days;
    let markup_scale = scale_of(<Markup as phx_core::FactDef>::ITEM);
    let Missing::Present(next) = rules::markup::update(
        markup / markup_scale,
        (m.sales_speed, m.seen_speed),
        (sold, expected_since),
        Missing::Absent,
        price,
    ) else {
        return;
    };
    let target_stock = expected * m.cover_days / m.production_days;
    let Missing::Present(pressure) = rules::price::pressure_stocked(sold, expected_since, target_stock, stock) else {
        return;
    };
    let wanted = rules::price::desired(next, cost, pressure, m.curvature);
    if let Some(markup) = whole(next * markup_scale) {
        ctx.write::<Markup>(row, markup);
    }
    ctx.write::<DeliveredAtReview>(row, delivered);
    ctx.write::<LastReview>(row, i64::from(ctx.day().get()));
    let Some(current) = whole(price) else { return };
    // The price posted now stands, as the firm expects, until its next review.
    let revenue = expected / m.production_days * price / chance;
    let menu_cost = m.menu_hours * wage;
    if let Some(point) = rules::price::reprice(&m.points_near(wanted), current, wanted, revenue, next, menu_cost) {
        ctx.write::<Price>(row, point);
    }
}

/// The firm's product, the industry it is in.
fn product_of<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot) -> Option<u16>
where
    H: HandlerDecl + Reads<Industry>,
    S: FactStore + ?Sized,
{
    match ctx.read::<Industry>(row) {
        Missing::Present(p) => u16::try_from(p).ok(),
        Missing::Absent => None,
    }
}

/// The attention a firm gives its price, on its production schedule. First its sales outlook takes in what it sold
/// since its last schedule, the units of its product it delivered: moved by its memory type's gain, and the width of
/// its surprises with it, a surprise beyond the widths it is sensitive to waking a review for tomorrow.
/// Then its daily chance of a review from the loss a gap costs it, read from its revenue and markup, against
/// the variance of its sales outlook, a review costing its staff's hours at their wage. A firm with no method yet
/// forecasts nothing, and counts its sales from today.
#[clause("REP.38", "REP.35", "VAL.4", "VAL.23", "FRM.13")]
fn attend<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<Industry>
        + Reads<ExpectedSales>
        + Reads<SalesWidth>
        + Reads<DeliveredSeen>
        + Reads<Method>
        + Reads<Markup>
        + Reads<Price>
        + Reads<WagePerHour>
        + Writes<PriceAttention>
        + Writes<ExpectedSales>
        + Writes<SalesWidth>
        + Writes<DeliveredSeen>,
    S: FactStore + ?Sized,
{
    let m: &Management = ctx.own::<crate::Own>().management();
    let Some(product) = product_of(ctx, row) else { return };
    let delivered = ctx.delivered(row, product);
    let (Some(mut expected), Some(mut width), Some(markup), Some(price), Some(wage)) = (
        read::<ExpectedSales, H, S>(ctx, row),
        read::<SalesWidth, H, S>(ctx, row),
        read::<Markup, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<WagePerHour, H, S>(ctx, row),
    ) else {
        ctx.write::<DeliveredSeen>(row, delivered);
        return;
    };
    let mut woke = false;
    let gain = read::<Method, H, S>(ctx, row).and_then(phx_rand::float::floor_to_u64).and_then(|method| {
        let types = phx_rand::float::len_u64(m.gains.len());
        if types == 0 { None } else { m.gains.get(usize::try_from(method % types).ok()?).copied() }
    });
    if let (Some(seen), Some(gain)) = (read::<DeliveredSeen, H, S>(ctx, row), gain) {
        let sold = from_i64(delivered) - seen;
        let surprise = phx_val::surprise::surprise(sold, expected);
        woke = phx_val::surprise::wakes(surprise, width, m.sensitivity);
        width = phx_val::surprise::width(Missing::Present(width), surprise, gain);
        expected = phx_val::heuristics::adaptive(expected, sold, gain);
        if let (Some(e), Some(w)) = (whole(expected), whole(width)) {
            ctx.write::<ExpectedSales>(row, e);
            ctx.write::<SalesWidth>(row, w);
        }
    }
    ctx.write::<DeliveredSeen>(row, delivered);
    let cost = m.review_hours * wage;
    // A review costs its staff's hours; a firm whose hour costs nothing holds no staff to review with.
    if cost <= 0.0 {
        return;
    }
    let revenue_per_day = expected * price / m.production_days;
    let markup = markup / scale_of(<Markup as phx_core::FactDef>::ITEM);
    let chance = if woke {
        1.0
    } else if expected > 0.0 {
        let relative = width / expected;
        // The public series a firm reads arrive with the markets' prints.
        rules::attention::review_chance(revenue_per_day, markup, (relative * relative / m.production_days, &[]), cost)
    } else {
        // A firm that expects to sell nothing loses nothing by a price left standing.
        0.0
    };
    if let Some(chance) = whole(chance * scale_of(<PriceAttention as phx_core::FactDef>::ITEM)) {
        ctx.write::<PriceAttention>(row, chance);
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
            gains: vec![0.5],
            sensitivity: 2.0,
        };
        assert_eq!(m.points_near(250.0), vec![10, 100, 199, 499, 999, 1000, 1990, 4990, 9990]);
        assert_eq!(m.points_near(2500.0), vec![100, 199, 499, 999, 1000, 1990, 4990, 9990, 10000, 19900, 49900, 99900]);
        assert!(m.points_near(0.0).is_empty());
        assert!(m.is_point(1990) && m.is_point(4990) && !m.is_point(2000));
        assert!(!m.points_near(25.0).contains(&20), "199 has no whole point a decade down");
    }
}
