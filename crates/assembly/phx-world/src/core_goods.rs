//! Goods on the core, a meeting a day. A good is a product at a region, held in the core's `Stocks`. Each firm holds
//! its output's days of cover and, of each stored input its way uses, what the days of making it takes and those
//! days cover use. Each day a firm makes what its planned output asks, within what its staff's hours make at its hours
//! a unit and what its stored inputs allow: the inputs are used up and the output made, the output carrying their
//! cost. Each firm then tops up its stored inputs from the firms of its region at their posted prices, and each
//! household due decides by the buffer-stock rule what it spends and asks each product's share of it at retail. Each
//! sale is a flow of money from buyer to seller and one of goods from the seller, held by a firm, used up by a
//! household. The goods' identity is read each day.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Bound, Cost, Good, Held, Holding, NATURE, UnitIds, breaks, nature_net};
use phx_core::wheel::DueWheel;
use phx_core::{OpeningCountry, Register, StreamDef, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_market::meet::{Buyer, GoodsLeg, Meeting, Place, Sale, Stall, Tastes, meet};
use phx_market::retail::{Want, Weights};
use phx_num::{Missing, violation};
use phx_rand::float::{floor_to_i64, from_i64, len_u64};
use phx_rand::{Subject, SubjectTag};

use crate::consts::firm::{
    EXPECTED, MARKUP, OUTPUT, PART_ONE, PRICE, PRODUCT, PRODUCTIVITY, PRODUCTIVITY_ONE, REGION, REVIEWED,
    SOLD as SOLD_UNITS,
};
use crate::consts::reason::{DELIVERED, MADE, SOLD, USED};
use crate::consts::{CORE_WHEEL_DAYS, DAYS_A_WEEK, DAYS_A_YEAR, MONTHS, MONTHS_A_YEAR};
use crate::core::{Core, kind_number};
use crate::opening::economy::table;

/// What a day's goods did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoodsDay {
    pub day: u32,
    pub made: i64,
    pub spenders: u64,
    pub wants: u64,
    pub sales: u64,
    pub spent: i64,
    pub inputs_wanted: u64,
    pub reviews: u64,
    pub repriced: u64,
    /// The goods whose units at the close are not their units at the open and what the day made less what it used.
    pub breaks: u64,
}

/// Goods' state on the core, kept from day to day.
#[derive(Debug, Default)]
pub struct CoreGoods {
    pub spenders: Option<DueWheel>,
    pub spend_days: u32,
    /// The day the core's making began, from which each firm's day's making is counted in whole units.
    pub began: u32,
    pub stocks: phx_core::goods::Stocks,
    pub units: UnitIds,
    /// Each country's ways' inputs of each product a unit of each, by country index; and which products are stored.
    pub inputs: Vec<Vec<Vec<f64>>>,
    pub stored: Vec<bool>,
    /// Each product's days a unit takes to make, and the days of sales a firm's stock covers.
    pub lead: Vec<f64>,
    pub cover: f64,
    /// Each country's GDP, and each product's collective consumption and fixed investment over it, by country.
    pub gdp: Vec<f64>,
    pub final_uses: Vec<Vec<[f64; 2]>>,
    /// Each region's share of its country's persons, and each firm's share of its country's turnover at the opening.
    pub region_share: Vec<f64>,
    pub turnover_share: BTreeMap<u32, f64>,
    /// Today's retail wants: each household's money asked of a product, at its region.
    pub wants: Vec<(u16, Buyer)>,
    pub meeting: Meeting,
    pub days: Vec<GoodsDay>,
}

/// What the goods day reads of the world besides the core.
pub struct GoodsCtx<'a> {
    pub register: &'a Register,
    pub calendar: &'a Calendar,
    pub streams: &'a Streams,
    pub rule: &'a sys_hh::Own,
    pub management: &'a sys_frm::decide::Management,
    pub regions: &'a [CountryId],
    pub weights: Weights,
}

impl std::fmt::Debug for GoodsCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoodsCtx").field("regions", &self.regions.len()).finish_non_exhaustive()
    }
}

/// A firm's record words the goods day reads.
#[derive(Clone, Copy, Debug)]
struct Firm {
    key: PartyKey,
    product: u16,
    region: u32,
    country: usize,
    productivity: f64,
    price: i64,
    output: f64,
}

/// A day `n` days after another.
fn after(day: Day, n: u64) -> Day {
    match u32::try_from(n).ok().and_then(|n| day.get().checked_add(n)) {
        Some(d) => Day::new(d),
        None => phx_num::capacity_exceeded!("day count", u32::MAX, n),
    }
}

