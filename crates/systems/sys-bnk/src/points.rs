//! Credit's decision points: a bank's loan officer's decline and quote, its chief executive's standard, and a
//! borrower's choice among the quotes.

use if_credit::central::{Request, RequestIn};
use if_credit::decisions::{ChooseIn, DeclineIn, QuoteIn, StandardIn};
use phx_core::decisions::DecisionPointDecl;
use phx_core::schedule::WakeKind;
use phx_num::Missing;

/// Whether a bank declines an application.
pub const DECLINE: DecisionPointDecl<DeclineIn, bool> = DecisionPointDecl {
    name: "BNK.decline",
    system: "BNK",
    rule: crate::credit::decline,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: false,
    clause: "BNK.4",
};

/// The rate a bank quotes an application it does not decline.
pub const QUOTE: DecisionPointDecl<QuoteIn, f64> = DecisionPointDecl {
    name: "BNK.quote",
    system: "BNK",
    rule: crate::credit::quote,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: false,
    clause: "BNK.4",
};

/// A bank's credit standard, reviewed monthly.
pub const STANDARD: DecisionPointDecl<StandardIn, u32> = DecisionPointDecl {
    name: "BNK.standard",
    system: "BNK",
    rule: crate::credit::standard,
    schedule: Missing::Present("BNK.monthly"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "BNK.5",
};

/// A borrower's choice among the quotes it was given.
pub const CHOOSE: DecisionPointDecl<ChooseIn, Missing<u32>> = DecisionPointDecl {
    name: "BNK.choose",
    system: "BNK",
    rule: crate::credit::choose,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: false,
    clause: "BNK.6",
};

/// A bank's request at the central bank's facilities, at each business day's fund stage.
pub const REQUEST: DecisionPointDecl<RequestIn, Request> = DecisionPointDecl {
    name: "BNK.request",
    system: "BNK",
    rule: crate::credit::request,
    schedule: Missing::Present("CB.fund_stage"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "CB.7",
};
