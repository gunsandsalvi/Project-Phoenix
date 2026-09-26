//! What a firm holds and expects, which its decisions read and write: a large firm's as facts of its row, a small
//! firm's as positions of its agent, the same names on both.

use phx_core::{FactDef, PositionDecl, declare_fact};

declare_fact! {
    /// Units of output the firm holds, in its product's quantity.
    pub Stock = "FRM.stock" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.1",
    }
}

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
    /// Units the firm has sold since its last price review.
    pub SalesSince = "FRM.sales_since" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.13",
    }
}

declare_fact! {
    /// The day of the firm's last price review, from which its sales since are counted.
    pub LastReview = "FRM.last_review" {
        value: Day, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "REP.21",
    }
}

declare_fact! {
    /// What a unit costs the firm to make, in its currency per unit.
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
    /// Units the firm starts a day, its standing production flow.
    pub OutputRate = "FRM.output_rate" {
        value: Qty, kinds: ["firm", "small_firm"], writer: "FRM", audience: Party, repr: Position, clause: "FRM.4",
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

/// A fact as the position a small firm's agent holds of the same name.
const fn from_fact<F: FactDef>() -> PositionDecl {
    PositionDecl { name: F::ITEM.name, clause: F::ITEM.clause }
}

/// Every position a small firm's agent holds, in the order its table keeps them.
pub const POSITIONS: [PositionDecl; 12] = [
    from_fact::<Stock>(),
    from_fact::<ExpectedSales>(),
    from_fact::<SalesWidth>(),
    from_fact::<SalesSince>(),
    from_fact::<LastReview>(),
    from_fact::<UnitCost>(),
    from_fact::<Markup>(),
    from_fact::<Price>(),
    from_fact::<OutputRate>(),
    from_fact::<PriceAttention>(),
    from_fact::<WagePerHour>(),
    from_fact::<RequiredReturn>(),
];

/// Every fact a large firm keeps, by name.
pub const FACTS: [&str; 12] = [
    Stock::ITEM.name,
    ExpectedSales::ITEM.name,
    SalesWidth::ITEM.name,
    SalesSince::ITEM.name,
    LastReview::ITEM.name,
    UnitCost::ITEM.name,
    Markup::ITEM.name,
    Price::ITEM.name,
    OutputRate::ITEM.name,
    PriceAttention::ITEM.name,
    WagePerHour::ITEM.name,
    RequiredReturn::ITEM.name,
];
