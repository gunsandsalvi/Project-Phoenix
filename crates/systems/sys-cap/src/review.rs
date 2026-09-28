//! The owner's reviews of its plant, on its own schedule. At the plant review the wear since the last is realised on
//! the rows visited, and the units a day the plant then lets its way make are found, the scarcest kind's, before the
//! day's production reads them. At the investment review, a sub-step earlier, the owner reads what it sold since
//! its last and, where its plant keeps it from making what its staff could and its sales call for, weighs buying more
//! of the scarcest kind.

use if_firm::facts::{Capacity, DeliveredAtInvest, OutputRate, Price, RequiredReturn, SoldAtInvest, UnitCost};
use if_firm::known::{Product, WayUsed};
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads, Writes};
use phx_core::{Emits, declare_handler};
use phx_id::Slot;
use phx_macros::clause;
use phx_market::intents::InvestIntent;
use phx_num::Missing;
use phx_rand::float::{floor_to_i64, from_i64};

use crate::consts::{DAYS_A_YEAR, FIXED_SCALE};
use crate::rules::capacity::{capacity, efficient_units};
use crate::rules::invest::{invests, waiting_multiple};

declare_handler! {
    /// A small firm's review of its plant.
    pub ReviewSmall = "CAP.review_small" {
        substep: S5c,
        table: "small_firm",
        reads: [WayUsed],
        writes: [Capacity],
        clause: "CAP.4",
        body: review,
    }
}

declare_handler! {
    /// A large firm's review of its plant.
    pub ReviewLarge = "CAP.review_large" {
        substep: S5c,
        table: "firm",
        reads: [WayUsed],
        writes: [Capacity],
        clause: "CAP.4",
        body: review,
    }
}

declare_handler! {
    /// A small firm's investment review.
    pub InvestSmall = "CAP.invest_small" {
        substep: S5b,
        table: "small_firm",
        reads: [WayUsed, Product, OutputRate, Price, UnitCost, RequiredReturn, DeliveredAtInvest, SoldAtInvest],
        writes: [DeliveredAtInvest, SoldAtInvest],
        intents: [InvestIntent],
        clause: "CAP.3",
        body: invest,
    }
}

