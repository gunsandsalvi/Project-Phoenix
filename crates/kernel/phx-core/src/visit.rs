//! When the rows of a kind are visited for a decision: a continuous decision on its schedule, each row at its own
//! phase within the period, or a lumpy decision's reviews at the chance each row's attention gives it. The kernel
//! books each row's next visit on the agenda and runs the decision's handler on the rows due.

use phx_macros::clause;

use crate::schedule::{DecisionSchedule, WakeKind};

/// What brings a row's next visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cadence {
    /// A continuous decision, every period of the schedule at the row's own phase within it.
    Schedule(DecisionSchedule),
    /// A lumpy decision's reviews: each day's chance of one is the position the row holds, in billionths; a row that
    /// holds none is not reviewed until it does.
    Attention { position: &'static str },
}

/// A decision taken on the rows of a kind as they come due: the handler run on them, when they come due, the stream
/// their phases and review days are drawn from, and what wakes a row before its day.
#[clause("TIME.5", "REP.21", "REP.38")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisitDecl {
    pub handler: &'static str,
    pub kind: &'static str,
    pub cadence: Cadence,
    pub stream: &'static str,
    pub wakes: &'static [WakeKind],
    pub clause: &'static str,
}
