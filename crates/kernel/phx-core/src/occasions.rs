use phx_macros::clause;

/// What makes a member reconsider a lumpy decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccasionKind {
    /// A review on the cell's review days, by the named schedule.
    Review {
        schedule: &'static str,
    },
    Need,
    Meeting,
    Notice,
}

/// An occasion of a decision point.
#[clause("REP.21")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccasionDecl {
    pub point: &'static str,
    pub kind: OccasionKind,
}
