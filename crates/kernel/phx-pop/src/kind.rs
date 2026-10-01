//! A population kind compiled from every system's items: its persons' roles and attributes, each attribute a fixed
//! field of a person's word, and the word a person begins with.

use phx_core::person_word::{Field, PersonWord, ROLE};
use phx_core::{PersonAttrDecl, PopEntry, PopItem, RoleDecl};
use phx_macros::{clause, opening};

/// An item as its kind holds it, with the system that declared it and writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared<T> {
    pub item: T,
    pub system: &'static str,
}

/// A population kind as its persons are laid out.
#[clause("REP.26", "Law 10")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopKindDecl {
    pub kind: &'static str,
    pub roles: Vec<Declared<RoleDecl>>,
    pub person_attrs: Vec<Declared<PersonAttrDecl>>,
    /// A person's word as the kind begins one: every attribute with an initial value at it, every other field nought.
    pub blank: PersonWord,
}

/// The values a field holds.
fn room(f: Field) -> u64 {
    1_u64 << f.bits
}

impl PopKindDecl {
    /// The kind compiled from the items declared for it.
    ///
    /// # Errors
    /// Every refusal at once: a name declared twice, two roles of one value or a role past its field, two attributes
    /// in one field, an attribute of no values or of more than its field holds, and an initial value past its values.
    #[opening]
    pub fn compile(kind: &'static str, entries: &[PopEntry]) -> Result<PopKindDecl, Vec<String>> {
        let mut errors = Vec::new();
        let mut roles: Vec<Declared<RoleDecl>> = Vec::new();
        let mut person_attrs: Vec<Declared<PersonAttrDecl>> = Vec::new();
        let mut blank = PersonWord(0);
        for e in entries.iter().filter(|e| e.kind == kind) {
            match e.item {
                PopItem::Role(r) => {
                    if u64::from(r.value) >= room(ROLE) || roles.iter().any(|o| o.item.value == r.value) {
                        errors.push(format!("`{kind}` role `{}` takes a value its field does not hold", r.name));
                    }
                    roles.push(Declared { item: r, system: e.system });
                }
                PopItem::PersonAttr(p) => {
                    if p.values == 0 || u64::from(p.values) > room(p.field) {
                        errors.push(format!(
                            "`{kind}` attribute `{}` takes {} values its field does not hold",
                            p.name, p.values
                        ));
                    }
                    if person_attrs.iter().any(|o| o.item.field == p.field) {
                        errors.push(format!("`{kind}` attribute `{}` shares its field", p.name));
                    }
                    match p.initial {
                        phx_num::Missing::Present(v) if v >= p.values => {
                            errors.push(format!("`{kind}` attribute `{}` begins past its values", p.name));
                        }
                        phx_num::Missing::Present(v) => blank = blank.with(p.field, v),
                        phx_num::Missing::Absent => {}
                    }
                    person_attrs.push(Declared { item: p, system: e.system });
                }
            }
        }
        let names: Vec<&str> =
            roles.iter().map(|r| r.item.name).chain(person_attrs.iter().map(|p| p.item.name)).collect();
        for (i, n) in names.iter().enumerate() {
            if names.iter().skip(i + 1).any(|m| m == n) {
                errors.push(format!("`{kind}` declares `{n}` twice"));
            }
        }
        if errors.is_empty() { Ok(PopKindDecl { kind, roles, person_attrs, blank }) } else { Err(errors) }
    }

    /// Whether the kind's agents hold persons.
    #[must_use]
    pub fn has_persons(&self) -> bool {
        !self.roles.is_empty()
    }
}

/// The kinds compiled from every system's items, in the order given.
///
/// # Errors
/// Every refusal of every kind at once.
pub fn compile_kinds(names: &[&'static str], entries: &[phx_core::PopEntry]) -> Result<Vec<PopKindDecl>, Vec<String>> {
    let mut errors = Vec::new();
    let mut out = Vec::with_capacity(names.len());
    for kind in names {
        match PopKindDecl::compile(kind, entries) {
            Ok(d) => out.push(d),
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use phx_core::person_word::{EDUCATION, LABOUR, SEX};
    use phx_core::{PersonAttrDecl, PopEntry, PopItem, RoleDecl};
    use phx_num::Missing;

    use super::PopKindDecl;

    fn entry(item: PopItem) -> PopEntry {
        PopEntry { system: "DEM", kind: "household", item }
    }

    fn attr(name: &'static str, field: phx_core::person_word::Field, values: u32, initial: Missing<u32>) -> PopEntry {
        entry(PopItem::PersonAttr(PersonAttrDecl { name, field, values, clause: "x", initial }))
    }

    #[test]
    fn blank_holds_the_initial_values() {
        let entries = [
            entry(PopItem::Role(RoleDecl { name: "head", value: 0, clause: "x" })),
            attr("sex", SEX, 2, Missing::Absent),
            attr("labour", LABOUR, 3, Missing::Present(2)),
        ];
        let k = PopKindDecl::compile("household", &entries).unwrap();
        assert_eq!((k.blank.get(LABOUR), k.blank.get(SEX)), (2, 0));
        assert!(k.has_persons());
    }

    #[test]
    fn refusals() {
        let head = entry(PopItem::Role(RoleDecl { name: "head", value: 0, clause: "x" }));
        let twin = entry(PopItem::Role(RoleDecl { name: "partner", value: 0, clause: "x" }));
        let wide = entry(PopItem::Role(RoleDecl { name: "child", value: 8, clause: "x" }));
        assert!(PopKindDecl::compile("household", &[head, head]).is_err(), "a name twice");
        assert!(PopKindDecl::compile("household", &[head, twin]).is_err(), "two roles of one value");
        assert!(PopKindDecl::compile("household", &[wide]).is_err(), "a role past its field");
        let big = attr("education", EDUCATION, 17, Missing::Absent);
        assert!(PopKindDecl::compile("household", &[big]).is_err(), "more values than the field holds");
        let (a, b) = (attr("a", SEX, 2, Missing::Absent), attr("b", SEX, 2, Missing::Absent));
        assert!(PopKindDecl::compile("household", &[a, b]).is_err(), "two attributes in one field");
        let late = attr("labour", LABOUR, 3, Missing::Present(3));
        assert!(PopKindDecl::compile("household", &[late]).is_err(), "an initial value past the values");
    }
}
