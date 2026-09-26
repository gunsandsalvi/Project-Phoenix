//! Goods at the world: what handlers ask of them applied — transformations of a row's goods at their apply point,
//! orders admitted into the day's book at 5d — the day's markets met at 6a, their matches made trades at 6d and
//! settled in stage 7 after the dues; and what a row's party may read of its goods, built for each run of rows a
//! handler is given.

use std::collections::BTreeMap;
use std::sync::Arc;

use phx_core::{FactDef, GoodsView, HeldGood, HeldRight, SubStep};
use phx_id::{Day, InstrumentId, MarketId, PartyId, Slot, ZoneId};
use phx_ledger::apply::ApplyAt;
use phx_ledger::goods::GoodKey;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_ledger::instrument::{InstrumentFamily, NewInstrument};
use phx_ledger::intents::Transform;
use phx_macros::clause;
use phx_market::intents::OrderIntent;
use phx_market::market::Form;
use phx_market::order::{Asked, Order, Poster, Side, Timing};
use phx_market::print::{Buyer, Match};
use phx_num::{Missing, Qty, UnitId, capacity_exceeded, violation};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use crate::world::World;

/// The products' list the goods are keyed by.
const PRODUCTS: &str = "TEC.products";

/// A product as goods are counted: its unit, and the least quantity it trades in, ten to its price's exponent, so a
/// quantity times a price is whole money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Traded {
    pub unit: UnitId,
    pub base: i64,
}

/// What the kernel reads to key goods, compiled at assembly: each product's unit, each deposit resource's product,
/// and each region's market zone.
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    pub products: Vec<Missing<Traded>>,
    pub resources: Vec<Missing<u16>>,
    pub market_zones: Vec<Missing<ZoneId>>,
}

impl Frame {
    /// The frame read from the register and the map.
    pub(crate) fn compile(register: &phx_core::Register, geo: &phx_geo::GeoState) -> Result<Frame, String> {
        let entries = register.products(PRODUCTS)?;
        let mut products = Vec::with_capacity(entries.len());
        let mut resources: Vec<Missing<u16>> = Vec::new();
        for (i, p) in (0_u16..).zip(entries) {
            let Missing::Present(unit) = register.units().named(&p.unit) else {
                return Err(format!(
                    "product `{}` is counted in `{}`, which the world does not declare",
                    p.name, p.unit
                ));
            };
            let Some(decl) = register.units().decl(unit) else {
                return Err(format!("unit `{}` has no declaration", p.unit));
            };
            let base = (0..decl.price_exp).try_fold(1_i64, |b, _| b.checked_mul(crate::consts::DECADE));
            let Some(base) = base else { return Err(format!("unit `{}`'s exponent beyond a quantity", p.unit)) };
            products.push(Missing::Present(Traded { unit, base }));
            if let Missing::Present(r) = p.extracts {
                let at = usize::from(r);
                if resources.len() <= at {
                    resources.resize(at + 1, Missing::Absent);
                }
                if let Some(slot) = resources.get_mut(at) {
                    if matches!(slot, Missing::Present(_)) {
                        return Err(format!("resource {r} is extracted as two products"));
                    }
                    *slot = Missing::Present(i);
                }
            }
        }
        Ok(Frame { products, resources, market_zones: geo.market_zones() })
    }

    fn traded(&self, product: u16) -> Traded {
        let Some(Missing::Present(t)) = self.products.get(usize::from(product)) else {
            violation!(clause = "GDS.1", "a good of a product the world does not declare", product = product);
        };
        *t
    }
}

/// The rows a handler's intents came from: a kind table's place among the books' holders, and whether its rows are
/// individuals; none for a kernel table, whose rows are no parties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rows {
    pub place: u16,
    pub individuals: bool,
}

/// A handler's intents as gathered, with the sub-step it ran at and the rows it ran on.
#[derive(Debug)]
pub(crate) struct Gathered {
    pub step: SubStep,
    pub rows: Missing<Rows>,
    pub intents: phx_core::Intents,
}

