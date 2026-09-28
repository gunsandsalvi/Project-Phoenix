//! The bench's goods. Every firm selling a good holds its product and a few inputs, in stocks sharded by the firms'
//! netting ranges, so each range's worker writes only its own. A sale covers the seller's units; after settlement they
//! are delivered — used up by a household's purchase, held by a firm at what it paid — or released where the payment
//! failed. Firms use up inputs and make their product, goods spoil weekly, a few firms ship to the next region, and the
//! goods' identity is read each day.

use phx_core::flows::{Denom, Flow};
use phx_core::goods::{
    Bound, Carriage, Cost, Good, Held, NATURE, Shipment, Shipments, Stocks, UnitIds, breaks, nature_net, spoil,
};
use phx_core::units::{Chain, Class, capacity, issue_chain, wear};
use phx_exec::Pool;
use phx_id::{Day, PartyKey, Slot};

use super::{GOODS_SHARE, THOUSAND, index_u64, to_u32, to_usize};

/// Days of making a goods firm holds of its product at the opening.
const OPENING_DAYS: i64 = 10;
/// Goods a goods firm holds as inputs at the opening, each of as many units as a day's making.
const INPUTS: u32 = 2;
/// Cost of a unit made, beyond the inputs used for it, in smallest units.
const MAKING_COST: i64 = 150;
/// Units of each input a goods firm uses up a day.
const USE: i64 = 1;
/// Days between spoilage visits, and a year's days.
const SPOIL_DAYS: u32 = 7;
const YEAR_DAYS: i64 = 365;
/// Every third product spoils, at this share a year.
const SPOILS_EVERY: u32 = 3;
const SPOIL_RATE: f64 = 0.2;
/// One goods firm in `SHIP_EVERY` ships a quarter of its free product to the next region each day, arriving after
/// `SHIP_DAYS`.
const SHIP_EVERY: u32 = 100;
const SHIP_PART: i64 = 4;
const SHIP_DAYS: u32 = 3;
/// Every firm's plant: its kind's conditions, the units it holds of each at the opening and what a unit costs new;
/// the chain's yearly rate of leaving a condition, a life of twenty years over four; each condition's efficiency and
/// value; the days between a firm's plant reviews, when it wears; and what a new unit makes a day.
const PLANT_CONDITIONS: u8 = 4;
const PLANT_UNITS: i64 = 10;
const PLANT_PRICE: i64 = 50_000;
const PLANT_LEAVING: f64 = 0.2;
const PLANT_EFFICIENCY: [f64; 4] = [1.0, 0.95, 0.85, 0.7];
const PLANT_VALUE: [f64; 4] = [1.0, 0.7, 0.45, 0.2];
const WEAR_DAYS: u32 = 30;
const MADE_A_UNIT: f64 = 1.0;
/// Days ahead the shipments' wheel reaches.
const SHIP_HORIZON: u32 = 16;
/// The goods flows' reasons: made, used, spoiled, shipped, arrived; a sale's goods leg is the purchases' own.
const MADE: u8 = 7;
const USED: u8 = 8;
const SPOILED: u8 = 9;
const WORN: u8 = 12;
const CARRIAGE: Carriage = Carriage { shipped: 10, arrived: 11 };

/// A sale's goods leg, covered at the sale and delivered after its payment settles, with what the buyer paid.
#[derive(Clone, Copy, Debug)]
pub(super) struct Sold {
    pub leg: Flow,
    pub paid: i64,
}

/// A range of the firms' stocks and shipments; the day's sales by its sellers, those covered so far, the firms'
/// purchases it holds for other ranges' buyers and those other ranges hold for its own; and the day's
/// transformations and arrivals. Each is kept across days, so a day allocates nothing once the heaviest has sized it.
#[derive(Default)]
struct Shard {
    stocks: Stocks,
    ships: Option<Shipments>,
    sold: Vec<Sold>,
    covered: usize,
    outgoing: Vec<Sold>,
    receipts: Vec<Sold>,
    flows: Vec<Flow>,
    inputs: Vec<(u16, i64)>,
    /// Covered sales that could not be delivered, which no bench day should have.
    short: u64,
}

pub(super) struct GoodsLoad {
    firm: u8,
    firms: u32,
    products: u32,
    regions: u32,
    range_bits: u32,
    unit_of: Vec<u16>,
    ids: UnitIds,
    chain: Chain,
    shards: Vec<Shard>,
    make: i64,
    open: Vec<i128>,
    /// The purchases whose payment failed today, as (buyer, seller) words, sorted.
    pub failed: Vec<(u32, u32)>,
}

