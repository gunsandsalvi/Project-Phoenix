//! The firms' decision points: the head of a line's — its first price, what it makes and orders, its attention, its
//! price reviews and its stance — and the chief executive's, whether to wind the firm down.

use phx_core::decisions::DecisionPointDecl;
use phx_core::schedule::WakeKind;
use phx_num::Missing;

use crate::rules::inputs::{OrdersIn, orders};
use crate::rules::produce::{Produce, ProduceIn, target};
use crate::rules::review::{
    AttendIn, CloseIn, DayZeroIn, RepriceIn, ReviewIn, ReviewOut, attend, close, day_zero, reprice, review,
};

/// A new firm's first price, at its founding.
pub const DAY_ZERO_PRICE: DecisionPointDecl<DayZeroIn, Option<i64>> = DecisionPointDecl {
    name: "FRM.day_zero_price",
    system: "FRM",
    rule: day_zero,
    schedule: Missing::Absent,
    wakes: &[WakeKind::EventConcerning],
    runs_on_non_business: true,
    clause: "GEN.13",
};

/// What a firm makes today.
pub const PRODUCE: DecisionPointDecl<ProduceIn, Produce> = DecisionPointDecl {
    name: "FRM.produce",
    system: "FRM",
    rule: target,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "FRM.4",
};

/// The stored inputs a firm orders today.
pub const INPUTS: DecisionPointDecl<OrdersIn, Vec<f64>> = DecisionPointDecl {
    name: "FRM.inputs",
    system: "FRM",
    rule: orders,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "FRM.7",
};

/// Whether a firm reviews its price today.
pub const ATTEND: DecisionPointDecl<AttendIn, bool> = DecisionPointDecl {
    name: "FRM.attend",
    system: "FRM",
    rule: attend,
    schedule: Missing::Absent,
    wakes: &[WakeKind::Surprise],
    runs_on_non_business: false,
    clause: "REP.38",
};

/// A firm's price review: its markup, its expected sales and the price it would like.
pub const REVIEW_PRICE: DecisionPointDecl<ReviewIn, ReviewOut> = DecisionPointDecl {
    name: "FRM.review_price",
    system: "FRM",
    rule: review,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[WakeKind::Surprise],
    runs_on_non_business: false,
    clause: "FRM.5",
};

/// Whether a firm moves its posted price to the point nearest the one it would like.
pub const REPRICE: DecisionPointDecl<RepriceIn, Option<i64>> = DecisionPointDecl {
    name: "FRM.reprice",
    system: "FRM",
    rule: reprice,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[WakeKind::Surprise],
    runs_on_non_business: false,
    clause: "FRM.5",
};

/// A firm's stance on the heuristics' menu, reconsidered at its price review.
pub const STANCE: DecisionPointDecl<phx_val::switching::StanceIn, usize> = DecisionPointDecl {
    name: "FRM.stance",
    system: "FRM",
    rule: phx_val::switching::choose,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[WakeKind::Surprise],
    runs_on_non_business: false,
    clause: "VAL.7",
};

/// Whether a solvent firm's owner winds it down, on its production schedule.
pub const CLOSE: DecisionPointDecl<CloseIn, bool> = DecisionPointDecl {
    name: "FRM.close",
    system: "FRM",
    rule: close,
    schedule: Missing::Present("FRM.production_days"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "FRM.15",
};
