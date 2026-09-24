use std::collections::BTreeMap;

use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Count, capacity_exceeded, violation};

use crate::algebra::{Leg, Side};

/// A row a commitment creates when drawn: its line, the side and the count of members it adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowSpec {
    pub line: LineId,
    pub side: Side,
    pub holder: PartyId,
    pub count: Count,
}

/// A commitment's standing: open until drawn or lapsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitmentState {
    Open,
    Drawn,
    Lapsed,
}

/// A commitment recorded on both parties' books as what it is — an undrawn credit line, a loan agreed, an unsettled
/// trade, uncalled capital — with, as its writer's declaration, the legs it contributes and the rows it creates or
/// retires when drawn, so the instruction that draws it writes no other system's rows.
#[clause("REG.10")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commitment {
    pub kind: u16,
    pub parties: [PartyId; 2],
    pub legs: Vec<Leg>,
    pub creates: Vec<RowSpec>,
    pub retires: Vec<RowSpec>,
    pub expires: Day,
    pub state: CommitmentState,
}

/// A commitment kind as a system declares it: what it may contribute when drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitmentKindDecl {
    pub name: &'static str,
    pub contributes_legs: bool,
    pub creates_rows: bool,
    pub retires_rows: bool,
}

/// Commitment kinds that contribute nothing when drawn, which assembly refuses.
#[must_use]
pub fn empty_kinds(kinds: &[CommitmentKindDecl]) -> Vec<String> {
    kinds
        .iter()
        .filter(|k| !k.contributes_legs && !k.creates_rows && !k.retires_rows)
        .map(|k| format!("commitment kind `{}` contributes no leg and no row when drawn", k.name))
        .collect()
}

/// A commitment's identity; commitments live across days and are saved.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitmentId(u32);

/// Every open commitment, by identity.
#[derive(Clone, Debug, Default)]
pub struct Commitments {
    open: BTreeMap<CommitmentId, Commitment>,
    next: u32,
}

impl Commitments {
    /// A commitment placed, open until drawn or it expires.
    pub fn place(&mut self, commitment: Commitment) -> CommitmentId {
        let id = CommitmentId(self.next);
        let Some(next) = self.next.checked_add(1) else {
            capacity_exceeded!("commitments", u32::MAX, self.next);
        };
        self.next = next;
        self.open.insert(id, Commitment { state: CommitmentState::Open, ..commitment });
        id
    }

    /// A commitment drawn: it leaves the book, its legs and rows given to the instruction that draws it.
    pub fn draw(&mut self, id: CommitmentId, day: Day) -> Commitment {
        let Some(c) = self.open.remove(&id) else {
            violation!(clause = "REG.10", "a commitment drawn that is not open", id = id.0);
        };
        if c.expires < day {
            violation!(clause = "REG.10", "a commitment drawn after it expired", id = id.0);
        }
        Commitment { state: CommitmentState::Drawn, ..c }
    }

    /// The commitments whose day has passed, lapsed and taken off the book.
    pub fn lapse(&mut self, day: Day) -> Vec<Commitment> {
        let gone: Vec<CommitmentId> = self.open.iter().filter(|(_, c)| c.expires < day).map(|(id, _)| *id).collect();
        gone.into_iter()
            .filter_map(|id| self.open.remove(&id))
            .map(|c| Commitment { state: CommitmentState::Lapsed, ..c })
            .collect()
    }

    /// A party's open commitments, on either side.
    pub fn of(&self, party: PartyId) -> impl Iterator<Item = &Commitment> {
        self.open.values().filter(move |c| c.parties.contains(&party))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.open.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}
