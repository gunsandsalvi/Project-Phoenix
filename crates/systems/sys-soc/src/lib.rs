//! SOC, social protection: each person over the state pension age draws the state pension, the rule's flat amount,
//! from the treasury, at the country's coverage, the opening's pensioners and each person who retires alike; and a person who loses its job may claim the benefit, a share of its
//! last wage for a declared term. Its other benefits and their rules arrive with their own steps.

pub mod benefit;
mod consts;
mod state_pension;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_prim};
use phx_num::{Count, Fixed};

pub use state_pension::{CoveredStream, PENSIONS, PensionStream, Pensions, STATE_PENSION, StatePension};

declare_prim! {
    /// The normal pension age by sex, in years.
    pub PENSION_AGE = "SOC.pension_age" {
        kind: Policy, decided_by: "parliament", value: Table1 { axis_exp: 0, exp: 2 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The mandatory schemes' gross replacement rate for a worker on average earnings, by sex.
    pub REPLACEMENT = "SOC.replacement_rate" {
        kind: Policy, decided_by: "parliament", value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The share of persons over the pension age receiving an old-age pension, by sex.
    pub COVERAGE = "SOC.pension_coverage" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The share of persons with severe disabilities receiving a disability benefit, by sex, which the benefits read.
    pub DISABILITY_COVERAGE = "SOC.disability_benefit_coverage" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The benefit's monthly amount as a share of the claimant's last monthly wage.
    pub BENEFIT_REPLACEMENT = "SOC.benefit_replacement" {
        kind: Policy, decided_by: "parliament", value: Fixed { exp: 3 }, clause: "SOC.1", scope: PerCountry
    }
}

declare_prim! {
    /// The months the benefit pays.
    pub BENEFIT_MONTHS = "SOC.benefit_months" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "SOC.1", scope: PerCountry
    }
}

declare_prim! {
    /// The hours claiming the benefit takes.
    pub CLAIM_HOURS = "SOC.claim_hours" { kind: Technology, value: Fixed { exp: 1 }, clause: "SOC.3", scope: Shared }
}

/// Social protection.
#[derive(Debug)]
pub struct Soc;

impl System for Soc {
    const CODE: &'static str = "SOC";

    fn declare(d: &mut Declarations) {
        d.stream(PensionStream::DECL);
        d.stream(CoveredStream::DECL);
        let _ =
            StatePension { age: d.prim(&PENSION_AGE), replacement: d.prim(&REPLACEMENT), coverage: d.prim(&COVERAGE) };
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&DISABILITY_COVERAGE);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&BENEFIT_REPLACEMENT);
        let _: phx_core::Prim<Count> = d.prim(&BENEFIT_MONTHS);
        let _: phx_core::Prim<Fixed<1>> = d.prim(&CLAIM_HOURS);
        d.market(Box::new(benefit::BENEFITS));
        d.market(Box::new(PENSIONS));
    }

    fn handlers(_: &mut HandlerTable) {}
}