/// A month counted from the calendar's year zero.
fn months(date: phx_id::Date) -> i64 {
    i64::from(date.year()) * MONTHS + i64::from(date.month())
}

use crate::consts::draws::{REGION_BITS, ROUND_BITS};

/// A subject's identity packing a party and a number within it, which must fit its bits.
fn packed(party: u64, within: u64, bits: u32) -> u64 {
    if within >= 1 << bits {
        phx_num::capacity_exceeded!("a number within a party's draws", 1_u64 << bits, within);
    }
    (party << bits) | within
}

/// Whole units of an amount, rounded half up.
fn whole_units(x: f64) -> i64 {
    floor_to_i64(x.round()).unwrap_or_else(|| violation!(clause = "REP.9", "units beyond a word"))
}

impl Core {
    fn record_word(&self, kind: usize, slot: Slot, at: usize) -> Option<i64> {
        match self.kinds.get(kind)?.record(slot).get(at).map(|w| w.get()) {
            Some(Missing::Present(v)) => Some(v),
            _ => None,
        }
    }

    fn set_record_word(&mut self, kind: usize, slot: Slot, at: usize, v: i64) {
        if let Some(w) = self.kinds.get_mut(kind).and_then(|k| k.record_mut(slot).get_mut(at)) {
            *w = phx_num::MaybeI64::present(v);
        }
    }

    fn goods_firm(&self, regions: &[CountryId], firm: usize, slot: Slot) -> Option<Firm> {
        let region = u32::try_from(self.record_word(firm, slot, REGION)?).ok()?;
        Some(Firm {
            key: PartyKey::new(kind_number(firm), slot),
            product: u16::try_from(self.record_word(firm, slot, PRODUCT)?).ok()?,
            region,
            country: usize::from(regions.get(usize::try_from(region).ok()?)?.get()),
            productivity: from_i64(self.record_word(firm, slot, PRODUCTIVITY)?) / PRODUCTIVITY_ONE,
            price: self.record_word(firm, slot, PRICE)?,
            output: from_i64(self.record_word(firm, slot, OUTPUT)?),
        })
    }

    fn unit_of(&mut self, product: u16, region: u32) -> u16 {
        self.goods.units.unit(Held::Good(Good { product, grade: 0, zone: region }))
    }

    fn free_units(&self, holder: PartyKey, product: u16, region: u32) -> i64 {
        let unit = self.goods.units.find(Held::Good(Good { product, grade: 0, zone: region }));
        unit.and_then(|u| self.goods.stocks.holding(holder, u)).map_or(0, Holding::free)
    }

    /// A firm's stored inputs a unit of its output, each with its product.
    fn stored_inputs(&self, f: &Firm) -> Vec<(u16, f64)> {
        let Some(ways) = self.goods.inputs.get(f.country) else { return Vec::new() };
        (0_u16..)
            .zip(ways)
            .filter(|(q, _)| self.goods.stored.get(usize::from(*q)).copied().unwrap_or(false))
            .filter_map(|(q, row)| row.get(usize::from(f.product)).copied().filter(|a| *a > 0.0).map(|a| (q, a)))
            .collect()
    }

    fn firm_slots(&self, firm: usize) -> Vec<Slot> {
        self.kinds.get(firm).map(|k| k.parties.live_slots().collect()).unwrap_or_default()
    }

