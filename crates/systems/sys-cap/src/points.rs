//! Plant's decision point: a firm's chief executive's fixed investment.

use phx_core::decisions::DecisionPointDecl;
use phx_macros::clause;
use phx_num::Missing;

/// What a firm's investment reads: what it invests over its production period, and each capital good's share of it.
#[derive(Clone, Debug, PartialEq)]
pub struct InvestIn {
    pub per_period: f64,
    pub shares: Vec<f64>,
}

/// What it buys of each capital good, in whole units of its currency.
#[clause("CAP.3")]
#[must_use]
pub fn invest(i: &InvestIn) -> Vec<i64> {
    i.shares.iter().map(|s| phx_rand::float::floor_to_i64(s * i.per_period).unwrap_or(0)).collect()
}

/// A firm's fixed investment, on its production schedule.
pub const INVEST: DecisionPointDecl<InvestIn, Vec<i64>> = DecisionPointDecl {
    name: "CAP.invest",
    system: "CAP",
    rule: invest,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "CAP.3",
};
