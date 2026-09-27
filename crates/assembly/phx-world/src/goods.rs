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

/// What the kernel reads to key goods, compiled at assembly: each product's unit and whether it is delivered as it is
/// made, each deposit resource's product, and each region's market zone.
#[derive(Clone, Debug)]
pub(crate) struct Frame {
    pub products: Vec<Missing<Traded>>,
    pub at_once: Vec<bool>,
    pub stored: Vec<bool>,
    pub resources: Vec<Missing<u16>>,
    pub market_zones: Vec<Missing<ZoneId>>,
}

impl Frame {
    /// The frame read from the register and the map.
    pub(crate) fn compile(register: &phx_core::Register, geo: &phx_geo::GeoState) -> Result<Frame, String> {
        let entries = register.products(PRODUCTS)?;
        let mut products = Vec::with_capacity(entries.len());
        let mut resources: Vec<Missing<u16>> = Vec::new();
        let at_once = entries.iter().map(|p| p.delivered_at_once).collect();
        let stored = entries.iter().map(|p| p.storable).collect();
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
        Ok(Frame { products, at_once, stored, resources, market_zones: geo.market_zones() })
    }

    /// Whether a product is made to order, as it is sold, holding no stock: every product that cannot be stored.
    pub(crate) fn made_to_order(&self, product: u16) -> bool {
        self.stored.get(usize::from(product)).is_some_and(|s| !s)
    }

    /// A product's least quantity traded, ten to its price's exponent.
    pub(crate) fn base(&self, product: u16) -> i64 {
        self.traded(product).base
    }

    fn traded(&self, product: u16) -> Traded {
        let Some(Missing::Present(t)) = self.products.get(usize::from(product)) else {
            violation!(clause = "GDS.1", "a good of a product the world does not declare", product = product);
        };
        *t
    }
}

/// Every method public series are forecast by, heuristic by heuristic on the menu and within each memory type by
/// memory type, so a method's index is its heuristic times the memory types plus its memory type; each with its
/// memory type's parameters: its speed of correction, a type of the adaptive gain's distribution, and the trend's and
/// anchor's shared pulls.
pub(crate) fn methods(
    p: &phx_val::prims::ValPrims,
    register: &phx_core::Register,
) -> Result<Vec<(phx_val::method::Method, phx_val::heuristic::Params)>, String> {
    let types = u16::try_from(p.memory_types.shared(register).get()).map_err(|e| e.to_string())?;
    let gain = p.adaptive_gain.shared(register);
    let set = phx_core::register::values::TypeSet::build(gain, types)?;
    let scale = (0..gain.exp).fold(1.0, |s, _| s * phx_rand::float::from_i64(crate::consts::DECADE));
    let (gamma, kappa) = (p.trend_gamma.shared(register).to_f64(), p.anchor_kappa.shared(register).to_f64());
    let mut out = Vec::new();
    for h in 0..phx_val::heuristic::MENU.len() {
        let heuristic = phx_val::heuristic::HeuristicId::new(u8::try_from(h).map_err(|e| e.to_string())?);
        for (m, t) in (0_u8..).zip(set.types()) {
            let method = phx_val::method::Method {
                heuristic,
                memory: phx_val::method::MemoryType(m),
                window: phx_val::method::AgeWindow(0),
            };
            let params =
                phx_val::heuristic::Params { lambda: phx_rand::float::from_i64(t.value) / scale, gamma, kappa };
            out.push((method, params));
        }
    }
    Ok(out)
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
    pub shops: Vec<crate::retail::Shop>,
    pub sales: Vec<crate::retail::Sale>,
    /// Each seller of a service at today's meetings, with the way it makes it by.
    pub makers: BTreeMap<PartyId, u32>,
    pub ships: Vec<crate::freight::Ship>,
    pub investments: Vec<crate::invest::Invest>,
    /// Each project stage among the day's trades, with its project and units.
    pub stages: BTreeMap<phx_ledger::instruction::InstructionId, (u64, i64)>,
    pub booked: Vec<crate::freight::Booked>,
    pub freight: Vec<(Instruction, crate::freight::Booked)>,
    pub tally: GoodsDay,
}

