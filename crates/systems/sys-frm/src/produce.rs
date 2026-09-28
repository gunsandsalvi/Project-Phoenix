//! A firm's production on its schedule: how much of its product to make over the period, within what its staff can
//! make and its stocks of inputs allow; what it offers other firms of what it then holds; and what it buys of the
//! inputs it will need, goods at the goods markets and services at retail, where a service is made as it is sold.

use if_firm::facts::{Capacity, ExpectedSales, HoursAUnit, OutputRate, Price, RequiredReturn, UnitCost};
use if_firm::known::{Product, WayUsed};
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads, Writes};
use phx_core::{Emits, Register};
use phx_id::Slot;
use phx_ledger::instruction::{Source, name_code};
use phx_ledger::intents::{Made, Transform};
use phx_macros::clause;
use phx_market::intents::{OrderIntent, ShipIntent, ShopIntent};
use phx_market::order::{Side, Step};
use phx_market::retail::Want;
use phx_num::{Missing, PriceRaw};
use phx_rand::float::{floor_to_i64, from_i64, from_u64};

use crate::consts::{DAYS_A_WEEK, DAYS_A_YEAR, FIXED_SCALE, PER_UNIT_SCALE, PPM};
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
/// cover them with and what it holds, its staff's output over the period — the hours a week its employment lines hold
/// at its hours a unit, written as its output rate — or its plant's, whichever is less, and its
/// price against its cost carried
/// over the days a unit takes at the return it requires; made, when its stocks of inputs allow, by its way, whose
/// inputs the kernel takes. Then its stock of its product offered to other firms at its posted price, less what that
/// price leaves it after financing its stock; and for the next period, each storable input it will be short of
/// bought at the goods market when its use is worth its price there, and the services its way uses bought at retail.
/// A firm without its product, way, hours a unit, outlook, cost or price decides nothing.
#[clause("FRM.4", "FRM.7", "GDS.5", "GDS.6", "TEC.9", "CAP.9")]
pub(crate) fn produce<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<Product>
        + Reads<WayUsed>
        + Reads<HoursAUnit>
        + Reads<ExpectedSales>
        + Reads<UnitCost>
        + Reads<Price>
        + Reads<RequiredReturn>
        + Reads<Capacity>
        + Writes<OutputRate>
        + Emits<Transform>
        + Emits<OrderIntent>
        + Emits<ShopIntent>
        + Emits<ShipIntent>,
    S: FactStore + ?Sized,
{
    let own: &crate::Own = ctx.own::<crate::Own>();
    let (m, plant) = (own.management(), own.plant());
    let (Some(p), Some(w), Some(hours_a_unit), Some(expected), Some(cost), Some(price), Some(required)) = (
        read::<Product, H, S>(ctx, row).and_then(|p| u16::try_from(p).ok()),
        read::<WayUsed, H, S>(ctx, row).and_then(|w| usize::try_from(w).ok()),
        read::<HoursAUnit, H, S>(ctx, row).filter(|h| *h > 0),
        read::<ExpectedSales, H, S>(ctx, row),
        read::<UnitCost, H, S>(ctx, row),
        read::<Price, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
    ) else {
        return;
    };
    let (Some(way), Some(traded)) = (plant.ways.get(w), plant.products.get(usize::from(p))) else { return };
    let Missing::Present(hours) = ctx.staff_hours(row) else { return };
    // What its staff make a day at its hours a unit, its standing flow as the other decisions read it.
    let staff = from_i64(hours) / DAYS_A_WEEK / (from_i64(hours_a_unit) / FIXED_SCALE);
    if let Some(rate) = floor_to_i64(f64::round(staff * phx_core::fact_scale(<OutputRate as phx_core::FactDef>::ITEM)))
    {
        ctx.write::<OutputRate>(row, rate);
    }
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
                Some(plant) if from_i64(plant) < staff => from_i64(plant) * days,
                _ => staff * days,
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
        // Every grade class of an input serves alike.
        let goods: Vec<(u16, i64)> = ctx.goods(row).iter().map(|(q, _, n)| (*q, *n)).collect();
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
            let Some(covers) = floor_to_i64(days).and_then(|d| u32::try_from(d).ok()) else { return };
            ctx.emit(&Transform { row, reason: name_code(crate::MADE.name), days: covers, legs: vec![leg] });
        }
        let offered = stock + make;
        let lots = offered - offered % traded.lot;
        if lots > 0 {
            let steps = vec![Step { limit: PriceRaw::from_raw(price), qty: lots }];
            ctx.emit(&OrderIntent {
                row,
                kind: traded.market,
                product: p,
                grade: 0,
                at: Missing::Absent,
                side: Side::Sell,
                steps,
            });
        }
    }
    let lot = from_i64(traded.lot);
    ship_home(ctx, row, (way, plant));
    buy_inputs(ctx, row, (way, plant), (planned, (from_i64(price) - from_i64(cost)) / lot, financing));
}

