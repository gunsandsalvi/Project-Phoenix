//! DEM's primitives: the demography and households each country opens with, and the representation of households.

use phx_core::register::values::{Distribution, Partition, Table2};
use phx_core::{Declarations, Prim, declare_prim};
use phx_num::{Count, Fixed};

declare_prim! {
    /// Brass's logit of survivorship to each age by sex, the country's life table's standard.
    pub SURVIVAL = "DEM.survival_logit_standard" {
        kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "POP.16", scope: PerCountry
    }
}
declare_prim! {
    /// Males born per hundred females.
    pub SEX_RATIO = "DEM.sex_ratio_at_birth" { kind: Technology, value: Fixed { exp: 2 }, clause: "POP.16", scope: PerCountry }
}
declare_prim! {
    /// The standard share of the population at each single age and sex.
    pub AGE_STANDARD = "DEM.age_standard" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// The share disabled lastingly by age band and sex.
    pub DISABILITY = "DEM.disability_prevalence" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// The yearly hazard of lasting disability's onset by age and sex.
    pub ONSET = "DEM.disability_onset" {
        kind: Technology, value: Table2 { row_exp: 2, column_exp: 0, exp: 6 }, clause: "POP.16", scope: PerCountry
    }
}
declare_prim! {
    /// Households' basic types as log ratios on the log of total fertility.
    pub TYPES = "DEM.household_types" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// Households' sizes as log ratios on the log of total fertility.
    pub SIZES = "DEM.household_sizes" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// How persons of 65 and over live, by sex.
    pub OLDER = "DEM.older_living_arrangements" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// A parent's expected living children by the parent's age and the children's age band.
    pub LIVING_CHILDREN = "DEM.living_children" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "POP.1", scope: PerCountry
    }
}
declare_prim! {
    /// A mother's expected living children at each single age under the age of majority.
    pub MINOR_CHILDREN = "DEM.minor_children" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "POP.1", scope: PerCountry
    }
}
declare_prim! {
    /// Women's highest education by age band.
    pub EDUCATION_FEMALE = "DEM.education_female" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// Men's highest education by age band.
    pub EDUCATION_MALE = "DEM.education_male" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}
declare_prim! {
    /// The shape of income, a lognormal body with a Pareto top.
    pub INCOME = "DEM.income_shape" { kind: Endowment, value: Distribution { exp: 6 }, clause: "GEN.2", scope: PerCountry }
}
declare_prim! {
    /// The shape of wealth, a lognormal body with a Pareto top.
    pub WEALTH = "DEM.wealth_shape" { kind: Endowment, value: Distribution { exp: 6 }, clause: "GEN.2", scope: PerCountry }
}
declare_prim! {
    /// Who each basic type of household holds besides its head: a partner, children, an older relative, another
    /// adult, one or none of each.
    pub MEMBERS = "DEM.household_members" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 0 }, clause: "GEN.2", scope: Shared
    }
}
declare_prim! {
    /// A man's age less his female partner's, in years.
    pub PARTNER_GAP = "DEM.partner_age_gap" { kind: Endowment, value: Distribution { exp: 2 }, clause: "GEN.2", scope: PerCountry }
}
declare_prim! {
    /// The age classes the key holds persons in, by their first ages.
    pub AGE_CLASSES = "DEM.age_classes" { kind: Resolution, value: Partition { exp: 0 }, clause: "REP.25", scope: Shared }
}
declare_prim! {
    /// The age of majority, below which a person is a child of its household.
    pub MAJORITY = "DEM.age_of_majority" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "POP.16", scope: PerCountry
    }
}
declare_prim! {
    /// The most household cells the representation keeps.
    pub CELL_BUDGET = "DEM.cell_budget" { kind: Resolution, value: Count, clause: "REP.18", scope: Shared }
}

/// DEM's primitives as its opening and processes read them.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub survival: Prim<Table2>,
    pub sex_ratio: Prim<Fixed<2>>,
    pub age_standard: Prim<Table2>,
    pub disability: Prim<Table2>,
    pub onset: Prim<Table2>,
    pub types: Prim<Table2>,
    pub sizes: Prim<Table2>,
    pub older: Prim<Table2>,
    pub living_children: Prim<Table2>,
    pub minor_children: Prim<Table2>,
    pub education: [Prim<Table2>; 2],
    pub income: Prim<Distribution>,
    pub wealth: Prim<Distribution>,
    pub age_classes: Prim<Partition>,
    pub majority: Prim<Count>,
    pub members: Prim<Table2>,
    pub partner_gap: Prim<Distribution>,
}

impl Prims {
    pub fn declare(d: &mut Declarations) -> Prims {
        let _: Prim<Count> = d.prim(&CELL_BUDGET);
        Prims {
            survival: d.prim(&SURVIVAL),
            sex_ratio: d.prim(&SEX_RATIO),
            age_standard: d.prim(&AGE_STANDARD),
            disability: d.prim(&DISABILITY),
            onset: d.prim(&ONSET),
            types: d.prim(&TYPES),
            sizes: d.prim(&SIZES),
            older: d.prim(&OLDER),
            living_children: d.prim(&LIVING_CHILDREN),
            minor_children: d.prim(&MINOR_CHILDREN),
            education: [d.prim(&EDUCATION_FEMALE), d.prim(&EDUCATION_MALE)],
            income: d.prim(&INCOME),
            wealth: d.prim(&WEALTH),
            age_classes: d.prim(&AGE_CLASSES),
            majority: d.prim(&MAJORITY),
            members: d.prim(&MEMBERS),
            partner_gap: d.prim(&PARTNER_GAP),
        }
    }
}
