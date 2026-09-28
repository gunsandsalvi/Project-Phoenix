//! The mapping over a group sheet written out by hand: the drawn levels land where they belong, the group's splits
//! are kept, and every identity holds; a draw the closures cannot balance is refused.
#![cfg(test)]

use super::{Drawn, map};
use crate::consts::sheet::{
    BANKS, CENTRAL_BANK, CURRENCY, DEPOSITS, FIRMS, GOVERNMENT, GOVERNMENT_PAPER, HOUSEHOLDS, LOANS_TO_HOUSEHOLDS,
};
use crate::opening::economy::{Stocks, stocks_breaks};

/// A group's sheet: currency 0.1 (households 0.08), deposits 0.8 (households 0.6, firms 0.15, government 0.05), loans
/// to households 0.5, firms' debt 0.9 (loans 0.6), government paper 0.6 (banks 0.2), and plant, dwellings and the
/// government's assets; its closures as the dataset's.
fn group() -> Stocks {
    let financial = vec![
        vec![0.08, 0.02, 0.0, -0.1, 0.0],
        vec![0.6, 0.15, -0.8, 0.0, 0.05],
        vec![-0.5, 0.0, 0.5, 0.0, 0.0],
        vec![0.0, -0.6, 0.6, 0.0, 0.0],
        vec![0.3, -0.3, 0.0, 0.0, 0.0],
        vec![0.2, 0.0, 0.2, 0.2, -0.6],
        vec![0.0, 0.0, 0.1, -0.1, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.3, 0.0, -0.3, 0.0, 0.0],
        vec![0.2, 0.0, -0.2, 0.0, 0.0],
        vec![1.3, -1.3, 0.0, 0.0, 0.0],
    ];
    let real = vec![vec![0.0, 2.0, 0.0, 0.0, 0.0], vec![2.5, 0.0, 0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0, 0.0, 0.7]];
    Stocks { financial, real }
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
    let s = map(&group(), &DRAWN).unwrap();
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert!(close(s.at(LOANS_TO_HOUSEHOLDS, HOUSEHOLDS), -0.7));
    assert!(close(s.at(GOVERNMENT_PAPER, GOVERNMENT), -0.9));
    assert!(close(-s.at(DEPOSITS, BANKS), 1.0));
    assert!(close(s.at(DEPOSITS, HOUSEHOLDS), 0.75), "households keep the group's share of deposits");
    assert!(close(s.at(CURRENCY, CENTRAL_BANK), -0.1), "currency is the group's");
    let firms_debt = -(0..11).filter(|r| *r != 10).map(|r| s.at(r, FIRMS)).filter(|v| *v < 0.0).sum::<f64>();
    assert!(close(firms_debt, 1.2), "firms owe what was drawn");
    let stocks = Stocks { financial: rows(&s.financial), real: rows(&s.real) };
    assert!(stocks_breaks(&stocks, 1e-12).is_empty(), "{:?}", stocks_breaks(&stocks, 1e-12));
}

#[test]
fn a_draw_the_closures_cannot_balance_is_refused() {
    let firms_owe_all = Drawn { firm_debt: 40.0, ..DRAWN };
    assert!(map(&group(), &firms_owe_all).is_err());
}
