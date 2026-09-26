//! The markets goods meet in: standardised commodities in a call at each place each business day, other goods
//! between firms at the prices their sellers post. Both deliver the day they trade.

use phx_id::MarketId;
use phx_market::market::{Form, MarketDecl, MarketKey, Ration, TieRule};
use phx_num::Missing;

/// The ties a commodity call breaks: the greatest volume, the least imbalance, the price nearest the last print, and
/// the lower price.
const TIES: &[TieRule] = &[TieRule::MaxVolume, TieRule::MinImbalance, TieRule::NearestLastPrint, TieRule::LowerPrice];

/// A commodity call at a place, over a good of one grade class there.
pub const COMMODITIES: MarketDecl = MarketDecl {
    id: MarketId::new(0),
    name: "commodity call",
    key: MarketKey { kind: "GDS.commodities", subject: 0 },
    form: Form::Call,
    operator: "the place's commodity exchange",
    meeting_days: "business days",
    settle_days: 0,
    participants: "firms",
    tick: 1,
    ties: TIES,
    ration: Ration::ProRata,
    stream: "GDS.lots",
    quantity_response: Missing::Absent,
    admission: Missing::Absent,
};

/// Goods between firms at a place, at the prices their sellers post.
pub const BETWEEN_FIRMS: MarketDecl = MarketDecl {
    id: MarketId::new(0),
    name: "goods between firms",
    key: MarketKey { kind: "GDS.between_firms", subject: 0 },
    form: Form::Posted,
    operator: "the sellers",
    meeting_days: "business days",
    settle_days: 0,
    participants: "firms",
    tick: 1,
    ties: &[],
    ration: Ration::ProRata,
    stream: "GDS.lots",
    quantity_response: Missing::Absent,
    admission: Missing::Absent,
};
