use phx_id::{CountryId, Day, PartyId};
use phx_macros::clause;
use phx_num::{Amount, Count, Money, Qty, QtyRaw, violation};

use crate::register::values::PrimType;
use crate::register::{Prim, Register};

/// Proof that a limit comes from a contract's terms; only the ledger builds one.
#[derive(Debug)]
pub struct TermsToken(());

impl TermsToken {
    #[doc(hidden)]
    #[must_use]
    pub fn new() -> TermsToken {
        TermsToken(())
    }
}

/// Proof that a capacity is read from held units or physical stock; only the ledger and the map build one.
#[derive(Debug)]
pub struct PhysicalToken(());

impl PhysicalToken {
    #[doc(hidden)]
    #[must_use]
    pub fn new() -> PhysicalToken {
        PhysicalToken(())
    }
}

/// A quantity a limit can bind: split into what is taken and what exceeds.
pub trait Limited: Copy {
    /// `(taken, excess)` of `wanted` against `limit`; a want below zero, or of another currency or unit, is no want a
    /// limit binds.
    fn split(wanted: Self, limit: Self) -> (Self, Self);
    /// The amount in its own smallest units, for the record of a binding.
    fn smallest_units(self) -> i128;
}

fn split_raw(wanted: i64, limit: i64) -> (i64, i64) {
    if wanted < 0 {
        violation!(clause = "Law 6", "a limit bound against a negative want", wanted = wanted);
    }
    if wanted <= limit { (wanted, 0) } else { (limit, wanted - limit) }
}

impl Limited for Amount {
    fn smallest_units(self) -> i128 {
        i128::from(self.raw())
    }

    fn split(wanted: Amount, limit: Amount) -> (Amount, Amount) {
        let (t, e) = split_raw(wanted.raw(), limit.raw());
        (Amount::from_raw(t), Amount::from_raw(e))
    }
}

impl Limited for QtyRaw {
    fn smallest_units(self) -> i128 {
        i128::from(self.raw())
    }

    fn split(wanted: QtyRaw, limit: QtyRaw) -> (QtyRaw, QtyRaw) {
        let (t, e) = split_raw(wanted.raw(), limit.raw());
        (QtyRaw::from_raw(t), QtyRaw::from_raw(e))
    }
}

impl Limited for Count {
    fn smallest_units(self) -> i128 {
        i128::from(self.get())
    }

    fn split(wanted: Count, limit: Count) -> (Count, Count) {
        if wanted.get() <= limit.get() {
            (wanted, Count::new(0))
        } else {
            (limit, Count::new(wanted.get() - limit.get()))
        }
    }
}

impl Limited for Money {
    fn smallest_units(self) -> i128 {
        i128::from(self.amt())
    }

    fn split(wanted: Money, limit: Money) -> (Money, Money) {
        if wanted.ccy() != limit.ccy() {
            violation!(clause = "NUM.5", "a limit in one currency bound against a want in another");
        }
        let (t, e) = split_raw(wanted.amt(), limit.amt());
        (Money::new(t, limit.ccy()), Money::new(e, limit.ccy()))
    }
}

impl Limited for Qty {
    fn smallest_units(self) -> i128 {
        i128::from(self.n())
    }

    fn split(wanted: Qty, limit: Qty) -> (Qty, Qty) {
        if wanted.unit() != limit.unit() {
            violation!(clause = "NUM.5", "a limit in one unit bound against a want in another");
        }
        let (t, e) = split_raw(wanted.n(), limit.n());
        (Qty::new(t, limit.unit()), Qty::new(e, limit.unit()))
    }
}

/// A real limit: from the register, a contract's terms, or held units or stock, and from nowhere else.
#[clause("Law 6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredLimit<T> {
    limit: T,
}

/// What a limit left of a want: the part taken, read only through the context that records the binding, and the
/// excess its party sees.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bound<T> {
    taken: T,
    excess: T,
}

impl<T: Limited> DeclaredLimit<T> {
    /// A limit the register declares for a country.
    #[must_use]
    pub fn from_prim<P: PrimType>(prim: Prim<P>, register: &Register, country: CountryId) -> DeclaredLimit<T>
    where
        for<'a> P::Read<'a>: Into<T>,
    {
        DeclaredLimit { limit: prim.get(register, country).into() }
    }

    /// A limit the world's register declares.
    #[must_use]
    pub fn from_shared_prim<P: PrimType>(prim: Prim<P>, register: &Register) -> DeclaredLimit<T>
    where
        for<'a> P::Read<'a>: Into<T>,
    {
        DeclaredLimit { limit: prim.shared(register).into() }
    }

    pub fn from_terms(_: TermsToken, limit: T) -> DeclaredLimit<T> {
        DeclaredLimit { limit }
    }

    pub fn from_physical(_: PhysicalToken, limit: T) -> DeclaredLimit<T> {
        DeclaredLimit { limit }
    }

    /// The want against the limit; a bound whose excess is not zero is an event its party sees.
    pub fn bind(&self, wanted: T) -> Bound<T> {
        let (taken, excess) = T::split(wanted, self.limit);
        Bound { taken, excess }
    }
}

impl<T: Copy> Bound<T> {
    #[must_use]
    pub fn excess(&self) -> T {
        self.excess
    }

    fn taken(&self) -> T {
        self.taken
    }
}

/// A limit that bound a party's want on a day: an event its party reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub party: PartyId,
    pub day: Day,
    pub excess: i128,
}

/// The one reader of what a limit left: it records every binding with an excess, and the handler context holds it.
#[derive(Debug, Default)]
pub struct Bindings {
    records: Vec<Binding>,
}

impl Bindings {
    /// The part taken, recording the binding when the want exceeded the limit.
    #[clause("Law 6")]
    pub fn take<T: Limited>(&mut self, party: PartyId, day: Day, bound: Bound<T>) -> T {
        let excess = bound.excess.smallest_units();
        if excess != 0 {
            self.records.push(Binding { party, day, excess });
        }
        bound.taken()
    }

    /// The bindings recorded since the last drain, in the order they were taken.
    pub fn drain(&mut self) -> impl Iterator<Item = Binding> + '_ {
        self.records.drain(..)
    }
}

#[cfg(test)]
mod tests {
    use phx_num::{Amount, Ccy, Money};

    use phx_id::{Day, PartyId};

    use super::{Binding, Bindings, DeclaredLimit, TermsToken};

    #[test]
    fn declared_limit_binds_and_reports_excess() {
        let limit = DeclaredLimit::from_terms(TermsToken::new(), Amount::from_raw(100));
        let mut bindings = Bindings::default();
        let (party, day) = (PartyId::new(7), Day::new(3));
        let bound = limit.bind(Amount::from_raw(120));
        assert_eq!(bound.excess(), Amount::from_raw(20));
        assert_eq!(bindings.take(party, day, bound), Amount::from_raw(100));
        let bound = limit.bind(Amount::from_raw(80));
        assert_eq!((bindings.take(party, day, bound), bound.excess()), (Amount::from_raw(80), Amount::from_raw(0)));
        assert_eq!(bindings.drain().collect::<Vec<_>>(), vec![Binding { party, day, excess: 20 }]);
        let money = DeclaredLimit::from_terms(TermsToken::new(), Money::new(5, Ccy::new(0)));
        let caught = std::panic::catch_unwind(|| money.bind(Money::new(1, Ccy::new(1))));
        let payload = caught.expect_err("two currencies stop the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("NUM.5"));
    }
}
