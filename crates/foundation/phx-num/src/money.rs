use std::ops::{Add, Neg, Sub};

use phx_macros::clause;

use crate::qty::Count;
use crate::violation;

/// A currency's index in the world's declared currency list; which currency it is, is data.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ccy(u8);

impl Ccy {
    pub const fn new(index: u8) -> Ccy {
        Ccy(index)
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Whole smallest units with no currency, for store columns whose currency the column fixes; it has no arithmetic.
#[repr(transparent)]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Amount(i64);

impl Amount {
    pub const fn from_raw(units: i64) -> Amount {
        Amount(units)
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

/// Money: whole smallest units of a named currency, whose arithmetic refuses a second currency and overflow.
#[clause("NUM.1", "NUM.2", "NUM.5", "MON.16", "Law 7")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Money {
    amt: i64,
    ccy: Ccy,
}

impl Money {
    pub const fn new(amt: i64, ccy: Ccy) -> Money {
        Money { amt, ccy }
    }

    pub const fn zero(ccy: Ccy) -> Money {
        Money { amt: 0, ccy }
    }

    pub const fn at(ccy: Ccy, amount: Amount) -> Money {
        Money { amt: amount.0, ccy }
    }

    pub const fn amount(self) -> Amount {
        Amount(self.amt)
    }

    #[must_use]
    pub const fn amt(self) -> i64 {
        self.amt
    }

    pub const fn ccy(self) -> Ccy {
        self.ccy
    }

    /// The sum of amounts all in `ccy`; naming the currency makes an empty sum a known zero.
    pub fn sum_in(ccy: Ccy, items: impl IntoIterator<Item = Money>) -> Money {
        items.into_iter().fold(Money::zero(ccy), |total, m| total + m)
    }

    /// This amount for each of `n` identical things.
    pub fn times(self, n: Count) -> Money {
        let product = i128::from(self.amt) * i128::from(n.get());
        let Ok(amt) = i64::try_from(product) else {
            violation!(clause = "Law 7", "money times a count overflows", amt = self.amt, n = n.get());
        };
        Money { amt, ccy: self.ccy }
    }
}

fn same_ccy(a: Money, b: Money) {
    if a.ccy != b.ccy {
        violation!(clause = "NUM.5", "money of two currencies combined", a = a.ccy.0, b = b.ccy.0);
    }
}

impl Add for Money {
    type Output = Money;

    fn add(self, other: Money) -> Money {
        same_ccy(self, other);
        let Some(amt) = self.amt.checked_add(other.amt) else {
            violation!(clause = "Law 7", "money overflows", a = self.amt, b = other.amt);
        };
        Money { amt, ccy: self.ccy }
    }
}

impl Sub for Money {
    type Output = Money;

    fn sub(self, other: Money) -> Money {
        same_ccy(self, other);
        let Some(amt) = self.amt.checked_sub(other.amt) else {
            violation!(clause = "Law 7", "money overflows", a = self.amt, b = other.amt);
        };
        Money { amt, ccy: self.ccy }
    }
}

impl Neg for Money {
    type Output = Money;

    fn neg(self) -> Money {
        let Some(amt) = self.amt.checked_neg() else {
            violation!(clause = "Law 7", "money overflows", amt = self.amt);
        };
        Money { amt, ccy: self.ccy }
    }
}

/// A figure in the reporting numéraire, a different number from any currency's money; built only by exchange.
#[clause("NUM.2")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reported<M> {
    value: M,
}

impl<M: Copy> Reported<M> {
    pub const fn value(&self) -> M {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::{Ccy, Money};
    use crate::qty::Count;
    use crate::violation::testing::violated_clause;

    #[test]
    fn money_add_two_currencies_violates() {
        let (a, b) = (Ccy::new(0), Ccy::new(1));
        assert_eq!(violated_clause(|| Money::new(1, a) + Money::new(1, b)), "NUM.5");
        assert_eq!(violated_clause(|| Money::new(1, a) - Money::new(1, b)), "NUM.5");
        assert_eq!(violated_clause(|| Money::sum_in(a, [Money::new(1, a), Money::new(1, b)])), "NUM.5");
    }

    #[test]
    fn money_overflow_violates() {
        let a = Ccy::new(0);
        assert_eq!(violated_clause(|| Money::new(i64::MAX, a) + Money::new(1, a)), "Law 7");
        assert_eq!(violated_clause(|| Money::new(i64::MIN, a) - Money::new(1, a)), "Law 7");
        assert_eq!(violated_clause(|| -Money::new(i64::MIN, a)), "Law 7");
        assert_eq!(violated_clause(|| Money::new(i64::MAX, a).times(Count::new(2))), "Law 7");
        assert_eq!(Money::new(-7, a).times(Count::new(3)), Money::new(-21, a));
        assert_eq!(Money::sum_in(a, []), Money::zero(a));
    }
}
