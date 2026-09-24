use phx_macros::clause;
use phx_num::violation;

/// A write-down or its reversal: the written-down amount it leaves, and the charge (negative) or recovery (positive)
/// to income on the day it is recognised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    pub written_down: i64,
    pub income: i64,
}

/// A position whose recoverable value moved: below its carrying value before any write-down, the difference is
/// written down and charged to income; a later recovery reverses the write-down, but only as far as the original
/// cost, so the carrying value never rises above what was paid.
#[clause("ACC.7")]
#[must_use]
pub fn write_to(unwritten: i64, written_down: i64, recoverable: i64) -> Written {
    if written_down < 0 {
        violation!(clause = "ACC.7", "a write-down below nothing");
    }
    let wanted = unwritten - recoverable;
    let target = if wanted > 0 { wanted } else { 0 };
    Written { written_down: target, income: written_down - target }
}

#[cfg(test)]
mod tests {
    use super::{Written, write_to};

    #[test]
    fn writedown_reversal_capped_at_cost() {
        // Inventory that cost 1 000 is recoverable at 700: 300 written down, charged to income.
        let down = write_to(1_000, 0, 700);
        assert_eq!(down, Written { written_down: 300, income: -300 });
        // It recovers to 900: 200 of the write-down reverses.
        let up = write_to(1_000, down.written_down, 900);
        assert_eq!(up, Written { written_down: 100, income: 200 });
        // Then to 1 200: only the last 100 reverses, never above what it cost.
        assert_eq!(write_to(1_000, up.written_down, 1_200), Written { written_down: 0, income: 100 });
    }
}
