//! DEM, population and demography: the household kind, its persons and their roles, and the opening's households;
//! mortality, illness, leaving school, the decision to try for a child and conception are processes on its persons.

mod births;
mod compose;
mod consts;
mod fertility;
mod household;
mod life;
mod opening;
pub mod points;
mod prims;
mod processes;
mod school;

use if_pop::fertility::{IDEAL, TRYING};
use if_pop::{ADULT, CHILD, EDUCATION, HEAD, HEALTH, HOUSEHOLD, PARTNER, REGION, SEX};
use phx_core::{
    Declarations, EventKindDecl, SetupValue, StreamDef, System, declare_hazard, declare_kind, declare_stream,
};

pub use births::{Conception, Fertility};
pub use fertility::{ChildIn, Scale, tries, value};
pub use opening::{Formed, draw_country};
pub use prims::{HEIRLESS_TO, Prims};
pub use processes::{Mortality, Onset};
pub use school::LeavingSchool;

declare_kind! { pub HOUSEHOLD_KIND = "household" { legal_form: "household", place: Sited, store: "households", clause: "POP.2" } }

declare_stream! { pub RegionsStream = "DEM.opening_regions" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub PersonsStream = "DEM.opening_persons" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub CompositionStream = "DEM.opening_composition" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub HealthStream = "DEM.opening_health" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub EducationStream = "DEM.opening_education" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub MeansStream = "DEM.opening_means" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub MortalityStream = "DEM.mortality" { family: World, purpose: Mortality, keyed: false, clause: "CHN.3" } }
declare_stream! { pub IllnessStream = "DEM.illness" { family: World, purpose: Illness, keyed: false, clause: "CHN.3" } }
declare_stream! { pub BirthdayStream = "DEM.birthdays" { family: World, purpose: Birthday, keyed: false, clause: "CHN.3" } }
declare_stream! { pub OccasionStream = "DEM.fertility_taste" { family: World, purpose: Taste, keyed: false, clause: "POP.10" } }
declare_stream! { pub ConceptionStream = "DEM.conception" { family: World, purpose: Conception, keyed: false, clause: "CHN.3" } }

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
        acts_on: Persons("household"), rate: "DEM.school_leaving_age", axes: ["DEM.age"],
        changes: [Birthday], outcome: "DEM.left_school", scheme: Scheduled, stream: "DEM.birthdays",
        source: "a child leaves school on its birthday at the country's school-leaving age", clause: "REP.25",
    }
}
declare_hazard! {
    pub OCCASION = "DEM.fertility_occasion" {
        acts_on: Persons("household"), rate: "DEM.ideal_children", axes: ["DEM.age"],
        changes: [Birthday], outcome: "DEM.decided", scheme: Scheduled, stream: "DEM.fertility_taste",
        source: "a household decides whether to try for a child on its head's birthday", clause: "POP.10",
    }
}
declare_hazard! {
    pub CONCEPTION = "DEM.conception" {
        acts_on: Persons("household"), rate: "DEM.fecundability", axes: ["DEM.age", "DEM.sex"],
        changes: [Birthday], outcome: "DEM.born", scheme: Scheduled, stream: "DEM.conception",
        source: "fecundability by age: Wesselink et al. (2017) and Leridon (2004)", clause: "POP.5",
    }
}

/// Population and demography.
#[derive(Debug)]
pub struct Dem;

/// The household kind's roles, attribute and person attributes, from the households' vocabulary, and where it is
/// sited.
fn declare_household(d: &mut Declarations) {
    let mut k = d.pop_kind(HOUSEHOLD);
    k.role(HEAD).role(PARTNER).role(ADULT).role(CHILD).attr(REGION).attr(TRYING).attr(IDEAL);
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
        for stream in [
            MortalityStream::DECL,
            IllnessStream::DECL,
            BirthdayStream::DECL,
            OccasionStream::DECL,
            ConceptionStream::DECL,
        ] {
            d.stream(stream);
        }
        d.event(EventKindDecl { name: "DEM.died", size_unit: "persons", clause: "POP.3" });
        d.event(EventKindDecl { name: "DEM.disabled", size_unit: "persons", clause: "POP.4" });
        d.event(EventKindDecl { name: "DEM.left_school", size_unit: "persons", clause: "REP.25" });
        d.event(EventKindDecl { name: "DEM.decided", size_unit: "persons", clause: "POP.10" });
        d.event(EventKindDecl { name: "DEM.born", size_unit: "persons", clause: "POP.5" });
        for hazard in [DEATH, ONSET, BIRTHDAY, OCCASION, CONCEPTION] {
            d.hazard(hazard);
        }
        let prims = Prims::declare(d);
        d.setup_value(SetupValue { prim: &prims::LIFE_EXPECTANCY, derived: "GEN.life_expectancy" });
        d.decision(&points::TRY_FOR_CHILD);
        d.pop_process(Box::new(Mortality::new(prims)));
        d.pop_process(Box::new(Onset { prims }));
        d.pop_process(Box::new(LeavingSchool::new(prims)));
        d.pop_process(Box::new(Fertility::new(prims)));
        d.pop_process(Box::new(Conception::new(prims)));
    }
}
