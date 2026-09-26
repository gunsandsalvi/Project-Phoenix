//! The firms' families: every revenue is somebody's outlay and every cost somebody's income, and what a party is
//! owed and owes is the sum of the claims it holds.

use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Unit, declare_family,
};
use phx_macros::clause;

declare_family! { pub REVENUE = "FRM.revenue" { mode: Streaming, clause: "FRM.17" } }
declare_family! { pub INVOICES = "FRM.invoices" { mode: Incremental, clause: "FRM.18" } }

/// The day's income over every party: what each earned from a due or a payment against what its payer spent.
#[derive(Debug)]
pub struct Revenue;

/// Each party's receivables and payables against the claims it holds: the invoices, until trade credit writes its own.
#[derive(Debug)]
pub struct Invoices;

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

impl AuditFamily for Invoices {
    fn decl(&self) -> FamilyDecl {
        INVOICES
    }

    #[clause("FRM.18")]
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let (checked, gaps) = ctx.accounts().invoices();
        for g in gaps {
            findings.record(Finding {
                family: INVOICES.name,
                clause: INVOICES.clause,
                owner: g.owner,
                size: g.size,
                unit: g.unit,
                day: ctx.day(),
                detail: g.detail,
            });
        }
        checked
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        phx_acct::audit::inject_invoices(target)
    }
}
