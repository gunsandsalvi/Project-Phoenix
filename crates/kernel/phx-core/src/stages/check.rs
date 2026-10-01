//! The stage table checked at assembly: its slots in the day's order and each once, so its stages in order, no slot both
//! business-only and run on a non-business day, every read naming a store some slot writes and never one written
//! later the same day, and the barriers within the day's budget.

use phx_macros::{clause, opening};

use super::{AsOf, Mode, SlotDecl, StageTable};
use crate::consts::{BARRIERS_BUSINESS, BARRIERS_HEAVY, BARRIERS_NON_BUSINESS};

// A heavy day walks the business day's slots, so the business day's budget holds it.
const _: () = assert!(BARRIERS_BUSINESS <= BARRIERS_HEAVY, "a heavy day's barriers within its budget");
use crate::slots::DAY_SLOT_ORDER;

/// The table, or every refusal it earns.
///
/// # Errors
/// Every inconsistency, each naming its slot.
#[opening]
#[clause("TIME.6", "TIME.8", "TIME.10")]
pub fn compile(decls: &[SlotDecl]) -> Result<StageTable, Vec<String>> {
    let mut refused = Vec::new();
    let order: Vec<_> = decls.iter().map(|d| d.slot).collect();
    if order != DAY_SLOT_ORDER {
        refused.push("the table's slots are not the day's, each once in its order".to_owned());
    }
    for d in decls {
        if d.business_only && d.on_non_business {
            refused.push(format!("slot {:?} runs only on business days and on non-business days", d.slot));
        }
        for r in d.reads {
            let writers: Vec<_> = decls.iter().filter(|w| w.writes.contains(&r.store)).map(|w| w.slot).collect();
            if writers.is_empty() {
                refused.push(format!("slot {:?} reads `{}`, which no slot writes", d.slot, r.store));
            }
            match r.as_of {
                AsOf::Today => {
                    if let Some(later) = writers.iter().find(|w| **w > d.slot) {
                        refused.push(format!(
                            "slot {:?} reads `{}` as today's, which slot {later:?} writes later",
                            d.slot, r.store
                        ));
                    }
                }
                AsOf::Through(upto) if upto > d.slot => {
                    refused.push(format!("slot {:?} reads `{}` through slot {upto:?}, after it", d.slot, r.store));
                }
                AsOf::Through(_) | AsOf::Yesterday => {}
            }
        }
    }
    let table = StageTable { slots: decls.to_vec() };
    for (any_business, budget, day) in
        [(true, BARRIERS_BUSINESS, "a business"), (false, BARRIERS_NON_BUSINESS, "a non-business")]
    {
        let barriers = table.barriers(Mode::Ordinary, any_business);
        if barriers > budget {
            refused.push(format!("{barriers} barriers on {day} day, past its {budget}"));
        }
    }
    if refused.is_empty() { Ok(table) } else { Err(refused) }
}
