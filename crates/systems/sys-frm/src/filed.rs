//! The firms' opening state beyond their parties: the product each makes, one of its industry's; the stocks it holds
//! of its product and of the storable inputs its way uses; and its latest filed accounts, read from its books once its
//! staff are hired: what an hour of its staff costs, what it makes a day, what a lot costs it to make, the price it
//! posts and the markup that price is over its cost, what it expects to sell, the return its management requires and
//! the method it forecasts by. Its decisions start from these.

use if_firm::facts::{
    DeliveredAtReview, ExpectedSales, HoursAUnit, LastReview, Markup, Method, OutputRate, Price, RequiredReturn,
    Switching, UnitCost, WagePerHour,
};
use if_firm::known::{Industry, PRODUCT, Product};
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::register::values::Table2;
use phx_core::{
    Contribution, FactDef, Opening, OpeningCountry, OpeningPhase, PHYSICAL_STOCK, PRESENT_VALUES, ProductEntry,
    Register, StreamDef, opening_subject,
};
use phx_geo::GeoState;
use phx_id::{PartyId, Slot, ZoneId};
use phx_ledger::algebra::{Leg, Side, Terms};
use phx_ledger::books::Books;
use phx_ledger::goods::GoodKey;
use phx_ledger::instruction::{Effect, LegRec, ReasonDecl};
use phx_ledger::instrument::{InstrumentFamily, NewInstrument};
use phx_ledger::opening::{currency, hold, key, whole};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_rand::float::{floor_to_i64, from_i64, from_u64, len_u64};
use phx_store::SystemBacking;

use crate::consts::{DAYS_A_WEEK, FILED_PURPOSES, METHODS, PRODUCTS_PURPOSE, RETURNS};
use crate::small::{SMALL_COUNTS, SMALL_FIRMS};
use crate::{FilingPrims, OpeningStream, SMALL_FIRM};

/// Each firm with the product it makes, for the deposits' rights and the stocks.
pub const PRODUCTS_DRAWN: &str = "FRM.firm_products";
const FIRMS: &str = "FRM.firms";

/// The opening's instructions for stocks: capital on both sides, since they open the books.
pub const REASON: ReasonDecl =
    ReasonDecl { name: "FRM opening", order: 0, paid: Effect::Equity, received: Effect::Equity, held: Missing::Absent };

/// The firms' declarations in the books: their opening's reason.
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "firm declarations"
    }
    fn phase(&self) -> OpeningPhase {
        phx_core::DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }
    fn contribute(&self, opening: &mut Opening<'_>) {
        let reasons = &mut phx_ledger::books::of(opening).ledger.reasons;
        let _ = reasons.declare(REASON);
        let _ = reasons.declare(crate::MADE);
    }
}

/// A firm as the opening reads it: its party, the persons it employs over all its twins, its twins, its industry,
/// and where its row is.
struct Firm {
    party: PartyId,
    twins: u64,
    industry: usize,
    agent: Option<(usize, Slot)>,
}

type Parts<'a> = (&'a mut Books, &'a mut Population, &'a GeoState, &'a mut phx_core::GenReport);

fn parts<'a>(opening: &'a mut Opening<'_>) -> Parts<'a> {
    let Opening { books, population, geo, report, .. } = opening;
    let (Some(books), Some(population), Some(geo)) =
        (books.downcast_mut::<Books>(), population.downcast_mut::<Population>(), geo.downcast_ref::<GeoState>())
    else {
        violation!(clause = "GEN.3", "an opening handed something other than the world's books, population and map");
    };
    (books, population, geo, report)
}

fn drawn(b: &Books, name: &str, c: &OpeningCountry) -> Vec<(PartyId, u64)> {
    b.drawn.get(&key(name, c.id)).cloned().unwrap_or_default()
}

fn small_kind(population: &Population) -> usize {
    let Some(at) = population.kinds.iter().position(|k| k.decl.kind == SMALL_FIRM.name) else {
        violation!(clause = "FRM.23", "a world that keeps no small firm kind");
    };
    at
}

