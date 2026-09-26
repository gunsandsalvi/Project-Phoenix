//! Ways of making a product, each in physical units per unit of output, and the identity of an interned set of
//! ways a firm or an industry knows.

use phx_macros::clause;
use phx_num::{Fixed, Missing, QtyRaw};

use crate::consts::PER_UNIT_EXP;
use crate::products::ProductId;

/// A quantity per unit of output.
pub type PerUnit = Fixed<PER_UNIT_EXP>;

/// A way by its place in the register; never reused.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WayId(u32);

impl WayId {
    pub const fn new(index: u32) -> WayId {
        WayId(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// An occupation family by its ISCO-08 major group.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccFamily(pub u8);

/// A kind of plant by its place in the declared kinds.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapKind(pub u8);

/// How a product is made, per unit of output and in physical units only: what it uses of each product, the hours
/// of each occupation family, the stock of each kind of plant per unit of output a year, the land a unit takes a
/// year, what it takes from a deposit, how long from start to finish, the least started at once, the share of what
/// is started that is finished, and what else comes out. Within a way nothing substitutes for anything.
#[clause("TEC.2", "TEC.12")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Way {
    pub product: ProductId,
    pub inputs: Box<[(ProductId, PerUnit)]>,
    pub labour: Box<[(OccFamily, PerUnit)]>,
    pub capital: Box<[(CapKind, PerUnit)]>,
    pub land: PerUnit,
    pub deposit: Missing<(u16, PerUnit)>,
    pub lead_time_days: u16,
    pub batch: QtyRaw,
    pub yield_ppm: u32,
    pub by_products: Box<[(ProductId, PerUnit)]>,
}

/// An interned set of ways, four bytes wherever a firm's or an industry's known ways are kept.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WaySetId(u32);

impl WaySetId {
    pub const fn new(index: u32) -> WaySetId {
        WaySetId(index)
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}