/// The inputs a period's output will use: each storable one the firm will be short of over the days a unit takes
/// and its stock's cover, counting what it holds elsewhere, bought where it lands for least — at its own place's
/// market, or at another place's with the freight home — when it is worth that there financed: its landed price, and
/// the margin a unit made earns over its cost, over what the unit takes of it; each service bought at retail as it
/// will be used, whole lots of it.
fn buy_inputs<H, S>(
    ctx: &mut Ctx<'_, H, S>,
    row: Slot,
    (way, plant): (&WayRow, &Plant),
    (planned, unit_margin, financing): (f64, f64, f64),
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
            let Some(((landed, at, freight), grade)) = cheapest(ctx, row, *q) else { continue };
            // An input's worth in use: what it costs landed, and what a unit made earns beyond its cost, over what the
            // unit takes of it.
            let price_per_unit = from_i64(landed) / from_i64(t.lot);
            let worth = price_per_unit + unit_margin * from_i64(PER_UNIT_SCALE) / from_i64(*per);
            let carried = inputs::carried_cost(price_per_unit, financing, lead + cover);
            let away: i64 = ctx.elsewhere(row).iter().filter(|h| h.product == *q).map(|h| h.units).sum();
            let here: i64 = ctx.goods(row).iter().filter(|(p, _, _)| p == q).map(|(_, _, n)| n).sum();
            let held = from_i64(here + away);
            let short = inputs::order(use_per_period, (lead, cover), held, worth, carried);
            let Some(units) = floor_to_i64(short) else { continue };
            let lots = units - units % t.lot + if units % t.lot > 0 { t.lot } else { 0 };
            // Where it buys at another place, the freight home is paid beside the price there.
            let Some(limit) = floor_to_i64(worth * from_i64(t.lot)).map(|l| l - freight) else { continue };
            if lots > 0 && limit > 0 {
                let steps = vec![Step { limit: PriceRaw::from_raw(limit), qty: lots }];
                ctx.emit(&OrderIntent { row, kind: t.market, product: *q, grade, at, side: Side::Buy, steps });
            }
        } else {
            let Some(units) = floor_to_i64(use_per_period.ceil()) else { continue };
            if units > 0 {
                ctx.emit(&ShopIntent { row, kind: name_code(RETAIL), product: *q, want: Want::Units(units) });
            }
        }
    }
}

/// Where a good lands at the row's place for least, a lot of it, over the grade classes it is marked in, which serve
/// alike: its own place's mark, or another place's mark with the freight home by the cheapest mode carriage is posted
/// in there, each where its market last met a seller; with the place, none for its own, and the freight; and the
/// class. None where no such place marks it.
fn cheapest<H, S>(ctx: &Ctx<'_, H, S>, row: Slot, product: u16) -> Option<(Landed, u8)>
where
    H: HandlerDecl,
    S: FactStore + ?Sized,
{
    ctx.grades(product)
        .into_iter()
        .filter_map(|grade| {
            // Its own place's market is no place to buy while it last met with no seller.
            let here = match ctx.mark(row, product, grade) {
                Missing::Present(m) if !ctx.unsold(row, product, grade) => Some((m, Missing::Absent, 0)),
                _ => None,
            };
            ctx.away(row, product, grade)
                .into_iter()
                .filter_map(|a| match a.inbound {
                    Missing::Present(f) => Some((a.there.checked_add(f)?, Missing::Present(a.zone), f)),
                    Missing::Absent => None,
                })
                .chain(here)
                // Equal landed prices go to its own place, then to the lower zone.
                .reduce(|a, b| if (b.0, rank(b.1)) < (a.0, rank(a.1)) { b } else { a })
                .map(|l| (l, grade))
        })
        // Equal landed prices go to the lower class.
        .reduce(|a, b| if b.0.0 < a.0.0 { b } else { a })
}

/// A lot's price landed at the row's place, the place it is bought at, none for its own, and the freight home.
type Landed = (i64, Missing<u32>, i64);

/// A place's rank among equal landed prices: its own first, then others by their zone.
fn rank(at: Missing<u32>) -> (bool, u32) {
    match at {
        Missing::Absent => (false, 0),
        Missing::Present(z) => (true, z),
    }
}

/// The inputs the firm holds at other places, free to leave, carried home in whole lots by the cheapest mode
/// carriage is posted in there, as the buyer who bought them at their origin ships them.
fn ship_home<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot, (way, plant): (&WayRow, &Plant))
where
    H: HandlerDecl + Emits<ShipIntent>,
    S: FactStore + ?Sized,
{
    let Missing::Present(home) = ctx.zone(row) else { return };
    let away: Vec<phx_core::HeldAway> = ctx.elsewhere(row).to_vec();
    for h in away.iter().filter(|h| way.inputs.iter().any(|(q, _)| *q == h.product)) {
        let Some(t) = plant.products.get(usize::from(h.product)) else { continue };
        let qty = h.free - h.free % t.lot;
        if qty <= 0 {
            continue;
        }
        let mode = ctx
            .away(row, h.product, h.grade)
            .into_iter()
            .filter(|a| a.zone == h.zone)
            .filter_map(|a| match a.inbound {
                Missing::Present(f) => Some((f, a.mode)),
                Missing::Absent => None,
            })
            .reduce(|a, b| if b < a { b } else { a });
        let Some((_, mode)) = mode else { continue };
        let kind = name_code(CARRIAGE);
        ctx.emit(&ShipIntent {
            row,
            kind,
            product: h.product,
            grade: h.grade,
            from: Missing::Present(h.zone),
            qty,
            to: home,
            mode,
        });
    }
}

/// The carriage kind a firm books room in to carry what it bought elsewhere home.
const CARRIAGE: &str = "FRT.carriage";

/// The retail kind a firm buys the services its way uses in.
const RETAIL: &str = "SRV.retail";
