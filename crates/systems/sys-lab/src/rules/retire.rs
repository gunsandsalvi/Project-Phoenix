//! A person's retirement, until households weigh it: at the age its country's pension begins.

use if_labour::decisions::RetireIn;
use phx_macros::clause;

/// Whether a person retires: once it has reached the age its country's pension begins at. A placeholder naming HH,
/// whose household's decision replaces it.
#[clause("LAB.6", "HH.6")]
#[must_use]
pub fn retire(i: &RetireIn) -> bool {
    i.age_months >= i.pension_months
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::RetireIn;

    use super::retire;

    #[test]
    fn retires_at_the_pension_age() {
        assert!(!retire(&RetireIn { age_months: 797, pension_months: 798 }));
        assert!(retire(&RetireIn { age_months: 798, pension_months: 798 }));
    }
}
