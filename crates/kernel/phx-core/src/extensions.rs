use phx_id::MarketId;

use crate::events::Event;

/// A group of members' demand at a price, which the population answers for the markets without either depending on
/// the other.
pub trait GroupDemand {
    fn quantity_at(&self, market: MarketId, group: u64, price_raw: i64) -> i64;
}

/// What becomes public at the close: the declared rule of which events somebody would notice.
pub trait PublicEventRule {
    fn is_public(&self, event: &Event) -> bool;
}
