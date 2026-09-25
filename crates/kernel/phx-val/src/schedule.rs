//! The investor schedule: the orders a party's value, the variance it sees and its risk aversion give at each tick.
//! The securities markets (`sys-sov`, Stage 3) fill it; until they exist no party bids on an instrument's price.

use phx_num::PriceRaw;

/// The orders at each tick: a bid where positive, an offer where negative.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    pub orders: Vec<(PriceRaw, i64)>,
}
