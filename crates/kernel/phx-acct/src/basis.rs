use phx_core::{CarryingBasis, Declarations, HeldFor, Permitted, Prim, declare_prim};
use phx_ledger::instrument::InstrumentFamily;
use phx_macros::clause;
use phx_num::Missing;

declare_prim! {
    /// The carrying bases the accounting standard permits each legal form for each purpose a position is held for,
    /// in the standard's order.
    pub CARRYING_BASES = "ACC.carrying_bases" {
        kind: Policy, decided_by: "accounting standard-setter", value: CarryingBases, clause: "ACC.17", scope: Shared
    }
}

/// The accounts kernel's primitives, declared with the kernel's.
#[derive(Debug)]
pub struct AcctPrims {
    pub carrying_bases: Prim<Vec<Permitted>>,
}

impl AcctPrims {
    pub fn declare(d: &mut Declarations) -> AcctPrims {
        AcctPrims { carrying_bases: d.prim(&CARRYING_BASES) }
    }
}

/// A holder's choice of basis for a position, refused where its legal form does not permit it for what the position
/// is held for.
///
/// # Errors
/// The basis refused, when the standard does not permit it.
#[clause("ACC.2", "ACC.17")]
pub fn choose(
    permitted: &[Permitted],
    form: &str,
    held_for: HeldFor,
    wanted: CarryingBasis,
) -> Result<CarryingBasis, CarryingBasis> {
    let allowed = permitted.iter().any(|p| p.form == form && p.held_for == held_for && p.bases.contains(&wanted));
    if allowed { Ok(wanted) } else { Err(wanted) }
}

/// The basis an opening position is carried at: the first its form is permitted for what it is held for, since
/// the opening's holders chose before the world began; absent where the standard permits the form nothing for it.
#[clause("ACC.2", "ACC.17")]
pub fn at_opening(permitted: &[Permitted], form: &str, held_for: HeldFor) -> Missing<CarryingBasis> {
    match permitted.iter().find(|p| p.form == form && p.held_for == held_for).and_then(|p| p.bases.first()) {
        Some(b) => Missing::Present(*b),
        None => Missing::Absent,
    }
}

/// What a position of an instrument family is held for, where its holder has not said: a contract or a debt to
/// collect, a share or a fund's unit to trade, a real asset to use, banknotes as money held.
#[must_use]
pub fn held_for(family: InstrumentFamily) -> HeldFor {
    match family {
        InstrumentFamily::Debt | InstrumentFamily::Contract | InstrumentFamily::Banknote => HeldFor::Collect,
        InstrumentFamily::Equity | InstrumentFamily::FundUnit => HeldFor::Trade,
        InstrumentFamily::RealAsset => HeldFor::Use,
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{CarryingBasis, HeldFor, Permitted};
    use phx_num::Missing;

    use super::{at_opening, choose};

    fn permitted() -> Vec<Permitted> {
        vec![
            Permitted {
                form: "bank".to_owned(),
                held_for: HeldFor::Collect,
                bases: vec![CarryingBasis::AmortisedCost, CarryingBasis::FairValue],
            },
            Permitted { form: "bank".to_owned(), held_for: HeldFor::Trade, bases: vec![CarryingBasis::FairValue] },
        ]
    }

    #[test]
    fn bases_within_the_standard() {
        let p = permitted();
        assert_eq!(choose(&p, "bank", HeldFor::Collect, CarryingBasis::FairValue), Ok(CarryingBasis::FairValue));
        assert_eq!(choose(&p, "bank", HeldFor::Trade, CarryingBasis::AmortisedCost), Err(CarryingBasis::AmortisedCost));
        assert_eq!(at_opening(&p, "bank", HeldFor::Collect), Missing::Present(CarryingBasis::AmortisedCost));
        assert_eq!(at_opening(&p, "company", HeldFor::Use), Missing::Absent, "nothing permitted, nothing chosen");
    }
}
