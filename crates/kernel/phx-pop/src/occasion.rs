use phx_core::OccasionKind;
use phx_id::{Day, Slot};
use phx_macros::clause;

/// Members of one cell with an occasion to reconsider one decision today: the profile value they hold in the
/// decision's group, and how many. The deciding system evaluates them with their count at 5c.
#[clause("REP.21")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occasion {
    pub row: Slot,
    pub decision: &'static str,
    pub profile_value: u32,
    pub count: u32,
    pub kind: OccasionKind,
}

/// What becomes of a need or notice on the day it reaches its members: decided that day where its decision point
/// runs, or carried as open business, a pin that keeps those members apart until it is decided.
#[clause("REP.21", "TIME.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaches {
    Decided,
    Carried { since: Day },
}

/// A need or notice reaching its members today: decided if its point runs today, carried from today otherwise. A
/// review is never carried: it is drawn only on a day its point runs.
#[must_use]
pub fn reaches(kind: OccasionKind, point_runs_today: bool, today: Day) -> Reaches {
    if point_runs_today {
        return Reaches::Decided;
    }
    match kind {
        OccasionKind::Review { .. } => {
            phx_num::violation!(clause = "TIME.8", "a review drawn on a day its decision point does not run")
        }
        OccasionKind::Need | OccasionKind::Meeting | OccasionKind::Notice => Reaches::Carried { since: today },
    }
}

#[cfg(test)]
mod tests {
    use phx_core::OccasionKind;
    use phx_id::Day;

    use super::{Reaches, reaches};

    #[test]
    fn needs_wait_for_their_point_and_reviews_never_do() {
        assert_eq!(reaches(OccasionKind::Need, true, Day::new(4)), Reaches::Decided);
        assert_eq!(reaches(OccasionKind::Notice, false, Day::new(4)), Reaches::Carried { since: Day::new(4) });
        let review = OccasionKind::Review { schedule: "HH.monthly" };
        assert_eq!(reaches(review, true, Day::new(4)), Reaches::Decided);
        assert!(std::panic::catch_unwind(|| reaches(review, false, Day::new(4))).is_err());
    }
}
