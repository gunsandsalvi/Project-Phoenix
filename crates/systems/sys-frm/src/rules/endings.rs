//! Whether a household founds a firm and whether an owner closes one: each the comparison of the owner's own values,
//! the venture's against the alternative, continuing against winding down.

use phx_macros::clause;

/// A founder founds when its value of the venture exceeds what its money earns otherwise.
#[clause("FRM.16")]
#[must_use]
pub fn founds(venture: f64, alternative: f64) -> bool {
    venture > alternative
}

/// An owner closes a solvent firm when continuing is worth less than what winding down returns: its stock and plant
/// at the prices it expects to fetch, less what it owes.
#[clause("FRM.15")]
#[must_use]
pub fn closes(continuing: f64, (stock_and_plant, owed): (f64, f64)) -> bool {
    continuing < stock_and_plant - owed
}

#[cfg(test)]
mod tests {
    use super::{closes, founds};

    #[test]
    fn found_compares_value_and_alternative() {
        assert!(founds(110.0, 100.0));
        assert!(!founds(100.0, 100.0));
    }

    #[test]
    fn close_compares_continuing_and_winding_down() {
        assert!(closes(50.0, (100.0, 40.0)));
        assert!(!closes(70.0, (100.0, 40.0)));
    }
}
