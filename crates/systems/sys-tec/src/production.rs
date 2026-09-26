//! The family of production: every unit made came by a registered way, from the inputs that way states for what
//! was started, and from nothing else.

use if_base::WayId;
use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Made, Unit, declare_family,
};
use phx_ledger::instruction::Denom;
use phx_macros::clause;
use phx_num::QtyRaw;

use crate::technology::Technology;

declare_family! { pub PRODUCTION = "TEC.production" { mode: Streaming, clause: "TEC.9" } }

/// The family, reading the technology its system compiled.
#[derive(Debug)]
pub struct Production;

/// What a production's legs lack against its way: none when they are what the way states.
#[clause("TEC.9")]
#[must_use]
pub fn gaps(tech: &Technology, made: &Made) -> Vec<String> {
    let Some(way) = tech.way(WayId::new(made.way)) else {
        return vec![format!(
            "instruction {} names way {}, which the register does not hold",
            made.instruction, made.way
        )];
    };
    let code = |p: if_base::ProductId| crate::products::get(&tech.products, p).map(|d| Denom::Unit(d.unit).code());
    let Some(output) = code(way.product) else {
        return vec![format!("way {} makes a product the register does not hold", made.way)];
    };
    let sum = |denom: u32, positive: bool| -> i64 {
        made.legs.iter().filter(|l| l.denom == denom && (l.qty > 0) == positive).map(|l| l.qty).sum()
    };
    let finished = sum(output, true);
    let started = crate::ways::started_for(way, QtyRaw::from_raw(finished));
    let mut out = Vec::new();
    let mut named = vec![output];
    for (product, per) in &way.inputs {
        let Some(denom) = code(*product) else { continue };
        named.push(denom);
        let took = -sum(denom, false);
        let states = crate::ways::takes(started, *per).raw();
        if took != states {
            out.push(format!(
                "instruction {} finished {finished} by way {} and used {took} of product {} where the way states {states}",
                made.instruction,
                made.way,
                product.index()
            ));
        }
    }
    for leg in made.legs.iter().filter(|l| !named.contains(&l.denom)) {
        out.push(format!(
            "instruction {} moved {} in a denomination way {} neither makes nor uses",
            made.instruction, leg.qty, made.way
        ));
    }
    out
}

impl AuditFamily for Production {
    fn decl(&self) -> FamilyDecl {
        PRODUCTION
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let Some(tech) = ctx.own::<Technology>(<crate::Tec as phx_core::System>::CODE) else { return 0 };
        let made = ctx.legs().made();
        for m in &made {
            for detail in gaps(tech, m) {
                findings.record(Finding {
                    family: PRODUCTION.name,
                    clause: PRODUCTION.clause,
                    owner: FindingOwner::Run,
                    size: 1,
                    unit: Unit::Count,
                    day: ctx.day(),
                    detail,
                });
            }
        }
        phx_rand::float::len_u64(made.len())
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        phx_ledger::audit::inject_production(target, 0)
    }
}
