//! What a population kind's persons hold, declared item by item by the system that writes each: their roles and their
//! attributes, each a fixed field of a person's word. The household's own words are its kind store's.

use phx_macros::clause;

use crate::system::Declarations;

/// A role a person holds in its household, and the value its word's role field holds for it.
#[clause("REP.26")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleDecl {
    pub name: &'static str,
    pub value: u32,
    pub clause: &'static str,
}

/// An attribute every person of a kind's agents holds in a field of its word, taking one of `values` values; a
/// person's birth date and role are held beside them. Its name only reports it.
#[clause("REP.26", "REP.25")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonAttrDecl {
    pub name: &'static str,
    pub field: crate::person_word::Field,
    pub values: u32,
    pub clause: &'static str,
    /// The value a person holds whom no system has given one, as a child holds its labour state before it works;
    /// none for an attribute its owner always sets.
    pub initial: phx_num::Missing<u32>,
}

/// One item of a population kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopItem {
    Role(RoleDecl),
    PersonAttr(PersonAttrDecl),
}

/// An item as declared: the system that declared it, which writes it, and the kind it belongs to.
#[clause("Law 10")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopEntry {
    pub system: &'static str,
    pub kind: &'static str,
    pub item: PopItem,
}

/// Adds items to one population kind, each recorded with the declaring system as its writer.
#[derive(Debug)]
pub struct PopKindBuilder<'a> {
    d: &'a mut Declarations,
    kind: &'static str,
}

impl<'a> PopKindBuilder<'a> {
    pub(crate) fn new(d: &'a mut Declarations, kind: &'static str) -> PopKindBuilder<'a> {
        PopKindBuilder { d, kind }
    }

    fn add(&mut self, item: PopItem) -> &mut Self {
        let (system, kind) = (self.d.system(), self.kind);
        self.d.pop.push(PopEntry { system, kind, item });
        self
    }

    pub fn role(&mut self, decl: RoleDecl) -> &mut Self {
        self.add(PopItem::Role(decl))
    }

    pub fn person_attr(&mut self, decl: PersonAttrDecl) -> &mut Self {
        self.add(PopItem::PersonAttr(decl))
    }
}
