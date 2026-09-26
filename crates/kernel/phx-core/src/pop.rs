//! What a population kind's agents hold, declared item by item by the system that writes each; the population
//! compiles them into the kind's layout.

use phx_macros::clause;

use crate::system::Declarations;

/// An attribute every agent of a kind holds exactly, taking one of `values` values.
#[clause("REP.41")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrDecl {
    pub name: &'static str,
    pub values: u32,
    pub clause: &'static str,
}

/// A role a person holds in its household.
#[clause("REP.26")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleDecl {
    pub name: &'static str,
    pub clause: &'static str,
}

/// An attribute every person of a kind's agents holds, taking one of `values` values; a person's birth date is held
/// beside them.
#[clause("REP.26", "REP.25")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonAttrDecl {
    pub name: &'static str,
    pub values: u32,
    pub clause: &'static str,
    /// The value a person holds whom no system has given one, as a child holds its labour state before it works;
    /// none for an attribute its owner always sets.
    pub initial: phx_num::Missing<u32>,
}

/// A position every agent of a kind holds: an amount, a stock or a rate of its own, missing until its
/// writer writes it.
#[clause("REP.20")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionDecl {
    pub name: &'static str,
    pub clause: &'static str,
}

/// One item of a population kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopItem {
    Attr(AttrDecl),
    Role(RoleDecl),
    PersonAttr(PersonAttrDecl),
    Position(PositionDecl),
    /// The attribute whose value is the region the agent lives in, where what it leaves behind is sited.
    SitedBy(&'static str),
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

    pub fn attr(&mut self, decl: AttrDecl) -> &mut Self {
        self.add(PopItem::Attr(decl))
    }

    pub fn role(&mut self, decl: RoleDecl) -> &mut Self {
        self.add(PopItem::Role(decl))
    }

    pub fn person_attr(&mut self, decl: PersonAttrDecl) -> &mut Self {
        self.add(PopItem::PersonAttr(decl))
    }

    pub fn position(&mut self, decl: PositionDecl) -> &mut Self {
        self.add(PopItem::Position(decl))
    }

    pub fn sited_by(&mut self, attr: &'static str) -> &mut Self {
        self.add(PopItem::SitedBy(attr))
    }
}