fn attr_at(population: &Population, kind: usize, name: &str) -> usize {
    let Some(i) = population.kinds.get(kind).and_then(|kd| kd.decl.attr(name)) else {
        violation!(clause = "REP.41", "a small firm kind without an attribute it needs");
    };
    i
}

/// Every firm of the country: the large firms, one twin each, and the small firms' agents.
fn firms(books: &Books, population: &Population, c: &OpeningCountry) -> Vec<Firm> {
    let index =
        |i: i64| usize::try_from(i).unwrap_or_else(|_| violation!(clause = "TEC.4", "a firm of no industry", at = i));
    let mut out = Vec::new();
    for (party, _) in drawn(books, FIRMS, c) {
        let Missing::Present(i) = books.parties.fact(party, <Industry as FactDef>::ITEM.name) else {
            violation!(clause = "TEC.4", "a firm with no industry", firm = party.get());
        };
        out.push(Firm { party, twins: 1, industry: index(i), agent: None });
    }
    let k = small_kind(population);
    let industry_at = attr_at(population, k, if_firm::known::INDUSTRY.name);
    let table = Population::table::<SystemBacking>(books.parties.cells(), k);
    for ((party, _), (_, twins)) in drawn(books, SMALL_FIRMS, c).into_iter().zip(drawn(books, SMALL_COUNTS, c)) {
        let slot = books.parties.row(party).1;
        let industry = index(i64::from(table.attr(slot, industry_at)));
        out.push(Firm { party, twins, industry, agent: Some((k, slot)) });
    }
    out
}

/// The products and the industries they declare, in the products' order.
fn catalogue(register: &Register) -> (Vec<ProductEntry>, Vec<String>) {
    let Ok(products) = register.products("TEC.products") else { violation!(clause = "TEC.1", "the products unread") };
    let mut industries: Vec<String> = Vec::new();
    for p in products {
        if !industries.contains(&p.industry) {
            industries.push(p.industry.clone());
        }
    }
    (products.to_vec(), industries)
}

/// The products an industry makes, by their places.
fn made_by(products: &[ProductEntry], industries: &[String], industry: usize) -> Vec<u16> {
    let Some(name) = industries.get(industry) else {
        violation!(clause = "TEC.4", "a firm of an industry the products do not declare", at = industry);
    };
    (0_u16..).zip(products).filter(|(_, p)| &p.industry == name).map(|(i, _)| i).collect()
}

/// Each of the products' deposits on the ground: in the country's land where it has any, else anywhere.
fn deposit_weights(geo: &GeoState, products: &[ProductEntry], made: &[u16], c: &OpeningCountry) -> Vec<u64> {
    let count = |here: bool| -> Vec<u64> {
        made.iter()
            .map(|p| {
                let resource = products.get(usize::from(*p)).and_then(|e| match e.extracts {
                    Missing::Present(r) => Some(r),
                    Missing::Absent => None,
                });
                len_u64(
                    geo.deposits
                        .iter()
                        .filter(|d| Some(d.resource) == resource && (!here || c.sites.contains(&d.tile)))
                        .count(),
                )
            })
            .collect()
    };
    let local = count(true);
    if local.iter().any(|w| *w > 0) { local } else { count(false) }
}

/// A pick weighted by whole weights.
fn weighted(weights: &[u64], draws: &mut phx_rand::Draws) -> Option<usize> {
    let total: u64 = weights.iter().sum();
    if total == 0 {
        return None;
    }
    let mut left = phx_rand::below_u64(draws, total);
    for (i, w) in weights.iter().enumerate() {
        if left < *w {
            return Some(i);
        }
        left -= w;
    }
    None
}

fn subject(c: &OpeningCountry, purpose: u32, ordinal: usize) -> phx_rand::Subject {
    let Ok(o) = u32::try_from(ordinal) else { violation!(clause = "GEN.3", "firms beyond counting") };
    opening_subject(u32::from(c.id.get()) * FILED_PURPOSES + purpose, o)
}

/// The product each firm makes: its industry's one, or, of an industry of several, one drawn by the deposits of each
/// on the ground, since what it extracts is what the ground gives.
#[clause("FRM.1", "TEC.4", "GEN.2", "GEN.3")]
#[derive(Debug)]
pub struct Products;

