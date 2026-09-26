//! The family of goods: per good and place, what was in existence at the day's opening plus what was made there and
//! what arrived equals what was used up, shipped out, spoiled and destroyed there plus what is left; trades only move
//! units between holders, and each transformation's source accounts for its units only in the way it can.

use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, GoodStock, InjectTarget, StockDay,
    Transformed, Unit, declare_family,
};
use phx_macros::clause;
use phx_num::Missing;

declare_family! { pub GOODS = "GDS.goods" { mode: Streaming, clause: "GDS.10" } }

/// The family, reading each good's day from the audit's own record of its legs.
#[derive(Debug)]
pub struct Goods;

/// Units a source can make: a way's output and a deposit's extraction; a shipment's arrivals are counted apart.
fn makes(source: Transformed) -> bool {
    matches!(source, Transformed::Way | Transformed::Deposit)
}

/// Units a source can use up: a way's inputs, a purchase consumed, spoiling in store and a hazard's destruction.
fn uses(source: Transformed) -> bool {
    matches!(source, Transformed::Way | Transformed::Purchase | Transformed::Spoilage | Transformed::Hazard)
}

/// Where a good's day does not balance, each with its size in units: units a source cannot account for, units a trade
/// made or lost, and units moved by no transformation, each once; then the opening plus what its ways and deposits
/// made and what shipments brought, less what its ways and purchases used up, what shipments took away, what spoiled
/// and what was destroyed, and what those strays moved, against what is in existence at the close.
#[clause("GDS.10", "GDS.12")]
#[must_use]
pub fn gaps(day: &StockDay, closing: i64) -> Vec<(i128, String)> {
    let mut out = Vec::new();
    let (mut produced, mut consumed, mut spoiled, mut destroyed) = (0_i128, 0_i128, 0_i128, 0_i128);
    let (mut arrived, mut shipped) = (0_i128, 0_i128);
    let mut strays = day.traded + day.unaccounted;
    for &(source, made, used) in &day.transformed {
        if source == Transformed::Carried {
            arrived += made;
            shipped += used;
            continue;
        }
        if makes(source) {
            produced += made;
        } else if made != 0 {
            strays += made;
            out.push((made, format!("{made} units made by {source:?}, which makes none")));
        }
        match source {
            Transformed::Spoilage => spoiled += used,
            Transformed::Hazard => destroyed += used,
            _ if uses(source) => consumed += used,
            _ if used != 0 => {
                strays -= used;
                out.push((-used, format!("{used} units used up by {source:?}, which uses none")));
            }
            _ => {}
        }
    }
    if day.traded != 0 {
        out.push((day.traded, format!("trades made {} units", day.traded)));
    }
    if day.unaccounted != 0 {
        out.push((day.unaccounted, format!("{} units moved by no transformation", day.unaccounted)));
    }
    let expected = i128::from(day.opening) + produced + arrived - consumed - shipped - spoiled - destroyed + strays;
    if expected != i128::from(closing) {
        out.push((
            i128::from(closing) - expected,
            format!(
                "opening {} plus {produced} produced and {arrived} arrived less {consumed} consumed, {shipped} \
                 shipped, {spoiled} spoiled and {destroyed} destroyed, with {strays} moved otherwise, makes \
                 {expected}, where {closing} are in existence",
                day.opening
            ),
        ));
    }
    out
}

impl AuditFamily for Goods {
    fn decl(&self) -> FamilyDecl {
        GOODS
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let books = ctx.books();
        let mut checked = 0;
        for day in ctx.legs().stocks() {
            let Missing::Present(GoodStock { instrument, product, grade, zone, issued }) = books.good(day.account)
            else {
                continue;
            };
            checked += 1;
            for (size, detail) in gaps(&day, issued) {
                findings.record(Finding {
                    family: GOODS.name,
                    clause: GOODS.clause,
                    owner: FindingOwner::Instrument(instrument),
                    size,
                    unit: Unit::Count,
                    day: ctx.day(),
                    detail: format!("good of product {product}, grade {grade} in zone {zone}: {detail}"),
                });
            }
        }
        checked
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        phx_ledger::audit::inject_good(target)
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{StockDay, Transformed};

    use super::gaps;

    fn day(transformed: Vec<(Transformed, i128, i128)>) -> StockDay {
        StockDay { account: 1, opening: 100, traded: 0, unaccounted: 0, transformed }
    }

    #[test]
    fn a_good_balances_by_place() {
        let d = day(vec![
            (Transformed::Deposit, 40, 0),
            (Transformed::Way, 10, 5),
            (Transformed::Purchase, 0, 20),
            (Transformed::Spoilage, 0, 3),
            (Transformed::Hazard, 0, 2),
        ]);
        assert!(gaps(&d, 120).is_empty(), "100 + 50 - 25 - 3 - 2 = 120");
        let carried = day(vec![(Transformed::Carried, 30, 12)]);
        assert!(gaps(&carried, 118).is_empty(), "100 + 30 arrived - 12 shipped = 118");
        assert_eq!(gaps(&d, 121).first().map(|g| g.0), Some(1));
        let wrong = day(vec![(Transformed::Spoilage, 4, 0), (Transformed::Deposit, 0, 4), (Transformed::Wear, 0, 1)]);
        assert_eq!(gaps(&wrong, 99).len(), 3, "spoilage never makes, a deposit never uses up, wear is not a good's");
        let traded = StockDay { traded: 2, unaccounted: -1, ..day(Vec::new()) };
        assert_eq!(gaps(&traded, 101).len(), 2, "strays are found once, not again in the balance");
        assert_eq!(gaps(&day(Vec::new()), 101).len(), 1, "units in existence that no leg moved");
    }
}
