//! Whether a household founds a firm and whether an owner closes one: each the comparison of the owner's own values,
//! the venture's against the alternative, continuing against winding down.

use phx_macros::clause;

/// A founder founds when its value of the venture exceeds what its money earns otherwise.
#[clause("FRM.16")]
#[must_use]
pub fn founds(venture: f64, alternative: f64) -> bool {
    venture > alternative
}

/// An owner exits a line it cannot make pay: when continuing is worth less than what ending adds — what its stock
/// and plant would fetch, less what ending alone costs. Its money and its debts are the owner's either way, so they
/// weigh on neither side.
#[clause("FRM.11")]
#[must_use]
pub fn closes(continuing: f64, (proceeds, ending_costs): (f64, f64)) -> bool {
    continuing < proceeds - ending_costs
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
        assert!(!closes(0.0, (0.0, 40.0)), "a line that pays nothing still saves its severance by going on");
        assert!(closes(-50.0, (0.0, 40.0)), "one that loses more than ending costs ends");
    }
}
