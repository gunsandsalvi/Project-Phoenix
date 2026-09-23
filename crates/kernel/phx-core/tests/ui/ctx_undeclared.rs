use phx_core::handler::{Ctx, FactStore};
use phx_core::{declare_fact, declare_handler};
use phx_id::Slot;

declare_fact! {
    pub Employment = "LAB.employment" {
        value: Flag, kinds: ["household"], writer: "LAB", audience: Party, repr: Key, clause: "LAB.1",
    }
}

declare_fact! {
    pub Wage = "LAB.wage" {
        value: Money, kinds: ["household"], writer: "LAB", audience: Party, repr: Position, clause: "LAB.2",
    }
}

declare_handler! {
    pub Hire = "LAB.hire" { substep: S5c, table: "household", reads: [Employment], clause: "LAB.9" }
}

fn wage<S: FactStore>(ctx: &Ctx<'_, Hire, S>) {
    let _ = ctx.read::<Wage>(Slot::new(0));
}

fn main() {}
