use phx_id::{Day, MarketId};
use phx_macros::clause;

/// Why a meeting formed no price: bids and offers did not overlap, nobody bid, nobody offered, the dealers asked
/// stepped back, or every party asked declined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum FailureKind {
    NoOverlap,
    NoBid,
    NoSeller,
    DealersStepped,
    Declined,
}

/// A meeting that formed no price, published for the participants' systems to read and act on: the issuer is not
/// funded, the seller keeps its stock, the borrower is refused.
#[clause("MKT.10")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct MarketFailure {
    pub market: MarketId,
    pub day: Day,
    pub kind: FailureKind,
}
