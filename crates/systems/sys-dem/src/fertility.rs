//! A household's decision to try for a child: it tries when the next child's value to it is above nought. The value
//! is the log of how far the child brings it towards its ideal number, less the log of how much the child adds to its
//! needs on the equivalence scale and of the care its youngest child still takes, and its taste at the decision.

use phx_macros::clause;

/// What the decision reads of a household.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChildIn {
    /// Its outlook of its income a year, if it has formed one.
    pub income: Option<f64>,
    /// Its members out of school, and its children in it.
    pub adults: u32,
    pub children: u32,
    /// Its youngest child's age in whole years, if it has a child.
    pub youngest: Option<i64>,
    /// Its ideal number of children.
    pub ideal: u32,
    /// Its taste at this decision, a standard logistic draw times the spread.
    pub taste: f64,
}

/// What a member adds to a household's needs, as a share of its first adult's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub adult: f64,
    pub child: f64,
}

impl Scale {
    /// A household's needs as a multiple of one adult's: its first adult, each other adult and each child.
    fn needs(self, adults: u32, children: u32) -> f64 {
        let others = adults.checked_sub(1).map_or(0.0, f64::from);
        1.0 + self.adult * others + self.child * f64::from(children)
    }
}

/// The next child's value to the household; none where it has no income to raise a child on.
#[clause("POP.10")]
#[must_use]
pub fn value(i: &ChildIn, scale: Scale) -> Option<f64> {
    if !i.income.is_some_and(|y| y > 0.0) {
        return None;
    }
    let (ideal, have) = (f64::from(i.ideal), f64::from(i.children));
    let toward = libm::log((1.0 + ideal) / (1.0 + have));
    let needs = libm::log(scale.needs(i.adults, i.children + 1) / scale.needs(i.adults, i.children));
    // A young child still takes its care: the younger it is, the more a newborn beside it would cost.
    let care = i.youngest.map_or(0.0, |a| libm::log1p(1.0 / (1.0 + phx_rand::float::from_i64(a))));
    Some(toward - needs - care + i.taste)
}

/// Whether the household tries for a child.
#[must_use]
pub fn tries(i: &ChildIn, scale: Scale) -> bool {
    value(i, scale).is_some_and(|v| v > 0.0)
}

/// A standard logistic draw at a uniform share.
#[must_use]
pub fn logistic(u: f64) -> f64 {
    libm::log(u / (1.0 - u))
}

#[cfg(test)]
mod tests {
    use super::{ChildIn, Scale, tries, value};

    const OECD: Scale = Scale { adult: 0.5, child: 0.3 };

    fn couple() -> ChildIn {
        ChildIn { income: Some(40_000.0), adults: 2, children: 0, youngest: None, ideal: 2, taste: 0.0 }
    }

    #[test]
    fn fertility_value_inputs() {
        let base = value(&couple(), OECD).unwrap();
        let one = ChildIn { children: 1, youngest: Some(4), ..couple() };
        let baby = ChildIn { youngest: Some(0), ..one };
        assert!(value(&one, OECD).unwrap() < base, "each child brings the household nearer its ideal");
        assert!(value(&baby, OECD).unwrap() < value(&one, OECD).unwrap(), "a younger youngest child lowers the value");
        let at_ideal = ChildIn { children: 2, youngest: Some(6), ..couple() };
        assert!(!tries(&at_ideal, OECD), "at its ideal, with no taste for more, it does not try");
        assert!(tries(&ChildIn { taste: 1.0, ..at_ideal }, OECD), "a taste for another carries it past its ideal");
        assert_eq!(value(&ChildIn { income: None, ..couple() }, OECD), None, "no income, no decision to try");
        assert_eq!(value(&ChildIn { income: Some(0.0), ..couple() }, OECD), None);
        // One rule for every composition: a single adult, a large household, one with no ideal of children.
        let single = ChildIn { adults: 1, ..couple() };
        assert!(value(&single, OECD).unwrap() < base, "the child is a larger share of a single adult's needs");
        assert!(value(&ChildIn { adults: 5, children: 3, youngest: Some(2), ideal: 4, ..couple() }, OECD).is_some());
        assert!(!tries(&ChildIn { ideal: 0, ..couple() }, OECD), "a household that wants no children does not try");
    }
}
