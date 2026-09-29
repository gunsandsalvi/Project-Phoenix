//! Labour's decision points: the persons' — searching, accepting, answering, retiring — and the employers' —
//! posting, selecting and offering at a pay round.

use if_labour::decisions::{AcceptIn, AnswerIn, PostIn, PostOut, RetireIn, ReviewIn, SearchIn, SelectIn};
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

/// An employee's answer at its contract's review: the least point it stays for.
pub const ANSWER: DecisionPointDecl<AnswerIn, i64> = DecisionPointDecl {
    name: "LAB.answer",
    system: "LAB",
    rule: rules::renegotiate::answer,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: true,
    clause: "LAB.17",
};

/// An employer's vacancies posted and withdrawn and the jobs it lays off, on its production schedule.
pub const POST: DecisionPointDecl<PostIn, PostOut> = DecisionPointDecl {
    name: "LAB.post",
    system: "LAB",
    rule: rules::post::post,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "LAB.4",
};

/// An employer's choice among the applicants to a vacancy.
pub const SELECT: DecisionPointDecl<SelectIn, Vec<u32>> = DecisionPointDecl {
    name: "LAB.select",
    system: "LAB",
    rule: rules::select::select,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Message],
    runs_on_non_business: false,
    clause: "LAB.5",
};

/// An employer's offer at a contract's pay round.
pub const OFFER: DecisionPointDecl<ReviewIn, i64> = DecisionPointDecl {
    name: "LAB.offer",
    system: "LAB",
    rule: rules::renegotiate::review,
    schedule: Missing::Present("LAB.review_months"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "LAB.17",
};
