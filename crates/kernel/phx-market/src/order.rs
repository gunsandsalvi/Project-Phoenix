use phx_id::{Day, MarketId, PartyId};
use phx_ledger::covered::Covered;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw};

/// Which way an order trades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Buy,
    Sell,
}

/// When a book order meets: on arrival, or in the closing call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timing {
    Continuous,
    AtTheClose,
}

/// One step of a schedule: up to `qty` at a price no worse than `limit` for its poster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub limit: PriceRaw,
    pub qty: i64,
}

/// A step as its poster's decision gave it, its limit possibly absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asked {
    pub limit: Missing<PriceRaw>,
    pub qty: i64,
}

/// What a market refuses in an order: a step with no limit, or at any price, which would take a price not yet
/// formed or make the poster a buyer or seller of last resort; a step of no quantity; a limit off the tick; an offer
/// of held units beyond the units covering it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoLimit,
    AnyPrice,
    NoQuantity,
    OffTick,
    OffLot,
    Uncovered,
}

/// An order: its poster, market and side, its schedule of steps, when it meets on a book, the day it was posted and
/// the decision point it came from, its place in a declared priority where the market rations by one, the lot it
/// trades in — an agent's multiplicity, so each twin's share stays whole, one for an individual — and the units
/// covering it when it offers units its poster holds.
#[clause("MKT.3", "MKT.9", "MKT.16", "MKT.17")]
#[derive(Debug, PartialEq, Eq)]
pub struct Order {
    pub party: PartyId,
    pub market: MarketId,
    pub side: Side,
    pub steps: Vec<Step>,
    pub timing: Timing,
    pub day: Day,
    pub reason: &'static str,
    pub priority: Missing<u32>,
    pub lot: i64,
    pub cover: Missing<Covered>,
}

/// Where an order comes from: its poster, market and side, when it meets, its day, its decision point, its priority
/// and its lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Poster {
    pub party: PartyId,
    pub market: MarketId,
    pub side: Side,
    pub timing: Timing,
    pub day: Day,
    pub reason: &'static str,
    pub priority: Missing<u32>,
    pub lot: i64,
}

impl Order {
    /// An order from its poster's steps on the market's tick, refused if any step has no limit or a limit at any
    /// price, no quantity, a limit off the tick, or a quantity that is not whole lots.
    ///
    /// # Errors
    /// The refusal, when a step breaks a rule.
    pub fn new(poster: Poster, asked: &[Asked], tick: i64) -> Result<Order, Refusal> {
        let mut steps = Vec::with_capacity(asked.len());
        for a in asked {
            let Missing::Present(limit) = a.limit else { return Err(Refusal::NoLimit) };
            if limit.raw() == i64::MAX || limit.raw() == i64::MIN {
                return Err(Refusal::AnyPrice);
            }
            if a.qty <= 0 {
                return Err(Refusal::NoQuantity);
            }
            if limit.raw() % tick != 0 {
                return Err(Refusal::OffTick);
            }
            if poster.lot <= 0 || a.qty % poster.lot != 0 {
                return Err(Refusal::OffLot);
            }
            steps.push(Step { limit, qty: a.qty });
        }
        if steps.is_empty() {
            return Err(Refusal::NoQuantity);
        }
        Ok(Order {
            party: poster.party,
            market: poster.market,
            side: poster.side,
            steps,
            timing: poster.timing,
            day: poster.day,
            reason: poster.reason,
            priority: poster.priority,
            lot: poster.lot,
            cover: Missing::Absent,
        })
    }

    /// An offer of units its poster holds or has borrowed, covered by the ledger's commitment on them: it offers no
    /// more than the cover holds.
    ///
    /// # Errors
    /// The refusal, when a step breaks a rule or the steps offer more than the cover; the cover then comes back to
    /// be released.
    pub fn offer_held(poster: Poster, asked: &[Asked], tick: i64, cover: Covered) -> Result<Order, (Refusal, Covered)> {
        if poster.side != Side::Sell {
            return Err((Refusal::Uncovered, cover));
        }
        let mut order = match Order::new(poster, asked, tick) {
            Ok(o) => o,
            Err(r) => return Err((r, cover)),
        };
        let offered: i128 = order.steps.iter().map(|s| i128::from(s.qty)).sum();
        if offered > i128::from(cover.qty().n()) {
            return Err((Refusal::Uncovered, cover));
        }
        order.cover = Missing::Present(cover);
        Ok(order)
    }

    /// Everything the order asks, over its steps.
    #[must_use]
    pub fn qty(&self) -> i128 {
        self.steps.iter().map(|s| i128::from(s.qty)).sum()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, InstrumentId, MarketId, PartyId};
    use phx_ledger::covered::Covers;
    use phx_num::{Missing, PriceRaw, Qty, UnitId};

    use super::{Asked, Order, Poster, Refusal, Side, Timing};

    fn poster(side: Side) -> Poster {
        Poster {
            party: PartyId::new(1),
            market: MarketId::new(0),
            side,
            timing: Timing::Continuous,
            day: Day::new(3),
            reason: "invest",
            priority: Missing::Absent,
            lot: 1,
        }
    }

    fn at(limit: i64, qty: i64) -> Asked {
        Asked { limit: Missing::Present(PriceRaw::from_raw(limit)), qty }
    }

    #[test]
    fn order_without_limit_refused() {
        let none = Asked { limit: Missing::Absent, qty: 5 };
        assert_eq!(Order::new(poster(Side::Buy), &[none], 1).err(), Some(Refusal::NoLimit));
        assert_eq!(Order::new(poster(Side::Sell), &[at(i64::MIN, 5)], 1).err(), Some(Refusal::AnyPrice));
        assert_eq!(Order::new(poster(Side::Buy), &[at(i64::MAX, 5)], 1).err(), Some(Refusal::AnyPrice));
        assert_eq!(Order::new(poster(Side::Buy), &[at(100, 0)], 1).err(), Some(Refusal::NoQuantity));
        assert_eq!(Order::new(poster(Side::Buy), &[at(101, 5)], 5).err(), Some(Refusal::OffTick));
        assert_eq!(Order::new(poster(Side::Buy), &[], 5).err(), Some(Refusal::NoQuantity));
        assert!(Order::new(poster(Side::Buy), &[at(100, 5), at(95, 5)], 5).is_ok());
    }

    #[test]
    fn offer_held_within_cover() {
        let mut covers = Covers::default();
        let unit = UnitId::new(0);
        let cover = covers.cover(PartyId::new(1), InstrumentId::new(2), Qty::new(10, unit), 10, 0).unwrap();
        let (refused, cover) = Order::offer_held(poster(Side::Sell), &[at(100, 11)], 1, cover).unwrap_err();
        assert_eq!(refused, Refusal::Uncovered, "no more than the cover holds");
        let order = Order::offer_held(poster(Side::Sell), &[at(100, 6), at(110, 4)], 1, cover).unwrap();
        assert_eq!(order.qty(), 10);
    }
}
