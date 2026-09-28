use phx_core::declare_handler;
use phx_core::flows::Denom;
use phx_core::handler::{Ctx, FactStore, ReasonDef};
use phx_id::{PartyKey, Slot};

struct Wages;
impl ReasonDef for Wages {
    const CODE: u8 = 3;
    const ORDER: u8 = 1;
}

declare_handler! {
    pub Hire = "LAB.hire" { substep: S5c, table: "household", clause: "LAB.9" }
}

fn pay<S: FactStore>(ctx: &mut Ctx<'_, Hire, S>) {
    let side = PartyKey::new(1, Slot::new(0));
    ctx.flow::<Wages>([side, side], 1, Denom::money(0), 0);
}

fn main() {}