impl GoodsLoad {
    /// The firms' goods at the opening, `make` units a day of making.
    pub fn new(
        pool: &Pool,
        (firm, firms): (u8, u32),
        (products, regions, range_bits): (u32, u32, u32),
        make: i64,
    ) -> GoodsLoad {
        let mut ids = UnitIds::default();
        let mut unit_of = Vec::new();
        for zone in 0..regions {
            for product in 0..products {
                unit_of.push(ids.unit(Held::Good(Good { product: to_u16(product), grade: 0, zone })));
            }
        }
        // Each product's plant is a kind, its chain issued at every region.
        let plant: Vec<Vec<u16>> = (0..regions)
            .flat_map(|zone| (0..products).map(move |p| Class { kind: to_u16(p), band: 0, condition: 0, zone }))
            .map(|newest| issue_chain(&mut ids, newest, PLANT_CONDITIONS))
            .collect();
        let chain = Chain {
            leaving_per_year: PLANT_LEAVING,
            efficiency: PLANT_EFFICIENCY.to_vec(),
            value: PLANT_VALUE.to_vec(),
        };
        let n_shards = to_usize(u64::from(firms.div_ceil(1 << range_bits)));
        let mut g = GoodsLoad {
            firm,
            firms,
            products,
            regions,
            range_bits,
            unit_of,
            ids,
            chain,
            shards: Vec::new(),
            make,
            open: Vec::new(),
            failed: Vec::new(),
        };
        let this = &g;
        let plant = &plant;
        g.shards = pool.map(n_shards, |r| {
            let mut stocks = Stocks::default();
            for s in this.slots(r) {
                let groups = this.regions * this.products;
                let chain = plant.get(to_usize(u64::from(s % groups))).map_or(&[][..], Vec::as_slice);
                for (unit, value) in chain.iter().zip(PLANT_VALUE) {
                    let cost =
                        phx_rand::float::floor_to_i64(value * phx_rand::float::from_i64(PLANT_UNITS * PLANT_PRICE));
                    stocks.receive(this.key(s), *unit, (PLANT_UNITS, cost.unwrap_or(0)), Day::new(0));
                }
                let Some((region, product)) = this.goods_seller(s) else { continue };
                let key = this.key(s);
                let units = make * OPENING_DAYS;
                stocks.receive(key, this.unit(region, product), (units, units * MAKING_COST), Day::new(0));
                for k in 1..=INPUTS {
                    let input = this.unit(region, this.goods_product(product + k));
                    stocks.receive(key, input, (make, make * MAKING_COST), Day::new(0));
                }
            }
            Shard { stocks, ships: Some(Shipments::new(Day::new(0), SHIP_HORIZON)), ..Shard::default() }
        });
        g.open = g.totals();
        g
    }

    fn key(&self, s: u32) -> PartyKey {
        PartyKey::new(self.firm, Slot::new(s))
    }

    fn slots(&self, shard: usize) -> std::ops::Range<u32> {
        let first = u64::from(to_u32(index_u64(shard))) << self.range_bits;
        let end = first + (1 << self.range_bits);
        to_u32(first)..if end < u64::from(self.firms) { to_u32(end) } else { self.firms }
    }

    fn shard_of(&self, p: PartyKey) -> usize {
        to_usize(u64::from(p.slot().get() >> self.range_bits))
    }

    fn unit(&self, region: u32, product: u32) -> u16 {
        self.unit_of.get(to_usize(u64::from(region * self.products + product))).copied().unwrap_or(u16::MAX)
    }

    fn is_goods(&self, product: u32) -> bool {
        u64::from(product) * THOUSAND < u64::from(self.products) * GOODS_SHARE
    }

    /// The `k`th goods product, round the goods.
    fn goods_product(&self, k: u32) -> u32 {
        let goods = (u64::from(self.products) * GOODS_SHARE).div_ceil(THOUSAND);
        to_u32(u64::from(k) % goods)
    }

    /// A seller's region and product, as the markets lay the firms out, if its product is a good.
    pub fn goods_seller(&self, s: u32) -> Option<(u32, u32)> {
        let groups = self.regions * self.products;
        let (region, product) = ((s % groups) / self.products, (s % groups) % self.products);
        self.is_goods(product).then_some((region, product))
    }

    /// The unit a seller's product is, where it is a good.
    pub fn seller_unit(&self, s: u32) -> Option<u16> {
        self.goods_seller(s).map(|(r, p)| self.unit(r, p))
    }

    /// A seller's free units of its product.
    pub fn free(&self, s: u32) -> Option<i64> {
        let unit = self.seller_unit(s)?;
        let key = self.key(s);
        self.shards.get(self.shard_of(key))?.stocks.holding(key, unit).map(phx_core::goods::Holding::free)
    }

