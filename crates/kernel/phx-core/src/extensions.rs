use phx_id::{MarketId, PartyId};

use crate::declare_prim;
use crate::events::Event;
use crate::streams::{Purpose, StreamDecl};

declare_prim! {
    /// How many members are drawn at the opening to be followed through splits: the observer's, not the world's.
    pub TRACERS = "OBS.tracers" { kind: Resolution, value: Count, clause: "OBS.9", scope: Shared }
}

/// The observer's own stream, which places tracers and follows them, and touches nothing of the world.
pub const TRACER_STREAM: StreamDecl =
    StreamDecl { name: "OBS.tracer", purpose: Purpose::Observer, keyed: false, clause: "REP.30" };

/// A group of members' demand at a price, which the population answers for the markets without either depending on
/// the other.
pub trait GroupDemand {
    fn quantity_at(&self, market: MarketId, group: u64, price_raw: i64) -> i64;
}

/// The observer's read-only set of cells holding a tracer, which the population reads to write its split log.
pub trait TracedCells {
    fn is_traced(&self, cell: PartyId) -> bool;
}

/// What becomes public at the close: the declared rule of which events somebody would notice.
pub trait PublicEventRule {
    fn is_public(&self, event: &Event) -> bool;
}
