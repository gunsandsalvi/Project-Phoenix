//! What an owner does with a unit of plant: keeps it as it is, or maintains or repairs it, sells it or scraps it,
//! whichever is worth most to it, and only when that is worth more than keeping it.

use phx_macros::clause;

/// What an owner can do with a unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Keep,
    Maintain,
    Repair,
    Sell,
    Scrap,
}

/// The choice worth most, each option's value what the unit is worth to the owner after it net of what it costs:
/// keeping as it is unless another option is worth strictly more, the first of those declared on a tie.
#[clause("CAP.4")]
#[must_use]
pub fn choose(keep: f64, others: &[(Choice, f64)]) -> Choice {
    let mut best = (Choice::Keep, keep);
    for (choice, worth) in others {
        if *worth > best.1 {
            best = (*choice, *worth);
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::{Choice, choose};

    #[test]
    fn maintain_when_worth_more_than_keeping() {
        assert_eq!(choose(100.0, &[(Choice::Maintain, 120.0), (Choice::Sell, 90.0)]), Choice::Maintain);
        assert_eq!(choose(100.0, &[(Choice::Repair, 100.0)]), Choice::Keep, "no gain, no change");
        assert_eq!(choose(10.0, &[(Choice::Sell, 30.0), (Choice::Scrap, 30.0)]), Choice::Sell, "the first on a tie");
    }
}
