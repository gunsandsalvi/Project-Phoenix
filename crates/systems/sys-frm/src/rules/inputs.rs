//! What inputs to order: what the planned production uses over the inputs' lead time and the input stock the firm
//! aims to hold, less what it holds and has on order, and only while a unit of the input earns more in use than it
//! costs, its financing counted.

use phx_macros::clause;

/// The units of an input to order, none where what it holds covers the plan.
#[clause("FRM.7", "GDS.5")]
#[must_use]
pub fn order(use_per_period: f64, (lead, cover): (f64, f64), held: f64, value_in_use: f64, cost: f64) -> f64 {
    if value_in_use <= cost {
        return 0.0;
    }
    let short = use_per_period * (lead + cover) - held;
    if short > 0.0 { short } else { 0.0 }
}

/// What a unit of an input costs the firm by the time its output sells: its price and the financing of the money
/// held in it over that time.
#[clause("FRM.4", "GDS.5")]
#[must_use]
pub fn carried_cost(price: f64, financing_rate: f64, periods_held: f64) -> f64 {
    price * (1.0 + financing_rate * periods_held)
}

/// An input a firm weighs ordering: what its plan uses of it a day, what it holds, its least price a unit, what a
/// unit is worth to the firm, and the most it pays for a lot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub use_a_day: f64,
    pub held: f64,
    pub price: f64,
    pub worth: f64,
    pub limit: i64,
}

/// What its orders read: its inputs, the days a unit of its product takes and its stock's cover, its financing rate a
/// day and the money it can spend.
#[derive(Clone, Debug, PartialEq)]
pub struct OrdersIn {
    pub lines: Vec<Line>,
    pub lead: f64,
    pub cover: f64,
    pub financing: f64,
    pub free: f64,
}

/// The units of each input ordered, cut alike to what the firm can spend at the least prices; none of an input worth
/// less than it costs carried or of which it holds enough.
#[clause("FRM.7", "GDS.5")]
#[must_use]
pub fn orders(i: &OrdersIn) -> Vec<f64> {
    let short: Vec<f64> = i
        .lines
        .iter()
        .map(|l| {
            let carried = carried_cost(l.price, i.financing, i.lead + i.cover);
            let s = order(l.use_a_day, (i.lead, i.cover), l.held, l.worth, carried);
            if s > 0.0 && l.limit > 0 { s } else { 0.0 }
        })
        .collect();
    let cost: f64 = short.iter().zip(&i.lines).map(|(s, l)| s * l.price).sum();
    let scale = if cost > i.free { i.free / cost } else { 1.0 };
    short.into_iter().map(|s| s * scale).collect()
}

#[cfg(test)]
mod tests {
    use super::{carried_cost, order};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn input_order_counts_financing() {
        assert!(close(order(10.0, (2.0, 1.0), 5.0, 12.0, 10.0), 25.0));
        assert!(close(order(10.0, (2.0, 1.0), 40.0, 12.0, 10.0), 0.0));
        let cost = carried_cost(10.0, 0.05, 5.0);
        assert!(close(cost, 12.5));
        assert!(close(order(10.0, (2.0, 1.0), 5.0, 12.0, cost), 0.0), "financing takes the value in use");
    }
}