/// A row as goods read it: its party, the zone its goods stand in and its twins, one for an individual.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Row {
    pub party: PartyId,
    pub zone: ZoneId,
    pub twins: i64,
}

/// The day's markets between their meeting and their settlement: the orders admitted, the matches each meeting
/// made, and the trades those became.
#[derive(Debug, Default)]
pub(crate) struct MarketDay {
    pub orders: Vec<Order>,
    pub matches: Vec<(MarketId, InstrumentId, Match)>,
    pub trades: Vec<(Instruction, PartyId, InstrumentId, i64)>,
    pub refused: u64,
    pub failed: u64,
}

/// Each good's latest mark where it stands, as the markets' marks give it, rebuilt after the day's marks.
pub(crate) type Marks = Arc<BTreeMap<GoodKey, i64>>;

/// A good a row holds or has delivered: its product, its grade class and its units, one twin's.
type Units = HeldGood;

/// What a run of rows may read of its goods, read before its handler runs.
#[derive(Debug, Default)]
pub(crate) struct RunGoods {
    start: u32,
    rows: Vec<RowGoods>,
    marks: Marks,
    outlooks: Marks,
}

#[derive(Debug)]
struct RowGoods {
    zone: Missing<ZoneId>,
    held: Vec<Units>,
    rights: Vec<HeldRight>,
    delivered: Vec<Units>,
}

impl RowGoods {
    /// A row that holds, has delivered and stands nowhere yet.
    fn none() -> RowGoods {
        RowGoods { zone: Missing::Absent, held: Vec::new(), rights: Vec::new(), delivered: Vec::new() }
    }
}

impl RunGoods {
    /// A good's price in a map, at the row's place.
    fn at_place(&self, map: &Marks, slot: Slot, product: u16, grade: u8) -> Missing<i64> {
        let Some(Missing::Present(zone)) = self.row(slot).map(|r| r.zone) else { return Missing::Absent };
        map.get(&GoodKey { product, grade, zone }).copied().map_or(Missing::Absent, Missing::Present)
    }

    fn row(&self, slot: Slot) -> Option<&RowGoods> {
        slot.get().checked_sub(self.start).and_then(|i| self.rows.get(usize::try_from(i).ok()?))
    }
}

fn units_of(list: &[Units], product: u16, grade: u8) -> i64 {
    list.iter().filter(|(p, g, _)| *p == product && *g == grade).map(|(_, _, q)| q).sum()
}

impl GoodsView for RunGoods {
    fn held(&self, slot: Slot, product: u16, grade: u8) -> i64 {
        self.row(slot).map_or(0, |r| units_of(&r.held, product, grade))
    }
    fn goods(&self, slot: Slot) -> &[HeldGood] {
        self.row(slot).map_or(&[], |r| r.held.as_slice())
    }
    fn rights(&self, slot: Slot) -> &[HeldRight] {
        self.row(slot).map_or(&[], |r| r.rights.as_slice())
    }
    fn delivered(&self, slot: Slot, product: u16, grade: u8) -> i64 {
        self.row(slot).map_or(0, |r| units_of(&r.delivered, product, grade))
    }
    fn mark(&self, slot: Slot, product: u16, grade: u8) -> Missing<i64> {
        self.at_place(&self.marks, slot, product, grade)
    }
    fn outlook(&self, slot: Slot, product: u16, grade: u8) -> Missing<i64> {
        self.at_place(&self.outlooks, slot, product, grade)
    }
}

/// A count for a quantity, as twins multiply it.
fn times(q: i64, twins: i64) -> i64 {
    let Some(t) = q.checked_mul(twins) else {
        capacity_exceeded!("a quantity for every twin", i64::MAX, q);
    };
    t
}

