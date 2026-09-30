//! The day's slots as the stage table documents them, in its order: each slot's ordinal addresses the draws made in
//! it. Ordinals are appended, never renumbered, since a renumbering changes every draw.

use phx_macros::clause;
use phx_rand::SlotOrdinal;

use crate::consts::DAY_SLOTS;

/// A slot of the day: stage and letter, and the save taken at its declared moments.
#[clause("TIME.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DaySlot {
    S1a,
    S1b,
    S2a,
    S2b,
    S2c,
    S2d,
    S2e,
    S2f,
    S3a,
    S3b,
    S4a,
    S4b,
    S4c,
    S5a,
    S5b,
    S5c,
    S5d,
    S6a,
    S6b,
    S6c,
    S6d,
    S7a,
    S7b,
    S7c,
    S8a,
    S8b,
    S8c,
    S8d,
    S8e,
    S8f,
    S9a,
    S9b,
    S9c,
    S9d,
    S10a,
    S10b,
    S10c,
    S10d,
    S10e,
    Save,
}

/// Every slot, in the table's order.
pub const DAY_SLOT_ORDER: [DaySlot; DAY_SLOTS] = [
    DaySlot::S1a,
    DaySlot::S1b,
    DaySlot::S2a,
    DaySlot::S2b,
    DaySlot::S2c,
    DaySlot::S2d,
    DaySlot::S2e,
    DaySlot::S2f,
    DaySlot::S3a,
    DaySlot::S3b,
    DaySlot::S4a,
    DaySlot::S4b,
    DaySlot::S4c,
    DaySlot::S5a,
    DaySlot::S5b,
    DaySlot::S5c,
    DaySlot::S5d,
    DaySlot::S6a,
    DaySlot::S6b,
    DaySlot::S6c,
    DaySlot::S6d,
    DaySlot::S7a,
    DaySlot::S7b,
    DaySlot::S7c,
    DaySlot::S8a,
    DaySlot::S8b,
    DaySlot::S8c,
    DaySlot::S8d,
    DaySlot::S8e,
    DaySlot::S8f,
    DaySlot::S9a,
    DaySlot::S9b,
    DaySlot::S9c,
    DaySlot::S9d,
    DaySlot::S10a,
    DaySlot::S10b,
    DaySlot::S10c,
    DaySlot::S10d,
    DaySlot::S10e,
    DaySlot::Save,
];

impl DaySlot {
    /// The slot's ordinal: its place in the table.
    pub fn ordinal(self) -> SlotOrdinal {
        let at = DAY_SLOT_ORDER.iter().position(|s| *s == self).and_then(|a| u32::try_from(a).ok());
        let Some(at) = at else {
            phx_num::violation!(clause = "TIME.6", "a slot missing from the day's table");
        };
        SlotOrdinal::new(at)
    }
}

#[cfg(test)]
mod tests {
    use super::{DAY_SLOT_ORDER, DaySlot};
    use crate::consts::OPENING_ORDINAL_BASE;

    #[test]
    fn ordinals_follow_the_table_below_the_opening() {
        let ordinals: Vec<u8> = DAY_SLOT_ORDER.iter().map(|s| s.ordinal().get()).collect();
        assert!(ordinals.windows(2).all(|w| w[1] == w[0] + 1), "each slot the next ordinal");
        assert_eq!(DaySlot::S1a.ordinal().get(), 0);
        assert!(DaySlot::Save.ordinal().get() < OPENING_ORDINAL_BASE, "no slot shares an opening phase's address");
    }
}
