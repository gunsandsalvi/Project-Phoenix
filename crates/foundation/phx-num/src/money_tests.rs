//! The viewpoints of money and their exchange over hand-given quotes: one rounding, the pair checked, absence kept.
#![cfg(test)]

use super::{Quote, QuoteSource, exchange, exchange_where};
use crate::missing::Missing;
use crate::money::{Ccy, HomeMoney, Money, NamedMoney, NumeraireMoney};
use crate::round::Round;
use crate::violation::testing::violated_clause;

const EUR: Ccy = Ccy::new(0);
const USD: Ccy = Ccy::new(1);
const NUM: Ccy = Ccy::new(2);

/// One and a half cents of euro a cent of dollar.
fn usd_eur() -> Quote {
    Quote::new((USD, EUR), (15, 1), 7, QuoteSource::Print)
}

#[test]
fn exchange_rounds_once() {
    let usd = NamedMoney::new(Money::new(1_001, USD));
    let even: HomeMoney = exchange(usd, &usd_eur(), Round::HalfEven);
    assert_eq!(even.money(), Money::new(1_502, EUR));
    let floor: HomeMoney = exchange(usd, &usd_eur(), Round::Floor);
    assert_eq!(floor.money(), Money::new(1_501, EUR));
    let loss: HomeMoney = exchange(NamedMoney::new(Money::new(-1_001, USD)), &usd_eur(), Round::Floor);
    assert_eq!(loss.money(), Money::new(-1_502, EUR));
    // Within a viewpoint the figures add as money does.
    assert_eq!((even + floor).money(), Money::new(3_003, EUR));
}

#[test]
fn quote_of_wrong_pair_refused() {
    let eur = HomeMoney::new(Money::new(100, EUR));
    assert_eq!(violated_clause(|| exchange::<NamedMoney, _>(eur, &usd_eur(), Round::Floor)), "NUM.5");
    assert_eq!(violated_clause(|| Quote::new((EUR, EUR), (1, 0), 0, QuoteSource::Declared)), "NUM.5");
    assert_eq!(violated_clause(|| Quote::new((EUR, USD), (0, 0), 0, QuoteSource::Declared)), "NUM.5");
}

#[test]
fn no_quote_is_missing() {
    let eur = HomeMoney::new(Money::new(100, EUR));
    let none: Missing<NumeraireMoney> = exchange_where(eur, Missing::Absent, Round::HalfEven);
    assert_eq!(none, Missing::Absent);
    let quote = Quote::new((EUR, NUM), (2, 0), 7, QuoteSource::Declared);
    let some: Missing<NumeraireMoney> = exchange_where(eur, Missing::Present(&quote), Round::HalfEven);
    assert_eq!(some, Missing::Present(exchange(eur, &quote, Round::HalfEven)));
}

#[test]
fn exchange_overflow_stops() {
    let big = NamedMoney::new(Money::new(i64::MAX / 2, USD));
    let dear = Quote::new((USD, EUR), (3, 0), 0, QuoteSource::Print);
    assert_eq!(violated_clause(|| exchange::<HomeMoney, _>(big, &dear, Round::Floor)), "Law 7");
}
