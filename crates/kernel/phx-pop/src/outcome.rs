//! What a process's outcome does to a cell's members, as the owning system declared it: a person's value changed in
//! place, or households split out as a part whose persons leave or change roles, then re-keyed.

use phx_core::{PersonsGo, RoleMove};
use phx_id::Slot;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Draws, multivariate_hypergeometric};
use phx_store::Backing;

use crate::key::KeyRecord;
use crate::kind::PopKindDecl;
use crate::part::Part;
use crate::profile::{Profile, ProfileLayout};
use crate::table::CellTable;

/// A group of the kind by its name.
#[must_use]
pub fn group(kind: &PopKindDecl, name: &str) -> Option<usize> {
    kind.groups.iter().position(|g| g.name == name)
}

/// A role of the kind by its name.
#[must_use]
pub fn role(kind: &PopKindDecl, name: &str) -> Option<usize> {
    kind.roles.iter().position(|r| r.item.name == name)
}

/// Persons holding one value of a group in a cell take another in place; nothing else about them changes, so none
/// leaves the cell and no total moves.
#[clause("REP.14", "REP.26")]
pub fn revalue<B: Backing>(table: &mut CellTable<B>, slot: Slot, group: usize, from: u32, to: u32, count: u64) {
    if from == to || count == 0 {
        return;
    }
    let held = table.profile_group(slot, group).iter().find(|(v, _)| *v == from).map_or(0, |(_, n)| *n);
    if u64::from(held) < count {
        violation!(clause = "REP.14", "more persons revalued than hold the value", held = held, count = count);
    }
    let Ok(by) = i64::try_from(count) else {
        phx_num::capacity_exceeded!("persons of a cell", i64::MAX, count);
    };
    table.shift_profile(slot, &[(group, from, -by), (group, to, by)]);
}

/// A key with each named attribute set to its value.
#[must_use]
pub fn rekeyed(kind: &PopKindDecl, mut key: KeyRecord, attrs: &[(&str, u32)]) -> KeyRecord {
    for (name, value) in attrs {
        let Some(attr) = kind.key_attrs.iter().position(|d| d.item.name == *name) else {
            violation!(clause = "REP.19", "a key attribute the kind does not hold");
        };
        kind.key.set(&mut key, attr, *value);
    }
    key
}

/// What a hit's outcome makes of the households it reached: the process's group, the persons each household lost in
/// it, where they go, the role moves after, and the key attributes set.
#[derive(Clone, Copy, Debug)]
pub struct Reshape<'a> {
    pub group: usize,
    pub persons: &'a [(u32, u32)],
    pub go: &'a PersonsGo,
    pub moves: &'a [RoleMove],
    pub key: &'a [(&'static str, u32)],
}

fn wide(n: u64) -> u32 {
    let Ok(m) = u32::try_from(n) else {
        phx_num::capacity_exceeded!("persons of a cell", u32::MAX, n);
    };
    m
}

/// `count` persons taken from a group of a profile, their values drawn without replacement from its counts.
fn take(profile: &mut Profile, g: usize, count: u64, d: &mut Draws) -> Vec<(u32, u32)> {
    let held = profile.held(g).to_vec();
    let counts: Vec<u64> = held.iter().map(|(_, n)| u64::from(*n)).collect();
    if counts.iter().sum::<u64>() < count {
        violation!(clause = "REP.14", "more persons taken from a group than it holds", group = g, count = count);
    }
    let mut out = vec![0_u64; counts.len()];
    if count > 0 {
        multivariate_hypergeometric(d, &counts, count, &mut out);
    }
    let taken: Vec<(u32, u32)> =
        held.iter().zip(out).filter(|(_, n)| *n > 0).map(|((v, _), n)| (*v, wide(n))).collect();
    for (v, n) in &taken {
        profile.remove(g, *v, *n);
    }
    taken
}

/// The group of `role` whose components are the same as `g`'s, which a person moving between roles carries its value
/// into.
fn counterpart(kind: &PopKindDecl, g: usize, role: usize) -> Option<usize> {
    let from = kind.groups.get(g)?;
    kind.groups.iter().position(|h| h.role == role && h.components == from.components)
}