/// What a day's goods did: the calls met, the transformations that took units from a deposit, the orders and wants
/// admitted, those refused or lapsed unmet, the transformations and trades that failed; and at retail, the buyers,
/// the sellers they had in reach, the rounds of choosing again, the buyers that found no seller, and the units of
/// services delivered at once that no buyer took, which are lost; in carriage, the shipments set on their way, the
/// bookings refused, and the arrivals; and in plant, the projects begun, the stages that waited on their builder, and
/// the projects completed; and the makings beyond their plant's capacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoodsDay {
    pub auctions: u64,
    pub extractions: u64,
    /// The transformations that made units by a way.
    pub made: u64,
    pub orders: u64,
    pub refused: u64,
    pub failed: u64,
    pub shoppers: u64,
    pub in_reach: u64,
    pub rounds: u64,
    pub unserved: u64,
    pub unused: u64,
    pub shipments: u64,
    pub refused_bookings: u64,
    pub arrivals: u64,
    pub projects: u64,
    pub waiting: u64,
    pub completed: u64,
    /// The makings of more units a twin than its maker's plant allows a day.
    pub beyond_capacity: u64,
}

/// Each good's latest mark where it stands, as the markets' marks give it, rebuilt after the day's marks.
pub(crate) type Marks = Arc<BTreeMap<GoodKey, i64>>;
/// What a shipper reads of other places, rebuilt with the marks: each good's mark at each region's market zone, and
/// the carriage market's mark at each origin and mode.
#[derive(Debug, Default)]
pub(crate) struct AwayTable {
    market: BTreeMap<(u16, u8), Vec<(ZoneId, i64)>>,
    carriage: BTreeMap<(ZoneId, u16), i64>,
}
/// Each good's public outlook where it stands, by method, rebuilt at 5a.
pub(crate) type Outlooks = Arc<BTreeMap<(GoodKey, u16), i64>>;

/// A good a row holds or has delivered: its product, its grade class and its units, one twin's.
type Units = HeldGood;

/// What a run of rows may read of its goods, read before its handler runs.
#[derive(Debug, Default)]
pub(crate) struct RunGoods {
    start: u32,
    rows: Vec<RowGoods>,
    marks: Marks,
    outlooks: Outlooks,
    away: Arc<AwayTable>,
    geo: Option<Arc<phx_geo::GeoState>>,
}

#[derive(Debug)]
struct RowGoods {
    zone: Missing<ZoneId>,
    held: Vec<Units>,
    plant: Vec<phx_core::HeldPlant>,
    rights: Vec<HeldRight>,
    delivered: Vec<Units>,
    money: Missing<i64>,
    net_assets: Missing<i64>,
}

impl RowGoods {
    /// A row that holds, has delivered and stands nowhere yet.
    fn none() -> RowGoods {
        RowGoods {
            zone: Missing::Absent,
            held: Vec::new(),
            plant: Vec::new(),
            rights: Vec::new(),
            delivered: Vec::new(),
            money: Missing::Absent,
            net_assets: Missing::Absent,
        }
    }
}

impl RunGoods {
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
    fn delivered(&self, slot: Slot, product: u16) -> i64 {
        self.row(slot).map_or(0, |r| r.delivered.iter().filter(|(p, _, _)| *p == product).map(|(_, _, q)| q).sum())
    }
    fn mark(&self, slot: Slot, product: u16, grade: u8) -> Missing<i64> {
        let Some(Missing::Present(zone)) = self.row(slot).map(|r| r.zone) else { return Missing::Absent };
        self.marks.get(&GoodKey { product, grade, zone }).copied().map_or(Missing::Absent, Missing::Present)
    }
    fn outlook(&self, slot: Slot, product: u16, grade: u8, method: u16) -> Missing<i64> {
        let Some(Missing::Present(zone)) = self.row(slot).map(|r| r.zone) else { return Missing::Absent };
        let key = (GoodKey { product, grade, zone }, method);
        self.outlooks.get(&key).copied().map_or(Missing::Absent, Missing::Present)
    }
    fn plant(&self, slot: Slot) -> &[phx_core::HeldPlant] {
        self.row(slot).map_or(&[], |r| r.plant.as_slice())
    }

    fn net_assets(&self, slot: Slot) -> Missing<i64> {
        self.row(slot).map_or(Missing::Absent, |r| r.net_assets)
    }

