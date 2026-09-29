//! Plant's decision point: a firm's chief executive's investment.

use phx_core::decisions::DecisionPointDecl;
use phx_macros::clause;
use phx_num::Missing;

use crate::rules::invest::{invests, waiting_multiple};

/// What a firm's investment reads: the project's value — the margin its extra output earns a year as an annuity over
/// the plant's life — what it costs, the return the firm requires, the margin a year, the volatility of its sales a
/// year, and the money it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvestIn {
    pub value: f64,
    pub outlay: f64,
    pub rate: f64,
    pub earned: f64,
    pub sigma: f64,
    pub money: f64,
}

/// Whether it invests: the value beats the cost after the value of waiting, and it can fund it.
#[clause("CAP.3", "CAP.11")]
#[must_use]
pub fn invest(i: &InvestIn) -> bool {
    let waiting = waiting_multiple(i.rate, i.earned / i.value, i.sigma);
    invests(i.value, i.outlay, 0.0, waiting, i.money >= i.outlay)
}

/// A firm's investment, on its plant's review.
pub const INVEST: DecisionPointDecl<InvestIn, bool> = DecisionPointDecl {
    name: "CAP.invest",
    system: "CAP",
    rule: invest,
    schedule: Missing::Present("CAP.review_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "CAP.3",
};
