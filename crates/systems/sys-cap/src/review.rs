//! The owner's review of its plant, on its own schedule: the wear since the last is realised on the rows visited;
//! maintaining, repairing, selling and scrapping are decided here once owners hold the prices they read.

use phx_core::declare_handler;
use phx_core::handler::{Ctx, FactStore, HandlerDecl};
use phx_id::Slot;

declare_handler! {
    /// A small firm's review of its plant.
    pub ReviewSmall = "CAP.review_small" {
        substep: S5b,
        table: "small_firm",
        reads: [],
        writes: [],
        clause: "CAP.4",
        body: review,
    }
}

declare_handler! {
    /// A large firm's review of its plant.
    pub ReviewLarge = "CAP.review_large" {
        substep: S5b,
        table: "firm",
        reads: [],
        writes: [],
        clause: "CAP.4",
        body: review,
    }
}

/// Nothing is decided yet: the review is the day its plant's wear is realised.
fn review<H, S>(_: &mut Ctx<'_, H, S>, _: Slot)
where
    H: HandlerDecl,
    S: FactStore + ?Sized,
{
}
