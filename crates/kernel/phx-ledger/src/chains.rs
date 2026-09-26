//! Chains of classes: instruments whose units pass from each to the next as they wear and leave the last as they
//! retire, each chain tagged with what its system reads it by. A wear leg moves units only along its chain, and what
//! one class gives the next receives. Each chain's plant under construction is held apart from its classes, and
//! enters its newest class only when the project that builds it completes.

use std::collections::BTreeMap;

use phx_id::{Day, InstrumentId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::instruction::{AccountRef, LegKind, LegRec, Source};

/// A chain: its system's tag and its classes, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Chain {
    pub tag: u32,
    pub classes: Vec<InstrumentId>,
}

/// A construction project: its owner, its builder, the chain it adds to, the good the builder delivers for it and the
/// way it makes it by when it is made as it is delivered, the price agreed for a lot and the lot, the units a stage
/// delivers, the units it builds and those left to deliver, and the day it was ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Project {
    pub owner: PartyId,
    pub builder: PartyId,
    pub chain: u32,
    pub good: InstrumentId,
    pub way: Missing<u32>,
    pub price: i64,
    pub lot: i64,
    pub stage: i64,
    pub total: i64,
    pub left: i64,
    pub ordered: Day,
}

/// The chains declared, each numbered in the order it was declared; each chain's instrument for its plant under
/// construction; and the projects building, by their number.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Chains {
    declared: Vec<Chain>,
    building: BTreeMap<u32, InstrumentId>,
    projects: BTreeMap<u64, Project>,
    numbered: u64,
}

impl Chains {
    /// A chain declared over its classes, which no other chain holds.
    #[clause("REP.24", "CAP.1")]
    pub fn declare(&mut self, tag: u32, classes: Vec<InstrumentId>) -> u32 {
        if let Some(i) = classes.iter().find(|c| matches!(self.of(**c), Missing::Present(_))) {
            violation!(clause = "CAP.1", "a class in two chains", instrument = i.get());
        }
        let Ok(n) = u32::try_from(self.declared.len()) else {
            phx_num::capacity_exceeded!("chains of classes", u32::MAX, self.declared.len());
        };
        self.declared.push(Chain { tag, classes });
        n
    }

    /// A chain's plant under construction declared, an instrument no chain holds as a class.
    #[clause("CAP.2")]
    pub fn declare_building(&mut self, chain: u32, instrument: InstrumentId) {
        if matches!(self.of(instrument), Missing::Present(_)) || self.building.values().any(|b| *b == instrument) {
            violation!(
                clause = "CAP.2",
                "plant under construction that is already plant",
                instrument = instrument.get()
            );
        }
        self.building.insert(chain, instrument);
    }

    /// A chain's plant under construction.
    pub fn building(&self, chain: u32) -> Missing<InstrumentId> {
        match self.building.get(&chain) {
            Some(b) => Missing::Present(*b),
            None => Missing::Absent,
        }
    }

    /// A project begun, by the number it is known by.
    #[clause("CAP.5", "CAP.2")]
    pub fn begin(&mut self, project: Project) -> u64 {
        if project.stage <= 0
            || project.left != project.total
            || project.total <= 0
            || matches!(self.building(project.chain), Missing::Absent)
        {
            violation!(clause = "CAP.5", "a project of nothing or of no chain", owner = project.owner.get());
        }
        let n = self.numbered;
        self.numbered = n.checked_add(1).unwrap_or_else(|| phx_num::capacity_exceeded!("projects", u64::MAX, n));
        self.projects.insert(n, project);
        n
    }

    /// Every project building, by its number.
    pub fn projects(&self) -> impl Iterator<Item = (u64, &Project)> {
        self.projects.iter().map(|(n, p)| (*n, p))
    }

    /// A stage delivered: the units left less those delivered; a project with none left is complete, and ends
    /// returned.
    #[clause("CAP.5")]
    pub fn delivered(&mut self, project: u64, units: i64) -> Missing<Project> {
        let Some(p) = self.projects.get_mut(&project) else {
            violation!(clause = "CAP.5", "a stage of no project", project = project);
        };
        if units <= 0 || units > p.left {
            violation!(clause = "CAP.5", "a stage beyond what its project has left", project = project);
        }
        p.left -= units;
        if p.left > 0 {
            return Missing::Absent;
        }
        match self.projects.remove(&project) {
            Some(done) => Missing::Present(done),
            None => Missing::Absent,
        }
    }

