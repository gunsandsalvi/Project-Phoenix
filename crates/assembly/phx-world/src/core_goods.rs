//! Goods on the core, a meeting a day. Each firm makes what its planned output asks a day, within what its staff's
//! hours make at its hours a unit, into its stock. Each household decides on its spending schedule, by the
//! buffer-stock rule at its cash on hand, what it spends and asks each product's share of it at retail. Each
//! product's retail meeting matches the day's buyers to its firms in their region at their posted prices, and each
//! sale is a flow from the household to the firm, its goods taken from the firm's stock and used as they are bought.

use phx_core::calendar::Calendar;
use phx_core::flows::{Denom, Flow};
use phx_core::wheel::DueWheel;
use phx_core::{Register, StreamDef, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_market::meet::{Buyer, Meeting, Place, Stall, Tastes, meet};
use phx_market::retail::{Want, Weights};
use phx_num::{MaybeI64, Missing, violation};
use phx_rand::float::{floor_to_i64, from_i64, len_u64};
use phx_rand::{Subject, SubjectTag};

use crate::consts::firm::{OUTPUT, PRICE, PRODUCT, REGION, STOCK};
use crate::consts::{CORE_WHEEL_DAYS, DAYS_A_WEEK, DAYS_A_YEAR};
use crate::core::{Core, kind_number};

/// What a day's goods did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoodsDay {
    pub day: u32,
    pub made: i64,
    pub spenders: u64,
    pub wants: u64,
    pub sales: u64,
    pub spent: i64,
}

/// Goods' state on the core, kept from day to day.
#[derive(Debug, Default)]
pub struct CoreGoods {
    pub spenders: Option<DueWheel>,
    pub spend_days: u32,
    /// The day the core's production began, from which each firm's day's making is counted in whole units.
    pub began: u32,
    /// Today's wants: each household's money asked of a product, at its region.
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
    pub regions: &'a [CountryId],
    pub weights: Weights,
}

