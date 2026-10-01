//! The capacity table: every store's rows at the design point with two years' growth, rounded to whole chunks, and
//! the code widths of chain links and wheel entries, so every constructor reads a declared constant and no store
//! stops the run at a literal ceiling.

use crate::consts::{
    CHUNK_ROWS, EVENT_WORDS, EVENTS_UNPRUNED, FAMILY_BITS, GROWTH_DIVISOR, GROWTH_YEARS, HOUSEHOLD_WORDS, INSTITUTIONS,
    INSTRUMENTS, LONGEST_QUARTER_DAYS, PERSONS, SLOT_BITS, STORES,
};

/// Days the due wheel files ahead.
pub use crate::consts::WHEEL_DAYS;

/// How a store's rows grow beyond the design point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Growth {
    /// With the persons, at most the growth bound a year.
    Persons,
    /// Not at all: the map's geometry and the products are the world's own.
    Fixed,
}

/// One store's capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacity {
    pub store: &'static str,
    pub design_rows: u32,
    pub growth_per_year: u32,
    pub rows: u32,
    pub row_bytes: u32,
}

const fn capacity(entry: &(&'static str, u32, u32, Growth)) -> Capacity {
    let (store, design_rows, row_bytes, growth) = *entry;
    let growth_per_year = match growth {
        Growth::Persons => design_rows.div_ceil(GROWTH_DIVISOR),
        Growth::Fixed => 0,
    };
    let needed = design_rows + GROWTH_YEARS * growth_per_year;
    Capacity { store, design_rows, growth_per_year, rows: needed.div_ceil(CHUNK_ROWS) * CHUNK_ROWS, row_bytes }
}

/// Every store's capacity, in the order the design point names them.
pub fn table() -> impl Iterator<Item = Capacity> {
    STORES.iter().map(capacity)
}

/// Rows of the largest population kind, the persons.
pub const AGENT_ROWS: u32 = capacity(&PERSONS).rows;
/// Rows of each kind table of institutions.
pub const KIND_ROWS: u32 = capacity(&INSTITUTIONS).rows;
/// Rows of the event log as it stands, every event of the run resident.
pub const EVENT_ROWS: u32 = capacity(&EVENTS_UNPRUNED).rows;
/// Words of the events' arena.
pub const ARENA_WORDS: u32 = EVENT_ROWS * EVENT_WORDS;
/// Words a chunk of households' arena reserves for their persons: each household's list at its most.
pub const PERSON_ARENA_WORDS: u32 = CHUNK_ROWS * HOUSEHOLD_WORDS;
/// Instruments the register holds.
pub const INSTRUMENT_ROWS: u32 = capacity(&INSTRUMENTS).rows;

/// Whether a slot fits beside its family code in one link.
#[must_use]
pub const fn slot_fits(slot: u32) -> bool {
    slot < 1 << SLOT_BITS
}

/// A link's width: its family code and its slot.
pub const LINK_BITS: u32 = FAMILY_BITS + SLOT_BITS;

// The wheel files a quarter's dues ahead, and a link is one word.
const _: () = assert!(WHEEL_DAYS >= LONGEST_QUARTER_DAYS);
const _: () = assert!(LINK_BITS == u32::BITS);

#[path = "capacity_tests.rs"]
mod tests;
