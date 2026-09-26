//! What to make in a period: expected demand plus the gap to the target stock closed over the adjustment time, as the
//! production-smoothing model has it, at most what the plant, inputs and labour can make, and nothing where the
//! margin at expected prices, counting the financing of the work in progress, is not positive.

use phx_macros::clause;

/// What the production decision reads, per period of the firm's schedule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProduceIn {
    /// Units the firm expects to sell in a period.
    pub expected_demand: f64,
    /// Units of output on hand.
    pub stock: f64,
    /// The stock it aims to hold, in periods of expected demand.
    pub cover: f64,
    /// Periods over which it closes the gap to that stock.
    pub adjustment: f64,
    /// The most it can make in a period: the least that its plant, inputs and labour allow.
    pub capacity: f64,
    /// The price it expects to sell at, and what a unit costs it.
    pub expected_price: f64,
    pub unit_cost: f64,
    /// What financing a unit's cost costs it a period, and the periods a unit takes to finish.
    pub financing_rate: f64,
    pub lead: f64,
}

/// The decision: units to start in the period, or nothing and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Produce {
    Make(f64),
    /// The stock on hand already covers what it expects to sell and aims to hold.
    StockCovers,
    /// A unit would not pay at the price it expects, counting its financing.
    NoMargin,
    /// Its plant, inputs or labour allow nothing this period.
    NoCapacity,
}

/// The period's target output.
#[clause("FRM.4")]
#[must_use]
pub fn target(p: &ProduceIn) -> Produce {
    let carried = p.unit_cost * (1.0 + p.financing_rate * p.lead);
    if p.expected_price <= carried {
        return Produce::NoMargin;
    }
    let wanted = p.expected_demand + (p.cover * p.expected_demand - p.stock) / p.adjustment;
    if wanted <= 0.0 {
        return Produce::StockCovers;
    }
    if p.capacity <= 0.0 {
        return Produce::NoCapacity;
    }
    Produce::Make(if wanted < p.capacity { wanted } else { p.capacity })
}

#[cfg(test)]
mod tests {
    use super::{Produce, ProduceIn, target};

    fn base() -> ProduceIn {
        ProduceIn {
            expected_demand: 100.0,
            stock: 200.0,
            cover: 2.0,
            adjustment: 4.0,
            capacity: 1000.0,
            expected_price: 10.0,
            unit_cost: 8.0,
            financing_rate: 0.01,
            lead: 1.0,
        }
    }

    #[test]
    fn produce_target_formula() {
        assert_eq!(target(&base()), Produce::Make(100.0));
        let short = ProduceIn { stock: 0.0, ..base() };
        assert_eq!(target(&short), Produce::Make(150.0));
        assert_eq!(target(&ProduceIn { capacity: 120.0, ..short }), Produce::Make(120.0));
    }

    #[test]
    fn produce_kinks_at_zero() {
        assert_eq!(target(&ProduceIn { stock: 700.0, ..base() }), Produce::StockCovers);
        assert_eq!(target(&ProduceIn { stock: 600.0, ..base() }), Produce::StockCovers);
        assert_eq!(target(&ProduceIn { capacity: 0.0, ..base() }), Produce::NoCapacity);
    }

    #[test]
    fn financing_can_take_the_margin() {
        assert_eq!(target(&ProduceIn { unit_cost: 9.95, ..base() }), Produce::NoMargin);
        assert_eq!(target(&ProduceIn { unit_cost: 9.0, lead: 20.0, ..base() }), Produce::NoMargin);
    }
}
