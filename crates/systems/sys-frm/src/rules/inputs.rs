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
