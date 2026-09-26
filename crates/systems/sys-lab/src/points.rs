//! Labour's decision points: the household's, which the player takes for its own, and the rules that take them for
//! every other.

use if_labour::decisions::{AcceptIn, RetireIn, SearchIn};
use phx_core::WakeKind;
use phx_core::decisions::DecisionPointDecl;
use phx_num::Missing;

use crate::rules;

/// A searcher's applications, on each round it searches.
pub const SEARCH: DecisionPointDecl<SearchIn, Vec<u32>> = DecisionPointDecl {
    name: "LAB.search",
    system: "LAB",
    rule: rules::search::search,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: true,
    clause: "LAB.5",
};

/// A searcher's answer to an offer that reached it.
pub const ACCEPT: DecisionPointDecl<AcceptIn, bool> = DecisionPointDecl {
    name: "LAB.accept",
    system: "LAB",
    rule: rules::accept::accept,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: true,
    clause: "LAB.5",
};

/// A person's retirement, on the day it reaches the pension's age.
pub const RETIRE: DecisionPointDecl<RetireIn, bool> = DecisionPointDecl {
    name: "LAB.retire",
    system: "LAB",
    rule: rules::retire::retire,
    schedule: Missing::Absent,
    wakes: &[WakeKind::KinkDay],
    runs_on_non_business: true,
    clause: "LAB.6",
};
