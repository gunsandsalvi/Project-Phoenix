//! What a household holds and expects, which its decisions read and write, as words of its kind store.

use phx_core::declare_fact;

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