    fn away(&self, slot: Slot, product: u16, grade: u8) -> Vec<phx_core::Away> {
        let (Some(Missing::Present(here)), Some(geo)) = (self.row(slot).map(|r| r.zone), self.geo.as_ref()) else {
            return Vec::new();
        };
        let modes: Vec<(u16, i64)> =
            self.away.carriage.range((here, 0)..=(here, u16::MAX)).map(|((_, m), p)| (*m, *p)).collect();
        let Some(places) = self.away.market.get(&(product, grade)) else { return Vec::new() };
        let mut out = Vec::new();
        for (zone, there) in places.iter().filter(|(z, _)| *z != here) {
            let Some(metres) = geo.distances.between(here, *zone) else { continue };
            out.extend(modes.iter().map(|(mode, carriage)| phx_core::Away {
                zone: zone.get(),
                mode: *mode,
                metres,
                there: *there,
                carriage: *carriage,
            }));
        }
        out
    }
    fn money(&self, slot: Slot) -> Missing<i64> {
        self.row(slot).map_or(Missing::Absent, |r| r.money)
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
    pub(crate) fn good(&mut self, key: GoodKey) -> InstrumentId {
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
            if let Source::Way(w) = leg.source {
                for (product, took) in self.way_inputs(w, leg.product, leg.qty) {
                    let good = self.good(GoodKey { product, grade: 0, zone });
                    let unit = self.goods_frame.traded(product).unit;
                    legs.push(LegRec {
                        party: row.party,
                        account: AccountRef::Instrument(good),
                        qty: -times(took, row.twins),
                        denom: Denom::Unit(unit),
                        kind: LegKind::Transformation { source: leg.source, cost: 0 },
                    });
                }
            }
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
            self.market_day.tally.failed += 1;
            return;
        }
        if t.legs.iter().any(|l| matches!(l.source, Source::Deposit(_))) {
            self.market_day.tally.extractions += 1;
        }
        if let Some(made) = t.legs.iter().find(|l| matches!(l.source, Source::Way(_)) && l.qty > 0).map(|l| l.qty) {
            self.market_day.tally.made += 1;
            self.within_capacity(row.party, made);
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

    /// What a way takes of each input that can be held to finish units of its product, a twin's: its register's
    /// statement for what is started, rounded as the technology declares. A way that makes another product than the
    /// leg's is a contract broken.
    #[clause("TEC.9", "GDS.2")]
    pub(crate) fn way_inputs(&self, way: u32, product: u16, finished: i64) -> Vec<(u16, i64)> {
        let tech =
            self.own.iter().find(|(code, _)| *code == "TEC").and_then(|(_, s)| s.downcast_ref::<sys_tec::Technology>());
        let Some(w) = tech.and_then(|t| t.way(if_base::WayId::new(way))) else {
            violation!(clause = "TEC.9", "a production by a way the technology does not hold", way = way);
        };
        if w.product.index() != product {
            violation!(clause = "TEC.9", "a production of a product its way does not make", way = way);
        }
        let started = sys_tec::ways::started_for(w, phx_num::QtyRaw::from_raw(finished));
        w.inputs
            .iter()
            .filter(|(p, _)| tech.and_then(|t| sys_tec::products::get(&t.products, *p)).is_some_and(|d| d.storable))
            .map(|(p, per)| (p.index(), sys_tec::ways::takes(started, *per).raw()))
            .filter(|(_, took)| *took > 0)
            .collect()
    }

    /// The most units of its product a party can make by a way at a zone from what it holds of the way's inputs that
    /// can be held: the least over them, each the most finished whose take of it the holding covers, each twin of
    /// an agent from its own share.
    #[clause("TEC.9")]
    /// A making of `each` units a twin counted when its maker's plant allows fewer a day.
    #[clause("CAP.9")]
    pub(crate) fn within_capacity(&mut self, maker: PartyId, each: i64) {
        let name = <if_firm::facts::Capacity as phx_core::FactDef>::ITEM.name;
        if let Missing::Present(most) = self.party_fact(maker, name)
            && each > most
        {
            self.market_day.tally.beyond_capacity += 1;
        }
    }

    /// A party's fact by name, from an individual's row or an agent's positions; absent where its kind keeps none.
    pub(crate) fn party_fact(&self, party: PartyId, name: &str) -> Missing<i64> {
        let (place, slot) = self.books.parties.row(party);
        let first = self.books.parties.first_cell_place();
        match place.checked_sub(first) {
            None => match self.books.parties.table(place).facet_named(name) {
                Some(column) => self.books.parties.table(place).fact(slot, column),
                None => Missing::Absent,
            },
            Some(k) => {
                let table = phx_pop::population::Population::table::<phx_store::SystemBacking>(
                    self.books.parties.cells(),
                    usize::from(k),
                );
                match table.position(name) {
                    Some(column) => table.fact(slot, column),
                    None => Missing::Absent,
                }
            }
        }
    }

    pub(crate) fn way_most(&self, way: u32, party: PartyId, zone: ZoneId) -> Missing<i64> {
        let tech =
            self.own.iter().find(|(code, _)| *code == "TEC").and_then(|(_, s)| s.downcast_ref::<sys_tec::Technology>());
        let Some((t, w)) = tech.and_then(|t| t.way(if_base::WayId::new(way)).map(|w| (t, w))) else {
            return Missing::Absent;
        };
        let (place, slot) = self.books.parties.row(party);
        let arenas = self.books.parties.holder(place);
        let twins = i64::from(self.books.parties.unit(party));
        let mut most: Missing<i64> = Missing::Absent;
        for (p, per) in
            w.inputs.iter().filter(|(p, _)| sys_tec::products::get(&t.products, *p).is_some_and(|d| d.storable))
        {
            let key = GoodKey { product: p.index(), grade: 0, zone };
            let held = match self.books.ledger.goods.of(key) {
                Missing::Present(g) => match phx_ledger::holding::holding(arenas, slot, g) {
                    Missing::Present(h) => h.quantity.raw(),
                    Missing::Absent => 0,
                },
                Missing::Absent => 0,
            } / twins;
            let takes =
                |f: i64| sys_tec::ways::takes(sys_tec::ways::started_for(w, phx_num::QtyRaw::from_raw(f)), *per).raw();
            // A take rounds up, so the most the holding covers is found by halving below a bound it cannot pass.
            let scale = phx_num::price::pow10(if_base::consts::PER_UNIT_EXP);
            if per.raw() <= 0 {
                continue;
            }
            let bound = i128::from(held) * scale / i128::from(per.raw()) + 1;
            let (mut low, mut high) = (0_i64, i64::try_from(bound).unwrap_or(i64::MAX));
            while low < high {
                let mid = low + (high - low) / 2 + (high - low) % 2;
                if takes(mid) <= held { low = mid } else { high = mid - 1 }
            }
            let f = low * twins;
            most = Missing::Present(match most {
                Missing::Present(m) if m < f => m,
                _ => f,
            });
        }
        most
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
            Some(order) => {
                self.market_day.orders.push(order);
                self.market_day.tally.orders += 1;
            }
            None => self.market_day.tally.refused += 1,
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
            // A place's markets meet on its country's business days; an order posted on another lapses unmet.
            let meets = match self.geo().zone_country(key.zone) {
                Missing::Present(country) => self.calendar.is_business(country, day),
                Missing::Absent => false,
            };
            if !meets {
                for o in &mut orders {
                    if let Missing::Present(cover) = std::mem::replace(&mut o.cover, Missing::Absent) {
                        self.books.ledger.covers.release(cover);
                    }
                }
                self.market_day.tally.refused += phx_rand::float::len_u64(orders.len());
                continue;
            }
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
                    self.market_day.tally.auctions += 1;
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
    pub(crate) fn markets_settle(&mut self, day: Day, step: SubStep) {
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
                    self.stage_settled(day, step, id);
                }
                Err(_) => self.market_day.tally.failed += 1,
            }
        }
        // A stage that failed waits for another day.
        self.market_day.stages.clear();
    }