impl World {
    /// A row's party, the zone its goods stand in and its twins; none when the row is no longer live.
    pub(crate) fn goods_row(&self, rows: Rows, slot: Slot) -> Option<Row> {
        let geo = self.geo();
        if rows.individuals {
            let t = self.books.parties.table(rows.place);
            if !t.is_live(slot) {
                return None;
            }
            let Missing::Present(zone) = geo.zone_of(t.site(slot)) else {
                violation!(clause = "GDS.2", "a party sited where no zone is", party = t.party(slot).get());
            };
            return Some(Row { party: t.party(slot), zone, twins: 1 });
        }
        let first = self.books.parties.first_cell_place();
        let k = usize::from(rows.place.checked_sub(first)?);
        let t = Population::table::<SystemBacking>(self.books.parties.cells(), k);
        if !t.is_live(slot) {
            return None;
        }
        let kd = self.population.kinds.get(k)?;
        let Missing::Present(at) = kd.decl.sited_by else {
            violation!(
                clause = "REP.24",
                "goods held by an agent of a kind that is not sited",
                party = t.party(slot).get()
            );
        };
        let region = t.attr(slot, at);
        let zone = usize::try_from(region).ok().and_then(|r| self.goods_frame.market_zones.get(r)).copied();
        let Some(Missing::Present(zone)) = zone else {
            violation!(clause = "REP.24", "an agent in a region with no zone", region = region);
        };
        Some(Row { party: t.party(slot), zone, twins: i64::from(t.multiplicity(slot).get()) })
    }

    /// A good's instrument, issued the first time something names it: a real asset in its product's unit, priced in
    /// its zone's country's currency.
    #[clause("GDS.1", "GDS.2")]
    fn good(&mut self, key: GoodKey) -> InstrumentId {
        if let Missing::Present(id) = self.books.ledger.goods.of(key) {
            return id;
        }
        let unit = self.goods_frame.traded(key.product).unit;
        let Missing::Present(country) = self.geo().zone_country(key.zone) else {
            violation!(clause = "GDS.1", "a good at a zone of no country", zone = key.zone.get());
        };
        let terms = self.goods_terms(country);
        let ccy = phx_ledger::opening::currency(country);
        let new = NewInstrument { family: InstrumentFamily::RealAsset, issuer: Missing::Absent, unit, ccy, terms };
        let ledger = &mut self.books.ledger;
        ledger.goods.issue(&mut ledger.instruments, key, new)
    }

    /// The terms a country's goods are issued under: an account in its currency, as plant's.
    fn goods_terms(&mut self, country: phx_id::CountryId) -> phx_ledger::terms::TermsId {
        use phx_core::calendar::bizday::BusinessDayConvention;
        use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
        let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
        let dates = ScheduleDates {
            anchor: self.calendar.date(self.day_zero),
            period: months,
            eom: EndOfMonth::Plain,
            convention: BusinessDayConvention::Following,
            country,
        };
        let ccy = phx_ledger::opening::currency(country);
        self.books.ledger.terms.intern(phx_ledger::algebra::Terms::account(ccy, dates))
    }

