//! A firm's production on its schedule: how much of its product to make over the period, within what its staff can
//! make and its stocks of inputs allow; what it offers other firms of what it then holds; and what it buys of the
//! inputs it will need, goods at the goods markets and services at retail, where a service is made as it is sold.

use if_firm::facts::{Capacity, ExpectedSales, OutputRate, Price, RequiredReturn, UnitCost};
use if_firm::known::{Product, WayUsed};
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads};
use phx_core::{Emits, Register};
use phx_id::Slot;
use phx_ledger::instruction::{Source, name_code};
use phx_ledger::intents::{Made, Transform};
use phx_macros::clause;
use phx_market::intents::{OrderIntent, ShopIntent};
use phx_market::order::{Side, Step};
use phx_market::retail::Want;
use phx_num::{Missing, PriceRaw};
use phx_rand::float::{floor_to_i64, from_i64, from_u64};

use crate::consts::{DAYS_A_YEAR, FIXED_SCALE, PER_UNIT_SCALE, PPM};
use crate::rules::inputs;
use crate::rules::produce::{Produce, ProduceIn, target};

/// A product as production and its trade read it: whether it can be held, the units its price is posted for, and
/// the market it is traded in between firms.
#[derive(Clone, Debug, PartialEq)]
pub struct Traded {
    pub storable: bool,
    pub lot: i64,
    pub market: u64,
}

/// A way as its user reads it: the product it makes, each input's units per unit started, its yield in millionths,
/// and the days a unit takes.
#[derive(Clone, Debug, PartialEq)]
pub struct WayRow {
    pub product: u16,
    pub inputs: Vec<(u16, i64)>,
    pub yield_ppm: i64,
    pub lead: f64,
}

/// What production reads, compiled once: each product's trade, and each way by its identity, each country's ways in
/// the products' order after the country before's, as the technology registers them.
#[derive(Clone, Debug, PartialEq)]
pub struct Plant {
    pub products: Vec<Traded>,
    pub ways: Vec<WayRow>,
    pub adjustment_days: f64,
}

impl Plant {
    /// # Errors
    /// A product in an undeclared unit, or ways' tables unread.
    pub fn compile(register: &Register, countries: usize, adjustment: u64) -> Result<Plant, String> {
        let entries = register.products("TEC.products")?;
        let standardised = register.table1("GDS.standardised")?;
        let mut products = Vec::with_capacity(entries.len());
        for (i, e) in (0_i64..).zip(entries) {
            let Missing::Present(unit) = register.units().named(&e.unit) else {
                return Err(format!("product `{}` in an undeclared unit", e.name));
            };
            let exp = register.units().decl(unit).map_or(0, |d| d.price_exp);
            let lot = (0..exp).try_fold(1_i64, |l, _| l.checked_mul(i64::from(crate::consts::TEN)));
            let Some(lot) = lot else { return Err(format!("`{}`'s price places beyond a quantity", e.unit)) };
            let kind = if standardised.at(i).is_ok_and(|v| v == 1) { "GDS.commodities" } else { "GDS.between_firms" };
            products.push(Traded { storable: e.storable, lot, market: name_code(kind) });
        }
        let (lead, yields) = (register.table1("TEC.lead_time")?, register.table1("TEC.yield")?);
        let mut ways = Vec::new();
        for c in 0..countries {
            let id = phx_id::CountryId::new(u8::try_from(c).map_err(|e| e.to_string())?);
            let inputs = register.table2_in("TEC.inputs", id)?;
            for j in 0..entries.len() {
                let col = i64::try_from(j).map_err(|e| e.to_string())?;
                let used = inputs
                    .rows()
                    .iter()
                    .filter_map(|r| {
                        let v = inputs.at(*r, col).ok()?;
                        (v != 0).then(|| u16::try_from(*r).ok().map(|q| (q, v)))?
                    })
                    .collect();
                ways.push(WayRow {
                    product: u16::try_from(j).map_err(|e| e.to_string())?,
                    inputs: used,
                    yield_ppm: yields.at(col).map_err(|_| format!("no yield of product {j}"))?,
                    lead: lead.at(col).map_or(0.0, from_i64),
                });
            }
        }
        Ok(Plant { products, ways, adjustment_days: from_u64(adjustment) })
    }
}

