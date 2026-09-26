//! Goods that spoil in stock: a system declares each product's yearly rate of loss and the visit at whose rows the
//! kernel realises the loss since the last, lot by lot from each lot's age, so a lot bought since the last visit loses
//! only for the days it was held.

use phx_macros::clause;
use phx_rand::float::from_i64;

use crate::register::Register;

/// The spoilage of a system's goods, realised on the rows of the visit whose handler `visit` names: each product's
/// yearly rate of loss in stock, by the product's place, read from the register at assembly.
#[clause("GDS.8")]
#[derive(Clone, Copy, Debug)]
pub struct SpoilageDecl {
    pub visit: &'static str,
    pub rates: fn(&Register) -> Result<Vec<f64>, String>,
    pub clause: &'static str,
}

/// Whole units one twin's stock loses over the visit's period: each lot, its units a twin's share of them, losing at
/// the yearly rate for the days it was held within the period, 1 − e^(−rate·t) of it, the sum rounded half to even.
/// `None` beyond an integer's reach.
#[clause("GDS.8")]
#[must_use]
pub fn lost(lots: &[(i64, i64)], period: i64, rate: f64, twins: i64, days_a_year: i64) -> Option<i64> {
    let mut sum = 0.0;
    for (held, age) in lots {
        let days = if *age < period { *age } else { period };
        sum += from_i64(*held) * -libm::expm1(-rate * from_i64(days) / from_i64(days_a_year));
    }
    crate::wear::half_even(sum / from_i64(twins))
}

#[cfg(test)]
mod tests {
    use super::lost;

    #[test]
    fn spoilage_exact_units() {
        // A thousand units held a year at a tenth a year lose 1000 × (1 − e^−0.1) = 95.16, so 95.
        assert_eq!(lost(&[(1000, 400)], 365, 0.1, 1, 365), Some(95));
        // A lot bought ten days ago loses only for those ten days.
        assert_eq!(lost(&[(1000, 10)], 365, 0.1, 1, 365), Some(3));
        // An agent of ten twins loses a twin's whole units, the same share of each lot.
        assert_eq!(lost(&[(10_000, 400)], 365, 0.1, 10, 365), Some(95));
        assert_eq!(lost(&[(1000, 400)], 365, 0.0, 1, 365), Some(0), "a good that does not spoil loses nothing");
    }
}
