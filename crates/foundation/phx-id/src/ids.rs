use std::fmt::{self, Write};

use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{
    GENERATION_BITS, GENERATION_SHIFT, KEY_KIND_BITS, KEY_SLOT_BITS, KIND_SHIFT, PARTY_ID_BITS, SYSTEM_CODE_MAX,
};

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
id!(MarketId(u32));
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

/// Where a party is held: its kind's table, its slot there, and the slot's generation when it took the party, so a
/// reference kept past the party's end is known stale once the slot holds another.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PartyRef(u64);

impl PartyRef {
    /// A kind's table, a slot and a generation below 2^24; a larger generation is refused.
    pub fn new(kind: u8, generation: u32, slot: Slot) -> PartyRef {
        let limit = 1_u32 << GENERATION_BITS;
        if generation >= limit {
            capacity_exceeded!("slot generations", limit, generation);
        }
        PartyRef(u64::from(kind) << KIND_SHIFT | u64::from(generation) << GENERATION_SHIFT | u64::from(slot.get()))
    }

    /// The reference held as one word, as a column stores it.
    pub const fn from_word(word: u64) -> PartyRef {
        PartyRef(word)
    }

    #[must_use]
    pub const fn word(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn kind(self) -> u8 {
        self.0.to_be_bytes()[0]
    }

    #[must_use]
    pub fn generation(self) -> u32 {
        let [_, a, b, c, ..] = self.0.to_be_bytes();
        u32::from_be_bytes([0, a, b, c])
    }

    pub fn slot(self) -> Slot {
        let [.., a, b, c, d] = self.0.to_be_bytes();
        Slot::new(u32::from_be_bytes([a, b, c, d]))
    }

    /// The party's place within the day.
    pub fn key(self) -> PartyKey {
        PartyKey::new(self.kind(), self.slot())
    }
}

/// A party's place within the day: its kind's table and its slot, in one word, as the day's flows and the contracts
/// between parties name it. It carries no generation: a contract's side moves when its party ends and a slot is
/// reused only after the day closes, so neither outlives the party it names.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PartyKey(u32);

impl PartyKey {
    /// A kind below 2^5 and a slot below 2^27; a larger one is refused.
    pub fn new(kind: u8, slot: Slot) -> PartyKey {
        let kinds = 1_u32 << KEY_KIND_BITS;
        if u32::from(kind) >= kinds {
            capacity_exceeded!("party kinds a key holds", kinds, kind);
        }
        let slots = 1_u32 << KEY_SLOT_BITS;
        if slot.get() >= slots {
            capacity_exceeded!("slots a party key holds", slots, slot.get());
        }
        PartyKey(u32::from(kind) << KEY_SLOT_BITS | slot.get())
    }

    /// The key held as one word, as a column stores it.
    pub const fn from_word(word: u32) -> PartyKey {
        PartyKey(word)
    }

    #[must_use]
    pub const fn word(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn kind(self) -> u8 {
        u8::try_from(self.0 >> KEY_SLOT_BITS).unwrap_or(u8::MAX)
    }

    pub fn slot(self) -> Slot {
        Slot::new(self.0 & ((1 << KEY_SLOT_BITS) - 1))
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

    #[test]
    fn party_ref_packs_kind_generation_and_slot() {
        let r = super::PartyRef::new(7, 0x00ab_cdef, crate::Slot::new(0xdead_beef));
        assert_eq!((r.kind(), r.generation(), r.slot().get()), (7, 0x00ab_cdef, 0xdead_beef));
        assert_eq!(super::PartyRef::from_word(r.word()), r);
        let over = std::panic::catch_unwind(|| super::PartyRef::new(0, 1 << 24, crate::Slot::new(0)));
        assert!(over.is_err(), "a generation past 24 bits is refused");
    }

    #[test]
    fn party_key_packs_kind_and_slot() {
        let k = super::PartyKey::new(21, crate::Slot::new(100_000_000));
        assert_eq!((k.kind(), k.slot().get()), (21, 100_000_000));
        assert!(std::panic::catch_unwind(|| super::PartyKey::new(32, crate::Slot::new(0))).is_err());
        assert!(std::panic::catch_unwind(|| super::PartyKey::new(0, crate::Slot::new(1 << 27))).is_err());
        let r = super::PartyRef::new(3, 9, crate::Slot::new(44));
        assert_eq!(r.key(), super::PartyKey::new(3, crate::Slot::new(44)));
    }
}
