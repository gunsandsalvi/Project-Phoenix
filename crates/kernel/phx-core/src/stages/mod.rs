//! The stage table: the day's order as data. The day's ten stages as slots, each saying whether it runs only for the
//! countries whose business day it is and whether it runs on a non-business day, what runs there, what it reads and
//! writes, whether a barrier follows it and how the pool takes it. The table holds no state; the day runner walks it.

pub mod check;
pub mod table;

use phx_macros::clause;

pub use check::compile;
pub use table::DAY_TABLE;

use crate::consts::DECIDE_STAGE;
use crate::slots::DaySlot;

/// What a slot's read sees of a store: today's value, every writer of the day at or before the slot; the store as the
/// day's slots up to a named one wrote it; or yesterday's close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsOf {
    Today,
    Through(DaySlot),
    Yesterday,
}

/// One store or column group a slot reads, and as of when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Read {
    pub store: &'static str,
    pub as_of: AsOf,
}

/// How the pool takes a slot: in chunks of its agenda, keys or ranges; as an apply by target range; or on one thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool {
    Chunked,
    Apply,
    Serial,
}

/// A slot of the day: whether it runs only for the countries whose business day it is, whether it runs on
/// a non-business day, the bases and rule sets that run there (none for a slot no rule fills yet), what it reads and
/// writes, whether a barrier follows it once it ran, and how the pool takes it.
#[clause("TIME.6", "TIME.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotDecl {
    pub slot: DaySlot,
    pub business_only: bool,
    pub on_non_business: bool,
    pub runs: &'static [&'static str],
    pub reads: &'static [Read],
    pub writes: &'static [&'static str],
    pub barrier_after: bool,
    pub pool: Pool,
}

/// How a day is run: an ordinary day; day zero, which decides from the opening's snapshot and nothing else; or a
/// settling day, an ordinary day of the settling run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Ordinary,
    DayZero,
    Settling,
}

/// The table compiled and checked: its slots in the day's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageTable {
    slots: Vec<SlotDecl>,
}

impl StageTable {
    #[must_use]
    pub fn slots(&self) -> &[SlotDecl] {
        &self.slots
    }

    /// The slots a day runs, in order: day zero stage 5 alone; otherwise, on a day some country does business, every
    /// slot, and on a day none does, the slots a non-business day runs. A business-only slot runs only where some
    /// country does business.
    #[clause("TIME.6", "TIME.8", "GEN.13")]
    pub fn walk(&self, mode: Mode, any_business: bool) -> impl Iterator<Item = &SlotDecl> {
        self.slots.iter().filter(move |s| match mode {
            Mode::DayZero => s.slot.stage() == DECIDE_STAGE,
            Mode::Ordinary | Mode::Settling => {
                if any_business {
                    true
                } else {
                    s.on_non_business && !s.business_only
                }
            }
        })
    }

    /// The barriers a day takes: one after each slot it runs that runs anything and declares one.
    #[must_use]
    pub fn barriers(&self, mode: Mode, any_business: bool) -> usize {
        self.walk(mode, any_business).filter(|s| s.barrier_after && !s.runs.is_empty()).count()
    }
}

#[cfg(test)]
#[path = "stages_tests.rs"]
mod tests;
