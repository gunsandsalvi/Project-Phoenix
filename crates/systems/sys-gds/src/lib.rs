//! GDS, goods and commodities: the markets goods meet in, the grades a commodity is classed by, extraction from the
//! deposits whose rights a firm holds, weighing today's price against its outlook, and the spoilage of what is held,
//! which the kernel realises at the firms' stock visits.

mod consts;
pub mod extract;
pub mod markets;
pub mod rules;

use phx_core::handler::HandlerDecl;
use phx_core::register::values::{Table1, Table2};
use phx_core::{
    Cadence, Contribution, DECLARATIONS, Declarations, HandlerTable, Opening, OpeningPhase, Register, RunsOn,
    SpoilageDecl, StreamDef, System, VisitDecl, declare_prim, declare_stream,
};
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_macros::clause;
use phx_num::{Count, Missing};

declare_stream! { pub LotsStream = "GDS.lots" { purpose: Meeting, keyed: false, clause: "MKT.3" } }
declare_stream! { pub VisitStream = "GDS.visits" { purpose: Occasion, keyed: false, clause: "REP.21" } }

declare_prim! {
    /// The upper bounds of each extracted product's grade classes (rows, the products' places; columns, the classes),
    /// in the deposit's grade index; a product with no row has one class.
    pub GRADE_BOUNDS = "GDS.grade_bounds" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 3 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// How fast a deposit's grade falls as it is worked, the richest part first: its log falls by this times the share
    /// of the deposit taken, by the extracted product's place.
    pub GRADE_FALL = "GDS.grade_fall" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// Each storable product's yearly rate of loss in stock, by its place.
    pub SPOILAGE_RATE = "GDS.spoilage_rate" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 4 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// The room each storable product is kept in, by its place: an open yard, a dry warehouse, a cold store, a tank or
    /// a silo, the storage bought for it from whoever owns such room.
    pub STORAGE = "GDS.storage" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GDS.13", scope: Shared
    }
}

declare_prim! {
    /// Whether each product is standardised and meets in a call at each place, by its place; the others meet at posted
    /// prices between firms.
    pub STANDARDISED = "GDS.standardised" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GDS.7", scope: Shared
    }
}

declare_prim! {
    /// Days between an extractor's decisions on its deposits.
    pub EXTRACTION_DAYS = "GDS.extraction_days" { kind: Preference, value: Count, clause: "GDS.4", scope: Shared }
}

declare_prim! {
    /// Days between the realisations of a holder's spoilage, each over the days since the last.
    pub SPOILAGE_DAYS = "GDS.spoilage_days" { kind: Resolution, value: Count, clause: "GDS.8", scope: Shared }
}

/// Units taken from a deposit, which no one gives: the right's holder receives them.
pub const EXTRACTED: ReasonDecl = ReasonDecl {
    name: "GDS extracted",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Asset,
    held: Missing::Absent,
};

/// The kinds of firm that hold goods, extract and trade.
pub const HOLDERS: [&str; 2] = ["firm", "small_firm"];

/// The goods' declarations in the books: the reason units taken from a deposit are made under.
#[clause("GDS.4")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "goods declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
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
        let _ = phx_ledger::books::of(opening).ledger.reasons.declare(EXTRACTED);
    }
}

/// A table primitive's values in its decimals.
fn decimals(values: &[i64], exp: u8) -> Vec<f64> {
    let scale = libm::pow(consts::TEN, f64::from(exp));
    values.iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect()
}

/// The places a table primitive's integers carry, as its declaration states them.
fn places(p: &phx_core::PrimDecl) -> u8 {
    match p.value {
        phx_core::ValueType::Table1 { exp, .. } | phx_core::ValueType::Table2 { exp, .. } => exp,
        _ => phx_num::violation!(clause = "GDS.13", "a goods table declared as another type"),
    }
}

/// Each product's yearly rate of loss in stock, by its place, none past the list: what the kernel realises at the
/// stock visits.
///
/// # Errors
/// When the table is not in the register or its axis is not the products' places in order.
pub fn spoilage_rates(register: &Register) -> Result<Vec<f64>, String> {
    let t = register.table1(SPOILAGE_RATE.id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err("the spoilage rates are not by the products' places in order".to_owned());
    }
    Ok(decimals(t.values(), places(&SPOILAGE_RATE)))
}

/// Goods and commodities.
#[derive(Debug)]
pub struct Gds;

impl System for Gds {
    const CODE: &'static str = "GDS";

    fn declare(d: &mut Declarations) {
        let prims = extract::Prims {
            bounds: d.prim(&GRADE_BOUNDS),
            fall: d.prim(&GRADE_FALL),
            days: d.prim(&EXTRACTION_DAYS),
            standardised: d.prim(&STANDARDISED),
        };
        let _ = d.prim::<Table1>(&SPOILAGE_RATE);
        let _ = d.prim::<Table1>(&STORAGE);
        let _ = d.prim::<Count>(&SPOILAGE_DAYS);
        d.stream(LotsStream::DECL);
        d.stream(VisitStream::DECL);
        d.contribution(Box::new(Declared));
        d.market(Box::new(markets::COMMODITIES));
        d.market(Box::new(markets::BETWEEN_FIRMS));
        d.compile(Box::new(move |register, _| Ok(Box::new(extract::Own::compile(&prims, register)?))));
        let extraction = Cadence::Schedule { days: EXTRACTION_DAYS.id, runs_on: RunsOn::Business };
        let stock = Cadence::Schedule { days: SPOILAGE_DAYS.id, runs_on: RunsOn::Any };
        for (handler, kind, cadence) in [
            (extract::ExtractSmall::NAME, HOLDERS[1], extraction),
            (extract::ExtractLarge::NAME, HOLDERS[0], extraction),
            (extract::StockSmall::NAME, HOLDERS[1], stock),
            (extract::StockLarge::NAME, HOLDERS[0], stock),
        ] {
            d.visit(VisitDecl { handler, kind, cadence, stream: VisitStream::DECL.name, wakes: &[], clause: "GDS.4" });
        }
        for visit in [extract::StockSmall::NAME, extract::StockLarge::NAME] {
            d.spoilage(SpoilageDecl { visit, rates: spoilage_rates, clause: "GDS.8" });
        }
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<extract::ExtractSmall>();
        h.add::<extract::ExtractLarge>();
        h.add::<extract::StockSmall>();
        h.add::<extract::StockLarge>();
    }
}

/// The handles the table of two axes is read by, which is not one of a kind's own.
pub type BoundsPrim = phx_core::Prim<Table2>;
