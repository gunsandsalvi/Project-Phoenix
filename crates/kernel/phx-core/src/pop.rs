//! What a population kind's cells carry, declared item by item by the system that writes each; the population
//! compiles them into the kind's layout.

use phx_macros::clause;

use crate::system::Declarations;

/// Whose a position is: each member's, or one total for each listed role within each member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionOf {
    Member,
    Roles(&'static [&'static str]),
}

/// What a position is measured against, so its steps are scale-free: another of the member's positions, or one of
/// its standing rates.
#[clause("REP.20")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleRef {
    Position(&'static str),
    Rate(&'static str),
}

/// How many persons of a role each member holds: one, or as many as one of the member's key attributes says, so a
/// household's children of an age band are one role counted in its key.
#[clause("REP.26", "REP.14")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleCount {
    One,
    Key(&'static str),
}

/// A role within each member: a person of a household, its dwelling, a firm's own; each member holds its count of
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleDecl {
    pub name: &'static str,
    pub per_member: RoleCount,
    pub clause: &'static str,
}

/// An attribute every member of a cell shares exactly, taking one of `values` values.
#[clause("REP.19")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyAttrDecl {
    pub name: &'static str,
    pub values: u32,
    pub clause: &'static str,
}

/// A continuous amount every member shares, held as the cell's total in `unit`, stepped by the partition primitive
/// `steps` on its scale.
#[clause("REP.20", "REP.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionDecl {
    pub name: &'static str,
    pub unit: &'static str,
    pub of: PositionOf,
    pub scale: ScaleRef,
    pub steps: &'static str,
    pub clause: &'static str,
}

/// A rate per member that holds every day until the cell next decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateDecl {
    pub name: &'static str,
    pub unit: &'static str,
    pub clause: &'static str,
}

/// One attribute of a profile group, taking one of `values` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileComponent {
    pub name: &'static str,
    pub values: u32,
}

/// A joint value of components, the first the most significant: each component's value, in turn, over the values of
/// those after it. A value beyond its component's stops the run.
#[clause("REP.32")]
#[must_use]
pub fn joint(components: &[ProfileComponent], values: &[u32]) -> u32 {
    if components.len() != values.len() {
        phx_num::violation!(clause = "REP.32", "a joint value of another number of components", given = values.len());
    }
    components.iter().zip(values).fold(0_u32, |acc, (c, v)| {
        if *v >= c.values {
            phx_num::capacity_exceeded!("values of a profile component", c.values, *v);
        }
        let Some(next) = acc.checked_mul(c.values).and_then(|a| a.checked_add(*v)) else {
            phx_num::capacity_exceeded!("joint values of a profile group", u32::MAX, acc);
        };
        next
    })
}

/// One component's value within a joint value.
#[clause("REP.32")]
#[must_use]
pub fn component(components: &[ProfileComponent], joint: u32, at: usize) -> u32 {
    let Some(c) = components.get(at) else {
        phx_num::violation!(clause = "REP.32", "a component beyond its group", at = at);
    };
    let after = components.iter().skip(at + 1).try_fold(1_u32, |acc, c| acc.checked_mul(c.values));
    let Some(after) = after else {
        phx_num::capacity_exceeded!("joint values of a profile group", u32::MAX, joint);
    };
    (joint / after) % c.values
}

/// Attributes members of a role do not share, counted jointly: each member of the role holds one value of every
/// component, and the cell counts its members per joint value.
#[clause("REP.32", "REP.33")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupDecl {
    pub name: &'static str,
    pub role: &'static str,
    pub components: &'static [ProfileComponent],
    pub clause: &'static str,
}

/// A kind of open business that belongs to particular members and keeps them apart until it closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinDecl {
    pub name: &'static str,
    pub clause: &'static str,
}

/// The ranks a kind's parties are read by each month: the position whose per-member value ranks them, and the
/// primitives holding the promotion rank and the lower rank an individual must fall below to rejoin the cells.
#[clause("REP.29", "REP.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankDecl {
    pub measure: &'static str,
    pub promote: &'static str,
    pub demote: &'static str,
}

/// How a kind is represented, each a RESOLUTION primitive the kind names: its cell budget; its ranks, if its parties
/// are ranked; and the order its positions widen in when their gaps tie.
#[clause("REP.4", "REP.18", "REP.28")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolutionDecl {
    pub cell_budget: &'static str,
    pub ranks: Option<RankDecl>,
    pub widen_order: &'static [&'static str],
    pub clause: &'static str,
}

/// One item of a population kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopItem {
    Role(RoleDecl),
    KeyAttr(KeyAttrDecl),
    Position(PositionDecl),
    StandingRate(RateDecl),
    ProfileGroup(GroupDecl),
    /// A lumpy decision's point, reviewed by the kind's members on their own exposure.
    ReviewKind(&'static str),
    Pin(PinDecl),
    Resolution(ResolutionDecl),
    /// The key attribute whose value is the region the members live in, where what they leave behind is sited.
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

    pub fn role(&mut self, decl: RoleDecl) -> &mut Self {
        self.add(PopItem::Role(decl))
    }

    pub fn key_attr(&mut self, decl: KeyAttrDecl) -> &mut Self {
        self.add(PopItem::KeyAttr(decl))
    }

    pub fn position(&mut self, decl: PositionDecl) -> &mut Self {
        self.add(PopItem::Position(decl))
    }

    pub fn standing_rate(&mut self, decl: RateDecl) -> &mut Self {
        self.add(PopItem::StandingRate(decl))
    }

    pub fn profile_group(&mut self, decl: GroupDecl) -> &mut Self {
        self.add(PopItem::ProfileGroup(decl))
    }

    pub fn review_kind(&mut self, decision: &'static str) -> &mut Self {
        self.add(PopItem::ReviewKind(decision))
    }

    pub fn pin(&mut self, decl: PinDecl) -> &mut Self {
        self.add(PopItem::Pin(decl))
    }

    pub fn resolution(&mut self, decl: ResolutionDecl) -> &mut Self {
        self.add(PopItem::Resolution(decl))
    }

    pub fn sited_by(&mut self, attr: &'static str) -> &mut Self {
        self.add(PopItem::SitedBy(attr))
    }
}

#[cfg(test)]
mod tests {
    use super::{ProfileComponent, component, joint};

    const LIFE: &[ProfileComponent] = &[
        ProfileComponent { name: "year", values: 256 },
        ProfileComponent { name: "sex", values: 2 },
        ProfileComponent { name: "health", values: 2 },
    ];

    #[test]
    fn a_joint_value_reads_back_to_its_components() {
        let v = joint(LIFE, &[125, 1, 0]);
        assert_eq!(v, (125 * 2 + 1) * 2);
        assert_eq!([0, 1, 2].map(|at| component(LIFE, v, at)), [125, 1, 0]);
    }

    #[test]
    fn a_value_beyond_its_component_is_refused() {
        assert!(std::panic::catch_unwind(|| joint(LIFE, &[0, 2, 0])).is_err());
    }
}
