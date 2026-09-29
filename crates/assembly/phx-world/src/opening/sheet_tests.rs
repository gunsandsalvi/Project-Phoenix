//! The mapping over a group's holdings and real assets written out by hand: the drawn levels land where they belong,
//! the group's splits are kept, and every identity holds; a draw the closures cannot balance is refused.
#![cfg(test)]

use super::{Drawn, Holdings, Real, map};
use crate::consts::sheet::{
    BANKS, CENTRAL_BANK, CURRENCY, DEPOSITS, FIRMS, GOVERNMENT, GOVERNMENT_PAPER, HOUSEHOLDS, LOANS_TO_HOUSEHOLDS,
};
use crate::opening::economy::{Stocks, stocks_breaks};

/// A group's holdings: currency 0.1 (households 0.8 of it), deposits held 0.6 by households and 0.15 by firms, firms'
/// debt 2/3 loans, banks a third of the government's paper.
const HELD: Holdings = Holdings {
    currency: 0.1,
    currency_households: 0.8,
    deposits_households: 0.6,
    deposits_firms: 0.15,
    firm_debt_loans: 2.0 / 3.0,
    government_paper_banks: 1.0 / 3.0,
};

/// Plant 2.0 of GDP in one kind, dwellings 2.5 and the government's assets 0.7.
fn assets() -> Real {
    Real { plant: vec![2.0], measured: [0.0, 0.0, 2.5, 0.0, 0.7] }
}

const DRAWN: Drawn = Drawn {
    household_debt: 0.7,
    firm_debt: 1.2,
    public_debt: 0.9,
    deposits: 1.0,
    capital_ratio: 0.08,
    reserves_ratio: 0.05,
};

fn rows(m: &[[f64; 5]]) -> Vec<Vec<f64>> {
    m.iter().map(|r| r.to_vec()).collect()
}

#[test]
fn the_drawn_levels_land_and_the_identities_hold() {
    let s = map(&HELD, &assets(), &DRAWN).unwrap();
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert!(close(s.at(LOANS_TO_HOUSEHOLDS, HOUSEHOLDS), -0.7));
    assert!(close(s.at(GOVERNMENT_PAPER, GOVERNMENT), -0.9));
    assert!(close(-s.at(DEPOSITS, BANKS), 1.0));
    assert!(close(s.at(DEPOSITS, HOUSEHOLDS), 0.6), "households keep the group's share of deposits");
    assert!(close(s.at(CURRENCY, CENTRAL_BANK), -0.1), "currency is the group's");
    let firms_debt = -(0..11).filter(|r| *r != 10).map(|r| s.at(r, FIRMS)).filter(|v| *v < 0.0).sum::<f64>();
    assert!(close(firms_debt, 1.2), "firms owe what was drawn");
    let stocks = Stocks { financial: rows(&s.financial), real: rows(&s.real) };
    assert!(stocks_breaks(&stocks, 1e-12).is_empty(), "{:?}", stocks_breaks(&stocks, 1e-12));
}

#[test]
fn a_draw_the_closures_cannot_balance_is_refused() {
    let firms_owe_all = Drawn { firm_debt: 40.0, ..DRAWN };
    assert!(map(&HELD, &assets(), &firms_owe_all).is_err());
}