    /// The opening's snapshot of the goods' markets: each good the opening holds, in the market its product
    /// meets in, marked at its product's opening price for a lot, and that price its public series' one print, from
    /// which every method's first outlook is that price.
    #[clause("GEN.5", "VAL.10", "MKT.12")]
    pub(crate) fn snapshot_markets(&mut self, day: Day) -> Result<(), String> {
        let prices = self.register.table1("GDS.opening_price")?.values().to_vec();
        let scale = phx_rand::float::from_i64(
            i64::try_from(phx_num::price::pow10(match self.register.decl_by_id("GDS.opening_price")?.value {
                phx_core::ValueType::Table1 { exp, .. } => exp,
                _ => return Err("`GDS.opening_price` is no table of one axis".to_owned()),
            }))
            .map_err(|e| e.to_string())?,
        );
        let standardised = self.register.table1("GDS.standardised")?.clone();
        let keys: Vec<GoodKey> = self.books.ledger.goods.iter().map(|(k, _)| k).collect();
        let methods = self.val_methods.len();
        for key in keys {
            let common = standardised.at(i64::from(key.product)).is_ok_and(|v| v == 1);
            let name = if common { "GDS.commodities" } else { "GDS.between_firms" };
            let Missing::Present(kind) = self.market_kinds.kind(phx_ledger::instruction::name_code(name)) else {
                continue;
            };
            let Some(per_unit) = prices.get(usize::from(key.product)) else { continue };
            let lot = phx_rand::float::from_i64(self.goods_frame.base(key.product));
            let Some(price) =
                phx_rand::float::floor_to_i64((phx_rand::float::from_i64(*per_unit) / scale * lot).round())
            else {
                return Err(format!("product {}'s opening price beyond a price", key.product));
            };
            let market = self.market_kinds.instance(&mut self.markets.made, kind, key.code());
            self.markets.tape.mark(phx_market::print::Mark {
                market,
                day,
                price: phx_num::PriceRaw::from_raw(price),
                source: phx_market::print::MarkSource::Snapshot,
            });
            self.markets.public.insert(
                market,
                phx_market::markets::PublicSeries {
                    day,
                    last: price,
                    before: price,
                    sum: i128::from(price),
                    count: 1,
                    outlooks: vec![price; methods],
                },
            );
        }
        self.goods_marks();
        Ok(())
    }

