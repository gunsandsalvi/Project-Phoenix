use std::ops::{Add, Sub};

use phx_macros::clause;
use phx_num::{Count, violation};

/// A cell's weight: the exact count of the real parties it is. It adds and subtracts with weights and becomes a count
/// for the arithmetic that multiplies per-member amounts by it; nothing scales it.
#[clause("PTY.14")]
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(u32);

impl Weight {
    pub const fn new(members: u32) -> Weight {
        Weight(members)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn count(self) -> Count {
        Count::new(u64::from(self.0))
    }
}

impl Add for Weight {
    type Output = Weight;
    fn add(self, other: Weight) -> Weight {
        let Some(sum) = self.0.checked_add(other.0) else {
            violation!(clause = "Law 7", "a weight beyond its width", a = self.0, b = other.0);
        };
        Weight(sum)
    }
}

impl Sub for Weight {
    type Output = Weight;
    fn sub(self, other: Weight) -> Weight {
        let Some(rest) = self.0.checked_sub(other.0) else {
            violation!(clause = "PTY.14", "a weight below zero", a = self.0, b = other.0);
        };
        Weight(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::Weight;

    #[test]
    fn weights_add_and_subtract_exactly() {
        assert_eq!(Weight::new(3) + Weight::new(4) - Weight::new(2), Weight::new(5));
        assert_eq!(Weight::new(9).count().get(), 9);
        assert!(std::panic::catch_unwind(|| Weight::new(1) - Weight::new(2)).is_err());
    }
}
