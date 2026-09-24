use phx_core::{AuditFamily, FamilyCtx, FamilyDecl, Finding, Findings, InjectTarget, declare_family};

declare_family! { pub PRICES = "MKT.prices" { mode: Incremental, clause: "MKT.14" } }

/// The day's prints, each traced to its match set and trading together what it says at its price, and the day's
/// marks, each from a print or a fixing of its market.
#[derive(Debug)]
pub struct Prices;

impl AuditFamily for Prices {
    fn decl(&self) -> FamilyDecl {
        PRICES
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let (checked, gaps) = ctx.markets().prices(ctx.day());
        for g in gaps {
            findings.record(Finding {
                family: PRICES.name,
                clause: PRICES.clause,
                owner: g.owner,
                size: g.size,
                unit: g.unit,
                day: ctx.day(),
                detail: g.detail,
            });
        }
        checked
    }

    fn inject(&self, _: &mut dyn InjectTarget) -> Result<(), String> {
        Err("the tape is injected into a save loaded apart, which persistence brings".to_owned())
    }
}