/// What starting enough to finish `finished` units takes of an input stated per unit: its units rounded up.
#[must_use]
pub fn takes(way: &WayRow, finished: i64, per: i64) -> i64 {
    if way.yield_ppm <= 0 {
        return 0;
    }
    let up = |a: i128, b: i128| phx_num::div_round(a, b, phx_num::Round::Ceil);
    let started = up(i128::from(finished) * i128::from(PPM), i128::from(way.yield_ppm));
    let took = up(started * i128::from(per), i128::from(PER_UNIT_SCALE));
    i64::try_from(took).unwrap_or(i64::MAX)
}

/// The most a way can finish from the units of each input held: the least over its storable inputs, each the most
/// finished whose take the holding covers, found by halving since a take rounds up.
#[must_use]
pub fn most_from(way: &WayRow, held: &dyn Fn(u16) -> i64, storable: &dyn Fn(u16) -> bool) -> Option<i64> {
    let mut most: Option<i64> = None;
    for (q, per) in way.inputs.iter().filter(|(q, _)| storable(*q)) {
        if *per <= 0 {
            continue;
        }
        let have = held(*q);
        let bound = i128::from(have) * i128::from(PER_UNIT_SCALE) / i128::from(*per) + 1;
        let (mut low, mut high) = (0_i64, i64::try_from(bound).unwrap_or(i64::MAX));
        while low < high {
            let mid = low + (high - low) / 2 + (high - low) % 2;
            if takes(way, mid, *per) <= have { low = mid } else { high = mid - 1 }
        }
        most = Some(most.map_or(low, |m| if low < m { low } else { m }));
    }
    most
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

/// A production period: the output the production rule sets from the sales the firm expects, the stock it aims to
/// cover them with and what it holds, its staff's or its plant's output over the period, whichever is less, and its
/// price against its cost carried
/// over the days a unit takes at the return it requires; made, when its stocks of inputs allow, by its way, whose
/// inputs the kernel takes. Then its stock of its product offered to other firms at its posted price, less what that
/// price leaves it after financing its stock; and for the next period, each storable input it will be short of
/// bought at the goods market when its use is worth its price there, and the services its way uses bought at retail.
/// A firm without its product, way, rate, outlook, cost or price decides nothing.
#[clause("FRM.4", "FRM.7", "GDS.5", "GDS.6", "TEC.9", "CAP.9")]
pub(crate) fn produce<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<Product>
        + Reads<WayUsed>
        + Reads<OutputRate>
        + Reads<ExpectedSales>
        + Reads<UnitCost>
        + Reads<Price>
        + Reads<RequiredReturn>
        + Reads<Capacity>
        + Emits<Transform>
        + Emits<OrderIntent>
        + Emits<ShopIntent>,
    S: FactStore + ?Sized,
{
    let own: &crate::Own = ctx.own::<crate::Own>();
    let (m, plant) = (own.management(), own.plant());
    let (Some(p), Some(w), Some(rate), Some(expected), Some(cost), Some(price), Some(required)) = (
        read::<Product, H, S>(ctx, row).and_then(|p| u16::try_from(p).ok()),
        read::<WayUsed, H, S>(ctx, row).and_then(|w| usize::try_from(w).ok()),
        read::<OutputRate, H, S>(ctx, row),
        read::<ExpectedSales, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
    ) else {
        return;
    };
    let (Some(way), Some(traded)) = (plant.ways.get(w), plant.products.get(usize::from(p))) else { return };
    let days = m.production_days;
    let financing = from_i64(required) / FIXED_SCALE * days / DAYS_A_YEAR;
    let storable = |q: u16| plant.products.get(usize::from(q)).is_some_and(|t| t.storable);
    let mut planned = from_i64(expected);
    if traded.storable {
        let stock = ctx.held(row, p, 0);
        let rule = ProduceIn {
            expected_demand: from_i64(expected),
            stock: from_i64(stock),
            cover: m.cover_days / days,
            adjustment: plant.adjustment_days / days,
            // The staff's output, and the plant's where its last review found one, whichever is less.
            capacity: match read::<Capacity, H, S>(ctx, row) {
                Some(plant) if plant < rate => from_i64(plant) * days,
                _ => from_i64(rate) * days,
            },
            expected_price: from_i64(price),
            unit_cost: from_i64(cost),
            financing_rate: financing,
            lead: way.lead / days,
        };
        planned = match target(&rule) {
            Produce::Make(q) => q,
            Produce::StockCovers | Produce::NoMargin | Produce::NoCapacity => 0.0,
        };
        let goods: Vec<(u16, i64)> =
            ctx.goods(row).iter().filter(|(_, g, _)| *g == 0).map(|(q, _, n)| (*q, *n)).collect();
        let held = |q: u16| goods.iter().filter(|(x, _)| *x == q).map(|(_, n)| n).sum::<i64>();
        let feasible = most_from(way, &held, &storable);
        let make = floor_to_i64(planned).map_or(0, |q| feasible.map_or(q, |f| if f < q { f } else { q }));
        if make > 0 {
            let leg = Made {
                product: p,
                grade: 0,
                qty: make,
                source: Source::Way(u32::try_from(w).unwrap_or(u32::MAX)),
                cost: 0,
            };
            ctx.emit(&Transform { row, reason: name_code(crate::MADE.name), legs: vec![leg] });
        }
        let offered = stock + make;
        let lots = offered - offered % traded.lot;
        if lots > 0 {
            let steps = vec![Step { limit: PriceRaw::from_raw(price), qty: lots }];
            ctx.emit(&OrderIntent { row, kind: traded.market, product: p, grade: 0, side: Side::Sell, steps });
        }
    }
    buy_inputs(ctx, row, (way, plant), (planned, from_i64(price) / from_i64(traded.lot), financing));
}

/// The inputs a period's output will use: each storable one the firm will be short of over the days a unit takes
/// and its stock's cover, bought at its market when an input's share of a unit's price is worth its price there
/// financed; each service bought at retail as it will be used, whole lots of it.
fn buy_inputs<H, S>(
    ctx: &mut Ctx<'_, H, S>,
    row: Slot,
    (way, plant): (&WayRow, &Plant),
    (planned, unit_price, financing): (f64, f64, f64),
) where
    H: HandlerDecl + Emits<OrderIntent> + Emits<ShopIntent>,
    S: FactStore + ?Sized,
{
    let m = ctx.own::<crate::Own>().management();
    let (cover, lead) = (m.cover_days / m.production_days, way.lead / m.production_days);
    let total: i64 = way.inputs.iter().map(|(_, per)| per).sum();
    if total <= 0 || planned <= 0.0 {
        return;
    }
    let Some(finished) = floor_to_i64(planned) else { return };
    // What the way uses of the firm's own product comes from its own stock, never from its own offer.
    for (q, per) in way.inputs.iter().filter(|(q, _)| *q != way.product) {
        let Some(t) = plant.products.get(usize::from(*q)) else { continue };
        let use_per_period = from_i64(takes(way, finished, *per));
        if t.storable {
            let Missing::Present(mark) = ctx.mark(row, *q, 0) else { continue };
            // An input's worth in use: its share, by what the way takes of it, of what a unit made fetches.
            let worth = unit_price * from_i64(PER_UNIT_SCALE) / from_i64(total);
            let carried = inputs::carried_cost(from_i64(mark) / from_i64(t.lot), financing, lead + cover);
            let short = inputs::order(use_per_period, (lead, cover), from_i64(ctx.held(row, *q, 0)), worth, carried);
            let Some(units) = floor_to_i64(short) else { continue };
            let lots = units - units % t.lot + if units % t.lot > 0 { t.lot } else { 0 };
            let Some(limit) = floor_to_i64(worth * from_i64(t.lot)) else { continue };
            if lots > 0 && limit > 0 {
                let steps = vec![Step { limit: PriceRaw::from_raw(limit), qty: lots }];
                ctx.emit(&OrderIntent { row, kind: t.market, product: *q, grade: 0, side: Side::Buy, steps });
            }
        } else {
            let Some(units) = floor_to_i64(use_per_period) else { continue };
            let lots = units - units % t.lot + if units % t.lot > 0 { t.lot } else { 0 };
            if lots > 0 {
                ctx.emit(&ShopIntent { row, kind: name_code(RETAIL), product: *q, want: Want::Units(lots) });
            }
        }
    }
}

/// The retail kind a firm buys the services its way uses in.
const RETAIL: &str = "SRV.retail";
