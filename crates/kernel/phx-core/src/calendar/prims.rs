use phx_macros::declare_prim;

declare_prim! {
    /// The day the world's days count from.
    pub EPOCH = "TIME.epoch" { kind: Endowment, value: Date, clause: "TIME.13", scope: Shared }
}

declare_prim! {
    /// A country's weekend and holiday rules.
    pub CALENDAR = "TIME.calendar" { kind: Endowment, value: Calendar, clause: "TIME.13", scope: PerCountry }
}
