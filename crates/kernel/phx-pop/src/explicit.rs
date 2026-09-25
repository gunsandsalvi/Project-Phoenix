//! A cell's households made explicit for a day's outcomes: the households its hits reached drawn out of its counts
//! with every person's values, changed by the processes' outcomes, and returned to the cell or grouped into parts by
//! the keys they have become.

use std::collections::BTreeMap;

use phx_core::RoleCount;
use phx_id::LineId;
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::{Draws, below_u64};

use crate::key::KeyRecord;
use crate::kind::PopKindDecl;
use crate::profile::{Profile, ProfileLayout, net};

/// A row a household or a person holds a member of: its line and side.
pub type Attached = (LineId, Side);

/// A person made explicit: its role, its value in each group of its role, in the kind's order of groups, and the rows
/// it holds a member of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub role: usize,
    pub values: Vec<(usize, u32)>,
    pub rows: Vec<Attached>,
}

impl Held {
    #[must_use]
    pub fn value(&self, group: usize) -> Option<u32> {
        self.values.iter().find(|(g, _)| *g == group).map(|(_, v)| *v)
    }
}

/// A household made explicit: its persons, the roles in the kind's order and each role's persons together, and the
/// rows the household itself holds a member of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Explicit {
    pub persons: Vec<Held>,
    pub rows: Vec<Attached>,
}

/// The households a day's hits on a cell reached, made explicit, and for each hit the persons it reached, each by its
/// household and its place there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Touched {
    pub households: Vec<Explicit>,
    pub reached: Vec<Vec<(usize, usize)>>,
}

fn wide(n: usize) -> u64 {
    phx_rand::float::len_u64(n)
}

/// The persons of each role a household of a key holds: one, or the key's count of them.
#[clause("REP.26", "REP.14")]
#[must_use]
pub fn role_counts(kind: &PopKindDecl, key: &KeyRecord) -> Vec<u32> {
    kind.roles
        .iter()
        .map(|r| match r.item.per_member {
            RoleCount::One => 1,
            RoleCount::Key(attr) => {
                let Some(f) = kind.key.named(attr) else {
                    violation!(clause = "REP.26", "a role counted by an attribute the key does not hold");
                };
                crate::key::read(&f, key)
            }
        })
        .collect()
}

/// The groups of a role, in the kind's order.
fn groups_of(kind: &PopKindDecl, role: usize) -> impl Iterator<Item = usize> + '_ {
    kind.groups.iter().enumerate().filter(move |(_, g)| g.role == role).map(|(i, _)| i)
}

fn role_of(kind: &PopKindDecl, group: usize) -> usize {
    let Some(g) = kind.groups.get(group) else {
        violation!(clause = "REP.32", "a profile group the kind does not hold", group = group);
    };
    g.role
}

/// The persons of a cell not yet made explicit, by group and value.
struct Pool(Vec<Vec<(u32, u64)>>);

impl Pool {
    fn of(profile: &Profile, groups: usize) -> Pool {
        Pool((0..groups).map(|g| profile.held(g).iter().map(|(v, n)| (*v, u64::from(*n))).collect()).collect())
    }

    fn group(&mut self, g: usize) -> &mut Vec<(u32, u64)> {
        let Some(held) = self.0.get_mut(g) else {
            violation!(clause = "REP.32", "a profile group the kind does not hold", group = g);
        };
        held
    }

    fn free(&self, g: usize, v: u32) -> u64 {
        self.0.get(g).and_then(|h| h.iter().find(|(x, _)| *x == v)).map_or(0, |(_, n)| *n)
    }

    fn take_value(&mut self, g: usize, v: u32) {
        let Some((_, n)) = self.group(g).iter_mut().find(|(x, n)| *x == v && *n > 0) else {
            violation!(clause = "REP.14", "a person taken from a value its cell no longer holds", value = v);
        };
        *n -= 1;
    }

    /// One person of a group, each as likely.
    fn take(&mut self, g: usize, d: &mut Draws) -> u32 {
        let held = self.group(g);
        let counts: Vec<u64> = held.iter().map(|(_, n)| *n).collect();
        let at = crate::pick::one_of(d, &counts);
        let Some((v, n)) = held.get_mut(at) else { violation!(clause = "CHN.2", "a pick beyond its counts") };
        *n -= 1;
        *v
    }
}

