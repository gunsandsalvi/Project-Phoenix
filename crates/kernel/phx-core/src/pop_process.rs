//! A process that acts on the members of population cells: its rate for a member of each joint profile value, read by
//! the kernel's screen, and its outcome for the members it hit, applied by the kernel as its owning system says.

use phx_id::{CountryId, Date, PartyId};
use phx_macros::clause;

use crate::register::Register;

/// A cell as a process reads it: its kind, its party, the country it lives in, the value of each of its key's
/// attributes by name, and the day.
pub struct CellView<'a> {
    pub kind: &'static str,
    pub party: PartyId,
    pub country: CountryId,
    pub key: &'a dyn Fn(&str) -> Option<u32>,
    pub date: Date,
}

impl core::fmt::Debug for CellView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CellView").field("kind", &self.kind).field("party", &self.party).finish_non_exhaustive()
    }
}

/// What a hit does to the members it reached who hold one joint value of the process's group, as the owning system
/// declares it. The kernel splits them from their cell as one part, carrying their other roles' profile values drawn
/// from the cell, then makes these changes to the part.
#[clause("REP.26", "REP.23")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberChange {
    /// The members' value in the group becomes another, in place: nothing else about them differs from the cell's,
    /// so they stay.
    Revalue { from: u32, to: u32, count: u64 },
    /// The members split out as a part: their value in the group becomes `to`, each named key attribute takes its
    /// value, and each named role's values move to another role's group (as when a partner becomes the head).
    Part { from: u32, count: u64, to: u32, key: Vec<(&'static str, u32)>, moves: Vec<(&'static str, &'static str)> },
    /// The members' households end: no member of them is left, and each becomes an estate.
    End { from: u32, count: u64 },
}

/// A process on a population kind's members, declared by the system that owns its outcome. The hazard it answers
/// names its rate table and stream; the process reads that table here, at a member's joint value in its group.
#[clause("CHN.2", "REP.7")]
pub trait PopProcess: Send + Sync {
    /// The hazard it answers, as declared.
    fn hazard(&self) -> &'static str;
    /// The population kind whose members it acts on.
    fn kind(&self) -> &'static str;
    /// The profile group whose joint values its rate is read at.
    fn group(&self) -> &'static str;
    /// A member's daily chance of a hit at a joint value, in a cell, on a day.
    fn rate(&self, register: &Register, cell: &CellView<'_>, value: u32) -> f64;
    /// The first day after `date` on which a rate may change though the cell is not visited, if there is one.
    fn changes_after(&self, date: Date) -> Option<Date>;
    /// The changes for the members hit in a cell, per joint value hit.
    fn outcome(&self, cell: &CellView<'_>, hits: &[(u32, u64)], out: &mut Vec<MemberChange>);
}
