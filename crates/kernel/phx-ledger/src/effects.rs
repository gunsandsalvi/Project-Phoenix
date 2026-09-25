use phx_id::PartyId;
use phx_macros::clause;
use phx_num::Money;

use crate::instruction::{Effect, InstructionId};

/// What became of a due in a day's settlement: paid, failed for the payer's funds, or held pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueOutcome {
    Settled,
    Failed,
    Pending,
}

/// A due that fell today on a contract, for the accounts: its line, who owes it and who is owed, the interest in it
/// and the principal it repays, and what became of it. The interest is earned the day it falls due, paid or not.
#[clause("ACC.1", "SET.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueRec {
    pub line: phx_id::LineId,
    pub payer: PartyId,
    pub payee: PartyId,
    pub interest: Money,
    pub principal: Money,
    pub outcome: DueOutcome,
}

/// A settled money leg's effect on its party's accounts, as its reason declares it, for the accounts to read: the
/// amount signed as the leg moved its party's net assets, in or out.
#[clause("SET.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectRec {
    pub instruction: InstructionId,
    pub party: PartyId,
    pub effect: Effect,
    pub amount: Money,
}
