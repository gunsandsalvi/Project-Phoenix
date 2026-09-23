use phx_core::FamilyCtx;
use phx_id::{PartyId, RowRef, Slot, TableId};

fn repair(ctx: &FamilyCtx<'_>) {
    ctx.directory().begin(RowRef { table: TableId::new(0), slot: Slot::new(0) });
    ctx.directory().retain(PartyId::new(1));
}

fn main() {}
