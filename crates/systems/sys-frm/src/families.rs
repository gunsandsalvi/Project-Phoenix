//! The firms' family: every revenue is somebody's outlay and every cost somebody's income.

use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Unit, declare_family,
};
use phx_macros::clause;

declare_family! { pub REVENUE = "FRM.revenue" { mode: Streaming, clause: "FRM.17" } }

/// The day's income over every party: what each earned from a due or a payment against what its payer spent.
#[derive(Debug)]
pub struct Revenue;

impl AuditFamily for Revenue {
    fn decl(&self) -> FamilyDecl {
        REVENUE
    }

    #[clause("FRM.17", "FRM.20")]
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let net = ctx.accounts().income();
        if net != 0 {
            findings.record(Finding {
                family: REVENUE.name,
                clause: REVENUE.clause,
                owner: FindingOwner::Run,
                size: net,
                unit: Unit::Count,
                day: ctx.day(),
                detail: format!("the day's income over every party sums to {net}, not nothing"),
            });
        }
        1
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        phx_acct::audit::inject_income(target)
    }
}
