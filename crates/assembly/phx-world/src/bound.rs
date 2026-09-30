//! The kinds and families the day's code routes by, each bound once to its place among the core's kinds and families —
//! as they are declared and opened, and again at load — so no day path finds one by its name.

use phx_macros::{clause, opening};

use crate::consts::families;
use crate::core_day::DatedFamily;

/// The kinds the core routes by, each the place of the kind its system declares; none where the world keeps none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindsBound {
    pub central_bank: Option<usize>,
    pub treasury: Option<usize>,
    pub bank: Option<usize>,
    pub firm: Option<usize>,
    pub estate: Option<usize>,
    pub household: Option<usize>,
    pub agency: Option<usize>,
}

/// The families the core routes by, each its place among the core's families; none until it is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FamiliesBound {
    pub employment: Option<usize>,
    pub public_employment: Option<usize>,
    pub household_loans: Option<usize>,
    pub firm_loans: Option<usize>,
    pub benefit: Option<usize>,
    pub pension: Option<usize>,
    pub deposit_facility: Option<usize>,
    pub lending_facility: Option<usize>,
    pub bills: Option<usize>,
}

/// The kinds and families bound, and whether each family's contracts are jobs, as its declaration says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bound {
    pub kinds: KindsBound,
    pub families: FamiliesBound,
    pub jobs: Vec<bool>,
}

impl Bound {
    /// The handles found among the core's kinds, by the names their systems declare them under, and its families, by
    /// the names they are opened under.
    #[opening]
    #[must_use]
    pub fn of(names: &[&str], opened: &[DatedFamily]) -> Bound {
        let kind = |name: &str| names.iter().position(|n| *n == name);
        let family = |name: &str| opened.iter().position(|f| f.name == name);
        Bound {
            kinds: KindsBound {
                central_bank: kind(sys_cb::CENTRAL_BANK.name),
                treasury: kind(sys_cb::TREASURY.name),
                bank: kind(sys_bnk::BANK.name),
                firm: kind(sys_frm::FIRM.name),
                estate: kind(phx_core::ESTATE_KIND.name),
                household: kind(sys_dem::HOUSEHOLD_KIND.name),
                agency: kind(sys_soc::AGENCY.name),
            },
            families: FamiliesBound {
                employment: family(families::EMPLOYMENT),
                public_employment: family(families::PUBLIC_EMPLOYMENT),
                household_loans: family(families::HOUSEHOLD_LOANS),
                firm_loans: family(families::FIRM_LOANS),
                benefit: family(families::BENEFIT),
                pension: family(families::PENSION),
                deposit_facility: family(families::DEPOSIT_FACILITY),
                lending_facility: family(families::LENDING_FACILITY),
                bills: family(families::BILLS),
            },
            jobs: opened.iter().map(|f| families::JOBS.contains(&f.name)).collect(),
        }
    }

    /// Whether a family's contracts are jobs; a family beyond those the core opened stops the run.
    #[clause("PTY.4")]
    #[must_use]
    pub fn is_jobs(&self, family: usize) -> bool {
        match self.jobs.get(family) {
            Some(jobs) => *jobs,
            None => phx_num::violation!(clause = "PTY.4", "a family the core never opened", family = family),
        }
    }
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