    /// Goods opened on the core: each country's ways' inputs and each product's lead, every firm's opening stocks
    /// held at the opening's prices, and every household's spending schedule begun at a phase drawn for it.
    ///
    /// # Errors
    /// A primitive the opening reads that the register does not hold, or a spending schedule of no days.
    #[clause("HH.4", "FRM.4", "GDS.5", "GEN.3")]
    pub fn open_goods(
        &mut self,
        ctx: &GoodsCtx<'_>,
        (countries, cover): (&[OpeningCountry], f64),
        today: Day,
    ) -> Result<(), String> {
        let (Some(place), Some(firm)) =
            (self.names.iter().position(|n| *n == "household"), self.names.iter().position(|n| *n == "firm"))
        else {
            return Ok(());
        };
        let days = floor_to_i64((ctx.rule.period * DAYS_A_YEAR).round()).and_then(|d| u32::try_from(d).ok());
        let Some(days) = days.filter(|d| *d > 0) else { return Err("a spending schedule of no days".to_owned()) };
        let mut goods = CoreGoods { spend_days: days, began: today.get(), cover, ..CoreGoods::default() };
        let products = ctx.register.products("TEC.products")?;
        goods.stored = products.iter().map(|p| p.storable).collect();
        let lead = ctx.register.table1("TEC.lead_time")?;
        goods.lead = (0_i64..).take(products.len()).map(|p| lead.at(p).map_or(0.0, from_i64)).collect();
        let mut prices: Vec<Vec<f64>> = Vec::new();
        for c in countries {
            goods.inputs.push(table(ctx.register, "TEC.inputs", c.id)?.0);
            prices.push(crate::core_firms::snapshot(ctx.register, c)?.price);
            let uses = table(ctx.register, "GEN.final_uses", c.id)?.0;
            let at = |row: &Vec<f64>, k: usize| row.get(k).copied().unwrap_or(0.0);
            goods.final_uses.push(uses.iter().take(products.len()).map(|r| [at(r, 1), at(r, 2)]).collect());
            goods.gdp.push(c.gdp);
        }
        goods.region_share = self.region_shares(ctx.regions);
        self.goods = goods;
        self.goods.turnover_share = self.turnover_shares(ctx, firm);
        self.open_stocks(ctx.regions, firm, &prices, today);
        let mut wheel = DueWheel::new(today.succ(), CORE_WHEEL_DAYS);
        let Some(stream) = ctx.streams.named(sys_hh::VisitStream::DECL.name) else {
            return Err("the households' visit stream is not declared".to_owned());
        };
        let slots: Vec<Slot> = self.kinds.get(place).map(|k| k.parties.live_slots().collect()).unwrap_or_default();
        for slot in slots {
            let Some(id) = self.kinds.get(place).and_then(|k| k.parties.id(slot)) else { continue };
            let mut d = ctx.streams.open(&stream, Subject::new(SubjectTag::Party, id.get()), today, 0);
            let phase = phx_rand::below_u64(&mut d, u64::from(days));
            wheel.schedule(slot.get(), after(today.succ(), phase));
        }
        self.goods.spenders = Some(wheel);
        Ok(())
    }

    /// Each region's share of its country's persons, by region.
    fn region_shares(&self, regions: &[CountryId]) -> Vec<f64> {
        let mut persons = vec![0.0; regions.len()];
        let Some(place) = self.names.iter().position(|n| *n == "household") else { return persons };
        let Some(decl) = self.household_decl.as_ref() else { return persons };
        let Missing::Present(at) = decl.sited_by else { return persons };
        if let (Some(store), Some(Some(ps))) = (self.kinds.get(place), self.persons.get(place)) {
            for slot in store.parties.live_slots() {
                let Some(Missing::Present(r)) = store.record(slot).get(at).map(|w| w.get()) else { continue };
                if let Some(p) = usize::try_from(r).ok().and_then(|r| persons.get_mut(r)) {
                    *p += from_i64(i64::try_from(ps.count(slot)).unwrap_or(0));
                }
            }
        }
        let mut totals: BTreeMap<u8, f64> = BTreeMap::new();
        for (r, c) in regions.iter().enumerate() {
            *totals.entry(c.get()).or_insert(0.0) += persons.get(r).copied().unwrap_or(0.0);
        }
        (0..regions.len())
            .map(|r| {
                let whole = regions.get(r).and_then(|c| totals.get(&c.get())).copied().unwrap_or(0.0);
                if whole > 0.0 { persons.get(r).copied().unwrap_or(0.0) / whole } else { 0.0 }
            })
            .collect()
    }

