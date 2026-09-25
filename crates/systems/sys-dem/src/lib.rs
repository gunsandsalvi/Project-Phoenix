//! DEM, population and demography: the household kind, its persons as roles, and the opening's households; mortality,
//! illness and ageing arrive as processes on them.

mod compose;
mod consts;
mod household;
mod life;
mod opening;
mod prims;
mod processes;

use if_pop::{
    ADULT, ADULT_COUNT, ADULT_GROUPS, CHILD_COUNTS, CHILD_GROUPS, CHILDREN, HEAD, HEAD_AGE, HOUSEHOLD, PARTNER,
    PARTNERS, REGION,
};
use phx_core::{
    Declarations, EventKindDecl, HandlerTable, ResolutionDecl, SetupValue, StreamDef, System, declare_hazard,
    declare_kind, declare_stream,
};

pub use opening::Households;
pub use prims::Prims;
pub use processes::{Birthdays, Mortality, Onset};

declare_kind! { pub HOUSEHOLD_KIND = "household" { legal_form: "household", table: Cells, clause: "POP.2" } }

declare_stream! { pub RegionsStream = "DEM.opening_regions" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub PersonsStream = "DEM.opening_persons" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub CompositionStream = "DEM.opening_composition" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub HealthStream = "DEM.opening_health" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub EducationStream = "DEM.opening_education" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub MortalityStream = "DEM.mortality" { purpose: Mortality, keyed: false, clause: "CHN.3" } }
declare_stream! { pub IllnessStream = "DEM.illness" { purpose: Illness, keyed: false, clause: "CHN.3" } }
declare_stream! { pub BirthdayStream = "DEM.birthdays" { purpose: Birthday, keyed: false, clause: "CHN.3" } }

declare_hazard! {
    pub DEATH = "DEM.death" {
        acts_on: Persons("household"), rate: "DEM.survival_logit_standard", axes: ["DEM.birth_year", "DEM.sex"],
        changes: [YearStart], outcome: "DEM.died", scheme: Scheduled, stream: "DEM.mortality",
        source: "UN World Population Prospects 2024 life tables by Brass's relational model", clause: "POP.3",
    }
}
declare_hazard! {
    pub ONSET = "DEM.disability_onset" {
        acts_on: Persons("household"), rate: "DEM.disability_onset", axes: ["DEM.birth_year", "DEM.sex"],
        changes: [YearStart], outcome: "DEM.disabled", scheme: Scheduled, stream: "DEM.illness",
        source: "disability prevalence by age and sex, by the owner's onset mapping", clause: "POP.4",
    }
}
declare_hazard! {
    pub BIRTHDAY = "DEM.birthday" {
        acts_on: Persons("household"), rate: "DEM.age_classes", axes: ["DEM.birth_year"],
        changes: [MonthStart], outcome: "DEM.aged", scheme: Scheduled, stream: "DEM.birthdays",
        source: "birthdays spread evenly over the year", clause: "REP.25",
    }
}

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
        for stream in [MortalityStream::DECL, IllnessStream::DECL, BirthdayStream::DECL] {
            d.stream(stream);
        }
        d.event(EventKindDecl { name: "DEM.died", size_unit: "persons", clause: "POP.3" });
        d.event(EventKindDecl { name: "DEM.disabled", size_unit: "persons", clause: "POP.4" });
        d.event(EventKindDecl { name: "DEM.aged", size_unit: "persons", clause: "REP.25" });
        for hazard in [DEATH, ONSET, BIRTHDAY] {
            d.hazard(hazard);
        }
        let prims = Prims::declare(d);
        d.setup_value(SetupValue { prim: &prims::LIFE_EXPECTANCY, derived: "GEN.life_expectancy" });
        d.contribution(Box::new(Households { prims }));
        d.pop_process(Box::new(Mortality::new(prims)));
        d.pop_process(Box::new(Onset { prims }));
        d.pop_process(Box::new(Birthdays::new(prims)));
    }

    fn handlers(_: &mut HandlerTable) {}
}
