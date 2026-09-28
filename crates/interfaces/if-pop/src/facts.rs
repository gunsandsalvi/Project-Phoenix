//! What a household holds and expects, which its decisions read and write, as positions of its agent.

use phx_core::{FactDef, PositionDecl, PositionOpening, declare_fact};

declare_fact! {
    /// The household's outlook of its permanent income a year: the mean of its adaptive outlook.
    pub Income = "HH.income" {
        value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.4",
    }
}

declare_fact! {
    /// What the household held after its last spending decision, from which the income since is counted.
    pub After = "HH.after" {
        value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.4",
    }
}

declare_fact! {
    /// The income the household has taken in since it last looked at its income, summed over its decisions.
    pub Received = "HH.received" {
        value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.2",
    }
}

declare_fact! {
    /// The month the household last looked at its income, counted in months from the calendar's year zero.
    pub Looked = "HH.looked" {
        value: Count, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.4",
    }
}

/// A fact as the position a household's agent holds of the same name, as the opening leaves it.
const fn from_fact<F: FactDef>(opening: PositionOpening) -> PositionDecl {
    PositionDecl { name: F::ITEM.name, clause: F::ITEM.clause, opening }
}

/// Every position a household's agent holds, in the order its table keeps them.
pub const POSITIONS: [PositionDecl; 4] = [
    from_fact::<Income>(PositionOpening::OwedAYear),
    from_fact::<After>(PositionOpening::Missing),
    from_fact::<Received>(PositionOpening::Missing),
    from_fact::<Looked>(PositionOpening::Missing),
];