/// Values taken from one role's groups added to another's: each carried into its counterpart, each group of the new
/// role with none given its `fill` value for every person.
fn carry(
    profile: &mut Profile,
    layout: &ProfileLayout,
    kind: &PopKindDecl,
    moved: &[(usize, Vec<(u32, u32)>)],
    to: usize,
    fill: &[(&'static str, u32)],
    persons: u32,
) {
    for (h, target) in kind.groups.iter().enumerate().filter(|(_, g)| g.role == to) {
        let source = moved.iter().find(|(g, _)| counterpart(kind, *g, to) == Some(h));
        if let Some((_, values)) = source {
            for (v, n) in values {
                profile.add(layout, h, *v, *n);
            }
        } else {
            let Some((_, v)) = fill.iter().find(|(name, _)| *name == target.name) else {
                violation!(clause = "REP.26", "a person moved into a role with a group nothing gives a value");
            };
            profile.add(layout, h, *v, persons);
        }
    }
}

/// Each group of `from` loses `each` persons of every one of `households` households, drawn from its counts, except the
/// groups given; returns what each group lost.
fn leave_role(
    profile: &mut Profile,
    kind: &PopKindDecl,
    from: usize,
    persons: u64,
    given: &[(usize, Vec<(u32, u32)>)],
    d: &mut Draws,
) -> Vec<(usize, Vec<(u32, u32)>)> {
    let mut out = given.to_vec();
    for (g, _) in kind.groups.iter().enumerate().filter(|(_, x)| x.role == from) {
        if given.iter().any(|(h, _)| *h == g) {
            continue;
        }
        out.push((g, take(profile, g, persons, d)));
    }
    out
}

/// A profile of `households` households reshaped as a hit's outcome says, then checked: every group holds the persons
/// its role's count under the new key gives.
#[clause("REP.26", "REP.14")]
fn reshape_profile(
    profile: &mut Profile,
    layout: &ProfileLayout,
    kind: &PopKindDecl,
    households: u64,
    r: &Reshape<'_>,
    key: &KeyRecord,
    d: &mut Draws,
) {
    let Some(hit_role) = kind.groups.get(r.group).map(|g| g.role) else {
        violation!(clause = "REP.26", "a hit in a group the kind does not hold", group = r.group);
    };
    let per_household: u64 = r.persons.iter().map(|(_, n)| u64::from(*n)).sum();
    let reached = per_household * households;
    let mut hit_values = Vec::with_capacity(r.persons.len());
    for (v, n) in r.persons {
        let n = wide(u64::from(*n) * households);
        profile.remove(r.group, *v, n);
        hit_values.push((*v, n));
    }
    let lost = leave_role(profile, kind, hit_role, reached, &[(r.group, hit_values)], d);
    if let PersonsGo::Into { role: name, fill } = r.go {
        let Some(to) = role(kind, name) else {
            violation!(clause = "REP.26", "persons moved into a role the kind does not hold");
        };
        carry(profile, layout, kind, &lost, to, fill, wide(reached));
    }
    for m in r.moves {
        let (Some(from), Some(to)) = (role(kind, m.from), role(kind, m.to)) else {
            violation!(clause = "REP.26", "a role move between roles the kind does not hold");
        };
        let moving = u64::from(m.persons) * households;
        let moved = leave_role(profile, kind, from, moving, &[], d);
        carry(profile, layout, kind, &moved, to, &[], wide(moving));
    }
    for g in 0..layout.groups.len() {
        if profile.members(g) != layout.persons(g, key, households) {
            violation!(clause = "REP.14", "an outcome left a group's persons other than its key counts", group = g);
        }
    }
}

/// A part split out by an outcome, reshaped as it says and re-keyed. Its weight, totals, rows and holdings are its
/// households' own and do not change.
#[clause("REP.26", "REP.14", "REP.19")]
pub fn reshape(part: &mut Part, kind: &PopKindDecl, layout: &ProfileLayout, r: &Reshape<'_>, d: &mut Draws) {
    part.key = rekeyed(kind, part.key, r.key);
    let households = u64::from(part.weight.get());
    reshape_profile(&mut part.profile, layout, kind, households, r, &part.key, d);
}

/// A cell every household of which an outcome reached, reshaped in place as a part would be; `key` is the key it
/// takes, which the caller sets, so the cell keeps its identity and re-keys at 10b.
#[clause("REP.26", "REP.14")]
pub fn reshape_cell<B: Backing>(
    table: &mut CellTable<B>,
    slot: Slot,
    kind: &PopKindDecl,
    r: &Reshape<'_>,
    key: &KeyRecord,
    d: &mut Draws,
) {
    let layout = table.profile_layout().clone();
    let households = u64::from(table.weight(slot).get());
    let mut profile = table.profile(slot);
    reshape_profile(&mut profile, &layout, kind, households, r, key, d);
    table.set_profile(slot, &profile);
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{
        GroupDecl, KeyAttrDecl, KinkRegistry, PersonsGo, PopEntry, PopItem, ProfileComponent, RoleCount, RoleDecl,
        RoleMove,
    };
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Reshape, rekeyed, reshape_profile};
    use crate::key::KeyRecord;
    use crate::kind::PopKindDecl;
    use crate::profile::{Profile, ProfileLayout};
    use crate::steps::StepTable;

    const AGE: &[ProfileComponent] = &[ProfileComponent { name: "age", values: 4 }];
    const WORK: &[ProfileComponent] = &[ProfileComponent { name: "work", values: 3 }];

    /// A household of a head and a partner (none or one), each with an age and work, and children counted in the key.
    fn kind() -> PopKindDecl {
        let e = |item| PopEntry { system: "DEM", kind: "household", item };
        let group = |name, role, components| PopItem::ProfileGroup(GroupDecl { name, role, components, clause: "x" });
        let entries = [
            e(PopItem::Role(RoleDecl { name: "head", per_member: RoleCount::One, clause: "x" })),
            e(PopItem::Role(RoleDecl { name: "partner", per_member: RoleCount::Key("partners"), clause: "x" })),
            e(PopItem::Role(RoleDecl { name: "child", per_member: RoleCount::Key("children"), clause: "x" })),
            e(PopItem::KeyAttr(KeyAttrDecl { name: "partners", values: 2, clause: "x" })),
            e(PopItem::KeyAttr(KeyAttrDecl { name: "children", values: 4, clause: "x" })),
            e(group("a_head_age", "head", AGE)),
            e(group("a_head_work", "head", WORK)),
            e(group("b_partner_age", "partner", AGE)),
            e(group("b_partner_work", "partner", WORK)),
            e(group("c_child_age", "child", AGE)),
        ];
        let steps = |_| StepTable::new(&Partition { exp: 0, bounds: [0, 1].into() });
        PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap()
    }

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(3), "outcome"), Subject::new(SubjectTag::World, 0), 0, 0)
    }

    /// Households each of a head, a partner and two children, profiled as given.
    fn households(kind: &PopKindDecl, layout: &ProfileLayout, counts: &[(usize, u32, u32)]) -> (Profile, KeyRecord) {
        let mut p = Profile::empty(layout);
        for (g, v, n) in counts {
            p.add(layout, *g, *v, *n);
        }
        (p, rekeyed(kind, KeyRecord::default(), &[("partners", 1), ("children", 2)]))
    }

    fn four(kind: &PopKindDecl, layout: &ProfileLayout) -> (Profile, KeyRecord) {
        households(kind, layout, &[(0, 2, 4), (1, 1, 4), (2, 2, 3), (2, 3, 1), (3, 0, 4), (4, 0, 5), (4, 1, 3)])
    }

    /// A child's death in each of two households: they leave, and the key counts one child fewer.
    #[test]
    fn persons_leaving_a_counted_role_rekey_its_count() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let (mut p, key) =
            households(&kind, &layout, &[(0, 2, 2), (1, 1, 2), (2, 2, 2), (3, 0, 2), (4, 0, 2), (4, 1, 2)]);
        let after = rekeyed(&kind, key, &[("children", 1)]);
        let r = Reshape { group: 4, persons: &[(1, 1)], go: &PersonsGo::Leave, moves: &[], key: &[("children", 1)] };
        reshape_profile(&mut p, &layout, &kind, 2, &r, &after, &mut draws());
        assert_eq!(p.held(4), &[(0, 2)], "the two children of value one left");
    }

    /// A head's death with the partner moving up: the partner's age and work become the head's, and no partner is
    /// left.
    #[test]
    fn a_partner_takes_the_heads_place() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let (mut p, key) = four(&kind, &layout);
        let after = rekeyed(&kind, key, &[("partners", 0)]);
        let moves = [RoleMove { from: "partner", to: "head", persons: 1 }];
        let r = Reshape { group: 0, persons: &[(2, 1)], go: &PersonsGo::Leave, moves: &moves, key: &[("partners", 0)] };
        reshape_profile(&mut p, &layout, &kind, 4, &r, &after, &mut draws());
        assert_eq!(p.held(0), &[(2, 3), (3, 1)], "the heads' ages are the partners'");
        assert_eq!(p.held(1), &[(0, 4)], "and their work");
        assert_eq!((p.members(2), p.members(3)), (0, 0), "no partner left");
    }

    /// A key that does not count what the outcome leaves stops the run.
    #[test]
    fn an_outcome_its_key_does_not_count_is_refused() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let (mut p, key) =
            households(&kind, &layout, &[(0, 2, 2), (1, 1, 2), (2, 2, 2), (3, 0, 2), (4, 0, 2), (4, 1, 2)]);
        let r = Reshape { group: 4, persons: &[(1, 1)], go: &PersonsGo::Leave, moves: &[], key: &[] };
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reshape_profile(&mut p, &layout, &kind, 2, &r, &key, &mut draws());
        }));
        assert!(refused.is_err(), "two children counted where one is left");
    }

    /// A child moving into the partner role of a household without one carries its age, and the partner's work is the
    /// fill given.
    #[test]
    fn a_person_moving_role_carries_its_values_and_fills_the_rest() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let (mut one, key) = households(&kind, &layout, &[(0, 2, 1), (1, 1, 1), (4, 1, 2)]);
        let before = rekeyed(&kind, key, &[("partners", 0)]);
        let go = PersonsGo::Into { role: "partner", fill: vec![("b_partner_work", 2)] };
        let r = Reshape { group: 4, persons: &[(1, 1)], go: &go, moves: &[], key: &[("partners", 1), ("children", 1)] };
        let after = rekeyed(&kind, before, &[("partners", 1), ("children", 1)]);
        reshape_profile(&mut one, &layout, &kind, 1, &r, &after, &mut draws());
        assert_eq!((one.held(2), one.held(3), one.held(4)), (&[(1, 1)][..], &[(2, 1)][..], &[(1, 1)][..]));
    }
}