    /// Takes in sales, each to its seller's range.
    pub fn add_sales(&mut self, sales: &[Sold]) {
        let bits = self.range_bits;
        for x in sales {
            if let Some(shard) = self.shards.get_mut(to_usize(u64::from(x.leg.payer.slot().get() >> bits))) {
                shard.sold.push(*x);
            }
        }
    }

    /// Covers the sales taken in since the last cover on their sellers' free units, range by range on the pool;
    /// returns those a seller's free units could not cover.
    pub fn cover(&mut self, pool: &Pool) -> u64 {
        let short: Vec<u64> = pool.map_items(self.shards.iter_mut().collect(), |shard| {
            let mut short = 0;
            for x in shard.sold.get(shard.covered..).unwrap_or(&[]) {
                let unit = x.leg.denomination.unit();
                if shard.stocks.bind(x.leg.payer, unit, x.leg.amount, (Bound::Free, Bound::Committed)).is_err() {
                    short += 1;
                }
            }
            shard.covered = shard.sold.len();
            short
        });
        short.iter().sum()
    }

    fn totals(&self) -> Vec<i128> {
        let mut out: Vec<i128> = Vec::new();
        for s in &self.shards {
            for (u, t) in s.stocks.totals().into_iter().enumerate() {
                if out.len() <= u {
                    out.resize(u + 1, 0);
                }
                if let Some(x) = out.get_mut(u) {
                    *x += t;
                }
            }
        }
        out
    }

    /// Each seller's range delivers what it covered, or releases it where the payment failed; a firm's purchase goes on
    /// to its buyer's range, to be held at what it paid.
    fn deliver(&mut self, pool: &Pool) {
        let failed = &self.failed;
        pool.map_items(self.shards.iter_mut().collect(), |shard| {
            let Shard { stocks, sold, outgoing, flows, short, .. } = shard;
            for x in sold.iter() {
                let (seller, unit, units) = (x.leg.payer, x.leg.denomination.unit(), x.leg.amount);
                let buyer = if x.leg.payee == NATURE { x.leg.source } else { x.leg.payee.word() };
                if failed.binary_search(&(buyer, seller.word())).is_ok() {
                    let _ = stocks.bind(seller, unit, units, (Bound::Committed, Bound::Free));
                    continue;
                }
                if stocks.deliver(seller, unit, units, Bound::Committed).is_err() {
                    *short += 1;
                    continue;
                }
                if x.leg.payee == NATURE {
                    flows.push(x.leg);
                } else {
                    outgoing.push(*x);
                }
            }
        });
        let bits = self.range_bits;
        for r in 0..self.shards.len() {
            let out = self.shards.get_mut(r).map(|s| std::mem::take(&mut s.outgoing)).unwrap_or_default();
            for x in &out {
                if let Some(to) = self.shards.get_mut(to_usize(u64::from(x.leg.payee.slot().get() >> bits))) {
                    to.receipts.push(*x);
                }
            }
            if let Some(s) = self.shards.get_mut(r) {
                s.outgoing = out;
                s.outgoing.clear();
            }
        }
    }

    /// The day's goods after settlement; returns the unit flows applied, or the goods' identity broken.
    pub fn day(&mut self, pool: &Pool, day: u32) -> Result<u64, String> {
        let today = Day::new(day);
        self.deliver(pool);
        let at = Making {
            today,
            make: self.make,
            firm: self.firm,
            products: self.products,
            regions: self.regions,
            units: &self.unit_of,
            ids: &self.ids,
            chain: &self.chain,
        };
        let this = &*self;
        let ranges: Vec<std::ops::Range<u32>> = (0..self.shards.len()).map(|r| this.slots(r)).collect();
        let jobs: Vec<_> = self.shards.iter_mut().zip(ranges).collect();
        pool.map_items(jobs, |(shard, slots)| {
            let Shard { stocks, ships, receipts, flows, inputs, .. } = shard;
            for x in receipts.iter() {
                stocks.receive(x.leg.payee, x.leg.denomination.unit(), (x.leg.amount, x.paid), today);
            }
            let Some(ships) = ships.as_mut() else { return };
            ships.arrive(today, stocks, CARRIAGE, flows);
            for s in slots {
                firm_day((stocks, ships, flows, inputs), s, at);
            }
            ships.close_day();
        });
        let close = self.totals();
        let mut applied = 0;
        let net = nature_net(self.shards.iter().flat_map(|s| {
            applied += index_u64(s.sold.len() + s.flows.len());
            s.flows.iter()
        }));
        if let Some((unit, expected, found)) = breaks(&self.open, &net, &close).first() {
            return Err(format!(
                "good {unit} holds {found} units on day {day} where {expected} were made, bought and kept"
            ));
        }
        if let Some(s) = self.shards.iter().find(|s| s.short > 0) {
            return Err(format!("{} sales covered on day {day} could not be delivered", s.short));
        }
        for s in &mut self.shards {
            s.flows.clear();
            s.sold.clear();
            s.receipts.clear();
            s.covered = 0;
        }
        self.failed.clear();
        self.open = close;
        Ok(applied)
    }
}