declare_handler! {
    /// A large firm's investment review.
    pub InvestLarge = "CAP.invest_large" {
        substep: S5b,
        table: "firm",
        reads: [WayUsed, Product, OutputRate, Price, UnitCost, RequiredReturn, DeliveredAtInvest, SoldAtInvest],
        writes: [DeliveredAtInvest, SoldAtInvest],
        intents: [InvestIntent],
        clause: "CAP.3",
        body: invest,
    }
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

/// The plant's efficient units of each kind, by the kind's place.
fn held<H, S>(ctx: &Ctx<'_, H, S>, row: Slot) -> Vec<f64>
where
    H: HandlerDecl,
    S: FactStore + ?Sized,
{
    let own: &crate::CapOwn = ctx.own::<crate::CapOwn>();
    let classes = own.kinds.classes;
    own.kinds
        .kinds
        .iter()
        .enumerate()
        .map(|(k, kind)| {
            let mut units = vec![0_i64; classes];
            for (_, class, n) in ctx.plant(row).iter().filter(|(pk, _, _)| usize::from(*pk) == k) {
                if let Some(u) = units.get_mut(usize::from(*class)) {
                    *u += n;
                }
            }
            efficient_units(&units, &kind.efficiencies(classes))
        })
        .collect()
}

/// The plant review: the capacity its way's needs of each kind find in the efficient units of the plant held, a
/// year's output over the days of a year. A way that needs no plant is not limited by it, and the firm keeps no
/// capacity.
#[clause("CAP.9", "CAP.1")]
fn review<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl + Reads<WayUsed> + Writes<Capacity>,
    S: FactStore + ?Sized,
{
    let Missing::Present(way) = ctx.read::<WayUsed>(row) else { return };
    let held = held(ctx, row);
    let own: &crate::CapOwn = ctx.own::<crate::CapOwn>();
    let Some(needs) = usize::try_from(way).ok().and_then(|w| own.needs.get(w)) else { return };
    let Missing::Present(a_year) = capacity(&held, needs) else { return };
    if let Some(units) = floor_to_i64(a_year / DAYS_A_YEAR) {
        ctx.write::<Capacity>(row, units);
    }
}

/// The investment review: what the firm sold since its last, a day's worth of it, and how much that moved from the
/// period before, its uncertainty about its sales. Where the plant makes less a day than its staff could and its
/// sales call for, it weighs buying the plant of the scarcest kind that closes the gap: the margin the extra output
/// earns a year over the kind's life, at the return it requires, against what the plant costs where it stands, by
/// the value of waiting that uncertainty gives; and buys it when that pays and its money covers it. A firm with no
/// period before to compare decides nothing.
#[clause("CAP.3", "CAP.5")]
fn invest<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<WayUsed>
        + Reads<Product>
        + Reads<OutputRate>
        + Reads<Price>
        + Reads<UnitCost>
        + Reads<RequiredReturn>
        + Reads<DeliveredAtInvest>
        + Reads<SoldAtInvest>
        + Writes<DeliveredAtInvest>
        + Writes<SoldAtInvest>
        + Emits<InvestIntent>,
    S: FactStore + ?Sized,
{
    let Some(product) = read::<Product, H, S>(ctx, row).and_then(|p| u16::try_from(floor_to_i64(p)?).ok()) else {
        return;
    };
    let delivered = ctx.delivered(row, product);
    let (seen, before) = (read::<DeliveredAtInvest, H, S>(ctx, row), read::<SoldAtInvest, H, S>(ctx, row));
    ctx.write::<DeliveredAtInvest>(row, delivered);
    let Some(seen) = seen else { return };
    let sold = from_i64(delivered) - seen;
    if let Some(units) = floor_to_i64(sold) {
        ctx.write::<SoldAtInvest>(row, units);
    }
    let (Some(before), Missing::Present(way), Some(staff), Some(price), Some(cost), Some(required)) = (
        before,
        ctx.read::<WayUsed>(row),
        read::<OutputRate, H, S>(ctx, row).map(|r| r / phx_core::fact_scale(<OutputRate as phx_core::FactDef>::ITEM)),
        read::<Price, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
    ) else {
        return;
    };
    let held = held(ctx, row);
    let own: &crate::CapOwn = ctx.own::<crate::CapOwn>();
    let Some(needs) = usize::try_from(way).ok().and_then(|w| own.needs.get(w)) else { return };
    // The plant's units a day as the plant review finds them, read from what is held rather than from the review's
    // fact, which the plant review writes later in the day.
    let Missing::Present(a_year) = capacity(&held, needs) else { return };
    let plant = a_year / DAYS_A_YEAR;
    let a_day = sold / own.review_days;
    let wanted = if a_day < staff { a_day } else { staff };
    let gap = wanted - plant;
    let Some(lot) = own.lots.get(usize::from(product)).copied() else { return };
    let margin = (price - cost) / lot;
    if gap <= 0.0 || margin <= 0.0 || sold <= 0.0 || before <= 0.0 {
        return;
    }
    // The scarcest kind is the one whose plant allows the least output.
    let mut scarcest: Option<(usize, f64, f64)> = None;
    for (k, per) in needs.iter().filter(|(_, per)| *per > 0.0) {
        let Some(allows) = held.get(*k).map(|h| h / per) else { continue };
        if scarcest.is_none_or(|(_, _, b)| allows < b) {
            scarcest = Some((*k, *per, allows));
        }
    }
    let Some((kind, per, _)) = scarcest else { return };
    // A kind no source gives a lead time for cannot be ordered, since its building would take no known time.
    let (Some(bought_as), Some(life), Some(Missing::Present(stages))) =
        (own.bought_as.get(kind).copied(), own.kinds.kinds.get(kind).map(|k| k.life), own.lead.get(kind).copied())
    else {
        return;
    };
    let units = gap * DAYS_A_YEAR * per;
    let Missing::Present(mark) = ctx.mark(row, bought_as, 0) else { return };
    let Some(unit_price) = own.lots.get(usize::from(bought_as)).map(|l| from_i64(mark) / l) else { return };
    let rate = required / FIXED_SCALE;
    let earned = gap * DAYS_A_YEAR * margin;
    let annuity = if rate > 0.0 { (1.0 - libm::pow(1.0 + rate, -life)) / rate } else { life };
    let value = earned * annuity;
    let outlay = units * unit_price;
    let money = match ctx.money(row) {
        Missing::Present(m) => from_i64(m),
        Missing::Absent => return,
    };
    // The change in sales from one period to the next, as a year's volatility over the periods a year holds.
    let sigma = libm::fabs((sold - before) / before) * libm::sqrt(DAYS_A_YEAR / own.review_days);
    let waiting = waiting_multiple(rate, earned / value, sigma);
    if invests(value, outlay, 0.0, waiting, money >= outlay)
        && let Some(units) = floor_to_i64(units).filter(|u| *u > 0)
        && let Ok(kind) = u8::try_from(kind)
    {
        ctx.emit(&InvestIntent { row, kind, product: bought_as, units, stages });
    }
}
