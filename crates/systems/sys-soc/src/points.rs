//! Social protection's decision points: a person's claim to the benefit, and a public agency's head's purchases and
//! staff.

use if_state::kinds::ClaimIn;
use phx_core::decisions::DecisionPointDecl;
use phx_core::schedule::WakeKind;
use phx_macros::clause;
use phx_num::Missing;

/// A person's claim to its country's benefit, on losing its job.
pub const CLAIM: DecisionPointDecl<ClaimIn, bool> = DecisionPointDecl {
    name: "SOC.claim",
    system: "SOC",
    rule: crate::benefit::claim,
    schedule: Missing::Absent,
    wakes: &[WakeKind::EventConcerning],
    runs_on_non_business: true,
    clause: "SOC.8",
};

/// What the state's consumption in a region reads: what it spends there a day, and each product's share of it.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumeIn {
    pub per_day: f64,
    pub shares: Vec<f64>,
}

/// What the state buys of each product today, in whole units of its currency.
#[clause("SOC.2")]
#[must_use]
pub fn consume(i: &ConsumeIn) -> Vec<i64> {
    i.shares.iter().map(|s| phx_rand::float::floor_to_i64(s * i.per_day).unwrap_or(0)).collect()
}

/// A public agency's purchases, each day.
pub const CONSUME: DecisionPointDecl<ConsumeIn, Vec<i64>> = DecisionPointDecl {
    name: "SOC.consume",
    system: "SOC",
    rule: consume,
    schedule: Missing::Present("SOC.daily"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "SOC.2",
};

/// A post a public agency is short of: its region and occupation, the jobs it lacks, and what a job's month pays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    pub region: u32,
    pub occupation: u32,
    pub jobs: u32,
    pub wage: f64,
}

/// What its staffing reads: the posts it is short of, what its appropriation gives it for wages a month, and what
/// its staff and its open vacancies take of it.
#[derive(Clone, Debug, PartialEq)]
pub struct StaffIn {
    pub gaps: Vec<Gap>,
    pub budget: f64,
    pub bill: f64,
}

/// The vacancies it posts, by region and occupation: each post it is short of, in order, as far as its appropriation
/// pays a month's wage for it.
#[clause("SOC.8")]
#[must_use]
pub fn staff(i: &StaffIn) -> Vec<(u32, u32, u32)> {
    let mut bill = i.bill;
    let mut out = Vec::new();
    for g in &i.gaps {
        let mut jobs = 0;
        while jobs < g.jobs && bill + g.wage <= i.budget {
            bill += g.wage;
            jobs += 1;
        }
        if jobs > 0 {
            out.push((g.region, g.occupation, jobs));
        }
    }
    out
}

/// A public agency's staffing, on each business day.
pub const STAFF: DecisionPointDecl<StaffIn, Vec<(u32, u32, u32)>> = DecisionPointDecl {
    name: "SOC.staff",
    system: "SOC",
    rule: staff,
    schedule: Missing::Present("SOC.daily"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "SOC.8",
};

#[cfg(test)]
mod tests {
    use super::{Gap, StaffIn, staff};

    #[test]
    fn staff_within_the_appropriation() {
        let gaps = vec![
            Gap { region: 0, occupation: 2, jobs: 3, wage: 10.0 },
            Gap { region: 1, occupation: 4, jobs: 2, wage: 5.0 },
        ];
        assert_eq!(staff(&StaffIn { gaps: gaps.clone(), budget: 100.0, bill: 50.0 }), vec![(0, 2, 3), (1, 4, 2)]);
        assert_eq!(
            staff(&StaffIn { gaps: gaps.clone(), budget: 100.0, bill: 75.0 }),
            vec![(0, 2, 2), (1, 4, 1)],
            "as far as it pays"
        );
        assert!(staff(&StaffIn { gaps, budget: 100.0, bill: 100.0 }).is_empty(), "nothing left to pay a wage");
    }
}