/// A household drawn from the persons not yet made explicit: each role's persons, each with a value in every group of
/// its role; `first` is a person of it already drawn, given its role, group and value, placed first of its role.
fn fresh(kind: &PopKindDecl, counts: &[u32], pool: &mut Pool, first: (usize, usize, u32), d: &mut Draws) -> Explicit {
    let (first_role, first_group, first_value) = first;
    let mut persons = Vec::new();
    for (role, count) in counts.iter().enumerate() {
        for i in 0..*count {
            let values = groups_of(kind, role)
                .map(|g| {
                    let given = role == first_role && i == 0 && g == first_group;
                    (g, if given { first_value } else { pool.take(g, d) })
                })
                .collect();
            persons.push(Held { role, values, rows: Vec::new() });
        }
    }
    Explicit { persons, rows: Vec::new() }
}

/// The households of a cell of `weight` households under `key` that the day's hits reached, made explicit. Each hit
/// lists the persons it reached by group, value and number. Within a hit its persons are drawn without replacement:
/// each falls on a person already made explicit that the hit has not reached, or on one not yet made explicit, each
/// holder of its value as likely; across hits the draws are independent, so two hits may reach one person. A person
/// not yet made explicit brings its household with it, the household's other persons drawn once from the persons of
/// the cell no household made explicit holds.
#[clause("REP.26", "REP.23", "CHN.7")]
#[must_use]
pub fn materialise(
    kind: &PopKindDecl,
    key: &KeyRecord,
    weight: u64,
    profile: &Profile,
    hits: &[Vec<(usize, u32, u64)>],
    d: &mut Draws,
) -> Touched {
    let counts = role_counts(kind, key);
    let mut pool = Pool::of(profile, kind.groups.len());
    let mut households: Vec<Explicit> = Vec::new();
    let mut reached = Vec::with_capacity(hits.len());
    for hit in hits {
        let mut mine: Vec<(usize, usize)> = Vec::new();
        for (g, v, n) in hit {
            let role = role_of(kind, *g);
            for _ in 0..*n {
                let known: Vec<(usize, usize)> = households
                    .iter()
                    .enumerate()
                    .flat_map(|(h, e)| {
                        e.persons
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| p.role == role && p.value(*g) == Some(*v))
                            .map(move |(i, _)| (h, i))
                    })
                    .filter(|at| !mine.contains(at))
                    .collect();
                let total = wide(known.len()) + pool.free(*g, *v);
                if total == 0 {
                    violation!(clause = "REP.14", "a hit reaching more persons of a value than its cell holds");
                }
                let at = below_u64(d, total);
                if let Some(person) = usize::try_from(at).ok().and_then(|i| known.get(i)) {
                    mine.push(*person);
                    continue;
                }
                if wide(households.len()) >= weight {
                    capacity_exceeded!("households made explicit", weight, wide(households.len()) + 1);
                }
                pool.take_value(*g, *v);
                let e = fresh(kind, &counts, &mut pool, (role, *g, *v), d);
                let Some(place) = e.persons.iter().position(|p| p.role == role) else {
                    violation!(clause = "REP.26", "a person reached in a role its households do not hold");
                };
                mine.push((households.len(), place));
                households.push(e);
            }
        }
        reached.push(mine);
    }
    Touched { households, reached }
}

/// A household as a process's outcome reads it: its key's attributes and its persons, by their names.
#[must_use]
pub fn named(kind: &PopKindDecl, key: &KeyRecord, e: &Explicit) -> phx_core::Household {
    let key = kind.key_attrs.iter().enumerate().map(|(i, a)| (a.item.name, kind.key.get(key, i))).collect();
    let persons = e
        .persons
        .iter()
        .map(|p| {
            let Some(role) = kind.roles.get(p.role) else {
                violation!(clause = "REP.26", "a person of a role the kind does not hold", role = p.role);
            };
            let values = p
                .values
                .iter()
                .map(|(g, v)| {
                    let Some(group) = kind.groups.get(*g) else {
                        violation!(clause = "REP.32", "a value of a group the kind does not hold", group = *g);
                    };
                    (group.name, *v)
                })
                .collect();
            phx_core::Person { role: role.item.name, values, gone: false }
        })
        .collect();
    phx_core::Household { key, persons }
}

/// A household as an outcome left it: its key, and its persons still there, each with one value in every group of
/// its role and none of another's.
#[clause("REP.26", "REP.19", "REP.32")]
#[must_use]
pub fn from_named(kind: &PopKindDecl, h: &phx_core::Household) -> (KeyRecord, Explicit) {
    let (key, e, _) = read(kind, h, None);
    (key, e)
}