    /// A transformation a handler asked for its row, applied by the one apply routine: its legs at the row's zone, or
    /// at the deposit's for units taken from one, each a twin's times the row's twins. Units taken from a deposit need
    /// the right to it and a product the deposit gives, and deplete it by GEO's own write. A row no longer live asks
    /// nothing; one whose units fall short fails, counted.
    #[clause("SET.9", "GDS.2", "GDS.4", "GDS.12", "GEO.9", "GEO.12")]
    pub(crate) fn apply_transform(&mut self, day: Day, step: SubStep, rows: Rows, t: &Transform) {
        let Some(row) = self.goods_row(rows, t.row) else { return };
        let Missing::Present(reason) = self.books.ledger.reasons.coded(t.reason) else {
            violation!(clause = "SET.1", "a transformation of a reason never declared", party = row.party.get());
        };
        let mut legs = Vec::with_capacity(t.legs.len());
        let mut taken: Vec<(Slot, i64)> = Vec::new();
        for leg in &t.legs {
            let zone = match leg.source {
                Source::Deposit(d) => {
                    let (zone, depleted) = self.extraction(row, d, leg.product, leg.qty);
                    if let Missing::Present(r) = depleted {
                        taken.push((r, times(leg.qty, row.twins)));
                    }
                    zone
                }
                _ => row.zone,
            };
            let good = self.good(GoodKey { product: leg.product, grade: leg.grade, zone });
            let unit = self.goods_frame.traded(leg.product).unit;
            legs.push(LegRec {
                party: row.party,
                account: AccountRef::Instrument(good),
                qty: times(leg.qty, row.twins),
                denom: Denom::Unit(unit),
                kind: LegKind::Transformation { source: leg.source, cost: times(leg.cost, row.twins) },
            });
        }
        let instruction = Instruction {
            id: self.books.ledger.next_id(day),
            reason,
            trade_day: day,
            settle_day: day,
            legs,
            pays: Missing::Absent,
            covers: Vec::new(),
        };
        if self.books.apply(ApplyAt::Day(step), instruction, self.audit.stream()).is_err() {
            self.market_day.failed += 1;
            return;
        }
        let deposits = self.tables.iter_mut().find(|k| k.name == phx_geo::audit::DEPOSIT_TABLE);
        let Some(table) = deposits else {
            violation!(clause = "GEO.12", "units taken from a deposit the world keeps no table of");
        };
        for (r, q) in taken {
            if phx_geo::deposits::extract(&mut table.columns, r, q).is_err() {
                violation!(clause = "GEO.12", "a deposit depleted beyond what it holds", row = r.get(), units = q);
            }
        }
    }

    /// A leg taking units from a deposit checked: made, not used up, of the product the deposit gives, by a party
    /// holding its right, and no more than it holds. Its zone, and its row in GEO's table when it is finite.
    fn extraction(&self, row: Row, deposit: u32, product: u16, qty: i64) -> (ZoneId, Missing<Slot>) {
        let geo = self.geo();
        let Some(d) = usize::try_from(deposit).ok().and_then(|i| geo.deposits.get(i)) else {
            violation!(clause = "GDS.12", "units taken where there is no deposit", deposit = deposit);
        };
        let gives = self.goods_frame.resources.get(usize::from(d.resource)).copied();
        if qty <= 0 || gives != Some(Missing::Present(product)) {
            violation!(clause = "GDS.12", "a deposit giving other than its resource", deposit = deposit);
        }
        if row.twins != 1 {
            violation!(clause = "GDS.3", "a deposit's right held by an agent, which has no whole unit for each twin");
        }
        let Missing::Present(right) = self.books.ledger.goods.right(deposit) else {
            violation!(clause = "GDS.3", "units taken from a deposit no right is over", deposit = deposit);
        };
        let (place, slot) = self.books.parties.row(row.party);
        let held = phx_ledger::holding::holding(self.books.parties.holder(place), slot, right);
        if !matches!(held, Missing::Present(h) if h.quantity.raw() > 0) {
            violation!(clause = "GDS.3", "units taken from a deposit without its right", party = row.party.get());
        }
        let Missing::Present(zone) = geo.zone_of(d.tile) else {
            violation!(clause = "GEO.6", "a deposit on a tile of no zone", deposit = deposit);
        };
        (zone, phx_geo::deposits::row_of(&geo.deposits, deposit))
    }

