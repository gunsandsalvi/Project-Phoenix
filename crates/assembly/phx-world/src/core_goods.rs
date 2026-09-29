//! Goods on the core, a meeting a day. A good is a product at a region, held in the core's `Stocks`. Each firm holds
//! its output's days of cover and, of each stored input its way uses, what the days of making it takes and those
//! days cover use. Each day a firm makes what its planned output asks, within what its staff's hours make at its hours
//! a unit and what its stored inputs allow: the inputs are used up and the output made, the output carrying their
//! cost. Each firm then tops up its stored inputs from the firms of its region at their posted prices, and each
//! household due decides by the buffer-stock rule what it spends and asks each product's share of it at retail. Each
//! sale is a flow of money from buyer to seller and one of goods from the seller, held by a firm, used up by a
//! household. A service is never held: its provider offers the day's capacity, makes what sells as it is sold, using
//! its inputs then, and the capacity no sale took is lost at the day's end. The goods' identity is read each day.

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
    EXPECTED, MARKUP, MEMORY, OUTPUT, PART_ONE, PRICE, PRODUCT, PRODUCTIVITY, PRODUCTIVITY_ONE, REGION, REVIEWED,
    SOLD as SOLD_UNITS, STANCE, SWITCHING,
};
use crate::consts::reason::{DELIVERED, MADE, PERISHED, SOLD, SPOILED, USED};
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
    /// What the day's sales took from their named buyers and what they credited their named sellers, and the sales
    /// that named no buyer or no seller.
    pub debits: i128,
    pub credits: i128,
    pub unnamed: u64,
}

/// A trade's price reviews over the run: the reviews, the prices moved, and the sum of the moves' sizes, each the
/// new price's difference from the old over the old.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PriceTally {
    pub reviews: u64,
    pub changes: u64,
    pub size: f64,
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
    /// Each product's days a unit takes to make, the days of sales a firm's stock covers and the days over which it
    /// closes the gap to that cover.
    pub lead: Vec<f64>,
    pub cover: f64,
    pub adjustment: f64,
    /// Each product's yearly rate of loss in stock, and the days between two realisations of it.
    pub spoil_rates: Vec<f64>,
    pub spoil_days: u32,
    /// Each product's lot, the units its price is posted for; and each country's stored inputs a unit of each product.
    pub lots: Vec<f64>,
    pub recipes: Vec<Vec<Vec<(u16, f64)>>>,
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
    /// Each product's least posted price a unit at each region, as the day opened.
    pub cheapest: BTreeMap<(u16, u32), f64>,
    /// Each product's sales at each region today, what they paid and their units; and its mark, the price a lot its
    /// last day of sales there paid on average.
    pub traded: BTreeMap<(u16, u32), (i128, i128)>,
    pub marks: BTreeMap<(u16, u32), f64>,
    /// The marks as public series, each method's outlooks of them, and the firms' stances by day.
    pub outlooks: crate::core_outlooks::Outlooks,
    /// Each trade's price reviews over the run.
    pub prices: BTreeMap<u16, PriceTally>,
    /// Today's sales' debits to named buyers, credits to named sellers, and sales naming neither.
    pub named: (i128, i128, u64),
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
    /// The workers a meeting's choices and sales run on, whose outcome is the same on any.
    pub pool: Option<&'a phx_exec::Pool>,
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

/// A stall at a meeting with its seller's unit of the product, currency and region.
type StallAt = (Stall, u16, u8, u32);

