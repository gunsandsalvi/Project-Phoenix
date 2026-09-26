//! Households: their spending reaches named sellers, those going without their needs are counted, and the circular
//! flow is live.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Every sale at retail is a match of a named buyer and a named seller; none is read before households buy.
fn spending_named(w: Inspector<'_>) -> Outcome {
    if w.goods_days().iter().all(|(_, g)| g.orders == 0) {
        return Outcome::NotYet("households ask for goods once they decide their spending");
    }
    Outcome::Pass
}

pub const LC_1_32: super::Check = live_check! {
    id: "LC-1-32",
    title: "HH.15: every household's spending reaches named sellers (through match-set records), and every unit of \
            income came from a named payer",
    from_step: "S1.12",
    check: spending_named,
};

/// Households going without their needs, which arrive with the needs themselves.
fn going_without(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("needs by quantity arrive with the households' finances (S2.05)")
}

pub const LC_1_33: super::Check = live_check! {
    id: "LC-1-33",
    title: "Households going without their needs are recorded as events and counted",
    from_step: "S1.12",
    check: going_without,
};

/// The circular flow's liveness, read once firms produce and sell to the households that ask.
fn circular_flow(w: Inspector<'_>) -> Outcome {
    if w.goods_days().iter().all(|(_, g)| g.shoppers == g.unserved) {
        return Outcome::NotYet("the circular flow closes once firms hold stocks and sell at retail (S1.15)");
    }
    Outcome::Pass
}

pub const LC_1_34: super::Check = live_check! {
    id: "LC-1-34",
    title: "Liveness (N2) for the circular flow: wages paid, spending received, production, employment and lending \
            are non-zero and respond when a primitive moves in the run by its owner's decision",
    from_step: "S1.12",
    check: circular_flow,
};