impl Contribution for Products {
    fn name(&self) -> &'static str {
        "firm products"
    }
    fn phase(&self) -> OpeningPhase {
        PHYSICAL_STOCK
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, SMALL_FIRMS]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[PRODUCTS_DRAWN]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[PRODUCTS_DRAWN]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (register, countries) = (opening.register, opening.countries);
        let (products, industries) = catalogue(register);
        for c in countries {
            let mut lot = opening.ctx.draws(&OpeningStream::DECL, subject(c, PRODUCTS_PURPOSE, 0));
            let (books, population, geo, _) = parts(opening);
            let list = firms(books, population, c);
            let mut chosen: Vec<(PartyId, u64)> = Vec::with_capacity(list.len());
            // An industry's products and their weights by the country's deposits, read once for all its firms.
            let mut by_industry: std::collections::BTreeMap<_, (Vec<u16>, Vec<u64>)> =
                std::collections::BTreeMap::new();
            for f in &list {
                let (made, weights) = by_industry.entry(f.industry).or_insert_with(|| {
                    let made = made_by(&products, &industries, f.industry);
                    let weights = if made.len() == 1 { Vec::new() } else { deposit_weights(geo, &products, &made, c) };
                    (made, weights)
                });
                let pick = match made.as_slice() {
                    [one] => Some(*one),
                    _ => weighted(weights, &mut lot).and_then(|i| made.get(i)).copied(),
                };
                let Some(product) = pick else {
                    violation!(clause = "TEC.4", "an industry none of whose products can be made", at = f.industry);
                };
                match f.agent {
                    None => books.parties.open_fact(f.party, <Product as FactDef>::ITEM.name, i64::from(product)),
                    Some((k, slot)) => {
                        let at = attr_at(population, k, PRODUCT.name);
                        let (tables, _, _) = books.parties.cells_mut();
                        Population::table_mut::<SystemBacking>(tables, k).set_attr(slot, at, u32::from(product));
                    }
                }
                chosen.push((f.party, u64::from(product)));
            }
            books.drawn.insert(key(PRODUCTS_DRAWN, c.id), chosen);
        }
    }
}

/// What a country's way for a product uses a unit: each product's units, and the days a unit takes.
struct Way {
    inputs: Vec<(u16, f64)>,
    lead: f64,
}

fn places(register: &Register, id: &str) -> f64 {
    match register.decl_by_id(id).map(|d| d.value) {
        Ok(phx_core::ValueType::Table2 { exp, .. } | phx_core::ValueType::Table1 { exp, .. }) => {
            libm::pow(crate::consts::DECADE, f64::from(exp))
        }
        _ => violation!(clause = "NUM.3", "a table read at no places"),
    }
}

fn way_of(register: &Register, c: &OpeningCountry, product: u16) -> Way {
    let (Ok(inputs), Ok(lead)) = (register.table2_in("TEC.inputs", c.id), register.table1("TEC.lead_time")) else {
        violation!(clause = "TEC.13", "a country's ways unread", country = c.id.get());
    };
    let column = |t: &Table2, row: i64| t.at(row, i64::from(product)).map_or(0.0, from_i64);
    let per_input = places(register, "TEC.inputs");
    let inputs = inputs
        .rows()
        .iter()
        .filter_map(|r| {
            let a = column(inputs, *r) / per_input;
            (a > 0.0).then(|| u16::try_from(*r).ok().map(|q| (q, a)))?
        })
        .collect();
    let lead = lead.at(i64::from(product)).map_or(0.0, from_i64);
    Way { inputs, lead }
}

/// The opening price of a unit of each product, in its currency's smallest units.
fn prices(register: &Register) -> Vec<f64> {
    let Ok(t) = register.table1(crate::OPENING_PRICE) else {
        violation!(clause = "GEN.5", "the products' opening prices unread");
    };
    let scale = places(register, crate::OPENING_PRICE);
    t.values().iter().map(|v| from_i64(*v) / scale).collect()
}

fn storable(products: &[ProductEntry], p: u16) -> bool {
    products.get(usize::from(p)).is_some_and(|e| e.storable)
}

