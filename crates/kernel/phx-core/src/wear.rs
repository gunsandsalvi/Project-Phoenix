//! Units that wear through classes as they age: a system declares, for each chain the ledger keeps by its tag, the
//! yearly rate units leave each class and each class's value as a share of cost new, and the visit at whose rows the
//! kernel realises the wear since the last.

use phx_macros::clause;
use phx_rand::float::{floor_to_i64, from_i64};

use crate::register::Register;

/// What wears along one tag's chains: the yearly rate units leave each class, and each class's value, newest first,
/// as a share of what a unit cost new.
#[derive(Clone, Debug, PartialEq)]
pub struct WearSpec {
    pub tag: u32,
    pub leaving_per_year: f64,
    pub values: Vec<f64>,
}

/// The wear a system's chains undergo, realised on the rows of the visit whose handler `visit` names, from the
/// register at assembly.
#[clause("CAP.6", "REP.24")]
#[derive(Clone, Copy, Debug)]
pub struct WearDecl {
    pub visit: &'static str,
    pub specs: fn(&Register) -> Result<Vec<WearSpec>, String>,
    pub clause: &'static str,
}

/// Whole units a twin's holding of a class loses to wear over `days`, half to even: the class's units times the share
/// that leaves over the days at the yearly rate, 1 − e^(−rate·t), each unit leaving at a constant hazard, so the
/// share never passes the whole. `None` beyond an integer's reach.
#[clause("CAP.6", "REP.24")]
#[must_use]
pub fn leaving(held: i64, days: i64, rate: f64, days_a_year: i64) -> Option<i64> {
    let share = -libm::expm1(-rate * from_i64(days) / from_i64(days_a_year));
    half_even(from_i64(held) * share)
}

/// What worn units carry to the next class: what their lots cost, times the next class's value over their own.
#[clause("CAP.6", "ACC.6")]
#[must_use]
pub fn carried(cost: i64, value_from: f64, value_to: f64) -> Option<i64> {
    half_even(from_i64(cost) * value_to / value_from)
}

/// The nearest integer, a half to the even one.
fn half_even(x: f64) -> Option<i64> {
    let floor = floor_to_i64(x)?;
    let twice = 2.0 * (x - from_i64(floor));
    Some(if twice > 1.0 || (twice >= 1.0 && floor % 2 != 0) { floor + 1 } else { floor })
}

#[cfg(test)]
mod tests {
    use super::half_even;

    #[test]
    fn halves_go_to_the_even_integer() {
        assert_eq!(
            (half_even(2.5), half_even(3.5), half_even(2.4), half_even(-0.5)),
            (Some(2), Some(4), Some(2), Some(0))
        );
    }
}
