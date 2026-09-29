//! Households' decision points: what a household spends, and its stance on the heuristics' menu.

use phx_core::decisions::DecisionPointDecl;
use phx_macros::clause;
use phx_num::Missing;

/// What a household's spending reads: its rule's spending at its target and propensity beyond it, the target, its
/// cash on hand in years of its income, its income's outlook a year, the share of a year to its next decision, and
/// what it can spend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpendIn {
    pub at_target: f64,
    pub kappa: f64,
    pub target: f64,
    pub cash: f64,
    pub income: f64,
    pub period: f64,
    pub free: f64,
}

/// What it spends until its next decision, never more than it can.
#[clause("HH.4", "HH.18")]
#[must_use]
pub fn spend(i: &SpendIn) -> f64 {
    let wanted = crate::buffer::spend_near((i.at_target, i.kappa, i.target), i.cash) * i.income * i.period;
    if wanted < i.free { wanted } else { i.free }
}

/// A household's spending, on its spending schedule.
pub const SPEND: DecisionPointDecl<SpendIn, f64> = DecisionPointDecl {
    name: "HH.spend",
    system: "HH",
    rule: spend,
    schedule: Missing::Present("HH.spending_days"),
    wakes: &[],
    runs_on_non_business: true,
    clause: "HH.4",
};

/// A household's stance, reconsidered on its spending occasion.
pub const STANCE: DecisionPointDecl<phx_val::switching::StanceIn, usize> = DecisionPointDecl {
    name: "HH.stance",
    system: "HH",
    rule: phx_val::switching::choose,
    schedule: Missing::Present("HH.spending_days"),
    wakes: &[],
    runs_on_non_business: true,
    clause: "VAL.7",
};
