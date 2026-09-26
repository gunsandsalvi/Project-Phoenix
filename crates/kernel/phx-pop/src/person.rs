//! A household's persons and its attachments as its arena keeps them: a word each.

use phx_core::Person;
use phx_core::calendar::{civil_date, civil_serial};
use phx_id::{Date, LineId};
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{BIRTH_BITS, HOLDER_SHIFT, HOUSEHOLD_MARK, LINE_BITS, MOST_PERSONS, ROLE_BITS, SIDE_BIT};
use crate::kind::PopKindDecl;

fn mask(bits: u32) -> u64 {
    if bits >= u64::BITS { u64::MAX } else { (1_u64 << bits) - 1 }
}

/// A person packed into a word: its birth date's civil serial, its role, then each of its kind's attributes.
#[clause("REP.26", "REP.25")]
#[must_use]
pub fn pack(kind: &PopKindDecl, p: &Person) -> u64 {
    let Ok(serial) = i32::try_from(civil_serial(p.born)) else {
        capacity_exceeded!("a birth date's civil serial", i64::from(i32::MAX), civil_serial(p.born));
    };
    let Some(role) = kind.role(p.role) else {
        violation!(clause = "REP.26", "a person of a role its kind does not hold");
    };
    let Ok(role) = u64::try_from(role) else {
        capacity_exceeded!("a person's role", u64::MAX, role);
    };
    let mut word = u64::from(serial.cast_unsigned()) | (role << BIRTH_BITS);
    let base = BIRTH_BITS + ROLE_BITS;
    for f in &kind.person_attrs {
        let v = match (p.attr(f.decl.name), f.decl.initial) {
            (Some(v), _) | (None, phx_num::Missing::Present(v)) => v,
            (None, phx_num::Missing::Absent) => {
                violation!(clause = "REP.26", "a person without an attribute its kind declares")
            }
        };
        if v >= f.decl.values {
            capacity_exceeded!("values of a person attribute", f.decl.values, v);
        }
        word |= u64::from(v) << (base + f.shift);
    }
    word
}

/// A person read back from its word, present.
#[clause("REP.26")]
#[must_use]
pub fn unpack(kind: &PopKindDecl, word: u64) -> Person {
    let mut attrs = Vec::new();
    let (role, born) = read(kind, word, &mut attrs);
    Person { role, born, attrs, gone: false }
}

/// A person read back from its word into one already held, present, reusing its attributes' buffer.
#[clause("REP.26")]
pub fn unpack_into(kind: &PopKindDecl, word: u64, p: &mut Person) {
    let (role, born) = read(kind, word, &mut p.attrs);
    p.role = role;
    p.born = born;
    p.gone = false;
}

/// A word's role and birth date, its attributes written to `attrs`.
fn read(kind: &PopKindDecl, word: u64, attrs: &mut Vec<(&'static str, u32)>) -> (&'static str, Date) {
    let Ok(low) = u32::try_from(word & mask(BIRTH_BITS)) else {
        violation!(clause = "REP.25", "a birth date wider than its bits");
    };
    let role = (word >> BIRTH_BITS) & mask(ROLE_BITS);
    let base = BIRTH_BITS + ROLE_BITS;
    attrs.clear();
    attrs.extend(kind.person_attrs.iter().map(|f| {
        let Ok(v) = u32::try_from((word >> (base + f.shift)) & mask(f.bits)) else {
            violation!(clause = "REP.26", "a person attribute wider than its bits");
        };
        (f.decl.name, v)
    }));
    (kind.role_name(phx_rand::float::index(role)), civil_date(i64::from(low.cast_signed())))
}

/// Who in a household holds a contract: the household itself, or one of its persons by place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    Household,
    Person(usize),
}

/// A contract a household or one of its persons holds on a line side.
#[clause("REP.3", "REP.31")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attachment {
    pub holder: Holder,
    pub line: LineId,
    pub side: Side,
}

impl Attachment {
    #[must_use]
    pub fn pack(self) -> u64 {
        let place = match self.holder {
            Holder::Household => HOUSEHOLD_MARK,
            Holder::Person(i) if i < MOST_PERSONS => phx_rand::float::len_u64(i),
            Holder::Person(i) => capacity_exceeded!("persons of a household", MOST_PERSONS, i),
        };
        u64::from(self.line.get()) | (u64::from(self.side == Side::Liability) << SIDE_BIT) | (place << HOLDER_SHIFT)
    }