    /// An order a handler asked for its row, admitted into the day's book: in its kind's instance over the good at
    /// the row's zone, its steps a twin's times the row's twins, traded in lots of the product's least quantity for
    /// each twin; an offer covered by the row's free units of the good. One refused is counted and never meets.
    #[clause("MKT.16", "MKT.17", "GDS.7", "REP.9")]
    pub(crate) fn admit_order(&mut self, day: Day, step: SubStep, rows: Rows, o: &OrderIntent) {
        if step.ordinal() > SubStep::S5d.ordinal() {
            violation!(clause = "TIME.6", "an order asked after the day's orders were admitted", step = step.ordinal());
        }
        let Some(row) = self.goods_row(rows, o.row) else { return };
        let Missing::Present(kind) = self.market_kinds.kind(o.kind) else {
            violation!(clause = "MKT.1", "an order in a market kind never declared", party = row.party.get());
        };
        let traded = self.goods_frame.traded(o.product);
        let key = GoodKey { product: o.product, grade: o.grade, zone: row.zone };
        let market = self.market_kinds.instance(&mut self.markets.made, kind, key.code());
        let decl = self.market_kinds.kind_decl(kind);
        let poster = Poster {
            party: row.party,
            market,
            side: o.side,
            timing: Timing::AtTheClose,
            day,
            reason: decl.key.kind,
            priority: Missing::Absent,
            lot: times(traded.base, row.twins),
        };
        let tick = decl.tick;
        let asked: Vec<Asked> =
            o.steps.iter().map(|s| Asked { limit: Missing::Present(s.limit), qty: times(s.qty, row.twins) }).collect();
        let order = match o.side {
            Side::Buy => Order::new(poster, &asked, tick).ok(),
            Side::Sell => self.covered_offer(poster, &asked, tick, key),
        };
        match order {
            Some(order) => self.market_day.orders.push(order),
            None => self.market_day.refused += 1,
        }
    }

