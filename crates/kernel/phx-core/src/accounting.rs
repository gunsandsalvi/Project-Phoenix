use phx_macros::clause;

/// How a position is carried in its holder's books.
#[clause("ACC.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CarryingBasis {
    FairValue,
    AmortisedCost,
    LowerOfCostAndRealisable,
    CostLessDepreciation,
}

/// What a position is held for, which decides the bases its holder may choose among.
#[clause("ACC.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeldFor {
    /// A contract held to collect what it pays: a loan, a deposit, a bond held to maturity.
    Collect,
    /// A position held to trade: a dealer's inventory, a trading book, a fund's assets.
    Trade,
    /// Goods held to sell.
    Sell,
    /// Plant, dwellings and infrastructure held for use.
    Use,
}

/// The bases the accounting standard permits a legal form for a purpose, in the order the standard lists them.
#[clause("ACC.17")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permitted {
    pub form: String,
    pub held_for: HeldFor,
    pub bases: Vec<CarryingBasis>,
}
