//! A population kind compiled from every system's items: its attributes, its persons' roles and attributes, and where
//! its agents are sited.

use phx_core::{AttrDecl, PersonAttrDecl, PopEntry, PopItem, PositionDecl, RoleDecl};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::consts::{PERSON_ATTR_BITS, ROLE_BITS};

/// An item as its kind holds it, with the system that declared it and writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared<T> {
    pub item: T,
    pub system: &'static str,
}

/// A person attribute's place in a person's word: its bits from `shift`, `bits` wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonField {
    pub decl: PersonAttrDecl,
    pub system: &'static str,
    pub shift: u32,
    pub bits: u32,
}

/// A population kind as its agents are laid out.
#[clause("REP.41", "REP.26", "Law 10")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopKindDecl {
    pub kind: &'static str,
    pub attrs: Vec<Declared<AttrDecl>>,
    pub roles: Vec<Declared<RoleDecl>>,
    pub person_attrs: Vec<PersonField>,
    /// The positions each agent holds, each a column of the kind's table in this order.
    pub positions: Vec<Declared<PositionDecl>>,
    /// The attribute whose value is the region an agent lives in.
    pub sited_by: Missing<usize>,
}

/// Bits that hold every value below `values`.
fn bits_for(values: u32) -> u32 {
    match values.checked_sub(1) {
        Some(most) => u32::BITS - most.leading_zeros(),
        None => 0,
    }
}

impl PopKindDecl {
    /// The kind compiled from the items declared for it.
    ///
    /// # Errors
    /// Every refusal at once: a name declared twice, roles or person attributes beyond a person's word, an attribute
    /// of no values, or a siting attribute the kind does not hold.
    pub fn compile(kind: &'static str, entries: &[PopEntry]) -> Result<PopKindDecl, Vec<String>> {
        let mut errors = Vec::new();
        let mut attrs = Vec::new();
        let mut roles = Vec::new();
        let mut person_attrs: Vec<PersonField> = Vec::new();
        let mut positions = Vec::new();
        let mut sited: Option<&'static str> = None;
        let mut shift = 0_u32;
        for e in entries.iter().filter(|e| e.kind == kind) {
            match e.item {
                PopItem::Attr(a) => {
                    if a.values == 0 {
                        errors.push(format!("`{kind}` attribute `{}` takes no values", a.name));
                    }
                    attrs.push(Declared { item: a, system: e.system });
                }
                PopItem::Role(r) => roles.push(Declared { item: r, system: e.system }),
                PopItem::PersonAttr(p) => {
                    if p.values == 0 {
                        errors.push(format!("`{kind}` person attribute `{}` takes no values", p.name));
                    }
                    let bits = bits_for(p.values);
                    person_attrs.push(PersonField { decl: p, system: e.system, shift, bits });
                    shift += bits;
                }
                PopItem::Position(p) => positions.push(Declared { item: p, system: e.system }),
                PopItem::SitedBy(name) => {
                    if sited.replace(name).is_some() {
                        errors.push(format!("`{kind}` sited by two attributes"));
                    }
                }
            }
        }
        let names: Vec<&str> = attrs
            .iter()
            .map(|a| a.item.name)
            .chain(roles.iter().map(|r| r.item.name))
            .chain(person_attrs.iter().map(|p| p.decl.name))
            .chain(positions.iter().map(|p| p.item.name))
            .collect();
        for (i, n) in names.iter().enumerate() {
            if names.iter().skip(i + 1).any(|m| m == n) {
                errors.push(format!("`{kind}` declares `{n}` twice"));
            }
        }
        if roles.len() > 1 << ROLE_BITS {
            errors.push(format!("`{kind}` declares {} roles, more than a person's word holds", roles.len()));
        }
        if shift > PERSON_ATTR_BITS {
            errors.push(format!("`{kind}`'s person attributes take {shift} bits, more than a person's word holds"));
        }
        let mut sited_by = Missing::Absent;
        if let Some(name) = sited {
            if let Some(i) = attrs.iter().position(|a| a.item.name == name) {
                sited_by = Missing::Present(i);
            } else {
                errors.push(format!("`{kind}` sited by `{name}`, an attribute it does not hold"));
            }
        }
        if errors.is_empty() {
            Ok(PopKindDecl { kind, attrs, roles, person_attrs, positions, sited_by })
        } else {
            Err(errors)
        }
    }

    /// An attribute's place among the kind's.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<usize> {
        self.attrs.iter().position(|a| a.item.name == name)
    }

    /// A position's place among the kind's.
    #[must_use]
    pub fn position(&self, name: &str) -> Option<usize> {
        self.positions.iter().position(|p| p.item.name == name)
    }

    /// A role's place among the kind's.
    #[must_use]
    pub fn role(&self, name: &str) -> Option<usize> {
        self.roles.iter().position(|r| r.item.name == name)
    }

    /// A role's name by its place.
    #[must_use]
    pub fn role_name(&self, place: usize) -> &'static str {
        let Some(r) = self.roles.get(place) else {
            violation!(clause = "REP.26", "a role beyond the kind's", role = place);
        };
        r.item.name
    }

    /// A person attribute's place among the kind's.
    #[must_use]
    pub fn person_attr(&self, name: &str) -> Option<usize> {
        self.person_attrs.iter().position(|p| p.decl.name == name)
    }

    /// Whether the kind's agents hold persons.
    #[must_use]
    pub fn has_persons(&self) -> bool {
        !self.roles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{AttrDecl, PersonAttrDecl, PopEntry, PopItem, RoleDecl};

    use super::{PopKindDecl, bits_for};

    fn entry(item: PopItem) -> PopEntry {
        PopEntry { system: "DEM", kind: "household", item }
    }

    #[test]
    fn bits_hold_every_value() {
        assert_eq!([1, 2, 3, 4, 5, 9, 256].map(bits_for), [0, 1, 2, 2, 3, 4, 8]);
    }

    #[test]
    fn person_attributes_pack_after_one_another() {
        let entries = [
            entry(PopItem::Attr(AttrDecl { name: "region", values: 25, clause: "x" })),
            entry(PopItem::Role(RoleDecl { name: "head", clause: "x" })),
            entry(PopItem::PersonAttr(PersonAttrDecl {
                name: "sex",
                values: 2,
                clause: "x",
                initial: phx_num::Missing::Absent,
            })),
            entry(PopItem::PersonAttr(PersonAttrDecl {
                name: "education",
                values: 10,
                clause: "x",
                initial: phx_num::Missing::Absent,
            })),
            entry(PopItem::SitedBy("region")),
        ];
        let k = PopKindDecl::compile("household", &entries).unwrap();
        let fields: Vec<(u32, u32)> = k.person_attrs.iter().map(|f| (f.shift, f.bits)).collect();
        assert_eq!(fields, [(0, 1), (1, 4)]);
        assert_eq!((k.attr("region"), k.role("head"), k.person_attr("education")), (Some(0), Some(0), Some(1)));
    }

    #[test]
    fn a_kind_sited_by_nothing_it_holds_is_refused() {
        let entries = [entry(PopItem::SitedBy("region"))];
        assert!(PopKindDecl::compile("household", &entries).is_err());
    }
}
