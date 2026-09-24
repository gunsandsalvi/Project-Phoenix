use phx_core::{AuditFamily, FamilyCtx, FamilyDecl, Finding, Findings, InjectTarget, declare_family};
use phx_id::MarketId;
use phx_num::PriceRaw;

use crate::markets::Markets;
use crate::print::{Mark, MarkSource};

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

    /// A mark dated today from a fixing the tape never published.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let day = target.day();
        let Some(markets) = target.markets().downcast_mut::<Markets>() else {
            return Err("the save's markets are not the market crate's".to_owned());
        };
        let fixing = markets.tape.fixings().len();
        let mark =
            Mark { market: MarketId::new(0), day, price: PriceRaw::from_raw(1), source: MarkSource::Fixing(fixing) };
        markets.tape.mark_untraced(mark);
        Ok(())
    }
}
