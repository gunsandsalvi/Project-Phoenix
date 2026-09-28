//! What a firm holds and expects, which its decisions read and write: a large firm's as facts of its row, a small
//! firm's as positions of its agent, the same names on both.

use phx_core::{FactDef, PositionDecl, declare_fact};

declare_fact! {
    /// Units the firm expects to sell in a production period: its sales outlook's mean.
    pub ExpectedSales = "FRM.expected_sales" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "VAL.23",
    }
}

declare_fact! {
    /// The width of the firm's sales outlook: the mean of its surprises' size, in units a period.
    pub SalesWidth = "FRM.sales_width" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "VAL.4",
    }
}

declare_fact! {
    /// Units of its product the firm had delivered by its last price review, from which its sales since are counted.
    pub DeliveredAtReview = "FRM.delivered_at_review" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.13",
    }
}

declare_fact! {
    /// Units of its product the firm had delivered by its last production schedule, from which the sales its outlook
    /// next observes are counted.
    pub DeliveredSeen = "FRM.delivered_seen" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "VAL.23",
    }
}

declare_fact! {
    /// The method the firm forecasts public series by, its heuristic and its memory type as one index: the heuristic
    /// times the memory types, plus the memory type.
    pub Method = "FRM.method" {
        value: Count, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "VAL.23",
    }
}

declare_fact! {
    /// The firm's switching type: how strongly it moves toward the heuristic that has forecast best, by the type's place.
    pub Switching = "FRM.switching" {
        value: Count, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "VAL.7",
    }
}

declare_fact! {
    /// The day of the firm's last price review, from which its sales since are counted.
    pub LastReview = "FRM.last_review" {
        value: Day, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "REP.21",
    }
}

declare_fact! {
    /// What a lot of its product, the units its price is posted for, costs the firm to make, in its currency.
    pub UnitCost = "FRM.unit_cost" {
        value: Money, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.14",
    }
}

declare_fact! {
    /// The firm's markup over expected unit cost.
    pub Markup = "FRM.markup" {
        value: Fixed { exp: 6 }, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position,
        clause: "FRM.5",
    }
}

declare_fact! {
    /// The price the firm posts, a point of its trade's table.
    pub Price = "FRM.price" {
        value: Money, kinds: ["firm", "small_firm"], writer: "FRM", audience: Public, repr: Position, clause: "REP.34",
    }
}

declare_fact! {
    /// Units the firm starts a day, its standing production flow, in millionths so a firm making less than a unit a day
    /// is counted.
    pub OutputRate = "FRM.output_rate" {
        value: Fixed { exp: 6 }, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position,
        clause: "FRM.4",
    }
}

declare_fact! {
    /// The firm's daily chance of reviewing its price, in billionths: its attention.
    pub PriceAttention = "FRM.price_attention" {
        value: Fixed { exp: 9 }, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position,
        clause: "REP.38",
    }
}

declare_fact! {
    /// The hours of work a unit of the firm's product takes it, in millionths: its productivity, fixed by its filed
    /// accounts, which its hiring spreads over the occupations by its way's mix.
    pub HoursAUnit = "FRM.hours_a_unit" {
        value: Fixed { exp: 6 }, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position,
        clause: "FRM.2",
    }
}

declare_fact! {
    /// What an hour of the firm's staff costs it, in its currency: its wage bill over its hours.
    pub WagePerHour = "FRM.wage_per_hour" {
        value: Money, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.14",
    }
}

declare_fact! {
    /// The return the firm's management requires of what it holds and does, a year: its hurdle, by which it
    /// discounts what it expects and weighs holding against selling.
    pub RequiredReturn = "FRM.required_return" {
        value: Fixed { exp: 6 }, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position,
        clause: "CAP.13",
    }
}

declare_fact! {
    /// The units a day the firm's plant lets its way make, as its last review of its plant found.
    pub Capacity = "CAP.capacity" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "CAP", audience: Party, repr: Position, clause: "CAP.9",
    }
}

declare_fact! {
    /// The units of its product the firm had delivered at its last investment review.
    pub DeliveredAtInvest = "CAP.delivered_seen" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "CAP", audience: Party, repr: Position, clause: "CAP.3",
    }
}

declare_fact! {
    /// The units the firm sold between its last two investment reviews.
    pub SoldAtInvest = "CAP.sales_seen" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "CAP", audience: Party, repr: Position, clause: "CAP.3",
    }
}

/// A fact as the position a small firm's agent holds of the same name.
const fn from_fact<F: FactDef>() -> PositionDecl {
    PositionDecl { name: F::ITEM.name, clause: F::ITEM.clause, opening: phx_core::PositionOpening::Missing }
}

/// Every position a small firm's agent holds, in the order its table keeps them.
pub const POSITIONS: [PositionDecl; 15] = [
    from_fact::<ExpectedSales>(),
    from_fact::<SalesWidth>(),
    from_fact::<DeliveredAtReview>(),
    from_fact::<DeliveredSeen>(),
    from_fact::<Method>(),
    from_fact::<Switching>(),
    from_fact::<LastReview>(),
    from_fact::<UnitCost>(),
    from_fact::<Markup>(),
    from_fact::<Price>(),
    from_fact::<OutputRate>(),
    from_fact::<PriceAttention>(),
    from_fact::<WagePerHour>(),
    from_fact::<RequiredReturn>(),
    from_fact::<HoursAUnit>(),
];

/// Every fact a large firm keeps, by name.
pub const FACTS: [&str; 15] = [
    ExpectedSales::ITEM.name,
    SalesWidth::ITEM.name,
    DeliveredAtReview::ITEM.name,
    DeliveredSeen::ITEM.name,
    Method::ITEM.name,
    Switching::ITEM.name,
    LastReview::ITEM.name,
    UnitCost::ITEM.name,
    Markup::ITEM.name,
    Price::ITEM.name,
    OutputRate::ITEM.name,
    PriceAttention::ITEM.name,
    WagePerHour::ITEM.name,
    RequiredReturn::ITEM.name,
    HoursAUnit::ITEM.name,
];
