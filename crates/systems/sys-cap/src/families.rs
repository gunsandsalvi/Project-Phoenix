//! The family of plant: wear moves units only along a chain of classes, each class after the newest receiving from
//! its holder exactly what the class before it gave, so no unit of plant is made or lost by wearing, only retired from
//! the last class.

use std::collections::{BTreeMap, BTreeSet};

use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Unit, Worn, declare_family,
};
use phx_id::PartyId;
use phx_macros::clause;

use crate::kinds::Kinds;

declare_family! { pub STOCK = "CAP.stock" { mode: Streaming, clause: "CAP.8" } }

/// The family, reading the day's wear from the audit's own record of its legs.
#[derive(Debug)]
pub struct Stock;

/// Where a wear's legs make or lose units: per holder and chain of `classes`, each class that received other than
/// what the class before it gave, the newest receiving nothing and the last's leavers retiring. Each gap is the
/// holder's, with its size in units.
#[clause("CAP.8")]
#[must_use]
pub fn gaps(worn: &Worn, classes: u32) -> Vec<(PartyId, i64, String)> {
    let mut moved: BTreeMap<(PartyId, u32, u32), (i64, i64)> = BTreeMap::new();
    for leg in &worn.legs {
        let (given, received) = moved.entry((leg.party, leg.chain, leg.class)).or_insert((0, 0));
        if leg.qty < 0 { *given -= leg.qty } else { *received += leg.qty }
    }
    let held: BTreeSet<(PartyId, u32)> = moved.keys().map(|(p, c, _)| (*p, *c)).collect();
    let mut out = Vec::new();
    for (party, chain) in held {
        let at = |class: u32| moved.get(&(party, chain, class)).copied().unwrap_or((0, 0));
        for class in 0..classes {
            let received = at(class).1;
            let before = class.checked_sub(1).map_or(0, |c| at(c).0);
            if received != before {
                out.push((
                    party,
                    received - before,
                    format!(
                        "instruction {}: class {class} of chain {chain} received {received} where the class before \
                         gave {before}",
                        worn.instruction
                    ),
                ));
            }
        }
        for (&(_, _, class), &(given, received)) in moved.range((party, chain, classes)..=(party, chain, u32::MAX)) {
            out.push((
                party,
                received - given,
                format!(
                    "instruction {}: class {class} of chain {chain} beyond its {classes} classes",
                    worn.instruction
                ),
            ));
        }
    }
    out
}

impl AuditFamily for Stock {
    fn decl(&self) -> FamilyDecl {
        STOCK
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let Some(kinds) = ctx.own::<Kinds>(<crate::Cap as phx_core::System>::CODE) else { return 0 };
        let Ok(classes) = u32::try_from(kinds.classes) else { return 0 };
        let worn = ctx.legs().worn();
        for w in &worn {
            for (party, size, detail) in gaps(w, classes) {
                findings.record(Finding {
                    family: STOCK.name,
                    clause: STOCK.clause,
                    owner: FindingOwner::Party(party),
                    size: i128::from(size),
                    unit: Unit::Count,
                    day: ctx.day(),
                    detail,
                });
            }
        }
        phx_rand::float::len_u64(worn.len())
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        phx_ledger::audit::inject_wear(target, 0)
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{Worn, WornLeg};
    use phx_id::PartyId;

    use super::gaps;

    fn leg(chain: u32, class: u32, qty: i64) -> WornLeg {
        WornLeg { party: PartyId::new(7), chain, class, qty }
    }

    #[test]
    fn wear_along_a_chain_makes_and_loses_nothing() {
        let good = vec![leg(0, 0, -5), leg(0, 1, 5), leg(0, 1, -2), leg(0, 2, 2), leg(0, 3, -1)];
        assert!(gaps(&Worn { instruction: 1, legs: good }, 4).is_empty(), "the last class's leavers retire");
        let made = vec![leg(0, 0, -5), leg(0, 1, 6)];
        assert_eq!(gaps(&Worn { instruction: 2, legs: made }, 4).first().map(|g| g.1), Some(1));
        let newest = vec![leg(1, 0, 3)];
        assert_eq!(gaps(&Worn { instruction: 3, legs: newest }, 4).len(), 1, "nothing wears into the newest class");
        let lost = vec![leg(0, 0, -5)];
        assert_eq!(gaps(&Worn { instruction: 4, legs: lost }, 4).first().map(|g| g.1), Some(-5));
        let beyond = vec![leg(0, 3, -1), leg(0, 4, 1)];
        assert_eq!(gaps(&Worn { instruction: 5, legs: beyond }, 4).len(), 1);
    }
}
