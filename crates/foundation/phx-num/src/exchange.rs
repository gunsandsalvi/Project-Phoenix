//! Exchange: the one way a figure passes from one viewpoint of money to another — an amount times a quote a market
//! formed or a conversion declared, rounded once by the governing convention.

use phx_macros::clause;

use crate::consts::MAX_PRICE_EXP;
use crate::missing::Missing;
use crate::money::{Ccy, HomeMoney, Money, NamedMoney, NumeraireMoney, Owner, PartyMoney};
use crate::price::pow10;
use crate::round::{Round, div_round};
use crate::violation;

pub(crate) mod sealed {
    /// A viewpoint made from the column form, which only an exchange does outside this crate's constructors.
    pub trait Made {
        fn made(m: super::Money) -> Self;
    }
}

/// One of the four viewpoints of money.
pub trait Viewpoint: sealed::Made + Copy {
    fn money(self) -> Money;
}

/// The directed pairs a clause names an exchange between: a named currency into the home one and back, either into
/// the numéraire, and a party's money into the owner's home currency and back.
pub trait ExchangesTo<T: Viewpoint>: Viewpoint {}

impl ExchangesTo<HomeMoney> for NamedMoney {}
impl ExchangesTo<NamedMoney> for HomeMoney {}
impl ExchangesTo<NumeraireMoney> for HomeMoney {}
impl ExchangesTo<NumeraireMoney> for NamedMoney {}
impl<P: Owner> ExchangesTo<HomeMoney> for PartyMoney<P> {}
impl<P: Owner> ExchangesTo<PartyMoney<P>> for HomeMoney {}

/// Where a quote comes from: a print a market formed, or a conversion declared (a recipe's content per unit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoteSource {
    Print,
    Declared,
}

/// The price of one currency in another on a day: `raw × 10⁻ᵉˣᵖ` smallest units of `to` for one smallest unit of
/// `from`.
#[clause("NUM.2", "NUM.5")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quote {
    from: Ccy,
    to: Ccy,
    raw: i64,
    exp: u8,
    day: u32,
    source: QuoteSource,
}

impl Quote {
    /// A quote of `from` in `to`, formed or declared on `day`; a currency priced in itself, a price not above
    /// nothing or an exponent past a price's stops the run.
    #[must_use]
    pub fn new((from, to): (Ccy, Ccy), (raw, exp): (i64, u8), day: u32, source: QuoteSource) -> Quote {
        if from == to || raw <= 0 || exp > MAX_PRICE_EXP {
            violation!(
                clause = "NUM.5",
                "a quote of no pair or no price",
                from = from.index(),
                to = to.index(),
                raw = raw
            );
        }
        Quote { from, to, raw, exp, day, source }
    }

    pub const fn pair(&self) -> (Ccy, Ccy) {
        (self.from, self.to)
    }

    /// The day the quote was formed or declared.
    #[must_use]
    pub const fn day(&self) -> u32 {
        self.day
    }

    #[must_use]
    pub const fn source(&self) -> QuoteSource {
        self.source
    }
}

/// `from` exchanged at `at` into the viewpoint `T`: its amount times the quote's price in checked `i128`, rounded once
/// by `round`. A figure not in the quote's first currency stops the run, as does a result past money's word.
#[clause("NUM.2", "NUM.5", "MON.16")]
pub fn exchange<T: Viewpoint, F: ExchangesTo<T>>(from: F, at: &Quote, round: Round) -> T {
    let m = from.money();
    if m.ccy() != at.from {
        violation!(
            clause = "NUM.5",
            "an exchange at a quote of another pair",
            ccy = m.ccy().index(),
            quote = at.from.index()
        );
    }
    let value = div_round(i128::from(m.amt()) * i128::from(at.raw), pow10(at.exp), round);
    let Ok(amt) = i64::try_from(value) else {
        violation!(clause = "Law 7", "an exchange overflows money", amt = m.amt(), raw = at.raw);
    };
    <T as sealed::Made>::made(Money::new(amt, at.to))
}

/// `from` exchanged where a quote is present; with none, as for a currency with no print that day, the figure is
/// absent, never zero.
pub fn exchange_where<T: Viewpoint, F: ExchangesTo<T>>(from: F, at: Missing<&Quote>, round: Round) -> Missing<T> {
    match at {
        Missing::Present(q) => Missing::Present(exchange(from, q, round)),
        Missing::Absent => Missing::Absent,
    }
}

#[cfg(test)]
#[path = "money_tests.rs"]
mod tests;