/// What a goods firm's day reads: the day, its making, the firms' kind, the products and regions and each good's
/// unit by region and product.
#[derive(Clone, Copy)]
struct Making<'a> {
    today: Day,
    make: i64,
    firm: u8,
    products: u32,
    regions: u32,
    units: &'a [u16],
    ids: &'a UnitIds,
    chain: &'a Chain,
}

/// A firm's day: on its plant review its plant wears. A goods firm then uses up some of each input it holds free and
/// makes its product, no more than its plant can, at their cost and the making's own; on a spoilage day its goods
/// spoil, cutting its pledges where goods on their way are lost; and one in `SHIP_EVERY` ships a quarter of its free
/// product to the next region. A firm selling a service holds no goods.
fn firm_day(
    (stocks, ships, flows, inputs): (&mut Stocks, &mut Shipments, &mut Vec<Flow>, &mut Vec<(u16, i64)>),
    s: u32,
    m: Making,
) {
    let groups = m.regions * m.products;
    let (region, product) = ((s % groups) / m.products, (s % groups) % m.products);
    let key = PartyKey::new(m.firm, Slot::new(s));
    let day = m.today.get();
    let kind = to_u16(product);
    if (s + day).is_multiple_of(WEAR_DAYS) {
        let own = |k: u16| (k == kind).then_some(m.chain);
        wear((stocks, m.ids), key, own, (i64::from(WEAR_DAYS), YEAR_DAYS), WORN, flows);
    }
    if u64::from(product) * THOUSAND >= u64::from(m.products) * GOODS_SHARE {
        return;
    }
    let unit = |r: u32| m.units.get(to_usize(u64::from(r * m.products + product))).copied();
    let (Some(own), Some(there)) = (unit(region), unit((region + 1) % m.regions)) else { return };
    // A firm makes no more than its plant here can.
    let can = capacity(stocks, m.ids, key, (kind, Some(region)), m.chain) * MADE_A_UNIT;
    let most = phx_rand::float::floor_to_i64(can).unwrap_or(0);
    let make = if most < m.make { most } else { m.make };
    let mut cost = make * MAKING_COST;
    inputs.clear();
    inputs.extend(
        stocks
            .holdings(key)
            .filter(|h| h.unit != own && h.free() > 0)
            .map(|h| (h.unit, if h.free() < USE { h.free() } else { USE })),
    );
    for (unit, used) in inputs.iter() {
        let f = transformation(key, NATURE, *unit, *used, USED);
        if let Ok(Some(c)) = stocks.apply(&f, Bound::Free, Cost::Carried, m.today) {
            cost += c;
            flows.push(f);
        }
    }
    let making = transformation(NATURE, key, own, make, MADE);
    if stocks.apply(&making, Bound::Free, Cost::At(cost), m.today).is_ok() {
        flows.push(making);
    }
    if day > 0 && day.is_multiple_of(SPOIL_DAYS) {
        let from = flows.len();
        let products = m.products;
        let spoils = |unit: u16| (u32::from(unit) % products).is_multiple_of(SPOILS_EVERY).then_some(SPOIL_RATE);
        spoil(stocks, key, spoils, (i64::from(SPOIL_DAYS), m.today, YEAR_DAYS), SPOILED, flows);
        for at in from..flows.len() {
            let Some(f) = flows.get(at).copied() else { continue };
            let (_, cut) = stocks.lose(key, f.denomination.unit(), f.amount);
            if cut > 0 {
                ships.shrink(key, f.denomination.unit(), cut);
            }
        }
    }
    if (s + day).is_multiple_of(SHIP_EVERY) {
        let part = stocks.holding(key, own).map_or(0, |h| h.free() / SHIP_PART);
        if part > 0 {
            let carrier = PartyKey::new(m.firm, Slot::new(s ^ 1));
            let trip = Shipment { owner: key, carrier, from: own, to: there, arrives: day + SHIP_DAYS, units: part };
            let _ = ships.depart(stocks, trip, (Bound::Free, m.today));
        }
    }
}

fn to_u16(n: u32) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// A transformation's flow of `units` of `unit`, its source the unit.
fn transformation(from: PartyKey, to: PartyKey, unit: u16, units: i64, reason: u8) -> Flow {
    Flow {
        payer: from,
        payee: to,
        amount: units,
        source: u32::from(unit),
        denomination: Denom::units(unit),
        reason,
        order: 0,
    }
}
