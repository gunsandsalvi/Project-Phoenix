//! SOC, social protection: here its opening alone — each person over the state pension age draws the state pension,
//! the rule's flat amount, from the treasury, at the country's coverage. Its claims, benefits and their rules arrive
//! with its own steps.

mod consts;
mod state_pension;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_prim};

pub use state_pension::{Declared, PensionStream, STATE_PENSION, StatePension};

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

/// Social protection.
#[derive(Debug)]
pub struct Soc;

impl System for Soc {
    const CODE: &'static str = "SOC";

    fn declare(d: &mut Declarations) {
        d.stream(PensionStream::DECL);
        let pension =
            StatePension { age: d.prim(&PENSION_AGE), replacement: d.prim(&REPLACEMENT), coverage: d.prim(&COVERAGE) };
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&DISABILITY_COVERAGE);
        d.contribution(Box::new(Declared));
        let draw: Box<dyn phx_ledger::attachments::AttachmentDraw> = Box::new(pension);
        d.attachment(Box::new(draw));
    }

    fn handlers(_: &mut HandlerTable) {}
}
