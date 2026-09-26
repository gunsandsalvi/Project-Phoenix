//! CAP, plant: capital goods of six kinds, held by condition class, worn from class to class at each owner's review
//! and retired at the end of their service, their depreciation charged to income and to the unit; every firm's plant
//! at the opening; the capacity a way's plant gives; and the rules by which owners invest, maintain, repair, sell and
//! scrap.

mod consts;
pub mod families;
pub mod kinds;
mod opening;
pub mod review;
pub mod rules;

use phx_core::handler::HandlerDecl;
use phx_core::register::values::Table1;
use phx_core::{
    Cadence, Declarations, HandlerTable, RunsOn, StreamDef, System, VisitDecl, WearDecl, declare_prim, declare_stream,
};
use phx_num::Count;

pub use opening::{Plant, STRUCTURES_HELD};

declare_stream! { pub VisitStream = "CAP.visits" { purpose: Occasion, keyed: false, clause: "CAP.4" } }
declare_stream! { pub OpeningStream = "CAP.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_prim! {
    /// Each kind's geometric rate of depreciation a year.
    pub DEPRECIATION = "CAP.depreciation" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 6 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Each kind's mean service life, in years.
    pub SERVICE_LIFE = "CAP.service_life" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 3 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Each kind's hyperbolic age-efficiency parameter.
    pub EFFICIENCY_SHAPE = "CAP.efficiency_shape" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 3 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Days from order to service of each kind a source measures.
    pub LEAD_DAYS = "CAP.lead_days" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// The product each kind is bought as, by its place among the products.
    pub BOUGHT_AS = "CAP.bought_as" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "CAP.5", scope: Shared
    }
}

declare_prim! {
    /// The classes of age each kind is held in.
    pub CONDITION_CLASSES = "CAP.condition_classes" { kind: Resolution, value: Count, clause: "REP.24", scope: Shared }
}

declare_prim! {
    /// Days between an owner's reviews of its plant.
    pub REVIEW_DAYS = "CAP.review_days" { kind: Preference, value: Count, clause: "CAP.13", scope: Shared }
}

declare_prim! {
    /// Each kind's net stock per unit of GDP at the opening.
    pub STOCK_PER_GDP = "CAP.stock_per_gdp" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

/// The kinds of firm that hold plant.
pub const HOLDERS: [&str; 2] = ["firm", "small_firm"];

/// Plant.
#[derive(Debug)]
pub struct Cap;

impl System for Cap {
    const CODE: &'static str = "CAP";

    fn declare(d: &mut Declarations) {
        let prims = kinds::Prims {
            depreciation: d.prim(&DEPRECIATION),
            life: d.prim(&SERVICE_LIFE),
            shape: d.prim(&EFFICIENCY_SHAPE),
            classes: d.prim(&CONDITION_CLASSES),
        };
        let _ = d.prim::<Table1>(&LEAD_DAYS);
        let _ = d.prim::<Table1>(&BOUGHT_AS);
        let _ = d.prim::<Count>(&REVIEW_DAYS);
        let stock = d.prim(&STOCK_PER_GDP);
        d.stream(OpeningStream::DECL);
        d.stream(VisitStream::DECL);
        d.compile(Box::new(move |register, _| Ok(Box::new(kinds::Kinds::compile(&prims, register)?))));
        d.contribution(Box::new(opening::Declared));
        d.contribution(Box::new(Plant { prims, stock }));
        let cadence = Cadence::Schedule { days: REVIEW_DAYS.id, runs_on: RunsOn::Any };
        for (handler, kind) in [(review::ReviewSmall::NAME, HOLDERS[1]), (review::ReviewLarge::NAME, HOLDERS[0])] {
            d.visit(VisitDecl { handler, kind, cadence, stream: VisitStream::DECL.name, wakes: &[], clause: "CAP.4" });
        }
        for visit in [review::ReviewSmall::NAME, review::ReviewLarge::NAME] {
            d.wear(WearDecl { visit, specs: kinds::wear_specs, clause: "CAP.6" });
        }
        d.family(Box::new(families::Stock));
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<review::ReviewSmall>();
        h.add::<review::ReviewLarge>();
    }
}