    /// Each good's mark where it stands, from the markets' marks, for the handlers' reads.
    pub(crate) fn goods_marks(&mut self) {
        let goods: Vec<u16> = ["GDS.commodities", "GDS.between_firms"]
            .iter()
            .filter_map(|n| match self.market_kinds.kind(phx_ledger::instruction::name_code(n)) {
                Missing::Present(k) => Some(k),
                Missing::Absent => None,
            })
            .collect();
        let carriage: Vec<u16> = self.trade.freight.iter().map(|f| f.kind).collect();
        let market_zones: std::collections::BTreeSet<ZoneId> = self
            .goods_frame
            .market_zones
            .iter()
            .filter_map(|z| match z {
                Missing::Present(z) => Some(*z),
                Missing::Absent => None,
            })
            .collect();
        let (mut marks, mut away) = (BTreeMap::new(), AwayTable::default());
        for (market, kind, subject) in self.markets.made.iter() {
            let Missing::Present(mark) = self.markets.tape.mark_of(market) else { continue };
            let price = mark.price.raw();
            if goods.contains(&kind) {
                let key = GoodKey::from_code(subject);
                marks.insert(key, price);
                if market_zones.contains(&key.zone) {
                    away.market.entry((key.product, key.grade)).or_default().push((key.zone, price));
                }
            } else if carriage.contains(&kind) {
                let (Ok(zone), Ok(mode)) =
                    (u32::try_from(subject >> u16::BITS), u16::try_from(subject & u64::from(u16::MAX)))
                else {
                    continue;
                };
                away.carriage.insert((ZoneId::new(zone), mode), price);
            }
        }
        self.marks = Arc::new(marks);
        self.away = Arc::new(away);
    }