/// A household as an outcome left it, as `from_named` reads it, with the rows it and its persons held as it was made
/// explicit: a person still there keeps its own, whatever role it now has; the rows of a person gone are returned
/// apart, one member each, to leave their lines.
#[clause("REP.26", "REP.19", "REP.32", "REP.23")]
#[must_use]
pub fn carried(kind: &PopKindDecl, h: &phx_core::Household, before: &Explicit) -> (KeyRecord, Explicit, Vec<Attached>) {
    if h.persons.len() != before.persons.len() {
        violation!(clause = "REP.26", "an outcome that added or took away persons rather than marking them gone");
    }
    read(kind, h, Some(before))
}

fn read(
    kind: &PopKindDecl,
    h: &phx_core::Household,
    before: Option<&Explicit>,
) -> (KeyRecord, Explicit, Vec<Attached>) {
    let mut key = KeyRecord::default();
    if h.key.len() != kind.key_attrs.len() {
        violation!(clause = "REP.19", "a household's key of other attributes than its kind's", attrs = h.key.len());
    }
    for (name, v) in &h.key {
        let Some(at) = kind.key_attrs.iter().position(|a| a.item.name == *name) else {
            violation!(clause = "REP.19", "a household keyed by an attribute its kind does not hold");
        };
        kind.key.set(&mut key, at, *v);
    }
    let mut persons: Vec<Held> = Vec::new();
    let mut left: Vec<Attached> = Vec::new();
    for (i, p) in h.persons.iter().enumerate() {
        let rows: &[Attached] = before.and_then(|b| b.persons.get(i)).map_or(&[], |was| &was.rows);
        if p.gone {
            left.extend(rows.iter().copied());
            continue;
        }
        let Some(role) = kind.roles.iter().position(|r| r.item.name == p.role) else {
            violation!(clause = "REP.26", "a person of a role its kind does not hold");
        };
        let values: Vec<(usize, u32)> = groups_of(kind, role)
            .map(|g| {
                let name = kind.groups.get(g).map(|x| x.name);
                let Some(v) = name.and_then(|n| p.value(n)) else {
                    violation!(clause = "REP.32", "a person without a value in a group of its role", group = g);
                };
                (g, v)
            })
            .collect();
        if values.len() != p.values.len() {
            violation!(clause = "REP.32", "a person with a value in a group not of its role");
        }
        persons.push(Held { role, values, rows: rows.to_vec() });
    }
    persons.sort_by_key(|p| p.role);
    let rows = before.map_or_else(Vec::new, |b| b.rows.clone());
    (key, Explicit { persons, rows }, left)
}

/// Households split out of a cell together: the key they share now, how many they are, their profile as they were
/// drawn out and as they are, and the members they and their persons hold of each of the cell's rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub key: KeyRecord,
    pub households: u32,
    pub before: Profile,
    pub after: Profile,
    pub rows: Vec<(Attached, u32)>,
}

/// What the day's outcomes make of a cell's explicit households: the changes of values of those that kept the cell's
/// key, made in place; those whose key changed, grouped by their new key; and those no one is left in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Regrouped {
    pub in_place: Vec<(usize, u32, i64)>,
    pub parts: Vec<Moved>,
    pub ended: Option<Moved>,
}

/// A cell every household of which moved together takes their profile as it is now; the caller gives it their key.
pub fn take_whole<B: phx_store::Backing>(table: &mut crate::table::CellTable<B>, slot: phx_id::Slot, moved: &Moved) {
    if table.weight(slot).get() != moved.households {
        violation!(clause = "REP.14", "a cell given the profile of households other than all of its own");
    }
    table.set_profile(slot, &moved.after);
}

fn profile_of<'a>(layout: &ProfileLayout, households: impl Iterator<Item = &'a Explicit>) -> Profile {
    let mut p = Profile::empty(layout);
    for e in households {
        for person in &e.persons {
            for (g, v) in &person.values {
                p.add(layout, *g, *v, 1);
            }
        }
    }
    p
}

/// The members households and their persons hold of each row, in the rows' order.
fn rows_of<'a>(households: impl Iterator<Item = &'a Explicit>) -> Vec<(Attached, u32)> {
    let mut by: BTreeMap<Attached, u32> = BTreeMap::new();
    for e in households {
        for r in e.rows.iter().chain(e.persons.iter().flat_map(|p| &p.rows)) {
            *by.entry(*r).or_insert(0) += 1;
        }
    }
    by.into_iter().collect()
}

