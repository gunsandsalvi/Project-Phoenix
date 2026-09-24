use phx_core::CarryingBasis;
use phx_macros::clause;
use phx_num::Missing;

/// A position as its carrying value is read: what it cost, what its effective-interest schedule carries it at today
/// (a contract's), the depreciation its declared method has charged (a real asset's), and what write-downs have taken
/// off and not reversed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub cost: i64,
    pub amortised: Missing<i64>,
    pub depreciation: i64,
    pub written_down: i64,
}

/// A position's carrying value, read on its basis: fair value reads the mark or a named valuer's valuation of the
/// units; amortised cost the effective-interest schedule; the lower of cost and realisable value both; cost less
/// depreciation the declared method. Where what the basis reads is absent, the value is absent — never cost, never
/// zero.
#[clause("ACC.2", "NUM.8")]
pub fn carrying(basis: CarryingBasis, held: &Held, marked: Missing<i64>) -> Missing<i64> {
    match basis {
        CarryingBasis::FairValue => marked,
        CarryingBasis::AmortisedCost => match held.amortised {
            Missing::Present(a) => Missing::Present(a - held.written_down),
            Missing::Absent => Missing::Absent,
        },
        CarryingBasis::LowerOfCostAndRealisable => match marked {
            Missing::Present(m) => {
                let cost = held.cost - held.written_down;
                Missing::Present(if m < cost { m } else { cost })
            }
            Missing::Absent => Missing::Absent,
        },
        CarryingBasis::CostLessDepreciation => Missing::Present(held.cost - held.depreciation - held.written_down),
    }
}

/// A position's unrealised difference, the units at the latest price less their carrying value: readable wherever a
/// price exists, whatever the basis keeps out of income.
#[clause("ACC.3")]
pub fn unrealised(marked: Missing<i64>, carried: Missing<i64>) -> Missing<i64> {
    match (marked, carried) {
        (Missing::Present(m), Missing::Present(c)) => Missing::Present(m - c),
        _ => Missing::Absent,
    }
}

/// What a move in the mark does to income: on fair value the whole move is income the day it happens; on any other
/// basis nothing, the move staying an unrealised difference until a sale realises it.
#[clause("ACC.2", "ACC.8")]
pub fn income_from_mark(basis: CarryingBasis, before: Missing<i64>, after: Missing<i64>) -> Missing<i64> {
    match (basis, before, after) {
        (CarryingBasis::FairValue, Missing::Present(b), Missing::Present(a)) => Missing::Present(a - b),
        (CarryingBasis::FairValue, _, _) => Missing::Absent,
        _ => Missing::Present(0),
    }
}

#[cfg(test)]
mod tests {
    use phx_core::CarryingBasis;
    use phx_num::Missing;

    use super::{Held, carrying, income_from_mark, unrealised};

    #[test]
    fn bank_bond_two_bases() {
        // A bank holds two bonds bought at 1 000, one to collect at amortised cost and one at fair value; the price
        // falls to 950.
        let bond = Held { cost: 1_000, amortised: Missing::Present(1_000), depreciation: 0, written_down: 0 };
        let (before, after) = (Missing::Present(1_000), Missing::Present(950));
        let held = carrying(CarryingBasis::AmortisedCost, &bond, after);
        let fair = carrying(CarryingBasis::FairValue, &bond, after);
        assert_eq!((held, fair), (Missing::Present(1_000), Missing::Present(950)));
        assert_eq!(income_from_mark(CarryingBasis::FairValue, before, after), Missing::Present(-50), "in income");
        assert_eq!(income_from_mark(CarryingBasis::AmortisedCost, before, after), Missing::Present(0));
        assert_eq!(unrealised(after, held), Missing::Present(-50), "hidden from income, never from a reader");
        assert_eq!(unrealised(after, fair), Missing::Present(0));
    }

    #[test]
    fn absent_mark_is_unreadable() {
        let stock = Held { cost: 500, amortised: Missing::Absent, depreciation: 0, written_down: 0 };
        assert_eq!(carrying(CarryingBasis::FairValue, &stock, Missing::Absent), Missing::Absent, "never at cost");
        assert_eq!(carrying(CarryingBasis::LowerOfCostAndRealisable, &stock, Missing::Absent), Missing::Absent);
        assert_eq!(
            carrying(CarryingBasis::LowerOfCostAndRealisable, &stock, Missing::Present(450)),
            Missing::Present(450)
        );
        let plant = Held { cost: 800, amortised: Missing::Absent, depreciation: 120, written_down: 0 };
        assert_eq!(carrying(CarryingBasis::CostLessDepreciation, &plant, Missing::Absent), Missing::Present(680));
    }
}
