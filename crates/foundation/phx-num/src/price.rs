use phx_macros::clause;

use crate::consts::{DECIMAL_BASE, MAX_PRICE_EXP};
use crate::error::NumError;
use crate::money::{Ccy, Money};
use crate::qty::{Qty, UnitId};
use crate::round::{Round, div_round};
use crate::violation;

/// A price in smallest money units × 10^-exp per unit, where the unit's declaration fixes `exp`; it may be negative.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Price {
    raw: i64,
    ccy: Ccy,
    unit: UnitId,
}

impl Price {
    pub const fn new(raw: i64, ccy: Ccy, unit: UnitId) -> Price {
        Price { raw, ccy, unit }
    }

    pub const fn at(ccy: Ccy, unit: UnitId, raw: PriceRaw) -> Price {
        Price { raw: raw.0, ccy, unit }
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.raw
    }

    pub const fn ccy(self) -> Ccy {
        self.ccy
    }

    pub const fn unit(self) -> UnitId {
        self.unit
    }
}

/// A price's column form, whose currency and unit the column fixes.
#[repr(transparent)]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PriceRaw(i64);

impl PriceRaw {
    pub const fn from_raw(raw: i64) -> PriceRaw {
        PriceRaw(raw)
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

/// Each unit's price exponent, indexed by `UnitId`; the unit's kind and name live with its declaration.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitTable {
    exps: Box<[u8]>,
}

impl UnitTable {
    /// Refuses an exponent whose power of ten does not fit a raw price.
    ///
    /// # Errors
    /// `OutOfRange` when an exponent exceeds the largest a raw price can carry.
    pub fn new(exps: Vec<u8>) -> Result<UnitTable, NumError> {
        if exps.iter().any(|e| *e > MAX_PRICE_EXP) {
            return Err(NumError::OutOfRange);
        }
        Ok(UnitTable { exps: exps.into_boxed_slice() })
    }

    #[must_use]
    pub fn exp(&self, unit: UnitId) -> u8 {
        match self.exps.get(usize::from(unit.index())) {
            Some(exp) => *exp,
            None => violation!(clause = "NUM.1", "a unit with no declaration", unit = unit.index()),
        }
    }
}

/// 10^exp, for an exponent the unit table has already held within `MAX_PRICE_EXP`.
#[must_use]
pub fn pow10(exp: u8) -> i128 {
    DECIMAL_BASE.pow(u32::from(exp))
}

/// A quantity's value at a price: the only route from a quantity to money, rounded once.
#[clause("NUM.1", "MON.16")]
pub fn value_of(q: Qty, p: Price, units: &UnitTable, r: Round) -> Money {
    if q.unit() != p.unit() {
        violation!(
            clause = "NUM.5",
            "a quantity valued at another unit's price",
            q = q.unit().index(),
            p = p.unit.index()
        );
    }
    let value = div_round(i128::from(q.n()) * i128::from(p.raw), pow10(units.exp(p.unit)), r);
    let Ok(amt) = i64::try_from(value) else {
        violation!(clause = "Law 7", "a value overflows money", n = q.n(), raw = p.raw);
    };
    Money::new(amt, p.ccy)
}

#[cfg(test)]
mod tests {
    use super::{Price, UnitTable, value_of};
    use crate::money::{Ccy, Money};
    use crate::qty::{Qty, UnitId};
    use crate::round::Round;
    use crate::violation::testing::violated_clause;

    const C: Ccy = Ccy::new(0);
    const U: UnitId = UnitId::new(0);

    fn value(n: i64, raw: i64, r: Round) -> i64 {
        let units = UnitTable::new(vec![3]).unwrap();
        value_of(Qty::new(n, U), Price::new(raw, C, U), &units, r).amt()
    }

    #[test]
    fn value_of_rounds_once() {
        assert_eq!(value(3, 3_333, Round::HalfEven), 10);
        assert_eq!(value(3, 3_333, Round::TowardZero), 9);
        assert_eq!(value(5, 1_500, Round::HalfEven), 8);
        assert_eq!(value(5, 1_500, Round::HalfAwayFromZero), 8);
        assert_eq!(value(1, 2_500, Round::HalfEven), 2);
        assert_eq!(value(1, 2_500, Round::HalfAwayFromZero), 3);
    }

    #[test]
    fn value_of_negative_price() {
        assert_eq!(value(4, -1_250, Round::HalfEven), -5);
    }

    #[test]
    fn value_of_refuses_mixed_units_and_undeclared_units() {
        let units = UnitTable::new(vec![3]).unwrap();
        let other = UnitId::new(1);
        let mixed = || value_of(Qty::new(1, U), Price::new(1, C, other), &units, Round::Floor);
        assert_eq!(violated_clause(mixed), "NUM.5");
        let undeclared = || value_of(Qty::new(1, other), Price::new(1, C, other), &units, Round::Floor);
        assert_eq!(violated_clause(undeclared), "NUM.1");
        assert!(UnitTable::new(vec![19]).is_err());
        assert_eq!(value_of(Qty::new(2, U), Price::new(7, C, U), &units, Round::Ceil), Money::new(1, C));
    }
}
