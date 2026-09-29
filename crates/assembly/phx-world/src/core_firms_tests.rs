//! The centring of a product's productivities written out by hand.
#![cfg(test)]

use super::{centring, mean_hours};
use sys_frm::rules::way::own_hours;

/// A unit all labour, its price its hours: the logit's chances do not move with the shift, so the shift is the log
/// of the mean hours the chances weigh.
#[test]
fn centring_makes_the_mean_hours_the_ways() {
    let cells = [(1.0, vec![0.0, core::f64::consts::LN_2])];
    let price = |log: f64| own_hours(1.0, log);
    let shift = centring(&cells, &price, 1.0);
    // Chances a third and two thirds; hours one and a half: a mean of two thirds, which the shift makes one.
    assert!((own_hours(1.0, shift) - 1.5).abs() < 1e-12, "{shift}");
    assert!((mean_hours(&cells, &price, 1.0, shift) - 1.0).abs() < 1e-12);
}

#[test]
fn cells_weigh_by_their_share() {
    let cells = [(0.25, vec![0.0]), (0.75, vec![core::f64::consts::LN_2])];
    let price = |log: f64| 1.0 + own_hours(1.0, log);
    let shift = centring(&cells, &price, 2.0);
    assert!((mean_hours(&cells, &price, 2.0, shift) - 1.0).abs() < 1e-12);
    assert!((own_hours(1.0, shift) - 1.0 / (0.25 + 0.75 / 2.0)).abs() < 1e-12, "one firm a cell: its chance is one");
}