    /// A chain by its number.
    #[must_use]
    pub fn get(&self, chain: u32) -> Option<&Chain> {
        usize::try_from(chain).ok().and_then(|i| self.declared.get(i))
    }

    /// The chain and class an instrument is, if it is one.
    pub fn of(&self, instrument: InstrumentId) -> Missing<(u32, usize)> {
        for (n, c) in (0_u32..).zip(&self.declared) {
            if let Some(at) = c.classes.iter().position(|i| *i == instrument) {
                return Missing::Present((n, at));
            }
        }
        Missing::Absent
    }

    /// Every chain in order.
    pub fn iter(&self) -> impl Iterator<Item = &Chain> {
        self.declared.iter()
    }

    /// An instruction's wear legs checked: each on a class of the chain it names, none entering a chain's newest
    /// class, and each class after the first receiving from a holder what the one before it gave that holder. Its
    /// construction's legs checked too: each on a chain's plant under construction or its newest class, the newest
    /// receiving from a holder what its plant under construction gave.
    #[clause("CAP.8", "REP.24", "CAP.12")]
    pub fn check(&self, legs: &[LegRec]) {
        self.check_built(legs);
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

impl Chains {
    fn check_built(&self, legs: &[LegRec]) {
        let mut completed: BTreeMap<(PartyId, u32), i64> = BTreeMap::new();
        for leg in legs {
            let (LegKind::Transformation { source: Source::Built(_), .. }, AccountRef::Instrument(id)) =
                (leg.kind, leg.account)
            else {
                continue;
            };
            if let Some((chain, _)) = self.building.iter().find(|(_, b)| **b == id) {
                if leg.qty < 0 {
                    *completed.entry((leg.party, *chain)).or_insert(0) -= leg.qty;
                }
                continue;
            }
            match self.of(id) {
                Missing::Present((chain, 0)) if leg.qty > 0 => {
                    *completed.entry((leg.party, chain)).or_insert(0) -= leg.qty;
                }
                _ => violation!(clause = "CAP.12", "plant built into what is no newest class", instrument = id.get()),
            }
        }
        if let Some(((party, _), _)) = completed.iter().find(|(_, n)| **n != 0) {
            violation!(
                clause = "CAP.12",
                "plant in service other than what its construction gave",
                party = party.get()
            );
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

    #[test]
    fn a_project_builds_into_the_newest_class_only() {
        let mut chains = Chains::default();
        let chain = chains.declare(0, (1..=3).map(InstrumentId::new).collect());
        chains.declare_building(chain, InstrumentId::new(9));
        let built = |instrument: u32, qty: i64| LegRec {
            kind: LegKind::Transformation { source: Source::Built(0), cost: 0 },
            ..leg(instrument, qty)
        };
        chains.check(&[built(9, 6)]);
        chains.check(&[built(9, -6), built(1, 6)]);
        assert_eq!(caught_clause(|| chains.check(&[built(9, -6), built(2, 6)])), Some("CAP.12"), "an older class");
        assert_eq!(caught_clause(|| chains.check(&[built(9, -6), built(1, 5)])), Some("CAP.12"), "units lost");
        let project = super::Project {
            owner: PartyId::new(4),
            builder: PartyId::new(5),
            chain,
            good: InstrumentId::new(20),
            way: Missing::Absent,
            price: 3,
            lot: 1,
            stage: 4,
            total: 6,
            left: 6,
            ordered: phx_id::Day::new(0),
        };
        let n = chains.begin(project);
        assert_eq!(chains.delivered(n, 4), Missing::Absent);
        assert!(matches!(chains.delivered(n, 2), Missing::Present(p) if p.total == 6 && p.left == 0));
        assert_eq!(chains.projects().count(), 0, "a complete project ends");
    }
}
