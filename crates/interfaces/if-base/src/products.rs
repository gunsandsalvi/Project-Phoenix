//! Products as the world declares them, each counted in its own physical unit and belonging to one industry.

use phx_macros::clause;
use phx_num::{Missing, UnitId};

/// A product by its place in the declared list.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductId(u16);

impl ProductId {
    pub const fn new(index: u16) -> ProductId {
        ProductId(index)
    }

    #[must_use]
    pub const fn index(self) -> u16 {
        self.0
    }
}

/// An industry by its place among the industries the products name, in the order first named.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryId(u16);

impl IndustryId {
    pub const fn new(index: u16) -> IndustryId {
        IndustryId(index)
    }

    #[must_use]
    pub const fn index(self) -> u16 {
        self.0
    }
}

/// A product: its unit, its industry, whether it can be stored, whether it is delivered as it is made, and the
/// deposit resource it is extracted from. A service is a product that cannot be stored and is delivered as made;
/// nothing else marks it.
#[clause("TEC.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductDecl {
    pub id: ProductId,
    pub unit: UnitId,
    pub industry: IndustryId,
    pub storable: bool,
    pub delivered_at_once: bool,
    pub extracts: Missing<u16>,
}

/// The declared products in order, with their names and the names of their industries; `sys-tec` builds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Products {
    pub decls: Vec<ProductDecl>,
    pub names: Vec<String>,
    pub industries: Vec<String>,
}
