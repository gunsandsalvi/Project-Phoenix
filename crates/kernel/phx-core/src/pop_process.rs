//! A process that acts on the members of population cells: its rate for a member of each joint profile value, read by
//! the kernel's screen, and its outcome for the members it hit, applied by the kernel as its owning system says.

use phx_id::{CountryId, Date, PartyId};
use phx_macros::clause;

use crate::register::Register;

/// A cell as a process reads it: its kind, its party, the value of each of its key's attributes by name, the country
/// each region lies in, and the day.
pub struct CellView<'a> {
    pub kind: &'static str,
    pub party: PartyId,
    pub key: &'a dyn Fn(&str) -> Option<u32>,
    pub country_of: &'a dyn Fn(u32) -> Option<CountryId>,
    pub date: Date,
}

impl core::fmt::Debug for CellView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CellView").field("kind", &self.kind).field("party", &self.party).finish_non_exhaustive()
    }
}

/// The persons a process reached in the households of one cell, grouped by household: `households` households, each
/// of which lost the same persons, counted by their joint value in the process's group.
#[clause("REP.26")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseholdHit {
    pub households: u64,
    pub persons: Vec<(u32, u32)>,
}

/// Where the persons a hit reached go: out of their household, or into another of its roles with every value they
/// hold; a value of the new role's groups no group of theirs corresponds to is given by `fill`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersonsGo {
    Leave,
    Into { role: &'static str, fill: Vec<(&'static str, u32)> },
}

/// Persons of one role moving to another within each household of a part, as many from each, their values drawn from
/// the part's: as when a partner becomes the head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleMove {
    pub from: &'static str,
    pub to: &'static str,
    pub persons: u32,
}

/// What a hit does, as the owning system declares it. The kernel applies each in the order given.
#[clause("REP.26", "REP.23")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberChange {
    /// Persons' value in the group becomes another, in place: nothing else about their households differs, so they
    /// stay.
    Revalue { from: u32, to: u32, count: u64 },
    /// The households of one of the hits split out as a part: the persons reached go where `persons` says, then each
    /// role move is made, and each named key attribute takes its value.
    Part { hit: usize, persons: PersonsGo, moves: Vec<RoleMove>, key: Vec<(&'static str, u32)> },
    /// The households of one of the hits end: no person of them is left, and each becomes an estate.
    End { hit: usize },
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
    /// The changes for the persons reached in a cell's households, grouped by household.
    fn outcome(&self, cell: &CellView<'_>, hits: &[HouseholdHit], out: &mut Vec<MemberChange>);
}
