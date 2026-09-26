//! Chains of classes: instruments whose units pass from each to the next as they wear and leave the last as they
//! retire, each chain tagged with what its system reads it by. A wear leg moves units only along its chain, and what
//! one class gives the next receives.

use std::collections::BTreeMap;

use phx_id::{InstrumentId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::instruction::{AccountRef, LegKind, LegRec, Source};

/// A chain: its system's tag and its classes, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Chain {
    pub tag: u32,
    pub classes: Vec<InstrumentId>,
}

/// The chains declared, each numbered in the order it was declared.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Chains {
    chains: Vec<Chain>,
}

impl Chains {
    /// A chain declared over its classes, which no other chain holds.
    #[clause("REP.24", "CAP.1")]
    pub fn declare(&mut self, tag: u32, classes: Vec<InstrumentId>) -> u32 {
        if let Some(i) = classes.iter().find(|c| matches!(self.of(**c), Missing::Present(_))) {
            violation!(clause = "CAP.1", "a class in two chains", instrument = i.get());
        }
        let Ok(n) = u32::try_from(self.chains.len()) else {
            phx_num::capacity_exceeded!("chains of classes", u32::MAX, self.chains.len());
        };
        self.chains.push(Chain { tag, classes });
        n
    }

    /// A chain by its number.
    #[must_use]
    pub fn get(&self, chain: u32) -> Option<&Chain> {
        usize::try_from(chain).ok().and_then(|i| self.chains.get(i))
    }

    /// The chain and class an instrument is, if it is one.
    pub fn of(&self, instrument: InstrumentId) -> Missing<(u32, usize)> {
        for (n, c) in (0_u32..).zip(&self.chains) {
            if let Some(at) = c.classes.iter().position(|i| *i == instrument) {
                return Missing::Present((n, at));
            }
        }
        Missing::Absent
    }

    /// Every chain in order.
    pub fn iter(&self) -> impl Iterator<Item = &Chain> {
        self.chains.iter()
    }

    /// An instruction's wear legs checked: each on a class of the chain it names, none entering a chain's newest
    /// class, and each class after the first receiving from a holder what the one before it gave that holder.
    #[clause("CAP.8", "REP.24")]
    pub fn check(&self, legs: &[LegRec]) {
        let mut moved: BTreeMap<(PartyId, u32, usize), (i64, i64)> = BTreeMap::new();
        for leg in legs {
            let (LegKind::Transformation { source: Source::Wear(chain), .. }, AccountRef::Instrument(id)) =
                (leg.kind, leg.account)
            else {
                continue;
            };
            let Missing::Present((on, class)) = self.of(id) else {
                violation!(clause = "CAP.8", "a wear leg on no chain's class", instrument = id.get());
            };
            if on != chain {
                violation!(clause = "CAP.8", "a wear leg on another chain's class", instrument = id.get());
            }
            let (given, received) = moved.entry((leg.party, chain, class)).or_insert((0, 0));
            if leg.qty < 0 { *given -= leg.qty } else { *received += leg.qty }
        }
        for ((party, chain, class), (_, received)) in &moved {
            if *received == 0 {
                continue;
            }
            let from = class.checked_sub(1).map(|c| moved.get(&(*party, *chain, c)).map_or(0, |m| m.0));
            if from != Some(*received) {
                violation!(
                    clause = "CAP.8",
                    "a class receiving other than what the one before gave",
                    party = party.get()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{InstrumentId, PartyId};
    use phx_num::{Missing, UnitId};

    use super::Chains;
    use crate::instruction::{AccountRef, Denom, LegKind, LegRec, Source};

    fn leg(instrument: u32, qty: i64) -> LegRec {
        LegRec {
            party: PartyId::new(4),
            account: AccountRef::Instrument(InstrumentId::new(instrument)),
            qty,
            denom: Denom::Unit(UnitId::new(1)),
            kind: LegKind::Transformation { source: Source::Wear(0), cost: 0 },
        }
    }

    fn caught_clause(f: impl FnOnce()) -> Option<&'static str> {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).expect_err("the run stops");
        payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause)
    }

    #[test]
    fn wear_moves_units_along_its_chain_only() {
        let mut chains = Chains::default();
        let chain = chains.declare(3, (1..=3).map(InstrumentId::new).collect());
        assert_eq!(chain, 0);
        assert_eq!(chains.of(InstrumentId::new(2)), Missing::Present((0, 1)));
        chains.check(&[leg(1, -5), leg(2, 5), leg(2, -2), leg(3, 2), leg(3, -1)]);
        assert_eq!(
            caught_clause(|| chains.check(&[leg(1, -5), leg(2, 4)])),
            Some("CAP.8"),
            "given and received differ"
        );
        assert_eq!(caught_clause(|| chains.check(&[leg(1, 5)])), Some("CAP.8"), "nothing wears into the newest");
        assert_eq!(caught_clause(|| chains.check(&[leg(9, -1)])), Some("CAP.8"), "a class of no chain");
        assert_eq!(
            caught_clause(|| {
                let _ = chains.declare(4, vec![InstrumentId::new(3)]);
            }),
            Some("CAP.1")
        );
    }
}
