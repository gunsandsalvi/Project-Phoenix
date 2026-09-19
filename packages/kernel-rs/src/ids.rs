//! Identity. An identifier is an identifier and never a display name.
//!
//! In the TypeScript engine every id is a branded STRING, so every lookup hashes one: measured,
//! `instruments.get` costs 47.80 ns over 38,676,668 calls a period, against 0.77 ns for the
//! same fetch by row index (`tools/calibrate`). Here an id IS the row — a `u32` index into the
//! store's columns — and the string it is displayed as lives beside it in one place.
//!
//! The brands are separate types for the reason the TypeScript ones were: passing a party where an
//! instrument is wanted is a type error and not a wrong answer. They are newtypes over `u32`, so
//! they cost nothing at run time.

/// Every id in this kernel is a row. `NONE` is the absence, and it is never 0 — row 0 is a row.
pub const NONE: u32 = u32::MAX;

macro_rules! row_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub u32);

        impl $name {
            #[inline]
            pub const fn at(row: u32) -> Self {
                Self(row)
            }
            #[inline]
            pub const fn row(self) -> usize {
                self.0 as usize
            }
            /// Whether this is an id at all. `Missing` is missing: there is no zero that means it.
            #[inline]
            pub const fn some(self) -> bool {
                self.0 != NONE
            }
            pub const NONE: Self = Self(NONE);
        }
    };
}

row_id!(
    /// A named party or a cell. The row is its place in `Parties`.
    PartyId
);
row_id!(
    /// An instrument: money, a claim, a share, a good, a plant.
    InstrumentId
);
row_id!(
    /// A book. What it delivers is a fact about the market, not about this id.
    MarketId
);
row_id!(
    /// A venue a module clears itself, where what is struck is not a transfer.
    VenueId
);
row_id!(
    /// A money. Price 1 for money is the only hard-coded price there is.
    CurrencyCode
);
row_id!(
    /// A unit of measure. Periodicity, price level and unit are part of the number.
    UnitId
);
row_id!(
    /// Where a thing is.
    RegionId
);
row_id!(
    /// A holding: one (holder, instrument) pair, and the row the register keeps it in.
    HoldingId
);

/// The names ids are DISPLAYED as, in one place, because Law 9 says an id is never a display name
/// and this is the seam where that is kept true. A store hands out rows; a reader that has to print
/// one asks here, and nothing in a mechanism ever needs to.
#[derive(Default)]
pub struct Names {
    of: Vec<String>,
    /// The way back, for a seed and for the journal, which name their subjects in the spec's words.
    row_of: std::collections::HashMap<String, u32>,
}

impl Names {
    pub fn new() -> Self {
        Self::default()
    }

    /// Name a new row. It is an error to name one twice: one fact, one writer.
    pub fn declare(&mut self, name: &str) -> u32 {
        assert!(
            !self.row_of.contains_key(name),
            "Law 4: {name} is declared twice"
        );
        let row = self.of.len() as u32;
        self.of.push(name.to_string());
        self.row_of.insert(name.to_string(), row);
        row
    }

    /// The row this name was declared as, or `NONE`. Asked once, at a boundary, never in a loop.
    pub fn row(&self, name: &str) -> u32 {
        match self.row_of.get(name) {
            Some(&row) => row,
            None => NONE,
        }
    }

    pub fn name(&self, row: u32) -> &str {
        &self.of[row as usize]
    }

    pub fn len(&self) -> usize {
        self.of.len()
    }

    pub fn is_empty(&self) -> bool {
        self.of.is_empty()
    }
}
