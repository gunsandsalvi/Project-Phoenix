use phx_id::InstrumentId;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Backing;

use crate::instrument::{InstrumentState, Instruments};

/// An instrument event kind as the system that decides it declares it: the states an instrument may be in when it
/// happens, and the state it leaves the instrument in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentEventDecl {
    pub name: &'static str,
    pub from: &'static [InstrumentState],
    pub to: InstrumentState,
}

/// A declared instrument event kind.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentEventKind(u16);

/// Instrument event kinds assembly refuses: any that can happen from no state.
#[must_use]
pub fn refusals(kinds: &[InstrumentEventDecl]) -> Vec<String> {
    kinds
        .iter()
        .filter(|k| k.from.is_empty())
        .map(|k| format!("instrument event `{}` names no state it can happen from", k.name))
        .collect()
}

/// The instrument events systems declare, applied here alone: an instrument's state has this one writer, and every
/// system reads it.
#[clause("REG.3")]
#[derive(Clone, Debug, Default)]
pub struct InstrumentEvents {
    kinds: Vec<InstrumentEventDecl>,
}

impl InstrumentEvents {
    pub fn declare(&mut self, decl: InstrumentEventDecl) -> InstrumentEventKind {
        let Ok(index) = u16::try_from(self.kinds.len()) else {
            capacity_exceeded!("instrument event kinds", u16::MAX, self.kinds.len());
        };
        self.kinds.push(decl);
        InstrumentEventKind(index)
    }

    /// An event applied to an instrument, which moves to the event's state; an instrument in a state the event cannot
    /// happen from stops the run.
    pub fn apply<B: Backing>(
        &self,
        instruments: &mut Instruments<B>,
        id: InstrumentId,
        kind: InstrumentEventKind,
    ) -> InstrumentState {
        let Some(decl) = self.kinds.get(usize::from(kind.0)) else {
            violation!(clause = "REG.3", "an instrument event of an undeclared kind", kind = kind.0);
        };
        let now = instruments.get(id).state;
        if !decl.from.contains(&now) {
            violation!(clause = "REG.3", "an instrument event from a state it cannot happen from", id = id.get());
        }
        instruments.set_state(id, decl.to);
        decl.to
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}
