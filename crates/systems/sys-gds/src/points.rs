//! Goods' decision points: an extractor's whether to work its deposits.

use phx_core::decisions::DecisionPointDecl;
use phx_macros::clause;
use phx_num::Missing;

/// What an extractor reads when it decides whether to work its deposits: its product's price and its cost of a unit,
/// the price it expects at its next decision, the return it requires a year, the years to that decision, and whether
/// what it works is finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtractIn {
    pub price: f64,
    pub cost: f64,
    pub outlook: f64,
    pub rate: f64,
    pub years: f64,
    pub finite: bool,
}

/// Whether to work its deposits now, by Hotelling's rule.
#[clause("GDS.4")]
#[must_use]
pub fn extract(i: &ExtractIn) -> bool {
    crate::rules::extract::works(i.price, i.cost, i.outlook, (i.rate, i.years), i.finite)
}

/// An extractor's decision on its deposits, on its extraction schedule.
pub const EXTRACT: DecisionPointDecl<ExtractIn, bool> = DecisionPointDecl {
    name: "GDS.extract",
    system: "GDS",
    rule: extract,
    schedule: Missing::Present("GDS.extraction_days"),
    wakes: &[],
    runs_on_non_business: true,
    clause: "GDS.4",
};
