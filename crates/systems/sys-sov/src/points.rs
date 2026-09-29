//! The bills' decision points: the treasury's minister sizing an auction, and a bank's chief executive bidding.

use if_state::kinds::{BidIn, SizeIn};
use phx_core::decisions::DecisionPointDecl;
use phx_num::Missing;

/// The face the treasury offers at its weekly auction.
pub const SIZE: DecisionPointDecl<SizeIn, f64> = DecisionPointDecl {
    name: "SOV.size",
    system: "SOV",
    rule: crate::rules::size,
    schedule: Missing::Present("SOV.auction_weekday"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "SOV.3",
};

/// A bank's bids at a bill auction.
pub const BID: DecisionPointDecl<BidIn, Vec<(f64, f64)>> = DecisionPointDecl {
    name: "SOV.bid",
    system: "SOV",
    rule: crate::rules::bid,
    schedule: Missing::Present("SOV.auction_weekday"),
    wakes: &[],
    runs_on_non_business: false,
    clause: "SOV.4",
};
