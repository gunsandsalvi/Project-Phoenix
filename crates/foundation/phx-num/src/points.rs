use phx_macros::clause;

use crate::missing::Missing;
use crate::violation;

/// A poster's price points: strictly increasing raw prices, held in the registry, never in a store.
#[clause("REP.34")]
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointTable {
    raw: Box<[i64]>,
}

/// An index into one point table.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PointIdx(u16);

impl PointIdx {
    pub const fn new(index: u16) -> PointIdx {
        PointIdx(index)
    }

    #[must_use]
    pub const fn index(self) -> u16 {
        self.0
    }
}

impl PointTable {
    /// `None` for a list that is empty, not strictly increasing, or longer than a `PointIdx` can index; the
    /// registry refuses the entry that gave it.
    #[must_use]
    pub fn new(raw: Vec<i64>) -> Option<PointTable> {
        let indexable = raw.len().checked_sub(1).is_some_and(|last| u16::try_from(last).is_ok());
        let increasing = raw.windows(2).all(|pair| matches!(pair, [a, b] if a < b));
        (indexable && increasing).then(|| PointTable { raw: raw.into_boxed_slice() })
    }

    #[must_use]
    pub fn point(&self, i: PointIdx) -> i64 {
        match self.raw.get(usize::from(i.0)) {
            Some(raw) => *raw,
            None => violation!(clause = "REP.34", "a point index beyond its table", index = i.0),
        }
    }

    /// The highest point at or below `raw`.
    pub fn at_or_below(&self, raw: i64) -> Missing<PointIdx> {
        let count = self.raw.partition_point(|p| *p <= raw);
        match count.checked_sub(1).map(u16::try_from) {
            Some(Ok(i)) => Missing::Present(PointIdx(i)),
            _ => Missing::Absent,
        }
    }

    /// The lowest point at or above `raw`.
    pub fn at_or_above(&self, raw: i64) -> Missing<PointIdx> {
        let i = self.raw.partition_point(|p| *p < raw);
        match (i < self.raw.len()).then(|| u16::try_from(i)) {
            Some(Ok(i)) => Missing::Present(PointIdx(i)),
            _ => Missing::Absent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PointIdx, PointTable};
    use crate::missing::Missing;

    #[test]
    fn point_table_search() {
        let t = PointTable::new(vec![100, 199, 250, 999]).unwrap();
        assert_eq!(t.at_or_below(250), Missing::Present(PointIdx::new(2)));
        assert_eq!(t.at_or_below(249), Missing::Present(PointIdx::new(1)));
        assert_eq!(t.at_or_below(99), Missing::Absent);
        assert_eq!(t.at_or_above(1000), Missing::Absent);
        assert_eq!(t.at_or_above(200), Missing::Present(PointIdx::new(2)));
        assert_eq!(t.point(PointIdx::new(3)), 999);
        assert!(PointTable::new(vec![1, 1]).is_none());
        assert!(PointTable::new(vec![]).is_none());
    }
}
