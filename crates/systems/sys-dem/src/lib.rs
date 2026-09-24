//! DEM, population and demography: the household kind, its persons as roles, and the opening's households; mortality,
//! illness and ageing arrive as processes on them.

mod compose;
mod consts;
mod opening;
mod prims;

use if_pop::{
    ADULT, ADULT_COUNT, ADULT_GROUPS, CHILD_COUNTS, CHILD_GROUPS, CHILDREN, HEAD, HEAD_AGE, HOUSEHOLD, PARTNER,
    PARTNERS, REGION,
};
use phx_core::{
    Declarations, HandlerTable, ResolutionDecl, SetupValue, StreamDef, System, declare_kind, declare_stream,
};

pub use opening::Households;
pub use prims::Prims;

declare_kind! { pub HOUSEHOLD_KIND = "household" { legal_form: "household", table: Cells, clause: "POP.2" } }

declare_stream! { pub RegionsStream = "DEM.opening_regions" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub PersonsStream = "DEM.opening_persons" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub CompositionStream = "DEM.opening_composition" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub HealthStream = "DEM.opening_health" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub EducationStream = "DEM.opening_education" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// Population and demography.
#[derive(Debug)]
pub struct Dem;

/// The household kind's roles, key and profile groups, from the households' vocabulary, and how it is represented.
fn declare_household(d: &mut Declarations) {
    let mut k = d.pop_kind(HOUSEHOLD);
    k.role(HEAD).role(PARTNER).role(ADULT).key_attr(REGION).key_attr(HEAD_AGE).key_attr(PARTNERS).key_attr(ADULT_COUNT);
    for (role, count) in CHILDREN.iter().zip(CHILD_COUNTS.iter()) {
        k.role(*role).key_attr(*count);
    }
    for (life, schooling) in ADULT_GROUPS.iter().copied() {
        k.profile_group(life).profile_group(schooling);
    }
    for life in CHILD_GROUPS.iter().copied() {
        k.profile_group(life);
    }
    k.resolution(ResolutionDecl { cell_budget: "DEM.cell_budget", ranks: None, widen_order: &[], clause: "REP.18" });
}

impl System for Dem {
    const CODE: &'static str = "DEM";

    fn declare(d: &mut Declarations) {
        d.kind(HOUSEHOLD_KIND);
        declare_household(d);
        for stream in [
            RegionsStream::DECL,
            PersonsStream::DECL,
            CompositionStream::DECL,
            HealthStream::DECL,
            EducationStream::DECL,
        ] {
            d.stream(stream);
        }
        let prims = Prims::declare(d);
        d.setup_value(SetupValue { prim: &prims::LIFE_EXPECTANCY, derived: "GEN.life_expectancy" });
        d.contribution(Box::new(Households { prims }));
    }

    fn handlers(_: &mut HandlerTable) {}
}