    /// 5a: every good's print series taken in since the last pass, and each method's outlook of it: the first print
    /// is every method's first outlook, then each method's rule over what it saw. Then each good's outlooks
    /// at its place for the handlers' reads.
    #[clause("VAL.23", "VAL.5", "VAL.10")]
    pub(crate) fn goods_outlooks(&mut self, day: Day) {
        let methods = &self.val_methods;
        let prints: Vec<(MarketId, Day, i64)> =
            self.markets.tape.last_prints().map(|(m, p)| (m, p.day(), p.price().raw())).collect();
        for (market, print_day, price) in prints {
            let series = self.markets.public.entry(market).or_insert_with(|| phx_market::markets::PublicSeries {
                day: print_day,
                last: price,
                before: price,
                sum: 0,
                count: 0,
                outlooks: Vec::new(),
            });
            if series.count > 0 && print_day <= series.day {
                continue;
            }
            let first = series.count == 0;
            series.before = if first { price } else { series.last };
            series.last = price;
            series.day = print_day;
            series.sum += i128::from(price);
            series.count += 1;
            let Some(mean) = i64::try_from(series.sum / i128::from(series.count)).ok() else {
                phx_num::capacity_exceeded!("a series' mean price", i64::MAX, series.count);
            };
            let level = phx_rand::float::from_i64(mean);
            let seen = |previous: f64| phx_val::heuristic::Seen {
                previous,
                last: phx_rand::float::from_i64(series.last),
                before: phx_rand::float::from_i64(series.before),
                level,
                announced: Missing::Absent,
                horizon_end: day,
            };
            let next: Vec<i64> = methods
                .iter()
                .enumerate()
                .map(|(i, (method, params))| {
                    let previous = match series.outlooks.get(i) {
                        Some(o) if !first => phx_rand::float::from_i64(*o),
                        _ => phx_rand::float::from_i64(price),
                    };
                    let x = phx_val::method::outlook(*method, &seen(previous), params);
                    let Some(o) = phx_rand::float::floor_to_i64(x + crate::consts::HALF) else {
                        violation!(clause = "VAL.23", "an outlook beyond a price", market = market.get());
                    };
                    o
                })
                .collect();
            series.outlooks = next;
        }
        let mut out = BTreeMap::new();
        for (market, series) in &self.markets.public {
            let Missing::Present(subject) = self.markets.made.subject_of(*market) else { continue };
            let key = GoodKey::from_code(subject);
            for (m, o) in (0_u16..).zip(&series.outlooks) {
                out.insert((key, m), *o);
            }
        }
        self.outlooks = Arc::new(out);
    }

    /// What a run of rows may read of its goods: for each row, the goods it holds at its zone and the rights it
    /// holds, one twin's for an agent, and what it has delivered.
    pub(crate) fn run_goods(&self, rows: Rows, run: core::ops::Range<u32>) -> RunGoods {
        let mut out = RunGoods {
            start: run.start,
            rows: Vec::new(),
            marks: Arc::clone(&self.marks),
            outlooks: Arc::clone(&self.outlooks),
            away: Arc::clone(&self.away),
            geo: Some(Arc::clone(crate::world::geo_arc(&self.own))),
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
            let mut goods_cost = 0_i128;
            for (instrument, quantity, cost) in phx_ledger::holding::held(arenas, at) {
                let units = quantity / row.twins;
                if let Missing::Present(key) = ledger.goods.key(instrument) {
                    goods_cost += i128::from(cost);
                    if key.zone == row.zone {
                        goods.held.push((key.product, key.grade, units));
                    }
                }
                if let Missing::Present(deposit) = ledger.goods.deposit(instrument)
                    && units > 0
                {
                    goods.rights.push(self.held_right(deposit));
                }
                if let Missing::Present((chain, class)) = ledger.chains.of(instrument)
                    && let Some(c) = ledger.chains.get(chain)
                    && let (Ok(kind), Ok(class)) = (u8::try_from(c.tag), u8::try_from(class))
                {
                    goods.plant.push((kind, class, units));
                }
            }
            if let Missing::Present(country) = self.geo().zone_country(row.zone) {
                let ccy = phx_ledger::opening::currency(country);
                goods.money = match self.books.money_held(row.party, ccy) {
                    Missing::Present(m) => Missing::Present(m / row.twins),
                    Missing::Absent => Missing::Absent,
                };
            }
            goods.net_assets = self.book_worth(place, at, (goods_cost, row.twins));
            for (good, q) in ledger.goods.delivered(row.party) {
                if let Missing::Present(key) = ledger.goods.key(good) {
                    goods.delivered.push((key.product, key.grade, q / row.twins));
                }
            }
            out.rows.push(goods);
        }
        out
    }

    /// What winding a row down would return beyond the money it keeps either way, one twin's: the goods it holds at
    /// their cost, as the row's holdings were read, less what it owes at its balances; its plant returns nothing, since no market buys used plant yet.
    /// None where a debt carries no balance to read.
    fn book_worth(&self, place: u16, slot: Slot, (goods_cost, twins): (i128, i64)) -> Missing<i64> {
        let arenas = self.books.parties.holder(place);
        let mut total = goods_cost;
        for r in phx_ledger::rows::iter(arenas, slot) {
            if r.side() != phx_ledger::algebra::Side::Liability {
                continue;
            }
            let Missing::Present(balance) = r.optional.balance else { return Missing::Absent };
            total -= i128::from(balance);
        }
        i64::try_from(total / i128::from(twins)).map_or(Missing::Absent, Missing::Present)
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
