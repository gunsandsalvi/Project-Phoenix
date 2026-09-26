//! The benefit for a job lost: a share of the last wage paid monthly by the treasury for a declared number of months
//! to a person who claims it on losing its job, a row on its country's line of that amount and term.

use if_state::kinds::{BenefitKind, BenefitLaw, ClaimIn};
use phx_core::{OpeningCountry, Register};
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::rows::BALANCE;
use phx_macros::clause;
use phx_num::Missing;

/// The benefit: the treasury owes it to each claimant, one member a claimant.
pub const BENEFIT: LineKindDecl = LineKindDecl {
    name: "unemployment benefit",
    asset: SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: false,
        holder_roles: &[if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name],
        exclusive: true,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &["treasury"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    dated: true,
    transfer_requesters: &["SOC"],
};

/// A claim joining its benefit's line.
pub const CLAIMED: ReasonDecl =
    ReasonDecl { name: "SOC claimed", order: 2, paid: Effect::Equity, received: Effect::Equity, held: Missing::Absent };

/// A country's benefit.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("SOC.1", "SOC.3")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<BenefitLaw, String> {
    Ok(BenefitLaw {
        replacement: register.fixed_in(crate::BENEFIT_REPLACEMENT.id, c.id)?,
        months: u32::try_from(register.count_in(crate::BENEFIT_MONTHS.id, c.id)?).map_err(|e| e.to_string())?,
        claim_hours: register.fixed(crate::CLAIM_HOURS.id)?,
    })
}

/// A person's claim: when what the benefit pays over its term is worth more than the hours claiming it takes.
#[clause("SOC.3", "SOC.7")]
#[must_use]
pub fn claim(i: &ClaimIn) -> bool {
    i.monthly * i.months > i.claiming_cost
}

/// The benefit, which the kernel binds.
pub const BENEFITS: BenefitKind = BenefitKind { line: BENEFIT.name, claimed: CLAIMED.name, law, claim };

#[cfg(test)]
mod tests {
    use if_state::kinds::ClaimIn;

    use super::claim;

    #[test]
    fn claim_decision_value_against_cost() {
        assert!(claim(&ClaimIn { monthly: 1_000.0, months: 6.0, claiming_cost: 50.0 }));
        assert!(!claim(&ClaimIn { monthly: 5.0, months: 2.0, claiming_cost: 50.0 }), "not worth the hours");
    }
}
