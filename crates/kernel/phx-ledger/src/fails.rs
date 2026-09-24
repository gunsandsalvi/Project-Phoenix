use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::Missing;

use crate::check::FailCause;
use crate::instruction::{DueRow, InstructionId, ReasonId};

/// An instruction due that did not settle: its cause, the party that could not, the day it was due, and the contract
/// row whose due it was paying, whose line kind's contract process turns it into arrears.
#[clause("SET.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fail {
    pub instruction: InstructionId,
    pub reason: ReasonId,
    pub cause: FailCause,
    pub party: PartyId,
    pub due: Day,
    pub row: Missing<DueRow>,
}
