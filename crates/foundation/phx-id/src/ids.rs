use std::fmt::{self, Write};

use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{PARTY_ID_BITS, SYSTEM_CODE_MAX};

/// An identifier over one raw width: comparable and hashable, never defaulted, never converted into another kind.
macro_rules! id {
    ($(#[$doc:meta])* $name:ident($raw:ty)) => {
        $(#[$doc])*
        #[must_use]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name($raw);

        impl $name {
            pub const fn new(raw: $raw) -> $name {
                $name(raw)
            }

            #[must_use]
            pub const fn get(self) -> $raw {
                self.0
            }
        }
    };
}

id!(
    /// A row's storage index, which may be recycled once its row is gone.
    Slot(u32)
);
id!(TableId(u16));
id!(
    /// A line keeps its identity after it is retired, so the identity is never handed out twice.
    LineId(u32)
);
id!(
    /// An instrument keeps its identity after it is retired, so the identity is never handed out twice.
    InstrumentId(u32)
);
id!(MarketId(u16));
id!(TileId(u32));
id!(ZoneId(u32));
id!(RegionId(u16));
id!(CountryId(u8));
id!(
    /// An identity valid within one day only.
    DayLocalId(u32)
);
id!(MsgId(u64));
id!(
    /// A published series: a price, rate or index some market or statistician prints under this one identity.
    SeriesId(u32)
);
id!(StreamId(u32));

/// A party's identity: allocated once from one monotone counter and never reused. It is never zero, but is held as a
/// plain integer so that every stored bit pattern is some identity.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PartyId(u64);

impl PartyId {
    /// Zero is no party; an identity at or above 2^60 would not fit a random draw's subject.
    #[clause("PTY.1")]
    pub fn new(raw: u64) -> PartyId {
        if raw == 0 {
            violation!(clause = "PTY.1", "party identity zero");
        }
        let limit = 1_u64 << PARTY_ID_BITS;
        if raw >= limit {
            capacity_exceeded!("party identity bits", limit, raw);
        }
        PartyId(raw)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A row: its table and its slot there.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RowRef {
    pub table: TableId,
    pub slot: Slot,
}

/// A system's code, two to four capital letters, held inline.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemCode {
    bytes: [u8; SYSTEM_CODE_MAX],
    len: u8,
}

impl SystemCode {
    /// A code of two to four ASCII capital letters, or none.
    #[must_use]
    pub fn new(code: &str) -> Option<SystemCode> {
        let given = code.as_bytes();
        if given.len() < 2 || given.len() > SYSTEM_CODE_MAX || !given.iter().all(u8::is_ascii_uppercase) {
            return None;
        }
        let mut bytes = [0; SYSTEM_CODE_MAX];
        bytes.iter_mut().zip(given).for_each(|(slot, b)| *slot = *b);
        Some(SystemCode { bytes, len: u8::try_from(given.len()).ok()? })
    }

    /// The code's letters, in order.
    pub fn letters(self) -> impl Iterator<Item = char> {
        self.bytes.into_iter().take(usize::from(self.len)).map(char::from)
    }
}

impl fmt::Display for SystemCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.letters().try_for_each(|c| f.write_char(c))
    }
}

#[cfg(test)]
mod tests {
    use super::{PartyId, SystemCode};
    use phx_num::{CapacityExceeded, Violation};

    #[test]
    fn party_id_refuses_2_pow_60() {
        assert_eq!(PartyId::new((1 << 60) - 1).get(), (1 << 60) - 1);
        let Err(payload) = std::panic::catch_unwind(|| PartyId::new(1 << 60)) else {
            panic!("2^60 accepted");
        };
        let c = payload.downcast_ref::<CapacityExceeded>().expect("a capacity payload");
        assert_eq!((c.declared, c.needed), (1 << 60, 1 << 60));
        let Err(payload) = std::panic::catch_unwind(|| PartyId::new(0)) else {
            panic!("zero accepted");
        };
        assert_eq!(payload.downcast_ref::<Violation>().expect("a violation").clause, "PTY.1");
    }

    #[test]
    fn system_codes_are_two_to_four_capitals() {
        assert_eq!(SystemCode::new("DEM").map(|c| c.to_string()).as_deref(), Some("DEM"));
        assert_eq!(SystemCode::new("LABR").map(|c| c.to_string()).as_deref(), Some("LABR"));
        for bad in ["", "D", "DEMOG", "dem", "DE1", "DÉ"] {
            assert!(SystemCode::new(bad).is_none(), "{bad}");
        }
    }
}
