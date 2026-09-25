//! The observer's hooks into the day: the traced cells it holds outside the world, which the day's splits consult,
//! and its reading at each day's end. The split log is written only for the splits of traced cells; it is not the
//! world's state, is never saved or hashed, and is cleared as each day begins, so a world run with an observer and one
//! run without are the same world.

use phx_core::TracedCells;
use phx_id::PartyId;
use phx_macros::clause;

use crate::Inspector;

/// What looks at the world: the cells it traces, and its reading once each day has ended.
pub trait Observer: TracedCells {
    fn day_closed(&mut self, w: Inspector<'_>);
}

/// Members counted by the values of their kind's groups counted once per member: (group, value, members).
pub type Counts = Vec<(usize, u32, u32)>;

/// Members that left a traced cell together, and the cell they are in at the day's end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    pub weight: u32,
    pub counts: Counts,
    pub landed: PartyId,
}

/// One split of a traced cell: what stayed in it and what left it, each with its members and their values.
#[clause("REP.30")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Split {
    pub kind: usize,
    pub origin: PartyId,
    pub stayed: u32,
    pub stayed_counts: Counts,
    pub left: Vec<Left>,
}

/// The day's splits of traced cells, in the order they happened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SplitLog {
    pub splits: Vec<Split>,
}

/// A profile's counts of the groups counted once per member.
pub(crate) fn once_per_member(kind: &phx_pop::kind::PopKindDecl, profile: &phx_pop::profile::Profile) -> Counts {
    let mut out = Vec::new();
    for (g, group) in kind.groups.iter().enumerate() {
        if matches!(group.per_member, phx_num::Missing::Absent) {
            out.extend(profile.held(g).iter().map(|(v, n)| (g, *v, *n)));
        }
    }
    out
}
