//! What a household holds and expects, which its decisions read and write, as positions of its agent, a twin's.

use phx_core::{FactDef, PositionDecl, declare_fact};

declare_fact! {
    /// The household's outlook of its permanent income a year, a twin's: the mean of its adaptive outlook.
    pub Income = "HH.income" {
        value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.4",
    }
}

declare_fact! {
    /// What the household held after its last spending decision, a twin's, from which the income since is counted.
    pub After = "HH.after" {
        value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.4",
    }
}

/// A fact as the position a household's agent holds of the same name.
const fn from_fact<F: FactDef>() -> PositionDecl {
    PositionDecl { name: F::ITEM.name, clause: F::ITEM.clause }
}

/// Every position a household's agent holds, in the order its table keeps them.
pub const POSITIONS: [PositionDecl; 2] = [from_fact::<Income>(), from_fact::<After>()];
