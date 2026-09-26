//! Which way to run: among the ways a firm knows and its plant supports, the one whose unit cost is lowest at the
//! prices it faces, each way's cost its inputs, hours and plant's charge per unit over the share of starts it finishes.

use phx_macros::clause;
use phx_num::Missing;

/// A way's quantities per unit of output, each with the price the firm faces for it: its inputs, its hours by
/// occupation, and its plant's charge per unit; and the share of what it starts that it finishes.
#[derive(Clone, Debug, PartialEq)]
pub struct Costed {
    pub inputs: Vec<(f64, Missing<f64>)>,
    pub hours: Vec<(f64, Missing<f64>)>,
    pub plant_charge: f64,
    pub finished_share: f64,
}

/// A way's unit cost, or none while any price it needs is missing.
#[clause("FRM.14", "FRM.6")]
pub fn unit_cost(way: &Costed) -> Missing<f64> {
    let mut total = way.plant_charge;
    for (quantity, price) in way.inputs.iter().chain(&way.hours) {
        match price {
            Missing::Present(p) => total += quantity * p,
            Missing::Absent => return Missing::Absent,
        }
    }
    if way.finished_share <= 0.0 {
        return Missing::Absent;
    }
    Missing::Present(total / way.finished_share)
}

/// The cheapest of the ways that can be costed, the first on a tie; none if none can.
#[clause("FRM.6")]
#[must_use]
pub fn cheapest<K: Copy>(ways: &[(K, Costed)]) -> Option<(K, f64)> {
    let mut best: Option<(K, f64)> = None;
    for (k, w) in ways {
        if let Missing::Present(c) = unit_cost(w) {
            match best {
                Some((_, b)) if b <= c => {}
                _ => best = Some((*k, c)),
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::{Costed, cheapest, unit_cost};

    fn way(input_price: Missing<f64>, wage: f64, finished: f64) -> Costed {
        Costed {
            inputs: vec![(2.0, input_price)],
            hours: vec![(0.5, Missing::Present(wage))],
            plant_charge: 1.0,
            finished_share: finished,
        }
    }

    #[test]
    fn way_choice_cheapest() {
        let a = way(Missing::Present(3.0), 10.0, 1.0);
        assert_eq!(unit_cost(&a), Missing::Present(12.0));
        assert_eq!(unit_cost(&way(Missing::Present(3.0), 10.0, 0.5)), Missing::Present(24.0));
        let b = way(Missing::Present(2.0), 10.0, 1.0);
        let c = way(Missing::Absent, 0.0, 1.0);
        assert_eq!(cheapest(&[(0, a), (1, b), (2, c)]), Some((1, 10.0)));
        assert_eq!(cheapest::<u8>(&[]), None);
    }
}