fn count32(n: usize) -> u32 {
    let Ok(n) = u32::try_from(n) else { capacity_exceeded!("households of a part", u32::MAX, wide(n)) };
    n
}

/// A household under a key: each role holds the persons the key counts for it.
#[clause("REP.14", "REP.26")]
pub fn check(kind: &PopKindDecl, key: &KeyRecord, e: &Explicit) {
    for (role, count) in role_counts(kind, key).into_iter().enumerate() {
        let held = e.persons.iter().filter(|p| p.role == role).count();
        if wide(held) != u64::from(count) {
            violation!(clause = "REP.14", "a household's persons of a role other than its key counts", role = role);
        }
    }
}

/// A row the households of one key hold together: its members, and the pool its balance is a share of with the
/// weights they drew summed, when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatheredRow {
    pub at: Attached,
    pub count: u32,
    pub pool: Missing<(u32, u64)>,
}

/// Explicit households gathered by key, as the opening forms them: each held to its key's counts, with one value in
/// every group of its role; the households of one key one part, weighing as many as they are, with their persons'
/// values as its profile and their rows' members and balance weights summed.
#[derive(Debug, Default)]
pub struct Gathered {
    cells: BTreeMap<KeyRecord, (u32, Profile, Rows)>,
}

/// A key's rows: each row's members and its balance's pool and summed weight.
type Rows = BTreeMap<Attached, (u32, Missing<(u32, u64)>)>;

impl Gathered {
    /// A household gathered, with the pool and weight of each of its rows' balances, where they have one.
    #[clause("REP.14", "REP.26", "GEN.3")]
    pub fn add(
        &mut self,
        kind: &PopKindDecl,
        layout: &ProfileLayout,
        key: KeyRecord,
        e: &Explicit,
        weights: &[(Attached, u32, u64)],
    ) {
        check(kind, &key, e);
        for person in &e.persons {
            if !person.values.iter().map(|(g, _)| *g).eq(groups_of(kind, person.role)) {
                violation!(
                    clause = "REP.32",
                    "a person without one value in each group of its role",
                    role = person.role
                );
            }
        }
        let (n, profile, rows) = self.cells.entry(key).or_insert_with(|| (0, Profile::empty(layout), BTreeMap::new()));
        let Some(more) = n.checked_add(1) else { capacity_exceeded!("households of a key", u32::MAX, *n) };
        *n = more;
        for person in &e.persons {
            for (g, v) in &person.values {
                profile.add(layout, *g, *v, 1);
            }
        }
        for at in e.rows.iter().chain(e.persons.iter().flat_map(|p| &p.rows)) {
            let (count, _) = rows.entry(*at).or_insert((0, Missing::Absent));
            let Some(more) = count.checked_add(1) else { capacity_exceeded!("members of a row", u32::MAX, *count) };
            *count = more;
        }
        for (at, pool, weight) in weights {
            let Some((_, held)) = rows.get_mut(at) else {
                violation!(
                    clause = "GEN.4",
                    "a balance's weight on a row its household does not hold",
                    line = at.0.get()
                );
            };
            *held = match *held {
                Missing::Absent => Missing::Present((*pool, *weight)),
                Missing::Present((p, w)) if p == *pool => Missing::Present((p, w + weight)),
                Missing::Present(_) => violation!(clause = "GEN.4", "one row's balance a share of two pools"),
            };
        }
    }

    /// The households gathered, a part for each key in the keys' order.
    #[must_use]
    pub fn into_drawn(self) -> Vec<crate::landing::Drawn> {
        self.cells
            .into_iter()
            .map(|(key, (n, profile, rows))| crate::landing::Drawn {
                key,
                weight: phx_core::Weight::new(n),
                profile,
                rows: rows.into_iter().map(|(at, (count, pool))| GatheredRow { at, count, pool }).collect(),
            })
            .collect()
    }
}