    /// An offer of held units covered by the ledger's commitment on the poster's free units of the good; none when
    /// they cannot cover it or its steps break a rule, the cover then released.
    fn covered_offer(&mut self, poster: Poster, asked: &[Asked], tick: i64, key: GoodKey) -> Option<Order> {
        let Missing::Present(good) = self.books.ledger.goods.of(key) else { return None };
        let traded_unit = self.goods_frame.traded(key.product).unit;
        let offered: i64 = asked.iter().map(|a| a.qty).sum();
        let (place, slot) = self.books.parties.row(poster.party);
        let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, good) {
            Missing::Present(h) => h.quantity.raw(),
            Missing::Absent => 0,
        };
        let pledged = self.books.ledger.liens.pledged(poster.party, good);
        let cover =
            self.books.ledger.covers.cover(poster.party, good, Qty::new(offered, traded_unit), held, pledged).ok()?;
        match Order::offer_held(poster, asked, tick, cover) {
            Ok(order) => Some(order),
            Err((_, cover)) => {
                self.books.ledger.covers.release(cover);
                None
            }
        }
    }

    /// 6a: every market with orders today met by its kind's form, in the order of the markets: a call's print or
    /// failure recorded, a posted meeting's sales; the matches kept for 6d, and every order's cover released, each
    /// trade covering again what it delivers.
    #[clause("MKT.2", "MKT.3", "MKT.6", "GDS.7")]
    pub(crate) fn markets_meet(&mut self, day: Day) {
        let mut by_market: BTreeMap<MarketId, Vec<Order>> = BTreeMap::new();
        for o in std::mem::take(&mut self.market_day.orders) {
            by_market.entry(o.market).or_default().push(o);
        }
        for (market, mut orders) in by_market {
            let decl = self.market_kinds.decl(&self.markets.made, market);
            let key = GoodKey::from_code(decl.key.subject);
            let good = self.good(key);
            let instrument = self.books.ledger.instruments.get(good);
            let Some(stream) = self.streams.named(decl.stream) else {
                violation!(clause = "CHN.1", "a market drawing from a stream never declared", market = market.get());
            };
            let subject = Subject::new(SubjectTag::Market, u64::from(market.get()));
            let mut lot = self.streams.open(&stream, subject, day, SubStep::S6a.ordinal());
            let quote = (instrument.unit, instrument.ccy);
            let matches: Vec<Match> = match decl.form {
                Form::Call => {
                    let last = match self.markets.tape.last_print(market) {
                        Missing::Present(p) => Missing::Present(p.price()),
                        Missing::Absent => Missing::Absent,
                    };
                    let rules = phx_market::call::CallRules { ties: decl.ties, ration: decl.ration, last };
                    let outcome = phx_market::call::call(&orders, rules, &mut lot);
                    let _ = self.markets.record_call(&decl, day, quote, &orders, &outcome);
                    match outcome {
                        phx_market::call::Outcome::Cleared(c) => c.matches,
                        phx_market::call::Outcome::Failed(_) => Vec::new(),
                    }
                }
                Form::Posted => {
                    let outcome = phx_market::posted::between(&orders, &mut lot);
                    let postings: Vec<phx_market::posted::Posting> = orders
                        .iter()
                        .filter(|o| o.side == Side::Sell)
                        .flat_map(|o| {
                            o.steps.iter().map(|s| phx_market::posted::Posting {
                                seller: o.party,
                                price: s.limit,
                                capacity: s.qty,
                            })
                        })
                        .collect();
                    self.markets.record_posted(market, day, quote, &postings, &outcome);
                    outcome.sales
                }
                _ => violation!(clause = "GDS.7", "goods meeting in a form their kind cannot", market = market.get()),
            };
            for o in &mut orders {
                if let Missing::Present(cover) = std::mem::replace(&mut o.cover, Missing::Absent) {
                    self.books.ledger.covers.release(cover);
                }
            }
            self.market_day.matches.extend(matches.into_iter().map(|m| (market, good, m)));
        }
    }

    /// 6d: every match made a numbered trade, due today: the seller's units against the buyer's money at the
    /// market's price, a quantity of whole lots so the money is whole, the units covered for it until it settles.
    #[clause("SET.1", "SET.2", "SET.4", "REG.10")]
    pub(crate) fn markets_trade(&mut self, day: Day) {
        for (market, good, m) in std::mem::take(&mut self.market_day.matches) {
            let Buyer::Party(buyer) = m.buyer else {
                violation!(clause = "GDS.7", "goods between firms bought by a group", market = market.get());
            };
            let Missing::Present(key) = self.books.ledger.goods.key(good) else {
                violation!(clause = "GDS.1", "a market over no good", market = market.get());
            };
            let base = self.goods_frame.traded(key.product).base;
            if m.qty % base != 0 {
                violation!(clause = "REP.9", "a match of less than whole lots", market = market.get());
            }
            let Some(amount) = (m.qty / base).checked_mul(m.price.raw()) else {
                capacity_exceeded!("a trade's money", i64::MAX, m.qty);
            };
            let ccy = self.books.ledger.instruments.get(good).ccy;
            let (place, slot) = self.books.parties.row(m.seller);
            let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, good) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            };
            let pledged = self.books.ledger.liens.pledged(m.seller, good);
            let covers = match self.books.ledger.covers.cover(
                m.seller,
                good,
                Qty::new(m.qty, self.books.ledger.instruments.get(good).unit),
                held,
                pledged,
            ) {
                Ok(c) => vec![c],
                Err(_) => violation!(
                    clause = "REG.10",
                    "a match of units its seller's cover held no more",
                    party = m.seller.get()
                ),
            };
            let instruction = Instruction {
                id: self.books.ledger.next_id(day),
                reason: self.books.dues.traded,
                trade_day: day,
                settle_day: day,
                legs: self.books.trade_legs((m.seller, buyer), (good, m.qty), (amount, ccy)),
                pays: Missing::Absent,
                covers,
            };
            self.market_day.trades.push((instruction, m.seller, good, m.qty));
        }
    }

    /// Stage 7, after the dues: the day's trades settled in declared order, each all or none; a buyer who cannot pay
    /// fails with its cause and the units stay with the seller. What each seller delivers is kept as its sales.
    #[clause("SET.3", "SET.4", "SET.6", "FRM.13")]
    pub(crate) fn markets_settle(&mut self, step: SubStep) {
        let trades = std::mem::take(&mut self.market_day.trades);
        if trades.is_empty() {
            return;
        }
        let mut sold: BTreeMap<phx_ledger::instruction::InstructionId, (PartyId, InstrumentId, i64)> = BTreeMap::new();
        let mut list = Vec::with_capacity(trades.len());
        for (i, seller, good, qty) in trades {
            sold.insert(i.id, (seller, good, qty));
            list.push(i);
        }
        let books = &mut self.books;
        let results = books.ledger.settle(&mut books.parties, ApplyAt::Day(step), list, self.audit.stream());
        for r in results {
            match r {
                Ok(id) => {
                    if let Some((seller, good, qty)) = sold.get(&id) {
                        self.books.ledger.goods.deliver(*seller, *good, *qty);
                    }
                }
                Err(_) => self.market_day.failed += 1,
            }
        }
    }

    /// Each good's mark where it stands, from the markets' marks, for the handlers' reads.
    pub(crate) fn goods_marks(&mut self) {
        let mut marks = BTreeMap::new();
        for (market, mark) in self.markets.tape.marks() {
            let Missing::Present(subject) = self.markets.made.subject_of(*market) else { continue };
            marks.insert(GoodKey::from_code(subject), mark.price.raw());
        }
        self.marks = Arc::new(marks);
    }

    /// What a run of rows may read of its goods: for each row, the goods it holds at its zone and the rights it
    /// holds, one twin's for an agent, and what it has delivered.
    pub(crate) fn run_goods(&self, rows: Rows, run: core::ops::Range<u32>) -> RunGoods {
        let mut out = RunGoods {
            start: run.start,
            rows: Vec::new(),
            marks: Arc::clone(&self.marks),
            outlooks: Arc::clone(&self.outlooks),
        };
        let ledger = &self.books.ledger;
        for s in run {
            let slot = Slot::new(s);
            let Some(row) = self.goods_row(rows, slot) else {
                out.rows.push(RowGoods::none());
                continue;
            };
            let mut goods = RowGoods { zone: Missing::Present(row.zone), ..RowGoods::none() };
            let (place, at) = self.books.parties.row(row.party);
            let arenas = self.books.parties.holder(place);
            for (instrument, _) in phx_ledger::holding::bases(arenas, at) {
                let Missing::Present(h) = phx_ledger::holding::holding(arenas, at, instrument) else { continue };
                let units = h.quantity.raw() / row.twins;
                if let Missing::Present(key) = ledger.goods.key(instrument)
                    && key.zone == row.zone
                {
                    goods.held.push((key.product, key.grade, units));
                }
                if let Missing::Present(deposit) = ledger.goods.deposit(instrument)
                    && units > 0
                {
                    goods.rights.push(self.held_right(deposit));
                }
            }
            for (good, q) in ledger.goods.delivered(row.party) {
                if let Missing::Present(key) = ledger.goods.key(good) {
                    goods.delivered.push((key.product, key.grade, q / row.twins));
                }
            }
            out.rows.push(goods);
        }
        out
    }

    /// A deposit as its right's holder sees it.
    fn held_right(&self, deposit: u32) -> HeldRight {
        let geo = self.geo();
        let Some(d) = usize::try_from(deposit).ok().and_then(|i| geo.deposits.get(i)) else {
            violation!(clause = "GDS.3", "a right over no deposit", deposit = deposit);
        };
        let Some(Missing::Present(product)) = self.goods_frame.resources.get(usize::from(d.resource)).copied() else {
            violation!(clause = "GDS.3", "a deposit of a resource no product is extracted as", deposit = deposit);
        };
        let (opening, remaining) = match phx_geo::deposits::row_of(&geo.deposits, deposit) {
            Missing::Present(r) => {
                let t = self.tables.iter().find(|k| k.name == phx_geo::audit::DEPOSIT_TABLE);
                let read = |fact: &str| t.map_or(Missing::Absent, |t| t.columns.value(fact, r));
                (read(phx_geo::audit::facts::Opening::ITEM.name), read(phx_geo::audit::facts::Remaining::ITEM.name))
            }
            Missing::Absent => (Missing::Absent, Missing::Absent),
        };
        HeldRight { deposit, product, grade: d.grade.raw(), opening, remaining }
    }
}