/// A maker at the opening: its slot, its output a year and its price a unit.
type Maker = (Slot, f64, f64);

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
    fn lot(&self, product: u16) -> f64 {
        self.goods.lots.get(usize::from(product)).copied().unwrap_or_else(|| {
            violation!(clause = "GDS.1", "a product the technology does not declare", product = product)
        })
    }

    fn is_stored(&self, product: u16) -> bool {
        self.goods.stored.get(usize::from(product)).copied().unwrap_or_else(|| {
            violation!(clause = "GDS.1", "a product the technology does not declare", product = product)
        })
    }

    fn stored_inputs(&self, f: &Firm) -> Vec<(u16, f64)> {
        self.recipe(f).to_vec()
    }

    /// A firm's stored inputs a unit of its output, as its country's ways give them, read from the day's table.
    fn recipe(&self, f: &Firm) -> &[(u16, f64)] {
        self.goods.recipes.get(f.country).and_then(|c| c.get(usize::from(f.product))).map_or(&[], Vec::as_slice)
    }

    /// Each country's stored inputs a unit of each product.
    fn recipes(&self) -> Vec<Vec<Vec<(u16, f64)>>> {
        self.goods
            .inputs
            .iter()
            .map(|ways| {
                let products = ways.first().map_or(0, Vec::len);
                (0..products)
                    .map(|p| {
                        (0_u16..)
                            .zip(ways)
                            .filter(|(q, _)| self.is_stored(*q))
                            .filter_map(|(q, row)| row.get(p).copied().filter(|a| *a > 0.0).map(|a| (q, a)))
                            .collect()
                    })
                    .collect()
            })
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
        (countries, (cover, adjustment)): (&[OpeningCountry], (f64, f64)),
        today: Day,
    ) -> Result<(), String> {
        let (Some(place), Some(firm)) =
            (self.names.iter().position(|n| *n == "household"), self.names.iter().position(|n| *n == "firm"))
        else {
            return Ok(());
        };
        let days = floor_to_i64((ctx.rule.period * DAYS_A_YEAR).round()).and_then(|d| u32::try_from(d).ok());
        let Some(days) = days.filter(|d| *d > 0) else { return Err("a spending schedule of no days".to_owned()) };
        let mut goods = CoreGoods { spend_days: days, began: today.get(), cover, adjustment, ..CoreGoods::default() };
        let products = ctx.register.products("TEC.products")?;
        goods.stored = products.iter().map(|p| p.storable).collect();
        let lead = ctx.register.table1("TEC.lead_time")?;
        goods.lead = (0_i64..).take(products.len()).map(|p| lead.at(p).map_or(0.0, from_i64)).collect();
        goods.lots = (0_u16..).take(products.len()).map(|p| sys_frm::FilingPrims::lot(ctx.register, p)).collect();
        goods.spoil_rates = sys_gds::spoilage_rates(ctx.register)?;
        goods.spoil_days = u32::try_from(ctx.register.count(sys_gds::SPOILAGE_DAYS.id)?).map_err(|e| e.to_string())?;
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
        self.goods.recipes = self.recipes();
        self.goods.turnover_share = self.turnover_shares(ctx, firm);
        self.open_prices(ctx, firm, &prices);
        self.open_expected(ctx, firm, today);
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

    /// Each firm's day-zero price posted from its own cost on the core: its markup in the accounts over what a unit
    /// costs it to make — its staff's wages over what they make, and its inputs at the opening's prices it holds them
    /// at — at the nearest point of its trade's table.
    #[clause("FRM.5", "GEN.13", "REP.34")]
    fn open_prices(&mut self, ctx: &GoodsCtx<'_>, firm: usize, prices: &[Vec<f64>]) {
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let Some(markup) = self.record_word(firm, slot, MARKUP).map(|m| from_i64(m) / PART_ONE) else { continue };
            let Some(price) = prices.get(f.country) else { continue };
            let Some(cost) = self.cost_at(&f, &|q| price.get(usize::from(q)).copied()) else { continue };
            let wanted = (1.0 + markup) * cost * self.lot(f.product);
            if let Some(p) = sys_frm::rules::price::nearest_point(&ctx.management.points_near(wanted), wanted) {
                self.set_record_word(firm, slot, PRICE, p);
            }
        }
    }

    /// Each country's households' spending a year at the opening, by the rule their first decision reads: the
    /// buffer-stock rule at their cash on hand, what they hold beyond what their contracts take before that decision
    /// over the income they expect.
    fn opening_spending(&self, ctx: &GoodsCtx<'_>, today: Day) -> BTreeMap<usize, f64> {
        let mut out: BTreeMap<usize, f64> = BTreeMap::new();
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.as_ref())
        else {
            return out;
        };
        let income_name = <if_pop::facts::Income as phx_core::FactDef>::ITEM.name;
        let Some(income_at) = decl.positions.iter().position(|p| p.item.name == income_name) else { return out };
        let income_at = decl.attrs.len() + income_at;
        let Missing::Present(region_at) = decl.sited_by else { return out };
        let next = after(today, u64::from(self.goods.spend_days));
        let Some(store) = self.kinds.get(place) else { return out };
        for slot in store.parties.live_slots() {
            let (Some(money), Some(income), Some(Missing::Present(region))) = (
                store.accounts.as_ref().and_then(|a| a.balance.get(slot)),
                self.record_word(place, slot, income_at).map(from_i64).filter(|y| *y > 0.0),
                store.record(slot).get(region_at).map(|w| w.get()),
            ) else {
                continue;
            };
            let Some(c) = usize::try_from(region).ok().and_then(|r| ctx.regions.get(r)) else { continue };
            let free = money - self.owed_until(PartyKey::new(kind_number(place), slot), (today, next), ctx.calendar);
            let cash = from_i64(free) / income + 1.0;
            *out.entry(usize::from(c.get())).or_insert(0.0) += sys_hh::buffer::spend(&ctx.rule.rule, cash) * income;
        }
        out
    }

    /// Each firm's expected sales at the opening: the demand the world's buyers bring at the opening's prices — the
    /// households' spending by their budget shares, the state's collective consumption and the firms' fixed
    /// investment, and what the firms' ways use of each stored product to make all of it — shared over each country's
    /// makers of a product by their output, so no firm expects a buyer the world does not hold.
    #[clause("GEN.2", "GEN.5", "FRM.14", "HH.5")]
    fn open_expected(&mut self, ctx: &GoodsCtx<'_>, firm: usize, today: Day) {
        let spending = self.opening_spending(ctx, today);
        let mut makers: BTreeMap<(usize, u16), Vec<Maker>> = BTreeMap::new();
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let lot = self.lot(f.product);
            makers.entry((f.country, f.product)).or_default().push((slot, f.output, from_i64(f.price) / lot));
        }
        for (country, uses) in self.goods.final_uses.clone().iter().enumerate() {
            let (Some(gdp), Some(shares), Some(inputs)) =
                (self.goods.gdp.get(country).copied(), ctx.rule.shares.get(country), self.goods.inputs.get(country))
            else {
                continue;
            };
            let households = spending.get(&country).copied().unwrap_or(0.0);
            let n = uses.len();
            // Each product's final demand in units a year at its makers' mean price, their output its weight.
            let mut last: Vec<f64> = (0..n)
                .map(|q| {
                    let Some(of) = u16::try_from(q).ok().and_then(|q| makers.get(&(country, q))) else { return 0.0 };
                    let weight: f64 = of.iter().map(|(_, w, _)| w).sum();
                    let price = if weight > 0.0 { of.iter().map(|(_, w, p)| w * p).sum::<f64>() / weight } else { 0.0 };
                    let value = households * shares.get(q).copied().unwrap_or(0.0)
                        + uses.get(q).map_or(0.0, |u| (u[0] + u[1]) * gdp);
                    if price > 0.0 { value / price } else { 0.0 }
                })
                .collect();
            let finals = last.clone();
            // What the ways use of each stored product to make it all, to the fixed point x = A·x + f.
            for _ in 0..n * n {
                let next: Vec<f64> = (0..n)
                    .map(|q| {
                        let stored = u16::try_from(q).is_ok_and(|q| self.is_stored(q));
                        let used: f64 = if stored {
                            inputs.get(q).map_or(0.0, |row| row.iter().zip(&last).map(|(a, x)| a * x).sum())
                        } else {
                            0.0
                        };
                        finals.get(q).copied().unwrap_or(0.0) + used
                    })
                    .collect();
                let moved = next.iter().zip(&last).any(|(a, b)| (a - b).abs() > f64::EPSILON * a.abs());
                last = next;
                if !moved {
                    break;
                }
            }
            for (q, x) in (0_u16..).zip(&last) {
                let Some(of) = makers.get(&(country, q)) else { continue };
                let weight: f64 = of.iter().map(|(_, w, _)| w).sum();
                if weight <= 0.0 {
                    continue;
                }
                for (slot, w, _) in of {
                    let per_day = x * w / weight / DAYS_A_YEAR;
                    self.set_record_word(firm, *slot, EXPECTED, whole_units(per_day * PART_ONE));
                }
            }
        }
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
            let lot = self.lot(f.product);
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
            let Some(per_day) = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE) else {
                continue;
            };
            let price = |q: u16| prices.get(f.country).and_then(|p| p.get(usize::from(q))).copied().unwrap_or(0.0);
            let mut wanted: Vec<(u16, f64)> = Vec::new();
            if self.is_stored(f.product) {
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
        self.goods.cheapest = self.cheapest(ctx);
        let mut moved: Vec<Flow> = Vec::new();
        self.spoil(day, &mut moved);
        record.made = self.make(ctx, day, &mut moved);
        record.inputs_wanted = self.buy_inputs(ctx, day, &mut moved);
        let (spenders, wants) = self.decide_spending(ctx, day);
        (record.spenders, record.wants) = (spenders, wants);
        let mut wants = std::mem::take(&mut self.goods.wants);
        wants.extend(self.public_wants(ctx.regions));
        let leg = |_: u16, unit: u16| GoodsLeg { unit: Denom::units(unit), reason: SOLD, order: 0, used: true };
        let (sales, spent) = self.meet_all(ctx, day, (&wants, crate::core_stats::Purchase::Final), &leg, &mut moved);
        let invest = self.investment_wants(ctx);
        // A service bought as investment is used as it is delivered, being made as it is sold; a good is held.
        let stored = self.goods.stored.clone();
        let held = |product: u16, unit: u16| GoodsLeg {
            unit: Denom::units(unit),
            reason: DELIVERED,
            order: 0,
            used: !stored.get(usize::from(product)).copied().unwrap_or(true),
        };
        let _ = self.meet_all(ctx, day, (&invest, crate::core_stats::Purchase::Investment), &held, &mut moved);
        self.close_services(ctx, day, &mut moved);
        (record.sales, record.spent) = (sales, spent);
        (record.debits, record.credits, record.unnamed) = std::mem::take(&mut self.goods.named);
        let types = &ctx.management.types;
        for ((product, region), (paid, units)) in std::mem::take(&mut self.goods.traded) {
            if units > 0 {
                let lot = self.lot(product);
                let mark = phx_rand::float::from_i128(paid) / phx_rand::float::from_i128(units) * lot;
                self.goods.marks.insert((product, region), mark);
                self.goods.outlooks.print((product, region), mark, day, types);
            }
        }
        (record.reviews, record.repriced) = self.review_prices(ctx, day);
        self.count_stances(day);
        let close = self.goods.stocks.totals();
        let broken = breaks(&open, &nature_net(&moved), &close);
        record.breaks = len_u64(broken.len());
        for (good, expected, held) in broken {
            self.found.push(phx_core::findings::Finding {
                family: "goods",
                clause: "GDS.10",
                owner: phx_core::findings::FindingOwner::Run,
                size: held - expected,
                unit: phx_core::findings::Unit::Count,
                day,
                detail: format!("good {good}: {held} units held where the day leaves {expected}"),
            });
        }
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

    /// A held good's average cost a unit; none known while none is held.
    fn average_cost(&self, holder: PartyKey, product: u16, region: u32) -> Option<f64> {
        let unit = self.goods.units.find(Held::Good(Good { product, grade: 0, zone: region }))?;
        let held = self.goods.stocks.holding(holder, unit).filter(|h| h.units > 0)?;
        Some(from_i64(held.cost) / from_i64(held.units))
    }

    /// A firm's cost of making a unit now: its wage bill a day over what its staff make a day and the inputs a unit uses
    /// at what they cost it, what it holds at what it paid and what it must buy at the least price it is sold at in its
    /// region.
    /// What it uses of its own product costs what making it costs, so the rest is grossed up by the share of a unit it
    /// uses of itself; its stock's cost, which mixes units it bought at others' prices, is not its cost of making.
    /// None known while its staff make nothing, or an input it uses is neither held nor sold in its region.
    fn unit_cost(&self, f: &Firm) -> Option<f64> {
        self.cost_at(f, &|q| {
            self.average_cost(f.key, q, f.region).or_else(|| self.goods.cheapest.get(&(q, f.region)).copied())
        })
    }

    /// A firm's cost of making a unit now, by its place.
    pub(crate) fn unit_cost_of(&self, regions: &[CountryId], firm: usize, slot: Slot) -> Option<f64> {
        self.unit_cost(&self.goods_firm(regions, firm, slot)?)
    }

    /// A firm's cost of making a unit with its inputs at the costs given.
    fn cost_at(&self, f: &Firm, input_cost: &dyn Fn(u16) -> Option<f64>) -> Option<f64> {
        let mut inputs = 0.0;
        let mut own = 0.0;
        for &(q, a) in self.recipe(f) {
            if q == f.product {
                own = a;
            } else {
                inputs += a * input_cost(q)?;
            }
        }
        let wage_bill: f64 = self.families.iter().find(|x| x.name == "LAB.employment").map_or(0.0, |fam| {
            fam.store.of(0, f.key.slot()).filter_map(|e| fam.store.edges.row(e)).map(|r| from_i64(r.amount)).sum()
        });
        let made = from_i64(self.staff_capacity(f)?);
        if made <= 0.0 {
            return None;
        }
        let labour = wage_bill * MONTHS_A_YEAR / DAYS_A_YEAR / made;
        (own < 1.0).then(|| (inputs + labour) / (1.0 - own))
    }

    /// A firm's making today by the production rule: the units, none where it makes nothing.
    fn making(&self, ctx: &GoodsCtx<'_>, (firm, slot): (usize, Slot)) -> Option<(Firm, i64)> {
        let m = ctx.management;
        let f = self.goods_firm(ctx.regions, firm, slot)?;
        let expected = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE)?;
        let stored = self.is_stored(f.product);
        let stock = if stored { self.free_units(f.key, f.product, f.region) } else { 0 };
        let mut capacity = self.staff_capacity(&f).map_or(f64::INFINITY, from_i64);
        for (q, a) in self.recipe(&f) {
            let can = from_i64(self.free_units(f.key, *q, f.region)) / a;
            capacity = if can < capacity { can } else { capacity };
        }
        let lot = floor_to_i64(self.lot(f.product))?;
        let unit_cost = self.unit_cost(&f)?;
        let rate = self.labour.financing.get(f.country).copied().unwrap_or(0.0);
        // A service's stall is the day's capacity, where a unit pays: it is made as it sells.
        if !stored && !capacity.is_finite() {
            return None;
        }
        let input = sys_frm::rules::produce::ProduceIn {
            expected_demand: if stored { expected } else { capacity },
            stock: from_i64(stock),
            cover: if stored { m.cover_days } else { 0.0 },
            adjustment: m.adjustment_days,
            capacity,
            expected_price: from_i64(f.price) / from_i64(lot),
            unit_cost,
            financing_rate: rate / DAYS_A_YEAR,
            lead: self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0),
        };
        let sys_frm::rules::produce::Produce::Make(units) = sys_frm::rules::produce::target(&input) else {
            return None;
        };
        let today = floor_to_i64(units.floor()).unwrap_or(0);
        (today > 0).then_some((f, today))
    }

    /// Each firm's making today by the production rule: the sales a day it expects and the gap to the stock its
    /// management aims to hold closed over its production period, within what its staff's hours make at its hours a
    /// unit and what its stored inputs allow, and nothing where a unit would not pay; the inputs used up and the
    /// output made, carrying their cost.
    #[clause("FRM.4", "FRM.14", "GDS.4", "TEC.9")]
    fn make(&mut self, ctx: &GoodsCtx<'_>, day: Day, moved: &mut Vec<Flow>) -> i64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let slots = self.firm_slots(firm);
        let plans: Vec<Option<(Firm, i64)>> = match ctx.pool {
            Some(pool) => pool.map(slots.len(), |i| slots.get(i).and_then(|s| self.making(ctx, (firm, *s)))),
            None => slots.iter().map(|s| self.making(ctx, (firm, *s))).collect(),
        };
        let mut made = 0;
        for (slot, (f, today)) in slots.into_iter().zip(plans).filter_map(|(s, p)| p.map(|p| (s, p))) {
            let stored = self.is_stored(f.product);
            let inputs = self.stored_inputs(&f);
            if !stored {
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
                let _ = self.move_goods(flow, Cost::At(0), day, moved);
                made += today;
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

    /// Every holder's goods lost in stock on each spoilage period's last day: of each holding of a product that spoils,
    /// what its yearly rate takes over the days its units were held within the period.
    #[clause("GDS.8", "GDS.10")]
    fn spoil(&mut self, day: Day, moved: &mut Vec<Flow>) {
        let period = self.goods.spoil_days;
        if period == 0 || !(day.get() - self.goods.began).is_multiple_of(period) {
            return;
        }
        let mut holders: Vec<PartyKey> = Vec::new();
        for (k, store) in self.kinds.iter().enumerate() {
            let Ok(kind) = u8::try_from(k) else { continue };
            holders.extend(store.parties.live_slots().map(|s| PartyKey::new(kind, s)));
        }
        let units = &self.goods.units;
        let rates = &self.goods.spoil_rates;
        let rate = |unit: u16| match units.held(unit) {
            Some(Held::Good(g)) => rates.get(usize::from(g.product)).copied().filter(|r| *r > 0.0),
            _ => None,
        };
        let mut lost = Vec::new();
        for holder in holders {
            phx_core::goods::spoil(
                &self.goods.stocks,
                holder,
                rate,
                (i64::from(period), day, phx_core::consts::DAYS_365),
                SPOILED,
                &mut lost,
            );
        }
        for f in lost {
            let _ = self.move_goods(f, Cost::Carried, day, moved);
        }
    }

    /// Each provider's day closed: the inputs its sales used, and the capacity no sale took lost.
    #[clause("SRV.1", "SRV.8", "GDS.4")]
    fn close_services(&mut self, ctx: &GoodsCtx<'_>, day: Day, moved: &mut Vec<Flow>) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return };
        let mut made_by: BTreeMap<u32, i64> = BTreeMap::new();
        for x in moved.iter().filter(|x| x.payer == NATURE && x.reason == MADE) {
            *made_by.entry(x.payee.word()).or_insert(0) += x.amount;
        }
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            if self.is_stored(f.product) {
                continue;
            }
            let made = made_by.get(&f.key.word()).copied().unwrap_or(0);
            let left = self.free_units(f.key, f.product, f.region);
            let sold = made - left;
            for (q, a) in self.stored_inputs(&f) {
                let used = whole_units(from_i64(sold) * a);
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
                let _ = self.move_goods(flow, Cost::Carried, day, moved);
            }
            if left > 0 {
                let unit = self.unit_of(f.product, f.region);
                let flow = Flow {
                    payer: f.key,
                    payee: NATURE,
                    amount: left,
                    source: slot.get(),
                    denomination: Denom::units(unit),
                    reason: PERISHED,
                    order: 0,
                };
                let _ = self.move_goods(flow, Cost::Carried, day, moved);
            }
        }
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

    /// Each product's least price a unit at each region among the sellers that can sell it: those holding it, and
    /// every provider of a service, which is made as it sells. A price no one can buy at is no cost.
    fn cheapest(&self, ctx: &GoodsCtx<'_>) -> BTreeMap<(u16, u32), f64> {
        let mut out: BTreeMap<(u16, u32), f64> = BTreeMap::new();
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return out };
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let offers = !self.is_stored(f.product) || self.free_units(f.key, f.product, f.region) > 0;
            if f.price > 0 && offers {
                let unit = from_i64(f.price) / self.lot(f.product);
                let e = out.entry((f.product, f.region)).or_insert(unit);
                *e = if unit < *e { unit } else { *e };
            }
        }
        out
    }

    /// A firm's orders of its stored inputs for its planned output, within the money it can spend on them.
    fn input_orders(&self, ctx: &GoodsCtx<'_>, (firm, slot): (usize, Slot), day: Day) -> Vec<(u16, Buyer)> {
        let m = ctx.management;
        let mut wants: Vec<(u16, Buyer)> = Vec::new();
        let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { return wants };
        let lead = self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0);
        let Some(expected) = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE) else {
            return wants;
        };
        let Some(unit_cost) = self.unit_cost(&f) else { return wants };
        let lot = self.lot(f.product);
        let margin = from_i64(f.price) / lot - unit_cost;
        let planned = self.planned(&f, expected);
        let financing = self.labour.financing.get(f.country).copied().unwrap_or(0.0) / DAYS_A_YEAR;
        // What it holds beyond what its contracts take before its next schedule is what it can spend on inputs.
        let Some(money) = self.kinds.get(firm).and_then(|k| k.accounts.as_ref()).and_then(|a| a.balance.get(slot))
        else {
            return wants;
        };
        let next = after(day, u64::from(self.labour.production_days));
        let free = from_i64(money - self.owed_until(f.key, (day, next), ctx.calendar));
        let mut orders: Vec<(u16, f64, f64, i64)> = Vec::new();
        for (q, a) in self.stored_inputs(&f).into_iter().filter(|(q, _)| *q != f.product) {
            let Some(per_unit) = self.goods.cheapest.get(&(q, f.region)).copied() else { continue };
            let input_lot = self.lot(q);
            let worth = per_unit + margin / a;
            let carried = sys_frm::rules::inputs::carried_cost(per_unit, financing, lead + m.cover_days);
            let held = from_i64(self.free_units(f.key, q, f.region));
            let use_a_day = if planned > 0.0 { a * planned } else { 0.0 };
            let short = sys_frm::rules::inputs::order(use_a_day, (lead, m.cover_days), held, worth, carried);
            let Some(limit) = floor_to_i64(worth * input_lot) else { continue };
            if short > 0.0 && limit > 0 {
                orders.push((q, short, per_unit, limit));
            }
        }
        // Its orders at the least prices posted, cut alike to what it can spend.
        let cost: f64 = orders.iter().map(|(_, units, price, _)| units * price).sum();
        let scale = if cost > free { free / cost } else { 1.0 };
        for (q, short, _, limit) in orders {
            let Some(units) = floor_to_i64((short * scale).floor()) else { continue };
            if units > 0 {
                let buyer = Buyer {
                    party: f.key,
                    subject: u64::from(f.key.word()),
                    want: Want::UpTo(units, limit),
                    place: f.region,
                };
                wants.push((q, buyer));
            }
        }
        wants
    }

    /// Each firm's stored inputs other than its own product, which it uses from its own stock, ordered up to what its
    /// planned output uses over the days a unit takes and its stock's cover, where a unit of the input is worth its
    /// least price in the region financed over those days — that price and the margin a unit made earns over its
    /// cost, over what the unit takes of it; bought from the firms of its region at no more than that worth, each
    /// purchase a flow of money and one of goods to the buyer.
    #[clause("FRM.7", "GDS.5", "MKT.6")]
    fn buy_inputs(&mut self, ctx: &GoodsCtx<'_>, day: Day, moved: &mut Vec<Flow>) -> u64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let slots = self.firm_slots(firm);
        let orders: Vec<Vec<(u16, Buyer)>> = match ctx.pool {
            Some(pool) => pool
                .map(slots.len(), |i| slots.get(i).map_or_else(Vec::new, |s| self.input_orders(ctx, (firm, *s), day))),
            None => slots.iter().map(|s| self.input_orders(ctx, (firm, *s), day)).collect(),
        };
        let wants: Vec<(u16, Buyer)> = orders.into_iter().flatten().collect();
        let n = len_u64(wants.len());
        let leg = |_: u16, unit: u16| GoodsLeg { unit: Denom::units(unit), reason: DELIVERED, order: 0, used: false };
        let _ = self.meet_all(ctx, day, (&wants, crate::core_stats::Purchase::Inputs), &leg, moved);
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
            if let Some(Missing::Present(region)) =
                self.kinds.get(place).and_then(|k| k.record(slot).get(region_at).map(|w| w.get()))
                && let Some(country) = usize::try_from(region).ok().and_then(|r| ctx.regions.get(r))
            {
                self.reconsider_household((ctx.streams, &ctx.rule.types), (place, slot, id), (country.get(), day));
            }
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
            // What its contracts will take before its next spending day is not its to spend.
            let party = PartyKey::new(kind_number(place), slot);
            let free = money - self.owed_until(party, (day, next), ctx.calendar);
            if free <= 0 {
                self.set_record_word(place, slot, after_at, money);
                continue;
            }
            let cash = from_i64(free) / income + 1.0;
            let wanted = sys_hh::buffer::spend(&ctx.rule.rule, cash) * income * ctx.rule.period;
            let held = from_i64(free);
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

    /// A firm's planned output a day: the sales it expects and, of a stored product, the gap between the stock it aims
    /// for and what it holds closed over its adjustment days; none below nothing.
    fn planned(&self, f: &Firm, expected: f64) -> f64 {
        let wanted = if self.is_stored(f.product) {
            let stock = from_i64(self.free_units(f.key, f.product, f.region));
            expected + (self.goods.cover * expected - stock) / self.goods.adjustment
        } else {
            expected
        };
        if wanted > 0.0 { wanted } else { 0.0 }
    }

    /// What a firm keeps of its own product for its own making, never offered: what its way uses of it over the days
    /// a unit takes and its stock's cover at the sales it expects.
    fn own_use(&self, ctx: &GoodsCtx<'_>, (firm, slot): (usize, Slot), f: &Firm) -> i64 {
        let Some(a) = self.recipe(f).iter().find(|(q, _)| *q == f.product).map(|(_, a)| *a) else { return 0 };
        let Some(expected) = self.record_word(firm, slot, EXPECTED).map(|e| from_i64(e) / PART_ONE) else { return 0 };
        let lead = self.goods.lead.get(usize::from(f.product)).copied().unwrap_or(0.0);
        whole_units(a * expected * (lead + ctx.management.cover_days))
    }

    /// The firms whose production schedule came today review their price: their stance reconsidered; their markup
    /// moved by their sales since their last review against those they expected and by what their stance expects
    /// their product's mark in their region to be; the sales a day they expect corrected toward those sales at their memory type's gain; the pressure of that demand and their
    /// stock against its target; the price they would like, their markup over their cost of making a unit raised by
    /// that pressure; and the move made only where it gains more than changing the price costs their staff's hours.
    #[clause("FRM.5", "FRM.14", "REP.34", "VAL.6", "VAL.7")]
    fn review_prices(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> (u64, u64) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return (0, 0) };
        let m = ctx.management;
        let due = std::mem::take(&mut self.labour.due_today);
        let (mut reviews, mut repriced) = (0, 0);
        let Some(stance_stream) = ctx.streams.named(sys_frm::StanceStream::DECL.name) else {
            violation!(clause = "VAL.7", "the firms' stance stream is not declared");
        };
        let mut stances = (0_u64, 0_u64);
        for s in due {
            let slot = Slot::new(s);
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let (Some(markup), Some(expected), Some(sold), Some(last), Some(memory), Some(switching), Some(stance)) = (
                self.record_word(firm, slot, MARKUP),
                self.record_word(firm, slot, EXPECTED),
                self.record_word(firm, slot, SOLD_UNITS),
                self.record_word(firm, slot, REVIEWED),
                self.record_word(firm, slot, MEMORY),
                self.record_word(firm, slot, SWITCHING),
                self.record_word(firm, slot, STANCE),
            ) else {
                continue;
            };
            let memory = usize::try_from(memory).unwrap_or(usize::MAX);
            let Some(gain) = m.types.gains.get(memory).copied() else {
                violation!(clause = "VAL.6", "a firm's memory type beyond the types", slot = s);
            };
            let Some(beta) = usize::try_from(switching).ok().and_then(|t| m.types.intensities.get(t)).copied() else {
                violation!(clause = "VAL.7", "a firm's switching type beyond the types", slot = s);
            };
            let days = i64::from(day.get()) - last;
            if days <= 0 {
                continue;
            }
            reviews += 1;
            self.goods.prices.entry(f.product).or_default().reviews += 1;
            self.set_record_word(firm, slot, REVIEWED, i64::from(day.get()));
            let (markup, expected) = (from_i64(markup) / PART_ONE, from_i64(expected) / PART_ONE);
            let demand = from_i64(sold) / from_i64(days);
            let price = from_i64(f.price);
            // Its stance reconsidered, then what it expects its product's next mark in its region to be by it, where
            // the mark has printed there.
            let Some(id) = self.kinds.get(firm).and_then(|k| k.parties.id(slot)) else { continue };
            let series = (f.product, f.region);
            let chosen =
                self.goods.outlooks.reconsider((series, memory, beta), (ctx.streams, &stance_stream), (id, day));
            stances.0 += 1;
            if i64::try_from(chosen).ok() != Some(stance) {
                stances.1 += 1;
                self.set_record_word(firm, slot, STANCE, i64::try_from(chosen).unwrap_or(i64::MAX));
            }
            let seen = self.goods.outlooks.outlook(series, memory, chosen);
            let markup = match sys_frm::rules::markup::update(
                markup,
                (m.sales_speed, m.seen_speed),
                (demand, expected),
                seen,
                price,
            ) {
                Missing::Present(x) => x,
                Missing::Absent => markup,
            };
            self.set_record_word(firm, slot, MARKUP, whole_units(markup * PART_ONE));
            let expected = phx_val::heuristics::adaptive(expected, demand, gain);
            self.set_record_word(firm, slot, EXPECTED, whole_units(expected * PART_ONE));
            self.set_record_word(firm, slot, SOLD_UNITS, 0);
            // A service is never held, so its provider's pressure is its demand's alone.
            let (stock, cover) = if self.is_stored(f.product) {
                (from_i64(self.free_units(f.key, f.product, f.region)), m.cover_days)
            } else {
                (0.0, 0.0)
            };
            let Some(unit_cost) = self.unit_cost(&f) else { continue };
            let Some(lot) = floor_to_i64(self.lot(f.product)) else { continue };
            let lot = from_i64(lot);
            let pressure = match sys_frm::rules::price::pressure_stocked(demand, expected, cover * expected, stock) {
                Missing::Present(p) => p,
                Missing::Absent => continue,
            };
            if unit_cost <= 0.0 {
                continue;
            }
            let wanted = sys_frm::rules::price::desired(markup, unit_cost * lot, pressure, m.curvature);
            let law = self.labour.laws.get(f.country);
            let hour = law.map_or(0.0, |l| l.mean_monthly / (l.weeks_a_month * f64::from(l.full_time_hours)));
            // What a gap costs it is read on the sales it expects, so a firm that sold nothing at a price it cannot
            // make at still weighs moving it.
            let revenue = expected * from_i64(f.price) / lot * m.production_days;
            let points = m.points_near(wanted);
            if let Some(p) =
                sys_frm::rules::price::reprice(&points, f.price, wanted, revenue, markup, m.menu_hours * hour)
            {
                self.set_record_word(firm, slot, PRICE, p);
                repriced += 1;
                let t = self.goods.prices.entry(f.product).or_default();
                (t.changes, t.size) = (t.changes + 1, t.size + (from_i64(p) / from_i64(f.price) - 1.0).abs());
            }
        }
        self.goods.outlooks.days.push(crate::core_outlooks::StanceDay {
            day: day.get(),
            reconsidered: stances.0,
            changed: stances.1,
            ..crate::core_outlooks::StanceDay::default()
        });
        (reviews, repriced)
    }

    /// The day's stances counted over every firm, after its reviews.
    fn count_stances(&mut self, day: Day) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return };
        let Some(store) = self.kinds.get(firm) else { return };
        let mut by = [0_u64; crate::core_outlooks::HEURISTICS];
        for slot in store.parties.live_slots() {
            if let Some(Missing::Present(h)) = store.record(slot).get(STANCE).map(|w| w.get())
                && let Some(n) = usize::try_from(h).ok().and_then(|h| by.get_mut(h))
            {
                *n += 1;
            }
        }
        if let Some(d) = self.goods.outlooks.days.last_mut().filter(|d| d.day == day.get()) {
            d.by_heuristic = by;
        }
    }

    /// Each wanted product's stalls: every maker holding it free beyond its own use, with a price, with its unit,
    /// currency and region.
    fn stalls(&mut self, ctx: &GoodsCtx<'_>, firm: usize, wants: &[(u16, Buyer)]) -> BTreeMap<u16, Vec<StallAt>> {
        let mut by_product: BTreeMap<u16, Vec<StallAt>> = BTreeMap::new();
        let wanted: std::collections::BTreeSet<u16> = wants.iter().map(|(p, _)| *p).collect();
        for slot in self.firm_slots(firm) {
            let product = self.record_word(firm, slot, PRODUCT).and_then(|p| u16::try_from(p).ok());
            if !product.is_some_and(|p| wanted.contains(&p)) {
                continue;
            }
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let free = self.free_units(f.key, f.product, f.region) - self.own_use(ctx, (firm, slot), &f);
            if free <= 0 || f.price <= 0 {
                continue;
            }
            let unit = self.unit_of(f.product, f.region);
            let Ok(ccy) = u8::try_from(f.country) else { continue };
            let stall = Stall { seller: f.key, price: f.price, units: free };
            by_product.entry(f.product).or_default().push((stall, unit, ccy, f.region));
        }
        by_product
    }

    /// Each product's posted-price meeting over its firms holding it free with a price, each region a place whose
    /// firms are in its reach at no distance until the finer cells: each sale's money a flow settled with the day's,
    /// and its goods moved at once from the seller at their price. Returns the sales and what they paid.
    #[clause("SRV.4", "SRV.5", "MKT.6", "GDS.4")]
    fn meet_all(
        &mut self,
        ctx: &GoodsCtx<'_>,
        day: Day,
        (wants, purpose): (&[(u16, Buyer)], crate::core_stats::Purchase),
        leg: &dyn Fn(u16, u16) -> GoodsLeg,
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
        let by_product = self.stalls(ctx, firm, wants);
        let (mut sales, mut spent) = (0, 0);
        let mut money = Vec::new();
        for (product, stalls) in by_product {
            let buyers: Vec<Buyer> = wants.iter().filter(|(p, _)| *p == product).map(|(_, b)| *b).collect();
            if buyers.is_empty() {
                continue;
            }
            // Each region's stalls, found in one pass over them.
            let mut places: Vec<Place> = (0..ctx.regions.len()).map(|_| Place { near: Vec::new() }).collect();
            for (i, x) in (0_u32..).zip(&stalls) {
                if let Some(place) = usize::try_from(x.3).ok().and_then(|r| places.get_mut(r)) {
                    place.near.push((i, 0.0));
                }
            }
            let plain: Vec<Stall> = stalls.iter().map(|x| x.0).collect();
            let Some(lot) = floor_to_i64(self.lot(product)) else { continue };
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
            meet(&mut meeting, ctx.pool, (&plain, &places, &buyers), (lot, ctx.weights), tastes, &lots);
            let made: Vec<Sale> = meeting.sales().copied().collect();
            self.goods.meeting = meeting;
            let by_seller: BTreeMap<PartyKey, usize> =
                stalls.iter().enumerate().map(|(i, x)| (x.0.seller, i)).collect();
            let mut out = Vec::new();
            // The meeting's sales by currency and the buyer's kind, recorded for the statistics at once.
            let mut recorded: BTreeMap<(u8, u8), (i64, i64)> = BTreeMap::new();
            for sale in made {
                let Some((_, unit, ccy, region)) = by_seller.get(&sale.seller).and_then(|i| stalls.get(*i)).copied()
                else {
                    continue;
                };
                out.clear();
                sale.flows((Denom::money(ccy), SOLD, 0), Some(leg(product, unit)), sale.seller.slot().get(), &mut out);
                if let (true, Some(Some(rate)), Some(included), Some(Some(treasury))) = (
                    purpose == crate::core_stats::Purchase::Final,
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
                for f in out.drain(..) {
                    if f.denomination.is_money() {
                        if f.payer == sale.buyer && f.payee == sale.seller {
                            self.goods.named.0 += i128::from(f.amount);
                            self.goods.named.1 += i128::from(f.amount);
                        } else {
                            self.goods.named.2 += 1;
                        }
                        money.push(f);
                    } else {
                        let _ = self.move_goods(f, Cost::At(sale.paid), day, moved);
                    }
                }
                if let Some(sold) = self.record_word(firm, sale.seller.slot(), SOLD_UNITS) {
                    self.set_record_word(firm, sale.seller.slot(), SOLD_UNITS, sold + sale.units);
                }
                let t = self.goods.traded.entry((product, region)).or_insert((0, 0));
                (t.0, t.1) = (t.0 + i128::from(sale.paid), t.1 + i128::from(sale.units));
                sales += 1;
                spent += sale.paid;
                let r = recorded.entry((ccy, sale.buyer.kind())).or_insert((0, 0));
                (r.0, r.1) = (r.0 + sale.paid, r.1 + sale.units);
            }
            for ((ccy, kind), (paid, units)) in recorded {
                self.record_sale(ccy, (kind, purpose), (product, paid, units));
            }
        }
        self.pending.append(&mut money);
        (sales, spent)
    }
}