/// The explicit households, as they were drawn out of a cell under `key` and as the outcomes left them, returned or
/// regrouped. A household no one is left in ends; one under the cell's key returns its changes in place; the rest
/// are grouped by the key they now hold, in the order of their keys, each held to its key's counts.
#[clause("REP.26", "REP.14", "REP.8", "PTY.11")]
#[must_use]
pub fn regroup(
    kind: &PopKindDecl,
    layout: &ProfileLayout,
    key: &KeyRecord,
    before: &[Explicit],
    after: &[(KeyRecord, Explicit)],
) -> Regrouped {
    if before.len() != after.len() {
        violation!(clause = "REP.26", "households made explicit and returned of different numbers");
    }
    let mut in_place: Vec<(usize, u32, i64)> = Vec::new();
    let mut moving: BTreeMap<KeyRecord, Vec<usize>> = BTreeMap::new();
    let mut ended: Vec<usize> = Vec::new();
    for (i, (b, (k, a))) in before.iter().zip(after).enumerate() {
        if a.persons.is_empty() {
            ended.push(i);
            continue;
        }
        check(kind, k, a);
        if k == key {
            for (sign, e) in [(-1, b), (1, a)] {
                in_place.extend(e.persons.iter().flat_map(|p| p.values.iter().map(move |(g, v)| (*g, *v, sign))));
            }
        } else {
            moving.entry(*k).or_default().push(i);
        }
    }
    let group = |key: KeyRecord, at: &[usize]| Moved {
        key,
        households: count32(at.len()),
        before: profile_of(layout, at.iter().filter_map(|i| before.get(*i))),
        after: profile_of(layout, at.iter().filter_map(|i| after.get(*i).map(|(_, e)| e))),
        rows: rows_of(at.iter().filter_map(|i| after.get(*i).map(|(_, e)| e))),
    };
    let parts = moving.into_iter().map(|(k, at)| group(k, &at)).collect();
    let ended = (!ended.is_empty()).then(|| group(*key, &ended));
    Regrouped { in_place: net(in_place), parts, ended }
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{GroupDecl, KeyAttrDecl, KinkRegistry, PopEntry, PopItem, ProfileComponent, RoleCount, RoleDecl};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use phx_id::LineId;
    use phx_ledger::algebra::Side;

    use super::{Attached, Explicit, from_named, materialise, named, regroup, role_counts};
    use crate::key::KeyRecord;
    use crate::kind::PopKindDecl;
    use crate::profile::{Profile, ProfileLayout};
    use crate::steps::StepTable;

    const AGE: &[ProfileComponent] = &[ProfileComponent { name: "age", values: 4 }];
    const WORK: &[ProfileComponent] = &[ProfileComponent { name: "work", values: 3 }];

    /// Households of a head with an age and work, a partner (none or one) with an age, and children counted by the key.
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
            e(group("c_child_age", "child", AGE)),
        ];
        let steps = |_| StepTable::new(&Partition { exp: 0, bounds: [0, 1].into() });
        PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap()
    }

    const HEAD_AGE: usize = 0;
    const HEAD_WORK: usize = 1;
    const PARTNER_AGE: usize = 2;
    const CHILD_AGE: usize = 3;

    fn draws(i: u64) -> Draws {
        Draws::new(stream_key(Seed::new(5), "explicit"), Subject::new(SubjectTag::World, i), 0, 0)
    }

    fn key(kind: &PopKindDecl, partners: u32, children: u32) -> KeyRecord {
        let mut k = KeyRecord::default();
        for (name, v) in [("partners", partners), ("children", children)] {
            let at = kind.key_attrs.iter().position(|a| a.item.name == name).unwrap();
            kind.key.set(&mut k, at, v);
        }
        k
    }

    /// Ten households, each a head, a partner and two children.
    fn cell(kind: &PopKindDecl) -> (KeyRecord, Profile, ProfileLayout) {
        let layout = ProfileLayout::new(&kind.groups);
        let mut p = Profile::empty(&layout);
        for (g, v, n) in [
            (HEAD_AGE, 2, 6),
            (HEAD_AGE, 3, 4),
            (HEAD_WORK, 0, 3),
            (HEAD_WORK, 1, 7),
            (PARTNER_AGE, 2, 10),
            (CHILD_AGE, 0, 12),
            (CHILD_AGE, 1, 8),
        ] {
            p.add(&layout, g, v, n);
        }
        (key(kind, 1, 2), p, layout)
    }

    /// Households gathered by key make parts that weigh as many as they are, each group counting the persons its
    /// role's count gives every household; a household that does not hold its key's persons is refused.
    #[test]
    fn households_gather_by_key_into_parts() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let role = |name: &str| kind.roles.iter().position(|r| r.item.name == name).unwrap();
        let held =
            |role: usize, values: &[(usize, u32)]| super::Held { role, values: values.to_vec(), rows: Vec::new() };
        let household = |partner: bool, children: u32| {
            let mut persons = vec![held(role("head"), &[(HEAD_AGE, 2), (HEAD_WORK, 1)])];
            if partner {
                persons.push(held(role("partner"), &[(PARTNER_AGE, 2)]));
            }
            persons.extend((0..children).map(|c| held(role("child"), &[(CHILD_AGE, c % 2)])));
            (key(&kind, u32::from(partner), children), Explicit { persons, rows: Vec::new() })
        };
        let mut gathered = super::Gathered::default();
        let mut d = draws(9);
        let mut drawn_persons = 0_u64;
        for _ in 0..200 {
            let partner = phx_rand::below_u64(&mut d, 2) == 1;
            let (k, e) = household(partner, u32::try_from(phx_rand::below_u64(&mut d, 4)).unwrap());
            drawn_persons += super::wide(e.persons.len());
            gathered.add(&kind, &layout, k, &e, &[]);
        }
        let parts = gathered.into_drawn();
        assert_eq!(parts.iter().map(|p| u64::from(p.weight.get())).sum::<u64>(), 200);
        let mut persons = 0_u64;
        for part in &parts {
            let counts = role_counts(&kind, &part.key);
            for (g, group) in kind.groups.iter().enumerate() {
                let held: u64 = part.profile.held(g).iter().map(|(_, n)| u64::from(*n)).sum();
                assert_eq!(held, u64::from(part.weight.get()) * u64::from(counts[group.role]));
                persons += held * u64::from(g != HEAD_WORK);
            }
        }
        assert_eq!(persons, drawn_persons, "every person gathered once");
        let (k, e) = household(true, 2);
        let wrong = key(&kind, 0, 2);
        assert!(std::panic::catch_unwind(|| super::Gathered::default().add(&kind, &layout, wrong, &e, &[])).is_err());
        let mut short = e.clone();
        short.persons[0].values.pop();
        assert!(std::panic::catch_unwind(|| super::Gathered::default().add(&kind, &layout, k, &short, &[])).is_err());
    }

    #[test]
    fn a_household_holds_the_persons_its_key_counts() {
        let kind = kind();
        let counts = role_counts(&kind, &key(&kind, 1, 3));
        let of = |name: &str| counts[kind.roles.iter().position(|r| r.item.name == name).unwrap()];
        assert_eq!([of("head"), of("partner"), of("child")], [1, 1, 3]);
    }

    /// Every household made explicit holds its key's persons, from the cell's values; each hit's persons hold the
    /// values it reached, none twice within the hit.
    #[test]
    fn households_are_drawn_whole_from_the_cells_values() {
        let kind = kind();
        let (key, profile, layout) = cell(&kind);
        for i in 0..300 {
            let hits = vec![vec![(HEAD_AGE, 3, 2), (CHILD_AGE, 1, 3)], vec![(CHILD_AGE, 1, 2)]];
            let t = materialise(&kind, &key, 10, &profile, &hits, &mut draws(i));
            for e in &t.households {
                super::check(&kind, &key, e);
            }
            let drawn = super::profile_of(&layout, t.households.iter());
            for g in 0..kind.groups.len() {
                for (v, n) in drawn.held(g) {
                    assert!(*n <= profile.count(g, *v), "no more of a value than the cell holds");
                }
            }
            for (hit, got) in hits.iter().zip(&t.reached) {
                let mut seen = got.clone();
                seen.sort_unstable();
                seen.dedup();
                assert_eq!(seen.len(), got.len(), "a hit reaches a person once");
                let wanted: u64 = hit.iter().map(|(_, _, n)| n).sum();
                assert_eq!(super::wide(got.len()), wanted);
                for ((h, p), (g, v, _)) in got.iter().zip(hit.iter().flat_map(|x| (0..x.2).map(move |_| x))) {
                    assert_eq!(t.households[*h].persons[*p].value(*g), Some(*v));
                }
            }
        }
    }

    /// Two persons reached among two households of two persons each share a household with chance one in three, as
    /// drawing persons without replacement gives.
    #[test]
    fn two_reached_share_a_household_at_the_urns_rate() {
        let kind = kind();
        let layout = ProfileLayout::new(&kind.groups);
        let mut p = Profile::empty(&layout);
        for (g, v, n) in [(HEAD_AGE, 0, 2), (HEAD_WORK, 0, 2), (CHILD_AGE, 0, 4)] {
            p.add(&layout, g, v, n);
        }
        let key = key(&kind, 0, 2);
        let (mut shared, trials) = (0_u32, 20_000_u32);
        for i in 0..u64::from(trials) {
            let t = materialise(&kind, &key, 2, &p, &[vec![(CHILD_AGE, 0, 2)]], &mut draws(i));
            shared += u32::from(t.households.len() == 1);
        }
        let rate = f64::from(shared) / f64::from(trials);
        assert!((rate - 1.0 / 3.0).abs() < 0.015, "shared {rate}");
    }

    /// Named and back is the household itself; an outcome's changes come back as its values and key.
    #[test]
    fn a_household_named_and_read_back_is_itself() {
        let kind = kind();
        let (key, profile, _) = cell(&kind);
        let t = materialise(&kind, &key, 10, &profile, &[vec![(HEAD_AGE, 2, 1)]], &mut draws(1));
        let e = &t.households[0];
        assert_eq!(from_named(&kind, &named(&kind, &key, e)), (key, e.clone()));
    }

    /// A child's death splits its household out under a key counting one child fewer; a household no one is left in
    /// ends; a value changed under the cell's key returns in place; a key that does not count what is left is refused.
    #[test]
    fn outcomes_regroup_by_the_keys_households_become() {
        let kind = kind();
        let (cell_key, profile, layout) = cell(&kind);
        let persons = |p: &super::Profile| (0..kind.groups.len()).map(|g| p.members(g)).sum::<u64>();
        for i in 0..100 {
            let hits = vec![vec![(CHILD_AGE, 1, 1)], vec![(HEAD_AGE, 3, 1)], vec![(HEAD_WORK, 0, 1)]];
            let mut t = materialise(&kind, &cell_key, 10, &profile, &hits, &mut draws(i));
            let (deposit, job) = ((LineId::new(1), Side::Asset), (LineId::new(2), Side::Asset));
            for e in &mut t.households {
                e.rows.push(deposit);
                e.persons.iter_mut().filter(|p| p.role == 0).for_each(|p| p.rows.push(job));
            }
            let mut named: Vec<_> = t.households.iter().map(|e| named(&kind, &cell_key, e)).collect();
            let [child, head, worker] = [0, 1, 2].map(|h| t.reached[h][0]);
            named[worker.0].persons[worker.1].values[1].1 = 2;
            named[head.0].persons.iter_mut().for_each(|x| x.gone = true);
            if !named[child.0].persons[child.1].gone {
                named[child.0].persons[child.1].gone = true;
                named[child.0].set_attr("children", 1);
            }
            let carried: Vec<_> = named.iter().zip(&t.households).map(|(h, e)| super::carried(&kind, h, e)).collect();
            let left: Vec<Attached> = carried.iter().flat_map(|(_, _, l)| l.iter().copied()).collect();
            let after: Vec<_> = carried.into_iter().map(|(k, e, _)| (k, e)).collect();
            let r = regroup(&kind, &layout, &cell_key, &t.households, &after);
            let ended = r.ended.as_ref().unwrap();
            assert_eq!((ended.households, persons(&ended.after)), (1, 0), "the household no one is left in");
            assert_eq!(ended.rows, [(deposit, 1)], "its own rows for its estate");
            assert!(left.contains(&job), "the dead head's job leaves its line");
            for m in &r.parts {
                assert_eq!(m.key, key(&kind, 1, 1));
                assert_eq!(persons(&m.before) - persons(&m.after), u64::from(m.households), "one child each");
                assert_eq!(m.rows, [(deposit, m.households), (job, m.households)], "each part's own rows");
            }
            let back: i64 = r.in_place.iter().map(|(_, _, n)| n).sum();
            assert_eq!(back, 0, "a value changed in place neither adds nor removes persons");
        }
        let t = materialise(&kind, &cell_key, 10, &profile, &[vec![(CHILD_AGE, 0, 1)]], &mut draws(7));
        let mut short = named(&kind, &cell_key, &t.households[0]);
        short.persons[t.reached[0][0].1].gone = true;
        let wrong = vec![from_named(&kind, &short)];
        let refused = std::panic::catch_unwind(|| regroup(&kind, &layout, &cell_key, &t.households, &wrong));
        assert!(refused.is_err(), "a household its key does not count");
    }

    /// Everyone accounted for: whatever a random outcome does to random households, the persons drawn out are the
    /// persons returned in place, split out and gone.
    #[test]
    fn every_person_drawn_out_is_accounted_for() {
        let kind = kind();
        let (key, profile, layout) = cell(&kind);
        for i in 0..200 {
            let mut d = draws(1_000 + i);
            let hits = vec![vec![(CHILD_AGE, 0, 2), (PARTNER_AGE, 2, 1)], vec![(HEAD_AGE, 2, 2)]];
            let t = materialise(&kind, &key, 10, &profile, &hits, &mut d);
            let mut gone = 0_u64;
            let after: Vec<(KeyRecord, Explicit)> = t
                .households
                .iter()
                .map(|e| {
                    let mut h = named(&kind, &key, e);
                    let kids = phx_rand::below_u64(&mut d, 3);
                    let leaving: Vec<usize> = h
                        .present()
                        .filter(|(_, p)| p.role == "child")
                        .map(|(i, _)| i)
                        .take(usize::try_from(kids).unwrap())
                        .collect();
                    for p in &leaving {
                        h.persons[*p].gone = true;
                    }
                    gone += super::wide(leaving.len());
                    h.set_attr("children", 2 - u32::try_from(leaving.len()).unwrap());
                    from_named(&kind, &h)
                })
                .collect();
            let r = regroup(&kind, &layout, &key, &t.households, &after);
            let persons = |p: &Profile| (0..kind.groups.len()).map(|g| p.members(g)).sum::<u64>();
            let drawn = persons(&super::profile_of(&layout, t.households.iter()));
            let returned: i64 = r.in_place.iter().map(|(_, _, n)| n).sum();
            let split: u64 = r.parts.iter().map(|m| persons(&m.before)).sum();
            let split_after: u64 = r.parts.iter().map(|m| persons(&m.after)).sum();
            assert_eq!(returned, 0, "values changed in place neither add nor remove persons");
            assert_eq!(split - split_after, gone, "the persons gone left from the households split out");
            assert!(split <= drawn);
        }
    }

    /// Households drawn out of a real cell and moved to another key split out with every value given: the part holds
    /// them as they were drawn, and the cell keeps the rest of its persons and members.
    #[test]
    fn households_moved_split_out_with_every_value_given() {
        use crate::fixture::{AGE, books, cell, cells, draws as fixture_draws, keys};
        use crate::split::{Parted, SplitSpec, split};

        let kind = crate::fixture::kind();
        let mut space = phx_store::AddressSpace::empty();
        let mut bk = books(&mut space);
        let (ks, key) = keys(&kind, 1, 1);
        let (mut tab, slot) = cell(&mut space, &kind, &mut bk, &ks);
        let whole = tab.profile(slot);
        let layout = tab.profile_layout().clone();
        let t = materialise(&kind, &key, 100, &whole, &[vec![(AGE, 2, 5)]], &mut draws(9));
        let mut new = key;
        kind.key.set(&mut new, 0, 2);
        let after: Vec<(KeyRecord, Explicit)> = t.households.iter().map(|e| (new, e.clone())).collect();
        let r = regroup(&kind, &layout, &key, &t.households, &after);
        let moved = &r.parts[0];
        let given: Vec<Vec<(u32, u64)>> = (0..layout.groups.len())
            .map(|g| moved.before.held(g).iter().map(|(v, n)| (*v, u64::from(*n))).collect())
            .collect();
        let given: Vec<(usize, &[(u32, u64)])> = given.iter().enumerate().map(|(g, v)| (g, &v[..])).collect();
        let spec = SplitSpec {
            count: moved.households,
            given: &given,
            rows: &[],
            own: &[],
            reviewed: phx_num::Missing::Absent,
            rounding: phx_num::round::Round::HalfEven,
        };
        let id = crate::part::PartId { origin: tab.party(slot), seq: 0 };
        let Parted::Part(part) = split(&mut cells(&mut bk, &mut tab, &ks), slot, id, &spec, &mut fixture_draws("x", 1))
        else {
            panic!("five of a hundred households leave as a part")
        };
        assert_eq!(part.weight.get(), moved.households);
        assert_eq!(part.profile, moved.before, "the part holds its households as they were drawn");
        let left = tab.profile(slot);
        for g in 0..layout.groups.len() {
            for (v, n) in whole.held(g) {
                assert_eq!(left.count(g, *v) + part.profile.count(g, *v), *n, "every person in one place");
            }
        }
        assert_eq!(tab.weight(slot).get() + moved.households, 100);
    }
}
