//! A firm's price review, as its line's head takes it: its markup moved by its sales and the mark it expects, its
//! expected sales taking in the surprise, and the price it would like from its markup over its unit cost under its
//! stock's pressure; then whether to move its posted price to the point nearest that, weighing what the move gains
//! against what it costs.

use phx_macros::clause;
use phx_num::Missing;

/// What a review reads: its markup, its management's speeds and curvature, its sales a day since its last review and
/// those it expected, the mark it expects its competitors to post, its price, its memory's gain, its stock and cover,
/// its unit cost and its product's lot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReviewIn {
    pub markup: f64,
    pub speeds: (f64, f64),
    pub curvature: f64,
    pub demand: f64,
    pub expected: f64,
    pub seen: Missing<f64>,
    pub price: f64,
    pub gain: f64,
    pub stock: f64,
    pub cover: f64,
    pub unit_cost: Missing<f64>,
    pub lot: f64,
}

/// What a review decides: the markup and the sales a day it expects from now, and the price of a lot it would like,
/// none where its cost or its stock's pressure cannot be read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReviewOut {
    pub markup: f64,
    pub expected: f64,
    pub wanted: Missing<f64>,
}

/// The review: the markup's update, the expected sales' surprise at the memory's gain, the pressure of the stock
/// against its cover, and the price wanted.
#[clause("FRM.5", "VAL.4")]
#[must_use]
pub fn review(i: &ReviewIn) -> ReviewOut {
    let markup = match super::markup::update(i.markup, i.speeds, (i.demand, i.expected), i.seen, i.price) {
        Missing::Present(x) => x,
        Missing::Absent => i.markup,
    };
    let expected = phx_val::heuristics::adaptive(i.expected, i.demand, i.gain);
    let pressure = super::price::pressure_stocked(i.demand, expected, i.cover * expected, i.stock);
    let wanted = match (i.unit_cost, pressure) {
        (Missing::Present(cost), Missing::Present(p)) if cost > 0.0 => {
            Missing::Present(super::price::desired(markup, cost * i.lot, p, i.curvature))
        }
        _ => Missing::Absent,
    };
    ReviewOut { markup, expected, wanted }
}

/// What a move reads: the points near the price wanted, the price posted, the price wanted, the revenue a gap is
/// weighed on, the markup and what a change costs.
#[derive(Clone, Debug, PartialEq)]
pub struct RepriceIn {
    pub points: Vec<i64>,
    pub current: i64,
    pub wanted: f64,
    pub revenue: f64,
    pub markup: f64,
    pub menu_cost: f64,
}

/// The point to post, where moving there gains more than it costs.
#[clause("FRM.5", "REP.34")]
#[must_use]
pub fn reprice(i: &RepriceIn) -> Option<i64> {
    super::price::reprice(&i.points, i.current, i.wanted, i.revenue, i.markup, i.menu_cost)
}

/// A new firm's first price: the points near the price its cost gives, and that price.
#[derive(Clone, Debug, PartialEq)]
pub struct DayZeroIn {
    pub points: Vec<i64>,
    pub wanted: f64,
}

/// The point nearest the price its cost gives.
#[clause("GEN.13", "REP.34")]
#[must_use]
pub fn day_zero(i: &DayZeroIn) -> Option<i64> {
    super::price::nearest_point(&i.points, i.wanted)
}

/// What a firm's attention reads: its revenue a day, its markup, the variance of its own sales' surprises and of each
/// public series it reads, what a review costs, and its draw of the day.
#[derive(Clone, Debug, PartialEq)]
pub struct AttendIn {
    pub revenue_per_day: f64,
    pub markup: f64,
    pub own: f64,
    pub public: Vec<f64>,
    pub cost: f64,
    pub draw: f64,
}

/// Whether it reviews its price today: its draw below its chance.
#[clause("REP.38")]
#[must_use]
pub fn attend(i: &AttendIn) -> bool {
    i.draw < super::attention::review_chance(i.revenue_per_day, i.markup, (i.own, &i.public), i.cost)
}

/// What winding down weighs: continuing's worth at the return required, against its money and goods less what ending
/// would owe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloseIn {
    pub continuing: f64,
    pub held: f64,
    pub owed: f64,
}

/// Whether its owner winds the firm down.
#[clause("FRM.15")]
#[must_use]
pub fn close(i: &CloseIn) -> bool {
    super::endings::closes(i.continuing, (i.held, i.owed))
}
