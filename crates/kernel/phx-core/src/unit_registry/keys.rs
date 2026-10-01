//! What a unit is, as the registry keys it: a product's grade at a zone, a capital class at a zone, an instrument, a
//! special unit at a place, or a deposit's right.

use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::consts::{
    CAPITAL_BAND_BITS, CAPITAL_BANDS, CAPITAL_KIND_BITS, PRODUCT_SHIFT, UNIT_CAPITAL, UNIT_GOOD, UNIT_INSTRUMENT,
    UNIT_RIGHT, UNIT_SPECIAL,
};

const _: () = assert!(CAPITAL_KIND_BITS + CAPITAL_BANDS * CAPITAL_BAND_BITS == u32::BITS, "a class fills one word");

/// What kind of thing a unit is; its code is the row's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyKind {
    Good,
    Capital,
    Instrument,
    Special,
    Right,
}

impl KeyKind {
    /// The kind's code, as its row holds it.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            KeyKind::Good => UNIT_GOOD,
            KeyKind::Capital => UNIT_CAPITAL,
            KeyKind::Instrument => UNIT_INSTRUMENT,
            KeyKind::Special => UNIT_SPECIAL,
            KeyKind::Right => UNIT_RIGHT,
        }
    }

    /// The kind a row's code names; a code no kind has stops the run.
    pub(crate) fn of(code: u8) -> KeyKind {
        match code {
            UNIT_GOOD => KeyKind::Good,
            UNIT_CAPITAL => KeyKind::Capital,
            UNIT_INSTRUMENT => KeyKind::Instrument,
            UNIT_SPECIAL => KeyKind::Special,
            UNIT_RIGHT => KeyKind::Right,
            _ => violation!(clause = "GDS.1", "a unit row of no kind", code = code),
        }
    }
}

/// A class of capital units: its capital kind, its size and quality bands, its condition and its age class, each band
/// within the bits its packing gives it.
#[clause("CAP.1", "REP.24")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapitalClass {
    pub kind: u8,
    pub size: u8,
    pub quality: u8,
    pub condition: u8,
    pub age: u8,
}

impl CapitalClass {
    /// The class in one word: the kind above the four bands; a band past its bits stops the run.
    fn packed(self) -> u32 {
        let band = |b: u8| {
            if u32::from(b) >> CAPITAL_BAND_BITS != 0 {
                violation!(clause = "CAP.1", "a capital class's band past its bits", band = b);
            }
            u32::from(b)
        };
        let bands = [self.size, self.quality, self.condition, self.age];
        bands.iter().fold(u32::from(self.kind), |word, b| (word << CAPITAL_BAND_BITS) | band(*b))
    }

    /// The class a word packs: the age in the lowest bits, the kind in the highest.
    pub(crate) fn unpacked(word: u32) -> CapitalClass {
        let mask = (1 << CAPITAL_BAND_BITS) - 1;
        let byte = |bits: u32| match u8::try_from(bits) {
            Ok(b) => b,
            Err(_) => violation!(clause = "CAP.1", "a capital class's word past its bits", word = word),
        };
        let band = |at: u32| byte((word >> (at * CAPITAL_BAND_BITS)) & mask);
        CapitalClass {
            kind: byte(word >> (CAPITAL_BANDS * CAPITAL_BAND_BITS)),
            size: band(CAPITAL_BANDS - 1),
            quality: band(CAPITAL_BANDS - 2),
            condition: band(1),
            age: band(0),
        }
    }
}

/// What a unit names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnitKey {
    Good { product: u16, grade: u8, zone: u16 },
    Capital { class: CapitalClass, zone: u16 },
    Instrument { id: u32 },
    Special { kind: u32, place: u16 },
    Right { deposit: u32 },
}

impl UnitKey {
    /// The key as a row holds it: its kind, its word, and its zone or place where it has one.
    pub(crate) fn parts(self) -> (KeyKind, u32, Missing<u16>) {
        match self {
            UnitKey::Good { product, grade, zone } => {
                (KeyKind::Good, (u32::from(product) << PRODUCT_SHIFT) | u32::from(grade), Missing::Present(zone))
            }
            UnitKey::Capital { class, zone } => (KeyKind::Capital, class.packed(), Missing::Present(zone)),
            UnitKey::Instrument { id } => (KeyKind::Instrument, id, Missing::Absent),
            UnitKey::Special { kind, place } => (KeyKind::Special, kind, Missing::Present(place)),
            UnitKey::Right { deposit } => (KeyKind::Right, deposit, Missing::Absent),
        }
    }

    /// The key a row's parts name; parts no key has stop the run.
    pub(crate) fn of((kind, word, zone): (KeyKind, u32, Missing<u16>)) -> UnitKey {
        let grade_mask = (1 << PRODUCT_SHIFT) - 1;
        match (kind, zone) {
            (KeyKind::Good, Missing::Present(zone)) => {
                let (Ok(product), Ok(grade)) = (u16::try_from(word >> PRODUCT_SHIFT), u8::try_from(word & grade_mask))
                else {
                    violation!(clause = "GDS.1", "a good's word past its product and grade", word = word);
                };
                UnitKey::Good { product, grade, zone }
            }
            (KeyKind::Capital, Missing::Present(zone)) => {
                UnitKey::Capital { class: CapitalClass::unpacked(word), zone }
            }
            (KeyKind::Instrument, Missing::Absent) => UnitKey::Instrument { id: word },
            (KeyKind::Special, Missing::Present(place)) => UnitKey::Special { kind: word, place },
            (KeyKind::Right, Missing::Absent) => UnitKey::Right { deposit: word },
            _ => violation!(clause = "GDS.1", "a unit row whose zone its kind does not hold", code = kind.code()),
        }
    }
}