    #[must_use]
    pub fn unpack(word: u64) -> Attachment {
        let Ok(line) = u32::try_from(word & mask(LINE_BITS)) else {
            violation!(clause = "REP.3", "an attachment's line wider than its bits");
        };
        let side = if (word >> SIDE_BIT) & 1 == 1 { Side::Liability } else { Side::Asset };
        let place = (word >> HOLDER_SHIFT) & mask(u64::BITS - HOLDER_SHIFT);
        let holder =
            if place == HOUSEHOLD_MARK { Holder::Household } else { Holder::Person(phx_rand::float::index(place)) };
        Attachment { holder, line: LineId::new(line), side }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{Person, PersonAttrDecl, PopEntry, PopItem, RoleDecl};
    use phx_id::{Date, LineId};
    use phx_ledger::algebra::Side;

    use super::{Attachment, Holder, pack, unpack, unpack_into};
    use crate::kind::PopKindDecl;

    fn kind() -> PopKindDecl {
        let e = |item| PopEntry { system: "DEM", kind: "household", item };
        let entries = [
            e(PopItem::Role(RoleDecl { name: "head", clause: "x" })),
            e(PopItem::Role(RoleDecl { name: "child", clause: "x" })),
            e(PopItem::PersonAttr(PersonAttrDecl {
                name: "sex",
                values: 2,
                clause: "x",
                initial: phx_num::Missing::Absent,
            })),
            e(PopItem::PersonAttr(PersonAttrDecl {
                name: "health",
                values: 2,
                clause: "x",
                initial: phx_num::Missing::Absent,
            })),
            e(PopItem::PersonAttr(PersonAttrDecl {
                name: "education",
                values: 9,
                clause: "x",
                initial: phx_num::Missing::Absent,
            })),
        ];
        PopKindDecl::compile("household", &entries).unwrap()
    }

    #[test]
    fn a_person_reads_back_as_packed_the_oldest_among_them() {
        let k = kind();
        for (y, m, d, role, sex, health, edu) in
            [(1911, 2, 28, "head", 1, 1, 8), (2025, 12, 31, "child", 0, 0, 0), (1950, 1, 1, "head", 0, 1, 4)]
        {
            let p = Person {
                role,
                born: Date::new(y, m, d).unwrap(),
                attrs: vec![("sex", sex), ("health", health), ("education", edu)],
                gone: false,
            };
            assert_eq!(unpack(&k, pack(&k, &p)), p);
        }
    }

    #[test]
    fn a_person_read_into_one_held_is_the_person_read_fresh() {
        let k = kind();
        let old = Person {
            role: "child",
            born: Date::new(2020, 6, 1).unwrap(),
            attrs: vec![("sex", 0), ("health", 0), ("education", 0)],
            gone: true,
        };
        let new = Person {
            role: "head",
            born: Date::new(1970, 3, 9).unwrap(),
            attrs: vec![("sex", 1), ("health", 1), ("education", 5)],
            gone: false,
        };
        let mut held = old;
        unpack_into(&k, pack(&k, &new), &mut held);
        assert_eq!(held, new);
        assert_eq!(held, unpack(&k, pack(&k, &new)));
    }

    #[test]
    fn an_attribute_beyond_its_values_is_refused() {
        let k = kind();
        let p = Person {
            role: "head",
            born: Date::new(1980, 1, 1).unwrap(),
            attrs: vec![("sex", 2), ("health", 0), ("education", 0)],
            gone: false,
        };
        assert!(std::panic::catch_unwind(|| pack(&k, &p)).is_err());
    }

    #[test]
    fn attachments_read_back() {
        for a in [
            Attachment { holder: Holder::Household, line: LineId::new(7), side: Side::Asset },
            Attachment { holder: Holder::Person(3), line: LineId::new(u32::MAX), side: Side::Liability },
        ] {
            assert_eq!(Attachment::unpack(a.pack()), a);
        }
    }
}
