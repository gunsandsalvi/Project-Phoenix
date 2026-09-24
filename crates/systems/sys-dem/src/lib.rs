//! DEM, population and demography: the household kind, its persons as roles, and the opening's households; mortality,
//! illness and ageing arrive as processes on them.

mod prims;

use if_pop::{
    ADULT_COUNTS, ADULT_GROUPS, ADULTS, CHILD_COUNTS, CHILD_GROUPS, CHILDREN, HEAD, HEAD_AGE, HOUSEHOLD, PARTNER,
    PARTNER_AGE, PARTNERS, REGION,
};
use phx_core::{Declarations, HandlerTable, ResolutionDecl, System, declare_kind};

pub use prims::Prims;

declare_kind! { pub HOUSEHOLD_KIND = "household" { legal_form: "household", table: Cells, clause: "POP.2" } }

/// Population and demography.
#[derive(Debug)]
pub struct Dem;

/// The household kind's roles, key and profile groups, from the households' vocabulary, and how it is represented.
fn declare_household(d: &mut Declarations) {
    let mut k = d.pop_kind(HOUSEHOLD);
    k.role(HEAD).role(PARTNER).key_attr(REGION).key_attr(HEAD_AGE).key_attr(PARTNERS).key_attr(PARTNER_AGE);
    for (role, count) in ADULTS.iter().zip(ADULT_COUNTS.iter()) {
        k.role(*role).key_attr(*count);
    }
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
        let _prims = Prims::declare(d);
    }

    fn handlers(_: &mut HandlerTable) {}
}
