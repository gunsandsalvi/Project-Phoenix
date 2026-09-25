//! DEM, population and demography: the household kind, its persons and their roles, and the opening's households;
//! mortality, illness and coming of age are processes on its persons.

mod compose;
mod consts;
mod household;
mod life;
mod lines;
mod opening;
mod prims;
mod processes;

use if_pop::{ADULT, CHILD, EDUCATION, HEAD, HEALTH, HOUSEHOLD, PARTNER, REGION, SEX};
use phx_core::{
    Declarations, EventKindDecl, HandlerTable, SetupValue, StreamDef, System, declare_hazard, declare_kind,
    declare_stream,
};

pub use opening::Households;
pub use prims::Prims;
pub use processes::{Majority, Mortality, Onset};

declare_kind! { pub HOUSEHOLD_KIND = "household" { legal_form: "household", table: Cells, clause: "POP.2" } }

declare_stream! { pub RegionsStream = "DEM.opening_regions" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub PersonsStream = "DEM.opening_persons" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub CompositionStream = "DEM.opening_composition" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub HealthStream = "DEM.opening_health" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub EducationStream = "DEM.opening_education" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub MeansStream = "DEM.opening_means" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub MortalityStream = "DEM.mortality" { purpose: Mortality, keyed: false, clause: "CHN.3" } }
declare_stream! { pub IllnessStream = "DEM.illness" { purpose: Illness, keyed: false, clause: "CHN.3" } }
declare_stream! { pub BirthdayStream = "DEM.birthdays" { purpose: Birthday, keyed: false, clause: "CHN.3" } }

declare_hazard! {
    pub DEATH = "DEM.death" {
        acts_on: Persons("household"), rate: "DEM.survival_logit_standard", axes: ["DEM.age", "DEM.sex"],
        changes: [Birthday], outcome: "DEM.died", scheme: Scheduled, stream: "DEM.mortality",
        source: "UN World Population Prospects 2024 life tables by Brass's relational model", clause: "POP.3",
    }
}
declare_hazard! {
    pub ONSET = "DEM.disability_onset" {
        acts_on: Persons("household"), rate: "DEM.disability_onset", axes: ["DEM.age", "DEM.sex"],
        changes: [Birthday], outcome: "DEM.disabled", scheme: Scheduled, stream: "DEM.illness",
        source: "disability prevalence by age and sex, by the owner's onset mapping", clause: "POP.4",
    }
}
declare_hazard! {
    pub BIRTHDAY = "DEM.birthday" {
        acts_on: Persons("household"), rate: "DEM.age_of_majority", axes: ["DEM.age"],
        changes: [Birthday], outcome: "DEM.aged", scheme: Scheduled, stream: "DEM.birthdays",
        source: "a child comes of age on its birthday at the country's age of majority", clause: "REP.25",
    }
}

/// Population and demography.
#[derive(Debug)]
pub struct Dem;

/// The household kind's roles, attribute and person attributes, from the households' vocabulary, and where it is
/// sited.
fn declare_household(d: &mut Declarations) {
    let mut k = d.pop_kind(HOUSEHOLD);
    k.role(HEAD).role(PARTNER).role(ADULT).role(CHILD).attr(REGION);
    k.person_attr(SEX).person_attr(HEALTH).person_attr(EDUCATION);
    k.sited_by(REGION.name);
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
            MeansStream::DECL,
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
        d.contribution(Box::new(opening::Declared));
        d.contribution(Box::new(Households { prims }));
        d.pop_process(Box::new(Mortality::new(prims)));
        d.pop_process(Box::new(Onset { prims }));
        d.pop_process(Box::new(Majority::new(prims)));
    }

    fn handlers(_: &mut HandlerTable) {}
}
