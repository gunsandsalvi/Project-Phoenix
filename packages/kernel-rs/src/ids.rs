//! Identity. An identifier is an identifier and never a display name.

/// Every id in this kernel is a row.
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
            /// Whether this is an id at all.
            #[inline]
            pub const fn some(self) -> bool {
                self.0 != NONE
            }
            pub const NONE: Self = Self(NONE);
        }
    };
}

row_id!(
    /// A named party or a cell.
    PartyId
);
row_id!(
    /// An instrument: money, a claim, a share, a good, a plant.
    InstrumentId
);
row_id!(
    /// A book.
    MarketId
);
row_id!(
    /// A venue a module clears itself, where what is struck is not a transfer.
    VenueId
);
row_id!(
    /// A money.
    CurrencyCode
);
row_id!(UnitId);
row_id!(
    /// Where a thing is.
    RegionId
);
row_id!(
    /// Whose law a place is under: one currency, one central bank, one treasury. A country is one
    /// thing, so the physical world and the registry point at the same rows.
    CountryId
);
row_id!(
    /// A holding: one (holder, instrument) pair, and the row the register keeps it in.
    HoldingId
);

/// The names ids are DISPLAYED as, in one place, because Law 9 says an id is never a display name
/// and this is the seam where that is kept true.
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

    /// Name a new row.
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

    /// The row this name was declared as, or `NONE`.
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

/// Allocate the conventional market id when first declaring a book for a line. After declaration,
/// `BookDecl` is authoritative; equal row numbers are an opening convention, not market identity.
#[inline]
pub fn book_of(line: InstrumentId) -> MarketId {
    MarketId::at(line.0)
}

/// A PLACED book's conventional id. A line with one book everywhere takes that line's own row, so
/// the books that have a place are counted down from the top of the space instead: the two blocks
/// cannot meet without more instruments than a row number can hold.
#[inline]
pub fn placed_book(nth: u32) -> MarketId {
    MarketId::at(u32::MAX - nth)
}
