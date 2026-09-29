//! Social protection's decision points: a person's claim to the benefit, and the minister's collective consumption.

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

/// The state's collective consumption, each day.
pub const CONSUME: DecisionPointDecl<ConsumeIn, Vec<i64>> = DecisionPointDecl {
    name: "SOC.consume",
    system: "SOC",
    rule: consume,
    schedule: Missing::Present("SOC.daily"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "SOC.2",
};
