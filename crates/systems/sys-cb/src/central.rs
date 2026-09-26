//! The corridor: each country's standing facilities' rates, compiled from its policy rate at the opening and the
//! declared spreads, and the haircut its lending facility applies to eligible loans.

use if_credit::central::Corridor;
use phx_core::{OpeningCountry, Register};
use phx_macros::clause;

use crate::consts::PERCENT;

/// A country's corridor. The policy rate is its level at the opening, a placeholder naming CB until its committee
/// sets it.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("CB.7", "CB.6", "MON.15")]
pub fn corridor(register: &Register, c: &OpeningCountry) -> Result<Corridor, String> {
    let policy = phx_ledger::opening::derived(c, "GEN.policy_rate") / PERCENT;
    Ok(Corridor {
        deposit_rate: policy - register.fixed(crate::DEPOSIT_SPREAD.id)?,
        lending_rate: policy + register.fixed(crate::LENDING_SPREAD.id)?,
        haircut: register.fixed(crate::LOAN_HAIRCUT.id)?,
    })
}
