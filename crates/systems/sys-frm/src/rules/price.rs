//! What to charge: the price the firm would like, its markup over expected unit cost raised by the pressure of its
//! demand, posted at the nearest point of its trade's table, and moved only when what the move gains over the review
//! interval exceeds what changing the price costs it.
//!
//! The gain is read from the firm's own markup. A seller who prices at a markup μ over cost believes its demand's
//! elasticity is (1 + μ) ÷ μ, and a price off its best by a log gap x then loses it R·x² ÷ (2μ) of its revenue R; a firm
//! whose markup is at or below nothing holds no such belief, and any gap loses it without bound.

use libm::{log, pow};
use phx_macros::clause;
use phx_num::Missing;

/// The pressure on a stocked good's seller: demand against expected, and the stock it aims for against what it holds,
/// each counting a period's expected demand, so it is defined at no stock; missing while it expects no demand.
#[clause("FRM.5", "REP.34")]
pub fn pressure_stocked(demand: f64, expected: f64, target_stock: f64, stock: f64) -> Missing<f64> {
    if expected <= 0.0 {
        return Missing::Absent;
    }
    Missing::Present((demand / expected) * ((target_stock + expected) / (stock + expected)))
}

/// The pressure on a service's seller: demand against expected, and the fill it aims for against its fill; missing
/// while it expects no demand or has no fill.
#[clause("FRM.5", "REP.34")]
pub fn pressure_service(demand: f64, expected: f64, target_fill: f64, fill: f64) -> Missing<f64> {
    if expected <= 0.0 || fill <= 0.0 {
        return Missing::Absent;
    }
    Missing::Present((demand / expected) * (target_fill / fill))
}

/// The price the firm would like: `(1 + μ)·E[unit cost]·π^η`.
#[clause("FRM.5")]
#[must_use]
pub fn desired(markup: f64, unit_cost: f64, pressure: f64, curvature: f64) -> f64 {
    (1.0 + markup) * unit_cost * pow(pressure, curvature)
}

/// The point of the trade's table nearest a price, the lower on a tie; none in an empty table.
#[clause("REP.34")]
#[must_use]
pub fn nearest_point(points: &[i64], price: f64) -> Option<i64> {
    let mut best: Option<(i64, f64)> = None;
    for p in points {
        let gap = (phx_rand::float::from_i64(*p) - price).abs();
        match best {
            Some((_, g)) if g <= gap => {}
            _ => best = Some((*p, gap)),
        }
    }
    best.map(|(p, _)| p)
}

/// What a firm loses over an interval of revenue `revenue` by posting `posted` where it would like `wanted`.
fn loss(revenue: f64, markup: f64, posted: f64, wanted: f64) -> f64 {
    let gap = log(posted / wanted);
    if markup <= 0.0 {
        return if gap == 0.0 { 0.0 } else { f64::INFINITY };
    }
    revenue * gap * gap / (2.0 * markup)
}

/// The review's choice: the point to post, if moving to it gains more over the interval than the move costs.
#[clause("FRM.5", "REP.34")]
#[must_use]
pub fn reprice(points: &[i64], current: i64, wanted: f64, revenue: f64, markup: f64, menu_cost: f64) -> Option<i64> {
    let point = nearest_point(points, wanted)?;
    if point == current {
        return None;
    }
    let (now, then) = (phx_rand::float::from_i64(current), phx_rand::float::from_i64(point));
    let gain = loss(revenue, markup, now, wanted) - loss(revenue, markup, then, wanted);
    (gain > menu_cost).then_some(point)
}

/// The loss's curvature a day in the log price gap, for the firm's attention: its daily revenue over its markup, or
/// none while its markup is at or below nothing and any gap costs it without bound.
#[clause("REP.38")]
pub fn curvature(revenue_per_day: f64, markup: f64) -> Missing<f64> {
    if markup <= 0.0 {
        return Missing::Absent;
    }
    Missing::Present(revenue_per_day / markup)
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::*;

    #[test]
    fn pressure_defined_at_zero_stock() {
        assert_eq!(pressure_stocked(100.0, 100.0, 200.0, 200.0), Missing::Present(1.0));
        assert_eq!(pressure_stocked(100.0, 100.0, 200.0, 0.0), Missing::Present(3.0));
        assert_eq!(pressure_stocked(100.0, 0.0, 200.0, 0.0), Missing::Absent);
        assert_eq!(pressure_service(120.0, 100.0, 0.8, 0.8), Missing::Present(1.2));
    }

    #[test]
    fn price_is_a_point() {
        let points = [99, 149, 199, 249];
        assert_eq!(nearest_point(&points, 160.0), Some(149));
        assert_eq!(nearest_point(&points, 175.0), Some(199));
        assert_eq!(nearest_point(&points, 174.0), Some(149), "a tie posts the lower");
        assert_eq!(nearest_point(&[], 10.0), None);
        assert!((desired(0.25, 100.0, 1.0, 0.5) - 125.0).abs() < 1e-12);
    }

    #[test]
    fn price_moves_only_past_menu_cost() {
        let points = [100, 110, 120, 130];
        // A gap of ln(1.1) on revenue 1000 at markup 0.25 loses about 18.2.
        assert_eq!(reprice(&points, 100, 110.0, 1000.0, 0.25, 10.0), Some(110));
        assert_eq!(reprice(&points, 100, 110.0, 1000.0, 0.25, 20.0), None);
        assert_eq!(reprice(&points, 110, 110.0, 1000.0, 0.25, 0.0), None);
        assert_eq!(reprice(&points, 100, 110.0, 1000.0, 0.0, 1e12), Some(110));
    }

    #[test]
    fn attention_curvature_from_markup() {
        assert_eq!(curvature(100.0, 0.25), Missing::Present(400.0));
        assert_eq!(curvature(100.0, 0.0), Missing::Absent);
    }
}
