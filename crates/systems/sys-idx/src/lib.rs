//! IDX, indices: the price indices the statistics agency publishes, chained so a change of constituents never jumps
//! the level, each opening at its publisher's base.

use if_state::stats::{IndexKind, Priced};
use phx_core::{Declarations, HandlerTable, OpeningCountry, Register, System, declare_prim};
use phx_macros::clause;

declare_prim! {
    /// The level a price index opens at.
    pub BASE = "IDX.base" {
        kind: Policy, decided_by: "statistics agency", value: Fixed { exp: 2 }, clause: "IDX.7", scope: PerCountry
    }
}

/// A country's base level.
///
/// # Errors
/// The primitive missing.
pub fn base(register: &Register, c: &OpeningCountry) -> Result<f64, String> {
    register.fixed_in(BASE.id, c.id)
}

/// A chain's link: the constituents priced in both periods, each price relative weighted by its share of the period
/// before's spending among them. None where no constituent is priced in both, or no weight is left.
#[clause("IDX.3", "IDX.4", "IDX.6")]
#[must_use]
pub fn link(constituents: &[Priced]) -> Option<f64> {
    let priced = constituents.iter().filter(|c| c.before > 0.0 && c.now > 0.0 && c.weight > 0.0);
    let (sum, weight) = priced.fold((0.0, 0.0), |(s, w), c| (s + c.weight * c.now / c.before, w + c.weight));
    (weight > 0.0).then(|| sum / weight)
}

/// The price indices' publisher.
pub const INDEX: IndexKind = IndexKind { base, link };

/// Indices.
#[derive(Debug)]
pub struct Idx;

impl System for Idx {
    const CODE: &'static str = "IDX";

    fn declare(d: &mut Declarations) {
        let _: phx_core::Prim<phx_num::Fixed<2>> = d.prim(&BASE);
        d.market(Box::new(INDEX));
    }

    fn handlers(_: &mut HandlerTable) {}
}

#[cfg(test)]
mod tests {
    use if_state::stats::Priced;

    use super::link;

    #[test]
    fn chained_index_no_jump() {
        let bread = Priced { before: 2.0, now: 2.2, weight: 0.5 };
        let rent = Priced { before: 500.0, now: 500.0, weight: 0.5 };
        let l = link(&[bread, rent]).unwrap();
        assert!((l - 1.05).abs() < 1e-12, "bread up a tenth at half the weight: {l}");
        // A constituent new this period, priced only now, joins the next link and does not move this one.
        let new = Priced { before: 0.0, now: 900.0, weight: 0.0 };
        assert_eq!(link(&[bread, rent, new]), Some(l));
        // One that left, priced only before, leaves the others' link as theirs alone.
        let gone = Priced { before: 3.0, now: 0.0, weight: 0.2 };
        assert_eq!(link(&[bread, rent, gone]), Some(l));
        assert_eq!(link(&[gone, new]), None, "no constituent priced in both periods: no link");
        let level = 100.0 * l * link(&[Priced { before: 2.2, now: 2.2, weight: 1.0 }]).unwrap();
        assert!((level - 105.0).abs() < 1e-9, "a period of unchanged prices leaves the level where it was");
    }
}