/// Where a firm's goods are: its site's zone, or its region's market zone for an agent.
fn zone_of(books: &Books, population: &Population, geo: &GeoState, f: &Firm) -> ZoneId {
    let zone = match f.agent {
        None => geo.zone_of(books.parties.site(f.party)),
        Some((k, slot)) => {
            let region = Population::table::<SystemBacking>(books.parties.cells(), k)
                .attr(slot, attr_at(population, k, crate::REGION.name));
            usize::try_from(region).ok().and_then(|r| geo.market_zones().get(r).copied()).unwrap_or(Missing::Absent)
        }
    };
    let Missing::Present(z) = zone else {
        violation!(clause = "GDS.2", "a firm where no zone is", firm = f.party.get())
    };
    z
}

/// The stocks each firm holds at the opening, from the output its filed accounts show: of its product, the days of sales
/// its management aims to cover; of each storable input its way uses, what the days of making it takes and those days
/// cover use. What is delivered as it is made is never stocked.
#[clause("FRM.4", "GDS.5", "GEN.2", "GEN.3")]
#[derive(Debug)]
pub struct Stocks {
    pub prims: FilingPrims,
}

impl Contribution for Stocks {
    fn name(&self) -> &'static str {
        "firm stocks"
    }
    fn phase(&self) -> OpeningPhase {
        PRESENT_VALUES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, SMALL_FIRMS, PRODUCTS_DRAWN, "FRM.filed_accounts"]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["FRM.stocks"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (register, countries, date) = (opening.register, opening.countries, opening.date);
        let (products, _) = catalogue(register);
        let price = prices(register);
        let cover = phx_rand::float::from_u64(self.prims.decide.cover_days.shared(register).get());
        for c in countries {
            let (books, population, geo, report) = parts(opening);
            let chosen = drawn(books, PRODUCTS_DRAWN, c);
            let list = firms(books, population, c);
            let terms = goods_terms(books, c, date);
            let ccy = currency(c.id);
            let reason = books.ledger.reasons.named(REASON.name);
            for (f, (_, product)) in list.iter().zip(&chosen) {
                let Ok(p) = u16::try_from(*product) else { continue };
                let way = way_of(register, c, p);
                // The firm's filed output a day, a twin's, which its stocks cover.
                let Missing::Present(per_day) = read_fact(books, f, <OutputRate as FactDef>::ITEM.name) else {
                    continue;
                };
                let per_day = from_i64(per_day);
                let zone = zone_of(books, population, geo, f);
                let mut wanted: Vec<(u16, f64)> = Vec::new();
                if storable(&products, p) {
                    wanted.push((p, cover * per_day));
                }
                for (q, a) in &way.inputs {
                    if storable(&products, *q) {
                        wanted.push((*q, a * per_day * (way.lead + cover)));
                    }
                }
                let mut legs: Vec<LegRec> = Vec::new();
                for (q, units) in wanted {
                    let Some(per_twin) = floor_to_i64(units) else { continue };
                    let Some(held) = per_twin.checked_mul(i64::try_from(f.twins).unwrap_or(i64::MAX)) else {
                        phx_num::capacity_exceeded!("a firm's opening stock", i64::MAX, per_twin);
                    };
                    if held <= 0 {
                        continue;
                    }
                    let Some(e) = products.get(usize::from(q)) else { continue };
                    let Missing::Present(unit) = register.units().named(&e.unit) else {
                        violation!(clause = "GDS.1", "a product in an undeclared unit", product = q);
                    };
                    let good = GoodKey { product: q, grade: 0, zone };
                    let new = NewInstrument {
                        family: InstrumentFamily::RealAsset,
                        issuer: Missing::Absent,
                        unit,
                        ccy,
                        terms,
                    };
                    let ledger = &mut books.ledger;
                    let instrument = ledger.goods.issue(&mut ledger.instruments, good, new);
                    let cost = whole(from_i64(held) * price.get(usize::from(q)).copied().unwrap_or(0.0));
                    legs.push(hold(f.party, instrument, held, unit, cost, f.party.get()));
                }
                if !legs.is_empty() {
                    books.open(reason, legs, f.party.get(), report);
                }
            }
        }
    }
}

