//! What a reason declares: its place in the payment order and how a change of money under it shows in each side's
//! accounts; and a declared name as one number.

use phx_num::Missing;

/// How a change of money shows in its party's accounts, as the reason declares it for each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Revenue,
    Expense,
    Asset,
    Liability,
    Equity,
}

/// A reason an instruction is made, as the system that makes it declares it: its place in the declared payment order
/// within a sub-step; its accounting effect on the side that pays and the side that receives; and, where it differs,
/// the effect of the cost units carry out of a holding and into one, as a sale's cost of goods is an expense while
/// its money received is revenue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonDecl {
    pub name: &'static str,
    pub order: u8,
    pub paid: Effect,
    pub received: Effect,
    pub held: Missing<(Effect, Effect)>,
}

/// A declared name as one number, as a handler's intent carries a reason or a market kind: its FNV-1a hash, the same
/// on every build.
#[must_use]
pub fn name_code(name: &str) -> u64 {
    phx_rand::key::fnv1a64(name.as_bytes())
}