impl std::fmt::Debug for GoodsCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoodsCtx").field("regions", &self.regions.len()).finish_non_exhaustive()
    }
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
    i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month())
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
            *w = MaybeI64::present(v);
        }
    }

    /// Goods opened on the core: every household's spending schedule begun at a phase drawn for it.
    ///
    /// # Errors
    /// A spending schedule of no days.
    #[clause("HH.4")]
    pub fn open_goods(&mut self, ctx: &GoodsCtx<'_>, today: Day) -> Result<(), String> {
        let Some(place) = self.names.iter().position(|n| *n == "household") else { return Ok(()) };
        let days = floor_to_i64((ctx.rule.period * DAYS_A_YEAR).round()).and_then(|d| u32::try_from(d).ok());
        let Some(days) = days.filter(|d| *d > 0) else { return Err("a spending schedule of no days".to_owned()) };
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
        self.goods = CoreGoods { spenders: Some(wheel), spend_days: days, began: today.get(), ..CoreGoods::default() };
        Ok(())
    }

    /// The day's goods: firms make, households due decide, and each product's meeting sells.
    #[clause("GDS.4", "HH.4", "SRV.4", "MKT.6")]
    pub fn goods_day(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> GoodsDay {
        let mut record = GoodsDay { day: day.get(), ..GoodsDay::default() };
        if self.goods.spenders.is_none() {
            return record;
        }
        record.made = self.make(ctx, day);
        let (spenders, wants) = self.decide_spending(ctx, day);
        (record.spenders, record.wants) = (spenders, wants);
        let (sales, spent) = self.sell(ctx, day);
        (record.sales, record.spent) = (sales, spent);
        self.goods.days.push(record);
        record
    }

    /// Each firm's making today: its planned output's day, in whole units counted from the core's first day, within
    /// what its staff's hours make at its hours a unit.
    #[clause("FRM.4", "GDS.4")]
    fn make(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> i64 {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return 0 };
        let n = i64::from(day.get()) - i64::from(self.goods.began);
        let slots: Vec<Slot> = self.kinds.get(firm).map(|k| k.parties.live_slots().collect()).unwrap_or_default();
        let mut made = 0;
        for slot in slots {
            let (Some(output), Some(stock), Some(region), Some(product)) = (
                self.record_word(firm, slot, OUTPUT),
                self.record_word(firm, slot, STOCK),
                self.record_word(firm, slot, REGION),
                self.record_word(firm, slot, PRODUCT),
            ) else {
                continue;
            };
            let upto = |k: i64| floor_to_i64((from_i64(output) * from_i64(k) / DAYS_A_YEAR).floor()).unwrap_or(0);
            let planned = upto(n + 1) - upto(n);
            let capacity = self.staff_capacity(ctx, PartyKey::new(kind_number(firm), slot), (region, product));
            let today = match capacity {
                Some(c) if c < planned => c,
                _ => planned,
            };
            self.set_record_word(firm, slot, STOCK, stock + today);
            made += today;
        }
        made
    }

    /// The whole units a firm's staff's hours a day make at its hours a unit; none known where it has no hours a unit.
    fn staff_capacity(&self, ctx: &GoodsCtx<'_>, firm: PartyKey, (region, product): (i64, i64)) -> Option<i64> {
        let family = self.families.iter().position(|f| f.name == "LAB.employment")?;
        let c = usize::from(ctx.regions.get(usize::try_from(region).ok()?)?.get());
        let (level, ways) = (self.labour.level.get(c)?, self.labour.ways.get(c)?);
        let productivity =
            from_i64(self.record_word(usize::from(firm.kind()), firm.slot(), crate::consts::firm::PRODUCTIVITY)?)
                / crate::consts::firm::PRODUCTIVITY_ONE;
        let p = usize::try_from(product).ok()?;
        let a_unit: f64 = level
            .iter()
            .zip(ways)
            .map(|(l, occ)| l * sys_frm::rules::way::own_hours(occ.get(p).copied().unwrap_or(0.0), productivity))
            .sum();
        if a_unit <= 0.0 {
            return None;
        }
        let f = self.families.get(family)?;
        let hours: f64 = f
            .store
            .of(0, firm.slot())
            .filter_map(|e| {
                let row = f.store.edges.row(e)?;
                f.classes.get(usize::try_from(row.schedule).ok()?).map(|k| f64::from(k[1]) / DAYS_A_WEEK)
            })
            .sum();
        floor_to_i64((hours / a_unit).floor())
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
                let seen = from_i64(received) * crate::consts::MONTHS_A_YEAR / from_i64(month - looked);
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

    /// Each product's retail meeting over its firms with stock and a price, each region a place whose firms are all in
    /// its reach; each sale a flow from the household to the firm, its units taken from the firm's stock.
    #[clause("SRV.4", "SRV.5", "MKT.6", "GDS.4")]
    fn sell(&mut self, ctx: &GoodsCtx<'_>, day: Day) -> (u64, i64) {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return (0, 0) };
        let wants = std::mem::take(&mut self.goods.wants);
        if wants.is_empty() {
            return (0, 0);
        }
        let (Some(taste), Some(lot_stream)) =
            (ctx.streams.named(sys_srv::TasteStream::DECL.name), ctx.streams.named(sys_srv::LotStream::DECL.name))
        else {
            return (0, 0);
        };
        let key = ctx.streams.key(&taste);
        let mut by_product: std::collections::BTreeMap<u16, Vec<Stall>> = std::collections::BTreeMap::new();
        let mut region_of: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        let slots: Vec<Slot> = self.kinds.get(firm).map(|k| k.parties.live_slots().collect()).unwrap_or_default();
        for slot in slots {
            let (Some(product), Some(price), Some(stock), Some(region)) = (
                self.record_word(firm, slot, PRODUCT),
                self.record_word(firm, slot, PRICE),
                self.record_word(firm, slot, STOCK),
                self.record_word(firm, slot, REGION),
            ) else {
                continue;
            };
            let (Ok(product), Ok(region)) = (u16::try_from(product), u32::try_from(region)) else { continue };
            if stock <= 0 || price <= 0 {
                continue;
            }
            let seller = PartyKey::new(kind_number(firm), slot);
            region_of.insert(seller.word(), region);
            by_product.entry(product).or_default().push(Stall { seller, price, units: stock });
        }
        let regions = len_u64(ctx.regions.len());
        let (mut sales, mut spent) = (0, 0);
        let mut flows = Vec::new();
        for (product, stalls) in by_product {
            let buyers: Vec<Buyer> = wants.iter().filter(|(p, _)| *p == product).map(|(_, b)| *b).collect();
            if buyers.is_empty() {
                continue;
            }
            let places: Vec<Place> = (0..regions)
                .map(|r| Place {
                    near: (0_u32..)
                        .zip(&stalls)
                        .filter(|(_, s)| region_of.get(&s.seller.word()).is_some_and(|x| u64::from(*x) == r))
                        .map(|(i, _)| (i, 0.0))
                        .collect(),
                })
                .collect();
            let Some(lot) = floor_to_i64(sys_frm::FilingPrims::lot(ctx.register, product)) else { continue };
            let tastes = Tastes { key, day: day.get(), substep: SubStep::S5c.ordinal() };
            let lots = |seller: PartyKey, round: u32| {
                ctx.streams.open(
                    &lot_stream,
                    Subject::new(SubjectTag::Party, (u64::from(seller.word()) << u32::BITS) | u64::from(round)),
                    day,
                    SubStep::S5c.ordinal(),
                )
            };
            let mut meeting = std::mem::take(&mut self.goods.meeting);
            meet(&mut meeting, None, (&stalls, &places, &buyers), (lot, ctx.weights), tastes, &lots);
            for sale in meeting.sales() {
                let Some(ccy) = region_of
                    .get(&sale.seller.word())
                    .and_then(|r| ctx.regions.get(usize::try_from(*r).ok()?))
                    .map(|c| c.get())
                else {
                    continue;
                };
                flows.push(Flow {
                    payer: sale.buyer,
                    payee: sale.seller,
                    amount: sale.paid,
                    source: sale.seller.slot().get(),
                    denomination: Denom::money(ccy),
                    reason: crate::consts::reason::SOLD,
                    order: 0,
                });
                if let Some(stock) = self.record_word(firm, sale.seller.slot(), STOCK) {
                    self.set_record_word(firm, sale.seller.slot(), STOCK, stock - sale.units);
                }
                sales += 1;
                spent += sale.paid;
            }
            self.goods.meeting = meeting;
        }
        self.pending.append(&mut flows);
        (sales, spent)
    }
}
