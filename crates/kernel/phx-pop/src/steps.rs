use phx_core::register::values::Partition;
use phx_macros::clause;
use phx_num::Missing;
use phx_num::price::pow10;

/// A position's step: the interval of its partition its member's scaled value lies in, at some level, or missing where
/// the member's scale is. A missing step is a value of its own, never a default step.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Step(u16);

impl Step {
    /// The step of a member whose scale is missing or not positive.
    pub const MISSING: Step = Step(u16::MAX);

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn is_missing(self) -> bool {
        self.0 == u16::MAX
    }
}

/// A position's base partition on the member's own scale, non-uniform where a response is steep, and the coarser
/// levels made from it by merging adjacent steps in pairs: the step at level k is the base step shifted right by k,
/// so widening a tolerance re-keys a cell by a shift.
#[clause("REP.4", "REP.10", "REP.20")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTable {
    exp: u8,
    bounds: Box<[i64]>,
}

impl StepTable {
    /// The table a partition primitive declares.
    ///
    /// # Errors
    /// A partition of more boundaries than a step can number beside the missing step.
    pub fn new(partition: &Partition) -> Result<StepTable, String> {
        if u16::try_from(partition.bounds.len()).map_or(true, |n| n == Step::MISSING.0) {
            return Err(format!("a partition of {} boundaries, more than a step numbers", partition.bounds.len()));
        }
        Ok(StepTable { exp: partition.exp, bounds: partition.bounds.clone() })
    }

    /// The base step of a member's scaled value, written to the partition's own places: how many boundaries lie at or
    /// below it.
    pub fn step_of(&self, per_member_scaled: i64) -> Step {
        Step(self.count(|b| i128::from(b) <= i128::from(per_member_scaled)))
    }

    /// The base step of a position whose total is `total` over a member scale whose total is `scale`: the member's
    /// value over its scale is the one ratio of totals, compared with each boundary exactly in `i128`, so no division
    /// rounds a member across one. A scale missing or not positive leaves the step missing.
    pub fn step_scaled(&self, total: i64, scale: Missing<i128>) -> Step {
        let Missing::Present(s) = scale else { return Step::MISSING };
        if s <= 0 {
            return Step::MISSING;
        }
        let Some(lhs) = i128::from(total).checked_mul(pow10(self.exp)) else {
            phx_num::violation!(clause = "REP.20", "a position too large to compare with its steps", total = total);
        };
        Step(self.count(|b| i128::from(b).checked_mul(s).is_some_and(|rhs| rhs <= lhs)))
    }

    /// A base step at a coarser level: adjacent pairs merged `level` times.
    pub fn at_level(step: Step, level: u8) -> Step {
        if step.is_missing() {
            return step;
        }
        // Merging every step of a table into one leaves step nought, as a shift past the width would.
        match step.0.checked_shr(u32::from(level)) {
            Some(merged) => Step(merged),
            None => Step(0),
        }
    }

    /// How many steps the base partition has.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.bounds.len() + 1
    }

    fn count(&self, at_or_below: impl Fn(i64) -> bool) -> u16 {
        let n = self.bounds.partition_point(|b| at_or_below(*b));
        let Ok(step) = u16::try_from(n) else {
            phx_num::capacity_exceeded!("steps of a partition", u16::MAX, n);
        };
        step
    }
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_num::Missing;

    use super::{Step, StepTable};

    fn table(exp: u8, bounds: &[i64]) -> StepTable {
        StepTable::new(&Partition { exp, bounds: bounds.into() }).unwrap()
    }

    #[test]
    fn step_of_non_uniform_boundaries() {
        let t = table(2, &[-100, 0, 10, 20, 50, 400]);
        let steps: Vec<u16> =
            [-500, -100, -1, 0, 5, 10, 19, 20, 49, 50, 399, 400, 9_999].map(|v| t.step_of(v).get()).into();
        assert_eq!(steps, [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6]);
        assert_eq!(t.steps(), 7);
    }

    #[test]
    fn merge_level_is_pairwise() {
        let t = table(0, &[1, 2, 3, 5, 8, 13, 21]);
        for v in -3..30 {
            let base = t.step_of(v);
            for level in 0..4_u8 {
                let merged = StepTable::at_level(base, level).get();
                assert_eq!(merged, base.get() >> level, "level {level} merges the level below it in pairs");
            }
        }
        assert_eq!(StepTable::at_level(Step::MISSING, 3), Step::MISSING, "a missing step stays missing");
        let next = |v: i64| StepTable::at_level(t.step_of(v), 1);
        assert_eq!((next(0), next(1), next(2), next(3), next(5)), (Step(0), Step(0), Step(1), Step(1), Step(2)));
    }

    #[test]
    fn step_of_scaled_exact_in_i128() {
        let t = table(4, &[2_500, 5_000, 10_000]);
        // A total of a third of the scale, whose ratio no decimal holds, lies between a quarter and a half.
        assert_eq!(t.step_scaled(1_000_000_001, Missing::Present(3_000_000_000)).get(), 1);
        assert_eq!(t.step_scaled(1, Missing::Present(2)).get(), 2, "exactly a half is at its boundary");
        assert_eq!(t.step_scaled(i64::MAX, Missing::Present(i128::from(i64::MAX))).get(), 3, "wide totals compare");
        assert_eq!(t.step_scaled(5, Missing::Absent), Step::MISSING);
        assert_eq!(t.step_scaled(5, Missing::Present(0)), Step::MISSING, "no scale to measure against");
        assert_eq!(t.step_scaled(-5, Missing::Present(-1)), Step::MISSING);
        assert!(StepTable::new(&Partition { exp: 0, bounds: (0..i64::from(u16::MAX)).collect() }).is_err());
    }
}
