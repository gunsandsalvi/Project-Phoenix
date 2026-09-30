use std::fmt::{self, Write};

use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use phx_num::Missing;

use crate::consts::{
    FAMILY_BITS, FAMILY_SLOT_BITS, GENERATION_BITS, GENERATION_SHIFT, KEY_KIND_BITS, KEY_SLOT_BITS, KIND_SHIFT,
    PARTY_ID_BITS, SYSTEM_CODE_MAX,
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
#[clause("PTY.1", "PTY.10")]
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
#[clause("PTY.1", "REP.3")]
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

/// A party's reference names whose money a figure is.
impl phx_num::Owner for PartyRef {}
impl phx_num::Owner for PartyKey {}

/// A contract row's link, as a person's chain and a wheel entry hold it: its family's code in the top byte and its
/// slot in the family's table below, in one word.
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ContractLink(u32);

impl ContractLink {
    /// A family's code below 2^8 and a slot below 2^24; a wider one stops the run.
    pub fn new(family: u32, slot: Slot) -> ContractLink {
        if family >> FAMILY_BITS != 0 {
            capacity_exceeded!("contract families a link holds", 1_u64 << FAMILY_BITS, u64::from(family) + 1);
        }
        if slot.get() >> FAMILY_SLOT_BITS != 0 {
            capacity_exceeded!("a family's rows a link holds", 1_u64 << FAMILY_SLOT_BITS, u64::from(slot.get()) + 1);
        }
        ContractLink(family << FAMILY_SLOT_BITS | slot.get())
    }

    /// The link held as one word, as a column stores it.
    pub const fn from_word(word: u32) -> ContractLink {
        ContractLink(word)
    }

    #[must_use]
    pub const fn word(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn family(self) -> u8 {
        self.0.to_be_bytes()[0]
    }

    pub const fn slot(self) -> Slot {
        Slot::new(self.0 & (u32::MAX >> FAMILY_BITS))
    }
}

/// A contract row as a holder keeps it past a day: its link and the low byte of the row's generation when taken.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ContractRef {
    link: ContractLink,
    generation: u8,
}

impl ContractRef {
    pub const fn new(link: ContractLink, generation: u8) -> ContractRef {
        ContractRef { link, generation }
    }

    pub const fn link(self) -> ContractLink {
        self.link
    }

    #[must_use]
    pub const fn generation(self) -> u8 {
        self.generation
    }

    /// The row it names while its slot's generation is still the one it was taken at; once the slot holds another
    /// row, none.
    pub fn resolve(self, current: u8) -> Missing<ContractLink> {
        if current == self.generation { Missing::Present(self.link) } else { Missing::Absent }
    }
}

/// A typed reference to a row of one table: the slot and the slot's generation when the reference was taken.
pub trait TableRef: Copy {
    fn from_parts(slot: Slot, generation: u32) -> Self;
    fn slot(self) -> Slot;
    fn generation(self) -> u32;
}

/// A reference to a row of one table, its own type: the generation in the high half of its word and the slot in the
/// low, with no conversion into another table's.
macro_rules! table_ref {
    ($(#[$doc:meta])* $name:ident(u64)) => {
        $(#[$doc])*
        #[must_use]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(u64);

        impl $name {
            /// The reference held as one word, as a column stores it.
            pub const fn from_word(word: u64) -> $name {
                $name(word)
            }

            #[must_use]
            pub const fn word(self) -> u64 {
                self.0
            }
        }

        impl TableRef for $name {
            fn from_parts(slot: Slot, generation: u32) -> $name {
                $name(u64::from(generation) << u32::BITS | u64::from(slot.get()))
            }

            fn slot(self) -> Slot {
                let [.., a, b, c, d] = self.0.to_be_bytes();
                Slot::new(u32::from_be_bytes([a, b, c, d]))
            }

            fn generation(self) -> u32 {
                let [a, b, c, d, ..] = self.0.to_be_bytes();
                u32::from_be_bytes([a, b, c, d])
            }
        }
    };
}

table_ref!(
    /// A holding: a holder's position in one good, unit or instrument.
    HoldingRef(u64)
);
table_ref!(
    /// A standing offer on a market.
    OfferRef(u64)
);
table_ref!(
    /// A message: a notice, an application or an offer sent between parties.
    MessageRef(u64)
);
table_ref!(
    /// A process in progress: a shipment, a project, production in flight, a spell.
    ProcessRef(u64)
);
table_ref!(
    /// An estate: a party's end being settled.
    EstateRef(u64)
);

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
#[path = "ids_tests.rs"]
mod tests;
