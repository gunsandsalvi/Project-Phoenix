//! What a process's outcome does to a cell's members, as the owning system declared it: a profile value changed in
//! place, or a part split out and reshaped before it lands.

use phx_core::GroupMove;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Backing;

use crate::key::KeyRecord;
use crate::kind::PopKindDecl;
use crate::part::Part;
use crate::profile::{Profile, ProfileLayout};
use crate::table::CellTable;

fn members(n: u64) -> u32 {
    let Ok(m) = u32::try_from(n) else {
        capacity_exceeded!("members of a cell", u32::MAX, n);
    };
    m
}

fn signed(n: u64) -> i64 {
    let Ok(m) = i64::try_from(n) else {
        capacity_exceeded!("members of a cell", i64::MAX, n);
    };
    m
}

/// A group of the kind by its name.
#[must_use]
pub fn group(kind: &PopKindDecl, name: &str) -> Option<usize> {
    kind.groups.iter().position(|g| g.name == name)
}

/// Members holding one value of a group in a cell take another in place; nothing else about them changes, so none
/// leaves the cell and no total moves.
#[clause("REP.14", "REP.26")]
pub fn revalue<B: Backing>(table: &mut CellTable<B>, slot: Slot, group: usize, from: u32, to: u32, count: u64) {
    if from == to || count == 0 {
        return;
    }
    let held = table.profile_group(slot, group).iter().find(|(v, _)| *v == from).map_or(0, |(_, n)| *n);
    if u64::from(held) < count {
        violation!(clause = "REP.14", "more members revalued than hold the value", held = held, count = count);
    }
    let by = signed(count);
    table.shift_profile(slot, &[(group, from, -by), (group, to, by)]);
}

/// A profile's members of a hit value in the group given `to`, then each move applied: a group given the values
/// another held, that other left at a value. `n` is the members the profile counts in every group.
fn reshape_profile(
    profile: &mut Profile,
    kind: &PopKindDecl,
    layout: &ProfileLayout,
    n: u32,
    hit: (usize, u32, u32),
    moves: &[GroupMove],
) {
    let (g, from, to) = hit;
    if from != to {
        profile.remove(g, from, n);
        profile.add(layout, g, to, n);
    }
    for m in moves {
        let (Some(a), Some(b)) = (group(kind, m.from), group(kind, m.to)) else {
            violation!(clause = "REP.26", "a move between groups the kind does not hold");
        };
        let given: Vec<(u32, u32)> = profile.held(a).to_vec();
        let taken: Vec<(u32, u32)> = profile.held(b).to_vec();
        for (v, k) in taken {
            profile.remove(b, v, k);
        }
        for (v, k) in given {
            profile.remove(a, v, k);
            profile.add(layout, b, v, k);
        }
        profile.add(layout, a, m.left, n);
    }
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

/// A part split out by an outcome, reshaped as it says: its hit value in the group becomes `to`; each move gives a
/// group the values another held, leaving that other at a value; and each named key attribute takes its value. Its
/// weight, totals, rows and holdings are the members' own and do not change.
#[clause("REP.26", "REP.14", "REP.19")]
pub fn reshape(
    part: &mut Part,
    kind: &PopKindDecl,
    layout: &ProfileLayout,
    hit: (usize, u32, u32),
    moves: &[GroupMove],
    key: &[(&str, u32)],
) {
    let n = members(part.weight.get().into());
    reshape_profile(&mut part.profile, kind, layout, n, hit, moves);
    part.key = rekeyed(kind, part.key, key);
}

/// A cell every member of which an outcome reached, reshaped in place as a part would be; its key is the caller's to
/// set, so the cell keeps its identity and re-keys at 10b.
#[clause("REP.26", "REP.14")]
pub fn reshape_cell<B: Backing>(
    table: &mut CellTable<B>,
    slot: Slot,
    kind: &PopKindDecl,
    hit: (usize, u32, u32),
    moves: &[GroupMove],
) {
    let layout = table.profile_layout().clone();
    let n = table.weight(slot).get();
    let mut profile = table.profile(slot);
    reshape_profile(&mut profile, kind, &layout, n, hit, moves);
    table.set_profile(slot, &profile);
}

#[cfg(test)]
mod tests {
    use phx_core::{GroupMove, Weight};

    use super::{reshape, revalue};
    use crate::fixture::{PLAIN, Ten};

    #[test]
    fn revalue_moves_counts_in_place() {
        let mut ten = Ten::new();
        let (_, slot) = ten.add(1, PLAIN);
        let g = 0;
        let (v, n) = ten.table.profile_group(slot, g)[0];
        let to = u32::from(v == 0);
        revalue(&mut ten.table, slot, g, v, to, 1);
        let after = ten.table.profile_group(slot, g);
        let at = |x: u32| after.iter().find(|(y, _)| *y == x).map_or(0, |(_, k)| *k);
        assert_eq!(at(v), n - 1);
        assert!(at(to) >= 1);
        assert_eq!(ten.table.profile(slot).members(g), u64::from(ten.table.weight(slot).get()));
    }

    #[test]
    fn reshape_moves_a_group_and_leaves_the_other() {
        let mut ten = Ten::new();
        let (_, slot) = ten.add(1, PLAIN);
        let mut part = ten.split(slot, 0, 3, "DEM.death");
        let layout = ten.table.profile_layout().clone();
        let kind = ten.kind.clone();
        assert!(kind.groups.len() >= 2, "the fixture's kind holds two groups");
        let (a, b) = (kind.groups[0].name, kind.groups[1].name);
        let before_a: Vec<(u32, u32)> = part.profile.held(0).to_vec();
        reshape(
            &mut part,
            &kind,
            &layout,
            (0, before_a[0].0, before_a[0].0),
            &[GroupMove { from: a, to: b, left: 0 }],
            &[],
        );
        assert_eq!(part.weight, Weight::new(3));
        let moved: u32 = part.profile.held(1).iter().map(|(_, k)| *k).sum();
        assert_eq!(moved, 3, "the second group holds what the first held");
        assert_eq!(part.profile.held(0), &[(0, 3)], "the first is left at its value");
    }
}
