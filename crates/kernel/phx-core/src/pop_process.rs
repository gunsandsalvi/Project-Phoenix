//! A process that acts on the persons of population cells: its rate for a person of each joint profile value, read by
//! the kernel's screen, and its outcome on each household its hits reached, made explicit by the kernel.

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

/// A person of a household made explicit for the day's outcomes: its role, its value in each group of its role by
/// the group's name, and whether it has gone — died or left — which keeps its place so reached persons keep theirs.
#[clause("REP.26")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub role: &'static str,
    pub values: Vec<(&'static str, u32)>,
    pub gone: bool,
}

impl Person {
    /// The person's value in a group of its role.
    #[must_use]
    pub fn value(&self, group: &str) -> Option<u32> {
        self.values.iter().find(|(g, _)| *g == group).map(|(_, v)| *v)
    }
}

/// A household of a cell made explicit for the day's outcomes: its key's attributes by name and its persons. An
/// outcome changes it; the kernel then returns it to its cell or splits it out under the key it has become.
#[clause("REP.26", "REP.19")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Household {
    pub key: Vec<(&'static str, u32)>,
    pub persons: Vec<Person>,
}

impl Household {
    /// A key attribute's value; every household holds each of its kind's.
    #[must_use]
    pub fn key(&self, name: &str) -> u32 {
        let Some((_, v)) = self.key.iter().find(|(n, _)| *n == name) else {
            phx_num::violation!(clause = "REP.19", "a household read by a key attribute its kind does not hold");
        };
        *v
    }

    /// A key attribute set to a value.
    pub fn set_attr(&mut self, name: &str, value: u32) {
        let Some((_, v)) = self.key.iter_mut().find(|(n, _)| *n == name) else {
            phx_num::violation!(clause = "REP.19", "a household keyed by an attribute its kind does not hold");
        };
        *v = value;
    }

    /// The persons still in the household.
    pub fn present(&self) -> impl Iterator<Item = (usize, &Person)> {
        self.persons.iter().enumerate().filter(|(_, p)| !p.gone)
    }
}

/// A process on a population kind's members, declared by the system that owns its outcome. The hazard it answers
/// names its rate table and stream; the process reads that table here, at a member's joint value in its group.
#[clause("CHN.2", "REP.7")]
pub trait PopProcess: Send + Sync {
    /// What its rate reads of the register, found once when the world binds it, before any day; it holds nothing
    /// else, so its rate is a pure read.
    fn bind(&mut self, register: &Register);
    /// The hazard it answers, as declared.
    fn hazard(&self) -> &'static str;
    /// The population kind whose members it acts on.
    fn kind(&self) -> &'static str;
    /// The profile groups whose joint values its rate is read at, of one set of components, so a value means the
    /// same in each: the groups of the roles it acts on.
    fn groups(&self) -> &'static [&'static str];
    /// A person's daily chance of a hit at a joint value of one of its groups, in a cell, on a day. Between one day
    /// `changes_after` names and the next it rises or falls but never turns, so its greatest over those days is at
    /// the first or the last.
    fn rate(&self, register: &Register, cell: &CellView<'_>, group: &'static str, value: u32) -> f64;
    /// The first day after `date` on which a rate may change though the cell is not visited, if there is one.
    fn changes_after(&self, date: Date) -> Option<Date>;
    /// What the persons it reached, by their places in the household, do to their household; what is left to chance
    /// is drawn from `draws`, the process's own for the cell and the day.
    fn outcome(
        &self,
        register: &Register,
        cell: &CellView<'_>,
        household: &mut Household,
        reached: &[usize],
        draws: &mut phx_rand::Draws,
    );
}
