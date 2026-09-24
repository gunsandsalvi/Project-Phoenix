use phx_id::PartyId;
use phx_macros::clause;
use phx_num::Money;

use crate::instruction::{Effect, InstructionId};

/// A settled money leg's effect on its party's accounts, as its reason declares it, for the accounts to read.
#[clause("SET.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectRec {
    pub instruction: InstructionId,
    pub party: PartyId,
    pub effect: Effect,
    pub amount: Money,
}
