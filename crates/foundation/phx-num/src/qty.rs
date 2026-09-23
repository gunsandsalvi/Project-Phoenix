use std::ops::{Add, Neg, Sub};

use phx_macros::clause;

use crate::violation;

/// A unit's index in the declared unit table, whose entries name the kind, the name and the price exponent.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnitId(u16);

impl UnitId {
    pub const fn new(index: u16) -> UnitId {
        UnitId(index)
    }

    #[must_use]
    pub const fn index(self) -> u16 {
        self.0
    }
}

/// A count of identical things — members, units — and never an arbitrary number.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Count(u64);

impl Count {
    pub const fn new(n: u64) -> Count {
        Count(n)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A quantity's column form, whose unit the column fixes; it has no arithmetic.
#[repr(transparent)]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QtyRaw(i64);

impl QtyRaw {
    pub const fn from_raw(n: i64) -> QtyRaw {
        QtyRaw(n)
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

/// A quantity with its unit; arithmetic across units stops the run.
#[clause("NUM.1", "NUM.5")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qty {
    n: i64,
    unit: UnitId,
}

impl Qty {
    pub const fn new(n: i64, unit: UnitId) -> Qty {
        Qty { n, unit }
    }

    pub const fn at(unit: UnitId, raw: QtyRaw) -> Qty {
        Qty { n: raw.0, unit }
    }

    #[must_use]
    pub const fn n(self) -> i64 {
        self.n
    }

    pub const fn unit(self) -> UnitId {
        self.unit
    }

    pub const fn raw(self) -> QtyRaw {
        QtyRaw(self.n)
    }

    /// The quantity both sides of a trade accept: the smaller of what is offered and what is wanted.
    pub fn matched(offered: Qty, wanted: Qty) -> Qty {
        same_unit(offered, wanted);
        if offered.n <= wanted.n { offered } else { wanted }
    }
}

fn same_unit(a: Qty, b: Qty) {
    if a.unit != b.unit {
        violation!(clause = "NUM.5", "quantities of two units combined", a = a.unit.0, b = b.unit.0);
    }
}

impl Add for Qty {
    type Output = Qty;

    fn add(self, other: Qty) -> Qty {
        same_unit(self, other);
        let Some(n) = self.n.checked_add(other.n) else {
            violation!(clause = "Law 7", "a quantity overflows", a = self.n, b = other.n);
        };
        Qty { n, unit: self.unit }
    }
}

impl Sub for Qty {
    type Output = Qty;

    fn sub(self, other: Qty) -> Qty {
        same_unit(self, other);
        let Some(n) = self.n.checked_sub(other.n) else {
            violation!(clause = "Law 7", "a quantity overflows", a = self.n, b = other.n);
        };
        Qty { n, unit: self.unit }
    }
}

impl Neg for Qty {
    type Output = Qty;

    fn neg(self) -> Qty {
        let Some(n) = self.n.checked_neg() else {
            violation!(clause = "Law 7", "a quantity overflows", n = self.n);
        };
        Qty { n, unit: self.unit }
    }
}

#[cfg(test)]
mod tests {
    use super::{Qty, UnitId};
    use crate::violation::testing::violated_clause;

    #[test]
    fn qty_refuses_mixed_units_and_overflow() {
        let (a, b) = (UnitId::new(1), UnitId::new(2));
        assert_eq!(Qty::new(3, a) + Qty::new(4, a), Qty::new(7, a));
        assert_eq!(violated_clause(|| Qty::new(3, a) + Qty::new(4, b)), "NUM.5");
        assert_eq!(violated_clause(|| Qty::new(i64::MAX, a) + Qty::new(1, a)), "Law 7");
        assert_eq!(Qty::matched(Qty::new(5, a), Qty::new(3, a)), Qty::new(3, a));
        assert_eq!(violated_clause(|| Qty::matched(Qty::new(5, a), Qty::new(3, b))), "NUM.5");
    }
}