/// The terms a country's goods are issued under: an account in its currency, monthly from the opening's day.
fn goods_terms(books: &mut Books, c: &OpeningCountry, date: phx_id::Date) -> phx_ledger::terms::TermsId {
    let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
    let dates = ScheduleDates {
        anchor: date,
        period: months,
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: c.id,
    };
    books.ledger.terms.intern(Terms::account(currency(c.id), dates))
}

/// A firm's staff as its employment rows count them: the members, their hours a week and their wages a month.
fn staff(books: &Books, party: PartyId) -> (f64, f64, f64) {
    let lines = &books.ledger.lines;
    let (place, slot) = books.parties.row(party);
    let holder = books.parties.holder(place);
    let (mut members, mut hours, mut wages) = (0.0, 0.0, 0.0);
    for r in phx_ledger::rows::iter(holder, slot) {
        if r.side() != Side::Liability || lines.kind_name(r.row.line) != if_labour::consts::EMPLOYMENT_LINE {
            continue;
        }
        let terms = books.ledger.terms.get(lines.terms(r.row.line));
        let (Some(Leg::FixedAmount(m)), Some(h)) =
            (terms.legs.first(), terms.class.get(if_labour::consts::HOURS).copied())
        else {
            continue;
        };
        let n = f64::from(r.row.count);
        members += n;
        hours += n * f64::from(h);
        wages += n * from_i64(m.amt());
    }
    (members, hours, wages)
}

/// Each firm's latest filed accounts, read from its books once its staff are hired: what an hour of its staff costs;
/// the units its staff's hours make a day; what a lot costs to make at the opening's prices and that wage; the posted
/// price nearest the lot's opening price and the markup it is over that cost; the sales that rate gives over a
/// production period; the day of its last review, today, and nothing delivered since; the return its management
/// requires, drawn; the method it forecasts by, drawn as its switching's taste alone chooses on day zero, and its
/// switching type, drawn. A firm
/// with no staff files no rate, cost or price.
#[clause("FRM.1", "FRM.2", "FRM.14", "GEN.5", "REP.34", "VAL.23")]
#[derive(Debug)]
pub struct Filed {
    pub prims: FilingPrims,
}