    /// Each firm's share of its country's turnover at its posted price and output, by its slot.
    fn turnover_shares(&self, ctx: &GoodsCtx<'_>, firm: usize) -> BTreeMap<u32, f64> {
        let mut by: Vec<(u32, usize, f64)> = Vec::new();
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let lot = sys_frm::FilingPrims::lot(ctx.register, f.product);
            by.push((slot.get(), f.country, from_i64(f.price) / lot * f.output));
        }
        let mut totals: BTreeMap<usize, f64> = BTreeMap::new();
        for (_, c, t) in &by {
            *totals.entry(*c).or_insert(0.0) += t;
        }
        by.into_iter().map(|(s, c, t)| (s, totals.get(&c).filter(|w| **w > 0.0).map_or(0.0, |w| t / w))).collect()
    }

    /// Each country's treasury's collective consumption today: each product's share of GDP a day, asked at retail in
    /// each region by its share of the country's persons, until the public agencies buy in its place.
    #[clause("SOC.2", "GEN.2")]
    fn public_wants(&self, regions: &[CountryId]) -> Vec<(u16, Buyer)> {
        let mut wants = Vec::new();
        for (r, c) in regions.iter().enumerate() {
            let country = usize::from(c.get());
            let Some(Some(treasury)) = self.treasuries.get(country).copied() else { continue };
            let (Some(gdp), Some(uses), Some(share)) =
                (self.goods.gdp.get(country), self.goods.final_uses.get(country), self.goods.region_share.get(r))
            else {
                continue;
            };
            let Ok(region) = u32::try_from(r) else { continue };
            for (p, u) in (0_u16..).zip(uses) {
                let amount = floor_to_i64(u[0] * gdp / DAYS_A_YEAR * share).unwrap_or(0);
                if amount > 0 {
                    let subject = packed(u64::from(treasury.word()), u64::from(region), REGION_BITS);
                    wants.push((p, Buyer { party: treasury, subject, want: Want::Money(amount), place: region }));
                }
            }
        }
        wants
    }

    /// The firms whose production schedule came today buy their share, by their opening turnover, of their country's
    /// fixed investment in each capital good over their production period, and hold it, until their plant decides
    /// what they invest.
    #[clause("CAP.3", "GEN.2")]
    fn investment_wants(&self, ctx: &GoodsCtx<'_>) -> Vec<(u16, Buyer)> {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Vec::new() };
        let mut wants = Vec::new();
        for s in &self.labour.due_today {
            let Some(f) = self.goods_firm(ctx.regions, firm, Slot::new(*s)) else { continue };
            let (Some(gdp), Some(uses), Some(share)) =
                (self.goods.gdp.get(f.country), self.goods.final_uses.get(f.country), self.goods.turnover_share.get(s))
            else {
                continue;
            };
            for (p, u) in (0_u16..).zip(uses) {
                let amount =
                    floor_to_i64(u[1] * gdp * ctx.management.production_days / DAYS_A_YEAR * share).unwrap_or(0);
                if amount > 0 {
                    let buyer = Buyer {
                        party: f.key,
                        subject: u64::from(f.key.word()),
                        want: Want::Money(amount),
                        place: f.region,
                    };
                    wants.push((p, buyer));
                }
            }
        }
        wants
    }

    /// Every firm's opening stocks: of its output, its days of cover of its sales; of each stored input its way uses,
    /// what the days of making it takes and those days cover use; each held at the opening's price of its product.
    fn open_stocks(&mut self, regions: &[CountryId], firm: usize, prices: &[Vec<f64>], today: Day) {
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(regions, firm, slot) else { continue };
            let per_day = f.output / DAYS_A_YEAR;
            let price = |q: u16| prices.get(f.country).and_then(|p| p.get(usize::from(q))).copied().unwrap_or(0.0);
            let mut wanted: Vec<(u16, f64)> = Vec::new();
            if self.goods.stored.get(usize::from(f.product)).copied().unwrap_or(false) {
                wanted.push((f.product, self.goods.cover * per_day));
            }
            let lead = self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0);
            for (q, a) in self.stored_inputs(&f) {
                wanted.push((q, a * per_day * (lead + self.goods.cover)));
            }
            for (q, units) in wanted {
                let held = whole_units(units);
                if held <= 0 {
                    continue;
                }
                let unit = self.unit_of(q, f.region);
                let cost = whole_units(from_i64(held) * price(q));
                self.goods.stocks.receive(f.key, unit, (held, cost), today);
            }
        }
    }

    /// The day's goods: firms make, firms top up their inputs, households due decide, and each product's meeting
    /// sells at retail; the goods' identity read over the day.
    #[clause("GDS.4", "GDS.10", "HH.4", "SRV.4", "MKT.6")]
    pub fn goods_day(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> GoodsDay {
        let mut record = GoodsDay { day: day.get(), ..GoodsDay::default() };
        if self.goods.spenders.is_none() {
            return record;
        }
        let open = self.goods.stocks.totals();
        let mut moved: Vec<Flow> = Vec::new();
        record.made = self.make(ctx, day, &mut moved);
        record.inputs_wanted = self.buy_inputs(ctx, day, &mut moved);
        let (spenders, wants) = self.decide_spending(ctx, day);
        (record.spenders, record.wants) = (spenders, wants);
        let mut wants = std::mem::take(&mut self.goods.wants);
        wants.extend(self.public_wants(ctx.regions));
        let leg = |unit: u16| GoodsLeg { unit: Denom::units(unit), reason: SOLD, order: 0, used: true };
        let (sales, spent) = self.meet_all(ctx, day, &wants, &leg, &mut moved);
        let invest = self.investment_wants(ctx);
        let held = |unit: u16| GoodsLeg { unit: Denom::units(unit), reason: DELIVERED, order: 0, used: false };
        let _ = self.meet_all(ctx, day, &invest, &held, &mut moved);
        (record.sales, record.spent) = (sales, spent);
        (record.reviews, record.repriced) = self.review_prices(ctx, day);
        let close = self.goods.stocks.totals();
        record.breaks = len_u64(breaks(&open, &nature_net(&moved), &close).len());
        self.goods.days.push(record);
        record
    }

    /// A goods flow applied at once from the payer's free units, its payee receiving them at `cost`; the cost the
    /// payer's units carried out, none where the payer held too few.
    fn move_goods(&mut self, flow: Flow, cost: Cost, day: Day, moved: &mut Vec<Flow>) -> Option<i64> {
        match self.goods.stocks.apply(&flow, Bound::Free, cost, day) {
            Ok(carried) => {
                moved.push(flow);
                carried
            }
            Err(_) => None,
        }
    }

    /// A firm's unit cost: its output's stock's average cost and its wage bill a unit of what it expects to sell.
    fn unit_cost(&self, f: &Firm, expected: f64) -> f64 {
        let unit = self.goods.units.find(Held::Good(Good { product: f.product, grade: 0, zone: f.region }));
        let held = unit.and_then(|u| self.goods.stocks.holding(f.key, u)).copied();
        let stock_cost = held.filter(|h| h.units > 0).map_or(0.0, |h| from_i64(h.cost) / from_i64(h.units));
        let wage_bill: f64 = self.families.iter().find(|x| x.name == "LAB.employment").map_or(0.0, |fam| {
            fam.store.of(0, f.key.slot()).filter_map(|e| fam.store.edges.row(e)).map(|r| from_i64(r.amount)).sum()
        });
        let labour = if expected > 0.0 { wage_bill * MONTHS_A_YEAR / DAYS_A_YEAR / expected } else { 0.0 };
        stock_cost + labour
    }

    /// Each firm's making today by the production rule: the sales a day it expects and the gap to the stock its
    /// management aims to hold closed over its production period, within what its staff's hours make at its hours a
    /// unit and what its stored inputs allow, and nothing where a unit would not pay; the inputs used up and the
    /// output made, carrying their cost.
    #[clause("FRM.4", "FRM.14", "GDS.4", "TEC.9")]
    fn make(&mut self, ctx: &GoodsCtx<'_>, day: Day, moved: &mut Vec<Flow>) -> i64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let m = ctx.management;
        let mut made = 0;
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let Some(expected) = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE) else {
                continue;
            };
            let stored = self.goods.stored.get(usize::from(f.product)).copied().unwrap_or(false);
            let stock = if stored { self.free_units(f.key, f.product, f.region) } else { 0 };
            let inputs = self.stored_inputs(&f);
            let mut capacity = self.staff_capacity(&f).map_or(f64::INFINITY, from_i64);
            for (q, a) in &inputs {
                let can = from_i64(self.free_units(f.key, *q, f.region)) / a;
                capacity = if can < capacity { can } else { capacity };
            }
            let Some(lot) = floor_to_i64(sys_frm::FilingPrims::lot(ctx.register, f.product)) else { continue };
            let rate = self.labour.financing.get(f.country).copied().unwrap_or(0.0);
            let input = sys_frm::rules::produce::ProduceIn {
                expected_demand: expected,
                stock: from_i64(stock),
                cover: if stored { m.cover_days } else { 0.0 },
                adjustment: m.production_days,
                capacity,
                expected_price: from_i64(f.price) / from_i64(lot),
                unit_cost: self.unit_cost(&f, expected),
                financing_rate: rate / DAYS_A_YEAR,
                lead: self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0),
            };
            let sys_frm::rules::produce::Produce::Make(units) = sys_frm::rules::produce::target(&input) else {
                continue;
            };
            let today = floor_to_i64(units.floor()).unwrap_or(0);
            if today <= 0 {
                continue;
            }
            let mut cost = 0;
            for (q, a) in inputs {
                let used = whole_units(from_i64(today) * a);
                if used <= 0 {
                    continue;
                }
                let unit = self.unit_of(q, f.region);
                let flow = Flow {
                    payer: f.key,
                    payee: NATURE,
                    amount: used,
                    source: slot.get(),
                    denomination: Denom::units(unit),
                    reason: USED,
                    order: 0,
                };
                cost += self.move_goods(flow, Cost::Carried, day, moved).unwrap_or(0);
            }
            let unit = self.unit_of(f.product, f.region);
            let flow = Flow {
                payer: NATURE,
                payee: f.key,
                amount: today,
                source: slot.get(),
                denomination: Denom::units(unit),
                reason: MADE,
                order: 0,
            };
            let _ = self.move_goods(flow, Cost::At(cost), day, moved);
            made += today;
        }
        made
    }

    /// The whole units a firm's staff's hours a day make at its hours a unit; none known where it has no hours a unit.
    fn staff_capacity(&self, f: &Firm) -> Option<i64> {
        let family = self.families.iter().position(|x| x.name == "LAB.employment")?;
        let (level, ways) = (self.labour.level.get(f.country)?, self.labour.ways.get(f.country)?);
        let p = usize::from(f.product);
        let a_unit: f64 = level
            .iter()
            .zip(ways)
            .map(|(l, occ)| l * sys_frm::rules::way::own_hours(occ.get(p).copied().unwrap_or(0.0), f.productivity))
            .sum();
        if a_unit <= 0.0 {
            return None;
        }
        let fam = self.families.get(family)?;
        let hours: f64 = fam
            .store
            .of(0, f.key.slot())
            .filter_map(|e| {
                let row = fam.store.edges.row(e)?;
                fam.classes.get(usize::try_from(row.schedule).ok()?).map(|k| f64::from(k[1]) / DAYS_A_WEEK)
            })
            .sum();
        floor_to_i64((hours / a_unit).floor())
    }

    /// Each firm's stored inputs topped up to what the days of making and cover its planned output asks, bought from
    /// the firms of its region at their posted prices; each purchase a flow of money and one of goods to the buyer.
    #[clause("FRM.7", "GDS.5", "MKT.6")]
    fn buy_inputs(&mut self, ctx: &GoodsCtx<'_>, day: Day, moved: &mut Vec<Flow>) -> u64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let mut wants: Vec<(u16, Buyer)> = Vec::new();
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let lead = self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0);
            let Some(per_day) = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE) else {
                continue;
            };
            for (q, a) in self.stored_inputs(&f) {
                let short = whole_units(a * per_day * (lead + self.goods.cover)) - self.free_units(f.key, q, f.region);
                if short > 0 {
                    let buyer = Buyer {
                        party: f.key,
                        subject: u64::from(f.key.word()),
                        want: Want::Units(short),
                        place: f.region,
                    };
                    wants.push((q, buyer));
                }
            }
        }
        let n = len_u64(wants.len());
        let leg = |unit: u16| GoodsLeg { unit: Denom::units(unit), reason: DELIVERED, order: 0, used: false };
        let _ = self.meet_all(ctx, day, &wants, &leg, moved);
        n
    }

    /// The households whose spending day came: each takes in what it received since it last decided, once a month
    /// its income into its outlook, spends by the buffer-stock rule at its cash on hand, never more than it holds,
    /// and asks its country's budget shares of that of each product at retail.
    #[clause("HH.1", "HH.2", "HH.4", "HH.5", "HH.18", "HH.19")]
    fn decide_spending(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> (u64, u64) {
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.clone())
        else {
            return (0, 0);
        };
        let position =
            |name: &str| decl.positions.iter().position(|p| p.item.name == name).map(|i| decl.attrs.len() + i);
        let (Some(income_at), Some(after_at), Some(received_at), Some(looked_at)) = (
            position(<if_pop::facts::Income as phx_core::FactDef>::ITEM.name),
            position(<if_pop::facts::After as phx_core::FactDef>::ITEM.name),
            position(<if_pop::facts::Received as phx_core::FactDef>::ITEM.name),
            position(<if_pop::facts::Looked as phx_core::FactDef>::ITEM.name),
        ) else {
            return (0, 0);
        };
        let Missing::Present(region_at) = decl.sited_by else { return (0, 0) };
        let mut due = Vec::new();
        if let Some(w) = self.goods.spenders.as_mut() {
            w.take(day, &mut due, None);
        }
        let month = months(ctx.calendar.date(day));
        let mut wants = Vec::new();
        let mut spenders = 0;
        for s in due {
            let slot = Slot::new(s);
            let Some(id) = self.kinds.get(place).and_then(|k| k.parties.id(slot)) else { continue };
            let next = after(day, u64::from(self.goods.spend_days));
            if let Some(w) = self.goods.spenders.as_mut() {
                w.schedule(s, next);
            }
            let Some(money) = self.kinds.get(place).and_then(|k| k.accounts.as_ref()).and_then(|a| a.balance.get(slot))
            else {
                continue;
            };
            spenders += 1;
            let read = |at: usize| self.record_word(place, slot, at);
            let (received, looked) = match (read(after_at), read(received_at), read(looked_at)) {
                (Some(after), Some(received), Some(looked)) => (received + money - after, looked),
                _ => (0, month),
            };
            let mut outlook = read(income_at).map(from_i64);
            let (received, looked) = if month > looked {
                let seen = from_i64(received) * MONTHS_A_YEAR / from_i64(month - looked);
                outlook = Some(match outlook {
                    Some(p) => phx_val::heuristics::adaptive(p, seen, ctx.rule.gain),
                    None => seen,
                });
                (0, month)
            } else {
                (received, looked)
            };
            self.set_record_word(place, slot, received_at, received);
            self.set_record_word(place, slot, looked_at, looked);
            let Some(income) = outlook.filter(|y| *y > 0.0) else {
                self.set_record_word(place, slot, after_at, money);
                continue;
            };
            let cash = from_i64(money) / income + 1.0;
            let wanted = sys_hh::buffer::spend(&ctx.rule.rule, cash) * income * ctx.rule.period;
            let held = from_i64(money);
            let spent = if wanted < held { wanted } else { held };
            let Some(Missing::Present(region)) =
                self.kinds.get(place).and_then(|k| k.record(slot).get(region_at).map(|w| w.get()))
            else {
                continue;
            };
            let Ok(region) = u32::try_from(region) else { continue };
            let c = ctx.regions.get(usize::try_from(region).unwrap_or(usize::MAX)).map(|c| usize::from(c.get()));
            let Some(shares) = c.and_then(|c| ctx.rule.shares.get(c)) else {
                violation!(clause = "HH.5", "a household's country with no budget shares", region = region);
            };
            let mut total = 0;
            for (product, share) in (0_u16..).zip(shares) {
                let amount = floor_to_i64(spent * share).unwrap_or(0);
                if amount > 0 {
                    let party = PartyKey::new(kind_number(place), slot);
                    wants.push((product, Buyer { party, subject: id.get(), want: Want::Money(amount), place: region }));
                    total += amount;
                }
            }
            if let Some(o) = outlook.and_then(floor_to_i64) {
                self.set_record_word(place, slot, income_at, o);
            }
            self.set_record_word(place, slot, after_at, money - total);
        }
        let n = len_u64(wants.len());
        self.goods.wants = wants;
        (spenders, n)
    }

    /// The firms whose production schedule came today review their price: the sales a day they expect corrected toward
    /// what they sold since their last review at their management's speed; the pressure of that demand and their stock
    /// against its target; the price they would like, their markup over their unit cost — their stock's average cost
    /// and their wage bill a unit — raised by that pressure; and the move made only where it gains more than changing
    /// the price costs their staff's hours.
    #[clause("FRM.5", "FRM.14", "REP.34", "VAL.6")]
    fn review_prices(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> (u64, u64) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return (0, 0) };
        let m = ctx.management;
        let due = std::mem::take(&mut self.labour.due_today);
        let (mut reviews, mut repriced) = (0, 0);
        for s in due {
            let slot = Slot::new(s);
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let (Some(markup), Some(expected), Some(sold), Some(last)) = (
                self.record_word(firm, slot, MARKUP),
                self.record_word(firm, slot, EXPECTED),
                self.record_word(firm, slot, SOLD_UNITS),
                self.record_word(firm, slot, REVIEWED),
            ) else {
                continue;
            };
            let days = i64::from(day.get()) - last;
            if days <= 0 {
                continue;
            }
            reviews += 1;
            self.set_record_word(firm, slot, REVIEWED, i64::from(day.get()));
            let (markup, expected) = (from_i64(markup) / PART_ONE, from_i64(expected) / PART_ONE);
            let demand = from_i64(sold) / from_i64(days);
            let expected = expected + m.sales_speed * (demand - expected);
            self.set_record_word(firm, slot, EXPECTED, whole_units(expected * PART_ONE));
            self.set_record_word(firm, slot, SOLD_UNITS, 0);
            let stock = from_i64(self.free_units(f.key, f.product, f.region));
            let unit_cost = self.unit_cost(&f, expected);
            let Some(lot) = floor_to_i64(sys_frm::FilingPrims::lot(ctx.register, f.product)) else { continue };
            let lot = from_i64(lot);
            let pressure =
                match sys_frm::rules::price::pressure_stocked(demand, expected, m.cover_days * expected, stock) {
                    Missing::Present(p) => p,
                    Missing::Absent => continue,
                };
            if unit_cost <= 0.0 {
                continue;
            }
            let wanted = sys_frm::rules::price::desired(markup, unit_cost * lot, pressure, m.curvature);
            let law = self.labour.laws.get(f.country);
            let hour = law.map_or(0.0, |l| l.mean_monthly / (l.weeks_a_month * f64::from(l.full_time_hours)));
            let revenue = demand * from_i64(f.price) / lot * m.production_days;
            let points = m.points_near(wanted);
            if let Some(p) =
                sys_frm::rules::price::reprice(&points, f.price, wanted, revenue, markup, m.menu_hours * hour)
            {
                self.set_record_word(firm, slot, PRICE, p);
                repriced += 1;
            }
        }
        (reviews, repriced)
    }

    /// Each product's posted-price meeting over its firms holding it free with a price, each region a place whose
    /// firms are in its reach at no distance until the finer cells: each sale's money a flow settled with the day's,
    /// and its goods moved at once from the seller at their price. Returns the sales and what they paid.
    #[clause("SRV.4", "SRV.5", "MKT.6", "GDS.4")]
    fn meet_all(
        &mut self,
        ctx: &GoodsCtx<'_>,
        day: Day,
        wants: &[(u16, Buyer)],
        leg: &dyn Fn(u16) -> GoodsLeg,
        moved: &mut Vec<Flow>,
    ) -> (u64, i64) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return (0, 0) };
        if wants.is_empty() {
            return (0, 0);
        }
        let (Some(taste), Some(lot_stream)) =
            (ctx.streams.named(sys_srv::TasteStream::DECL.name), ctx.streams.named(sys_srv::LotStream::DECL.name))
        else {
            return (0, 0);
        };
        let key = ctx.streams.key(&taste);
        // Each product's stalls, with each seller's unit, currency and region.
        let mut by_product: BTreeMap<u16, Vec<(Stall, u16, u8, u32)>> = BTreeMap::new();
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let free = self.free_units(f.key, f.product, f.region);
            if free <= 0 || f.price <= 0 {
                continue;
            }
            let unit = self.unit_of(f.product, f.region);
            let Ok(ccy) = u8::try_from(f.country) else { continue };
            let stall = Stall { seller: f.key, price: f.price, units: free };
            by_product.entry(f.product).or_default().push((stall, unit, ccy, f.region));
        }
        let (mut sales, mut spent) = (0, 0);
        let mut money = Vec::new();
        for (product, stalls) in by_product {
            let buyers: Vec<Buyer> = wants.iter().filter(|(p, _)| *p == product).map(|(_, b)| *b).collect();
            if buyers.is_empty() {
                continue;
            }
            let places: Vec<Place> = (0_u32..)
                .take(ctx.regions.len())
                .map(|r| Place {
                    near: (0_u32..).zip(&stalls).filter(|(_, x)| x.3 == r).map(|(i, _)| (i, 0.0)).collect(),
                })
                .collect();
            let plain: Vec<Stall> = stalls.iter().map(|x| x.0).collect();
            let Some(lot) = floor_to_i64(sys_frm::FilingPrims::lot(ctx.register, product)) else { continue };
            let tastes = Tastes { key, day: day.get(), substep: SubStep::S5c.ordinal() };
            let lots = |seller: PartyKey, round: u32| {
                ctx.streams.open(
                    &lot_stream,
                    Subject::new(SubjectTag::Party, packed(u64::from(seller.word()), u64::from(round), ROUND_BITS)),
                    day,
                    SubStep::S5c.ordinal(),
                )
            };
            let mut meeting = std::mem::take(&mut self.goods.meeting);
            meet(&mut meeting, None, (&plain, &places, &buyers), (lot, ctx.weights), tastes, &lots);
            let made: Vec<Sale> = meeting.sales().copied().collect();
            self.goods.meeting = meeting;
            for sale in made {
                let Some((_, unit, ccy, _)) = stalls.iter().find(|x| x.0.seller == sale.seller).copied() else {
                    continue;
                };
                let mut out = Vec::new();
                sale.flows((Denom::money(ccy), SOLD, 0), Some(leg(unit)), sale.seller.slot().get(), &mut out);
                if let (true, Some(Some(rate)), Some(included), Some(Some(treasury))) = (
                    leg(unit).used,
                    self.state.consumption.get(usize::from(ccy)).copied(),
                    self.state.included,
                    self.treasuries.get(usize::from(ccy)).copied(),
                ) {
                    // The consumption tax a price paid includes, which the seller owes its treasury.
                    let tax = phx_ledger::opening::whole(included(from_i64(sale.paid), rate));
                    if tax > 0 {
                        money.push(Flow {
                            payer: sale.seller,
                            payee: treasury,
                            amount: tax,
                            source: sale.seller.slot().get(),
                            denomination: Denom::money(ccy),
                            reason: crate::consts::reason::TAXED,
                            order: 0,
                        });
                    }
                }
                for f in out {
                    if f.denomination.is_money() {
                        money.push(f);
                    } else {
                        let _ = self.move_goods(f, Cost::At(sale.paid), day, moved);
                    }
                }
                if let Some(sold) = self.record_word(firm, sale.seller.slot(), SOLD_UNITS) {
                    self.set_record_word(firm, sale.seller.slot(), SOLD_UNITS, sold + sale.units);
                }
                sales += 1;
                spent += sale.paid;
            }
        }
        self.pending.append(&mut money);
        (sales, spent)
    }
}
