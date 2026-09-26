//! The owner's review of its plant, on its own schedule: the wear since the last is realised on the rows visited; the
//! units a day its plant then lets its way make are found, the scarcest kind's; and where that plant keeps the firm
//! from making what its staff could and its sales call for, whether to buy more of the scarcest kind. Maintaining,
//! repairing, selling and scrapping are decided here once owners hold the prices they read.

use if_firm::facts::{Capacity, ExpectedSales, OutputRate, Price, RequiredReturn, SalesWidth, UnitCost};
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
        substep: S4a,
        table: "small_firm",
        reads: [WayUsed, Product, ExpectedSales, SalesWidth, OutputRate, Price, UnitCost, RequiredReturn],
        writes: [Capacity],
        intents: [InvestIntent],
        clause: "CAP.4",
        body: review,
    }
}

declare_handler! {
    /// A large firm's review of its plant.
    pub ReviewLarge = "CAP.review_large" {
        substep: S4a,
        table: "firm",
        reads: [WayUsed, Product, ExpectedSales, SalesWidth, OutputRate, Price, UnitCost, RequiredReturn],
        writes: [Capacity],
        intents: [InvestIntent],
        clause: "CAP.4",
        body: review,
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

/// A review: the plant's wear realised, and the capacity its way's needs of each kind find in the efficient units of
/// the plant held, a year's output over the days of a year. A way that needs no plant is not limited by it, and the
/// firm keeps no capacity. Where the plant makes less than the firm's staff could and its sales call for, it weighs
/// buying the plant of the scarcest kind that closes the gap: the margin the extra output earns a year over the
/// kind's life, at the return it requires, against what the plant costs where it stands, by the value of waiting its
/// uncertainty about its sales gives; and buys it when that pays and its money covers it.
#[clause("CAP.9", "CAP.1", "CAP.3", "CAP.4", "CAP.5")]
fn review<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<WayUsed>
        + Reads<Product>
        + Reads<ExpectedSales>
        + Reads<SalesWidth>
        + Reads<OutputRate>
        + Reads<Price>
        + Reads<UnitCost>
        + Reads<RequiredReturn>
        + Writes<Capacity>
        + Emits<InvestIntent>,
    S: FactStore + ?Sized,
{
    let Missing::Present(way) = ctx.read::<WayUsed>(row) else { return };
    let own: &crate::CapOwn = ctx.own::<crate::CapOwn>();
    let Some(needs) = usize::try_from(way).ok().and_then(|w| own.needs.get(w)) else { return };
    let classes = own.kinds.classes;
    let held: Vec<f64> = own
        .kinds
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
        .collect();
    let Missing::Present(a_year) = capacity(&held, needs) else { return };
    let a_day = a_year / DAYS_A_YEAR;
    if let Some(units) = floor_to_i64(a_day) {
        ctx.write::<Capacity>(row, units);
    }
    let (Some(product), Some(expected), Some(width), Some(staff), Some(price), Some(cost), Some(required)) = (
        read::<Product, H, S>(ctx, row).and_then(|p| u16::try_from(floor_to_i64(p)?).ok()),
        read::<ExpectedSales, H, S>(ctx, row),
        read::<SalesWidth, H, S>(ctx, row),
        read::<OutputRate, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
    ) else {
        return;
    };
    let wanted = {
        let sales = expected / own.production_days;
        if sales < staff { sales } else { staff }
    };
    let gap = wanted - a_day;
    let Some(lot) = own.lots.get(usize::from(product)).copied() else { return };
    let margin = (price - cost) / lot;
    if gap <= 0.0 || margin <= 0.0 || expected <= 0.0 {
        return;
    }
    // The scarcest kind is the one whose plant allows the least output.
    let scarcest =
        needs.iter().filter(|(_, per)| *per > 0.0).fold(None, |best: Option<(usize, f64, f64)>, (k, per)| {
            let allows = held.get(*k).copied().unwrap_or(0.0) / per;
            match best {
                Some((_, _, b)) if b <= allows => best,
                _ => Some((*k, *per, allows)),
            }
        });
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
    let waiting = waiting_multiple(rate, earned / value, width / expected);
    if invests(value, outlay, 0.0, waiting, money >= outlay)
        && let Some(units) = floor_to_i64(units).filter(|u| *u > 0)
        && let Ok(kind) = u8::try_from(kind)
    {
        ctx.emit(&InvestIntent { row, kind, product: bought_as, units, stages });
    }
}