impl Contribution for Filed {
    fn name(&self) -> &'static str {
        "filed accounts"
    }
    fn phase(&self) -> OpeningPhase {
        PRESENT_VALUES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[FIRMS, SMALL_FIRMS, PRODUCTS_DRAWN]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &["FRM.required_return", "FRM.method"]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["FRM.filed_accounts"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (register, countries, day) = (opening.register, opening.countries, opening.day);
        let price = prices(register);
        let Ok(m) = crate::decide::Management::compile(&self.prims.decide, register) else {
            violation!(clause = "FRM.5", "the firms' management unread at the opening");
        };
        let methods = FilingPrims::methods(register);
        let Ok(switching_types) = register.count("VAL.switching_types") else {
            violation!(clause = "VAL.7", "the switching types unread at the opening");
        };
        for c in countries {
            let mut returns = opening.ctx.draws(&OpeningStream::DECL, subject(c, RETURNS, 0));
            let mut method_lot = opening.ctx.draws(&OpeningStream::DECL, subject(c, METHODS, 0));
            let hurdle = self.prims.required_return.shared(register).clone();
            let weeks_a_month = crate::consts::DAYS_A_YEAR / DAYS_A_WEEK / crate::consts::MONTHS_A_YEAR;
            let hurdle = &hurdle;
            let (books, population, _, _) = parts(opening);
            let chosen = drawn(books, PRODUCTS_DRAWN, c);
            let list = firms(books, population, c);
            // Labour's share of what the country's firms add, as its wages were drawn from.
            let labour_share = phx_ledger::opening::derived(c, "GEN.labour_share") / crate::consts::PERCENT;
            for (f, (_, product)) in list.iter().zip(&chosen) {
                let Ok(p) = u16::try_from(*product) else { continue };
                let way = way_of(register, c, p);
                let lot = FilingPrims::lot(register, p);
                let (members, hours, wages) = staff(books, f.party);
                let required = hurdle.draw(&mut returns);
                let method = phx_rand::below_u64(&mut method_lot, methods);
                let switching = phx_rand::below_u64(&mut method_lot, switching_types);
                let mut facts: Vec<(&'static str, i64)> = vec![
                    (<LastReview as FactDef>::ITEM.name, i64::from(day.get())),
                    (<DeliveredAtReview as FactDef>::ITEM.name, 0),
                    (<Method as FactDef>::ITEM.name, i64::try_from(method).unwrap_or(0)),
                    (<Switching as FactDef>::ITEM.name, i64::try_from(switching).unwrap_or(0)),
                ];
                if let Some(r) = floor_to_i64(f64::round(required * crate::consts::FIXED_SCALE)) {
                    facts.push((<RequiredReturn as FactDef>::ITEM.name, r));
                }
                let materials: f64 =
                    way.inputs.iter().map(|(q, a)| a * price.get(usize::from(*q)).copied().unwrap_or(0.0)).sum();
                let added = price.get(usize::from(p)).copied().unwrap_or(0.0) - materials;
                if members > 0.0 && hours > 0.0 && added > 0.0 {
                    let wage = wages / (hours * weeks_a_month);
                    // A firm's filed output is what its wage bill pays for at labour's share of what a unit adds, so
                    // a firm that pays more makes more an hour, and its accounts show the rest of what it adds.
                    let hours_a_unit = labour_share * added / wage;
                    let per_day = hours / from_u64(f.twins) / DAYS_A_WEEK / hours_a_unit;
                    let cost = lot * (materials + hours_a_unit * wage);
                    let snapshot = lot * price.get(usize::from(p)).copied().unwrap_or(0.0);
                    let posted = m.points_near(snapshot).into_iter().fold(None, |best: Option<i64>, x| match best {
                        Some(b) if (from_i64(b) - snapshot).abs() <= (from_i64(x) - snapshot).abs() => Some(b),
                        _ => Some(x),
                    });
                    facts.push((<WagePerHour as FactDef>::ITEM.name, whole(wage)));
                    facts.push((<HoursAUnit as FactDef>::ITEM.name, whole(hours_a_unit * crate::consts::FIXED_SCALE)));
                    facts.push((<OutputRate as FactDef>::ITEM.name, whole(per_day)));
                    facts.push((<ExpectedSales as FactDef>::ITEM.name, whole(per_day * m.production_days)));
                    facts.push((<UnitCost as FactDef>::ITEM.name, whole(cost)));
                    if let Some(posted) = posted {
                        facts.push((<Price as FactDef>::ITEM.name, posted));
                        if cost > 0.0 {
                            let markup = from_i64(posted) / cost - 1.0;
                            facts.push((<Markup as FactDef>::ITEM.name, whole(markup * crate::consts::FIXED_SCALE)));
                        }
                    }
                }
                write_facts(books, f, &facts);
            }
        }
    }
}

/// A firm's fact as the opening wrote it: a large firm's from its row, an agent's from its positions, one twin's.
fn read_fact(books: &mut Books, f: &Firm, name: &str) -> Missing<i64> {
    match f.agent {
        None => books.parties.fact(f.party, name),
        Some((k, slot)) => {
            let (tables, _, _) = books.parties.cells_mut();
            let table = Population::table_mut::<SystemBacking>(tables, k);
            match table.position(name) {
                Some(column) => table.fact(slot, column),
                None => Missing::Absent,
            }
        }
    }
}

/// A firm's facts written: a large firm's to its row, an agent's to its positions, one twin's.
fn write_facts(books: &mut Books, f: &Firm, facts: &[(&'static str, i64)]) {
    match f.agent {
        None => {
            for (name, v) in facts {
                books.parties.open_fact(f.party, name, *v);
            }
        }
        Some((k, slot)) => {
            let (tables, _, _) = books.parties.cells_mut();
            let table = Population::table_mut::<SystemBacking>(tables, k);
            for (name, v) in facts {
                table.open_fact(slot, name, *v);
            }
        }
    }
}
