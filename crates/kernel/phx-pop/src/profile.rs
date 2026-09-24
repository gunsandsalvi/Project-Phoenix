use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{DENSE_MAX_VALUES, VARINT_BITS, VARINT_MORE};
use crate::kind::Group;

/// How one profile group is kept: how many joint values it takes, and whether as a dense histogram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupShape {
    pub role: usize,
    pub values: u32,
    pub dense: bool,
}

/// The shapes of a kind's profile groups, in the kind's group order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileLayout {
    pub groups: Vec<GroupShape>,
}

impl ProfileLayout {
    /// Each group dense when its joint values are few, sparse otherwise.
    #[must_use]
    pub fn new(groups: &[Group]) -> ProfileLayout {
        let shapes = groups
            .iter()
            .map(|g| GroupShape { role: g.role, values: g.values, dense: g.values <= DENSE_MAX_VALUES })
            .collect();
        ProfileLayout { groups: shapes }
    }
}

/// A cell's profiles: for each group, how many members of its role hold each joint value, the values held in order
/// and none held by nobody.
#[clause("REP.32", "REP.14")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    groups: Vec<Vec<(u32, u32)>>,
}

impl Profile {
    /// A profile of no members, for a kind's groups.
    #[must_use]
    pub fn empty(layout: &ProfileLayout) -> Profile {
        Profile { groups: layout.groups.iter().map(|_| Vec::new()).collect() }
    }

    fn group(&self, group: usize) -> &Vec<(u32, u32)> {
        let Some(g) = self.groups.get(group) else {
            violation!(clause = "REP.32", "a profile group the kind does not declare", group = group);
        };
        g
    }

    fn group_mut(&mut self, group: usize) -> &mut Vec<(u32, u32)> {
        let Some(g) = self.groups.get_mut(group) else {
            violation!(clause = "REP.32", "a profile group the kind does not declare", group = group);
        };
        g
    }

    /// Members of the group's role holding a joint value.
    #[must_use]
    pub fn count(&self, group: usize, value: u32) -> u32 {
        let g = self.group(group);
        match g.binary_search_by_key(&value, |(v, _)| *v) {
            Ok(i) => g.get(i).map_or(0, |(_, n)| *n),
            Err(_) => 0,
        }
    }

    /// Members counted in a group: its role's members, which must be the cell's weight.
    #[must_use]
    pub fn members(&self, group: usize) -> u64 {
        self.group(group).iter().map(|(_, n)| u64::from(*n)).sum()
    }

    /// Joint values held, over every group.
    #[must_use]
    pub fn entries(&self) -> usize {
        self.groups.iter().map(Vec::len).sum()
    }

    /// The values held in a group, with their counts, in value order.
    #[must_use]
    pub fn held(&self, group: usize) -> &[(u32, u32)] {
        self.group(group)
    }

    /// Members added holding a joint value, which must be one of the group's.
    pub fn add(&mut self, layout: &ProfileLayout, group: usize, value: u32, members: u32) {
        let Some(shape) = layout.groups.get(group) else {
            violation!(clause = "REP.32", "a profile group the kind does not declare", group = group);
        };
        if value >= shape.values {
            capacity_exceeded!("joint values of a profile group", shape.values, value);
        }
        if members == 0 {
            return;
        }
        let g = self.group_mut(group);
        match g.binary_search_by_key(&value, |(v, _)| *v) {
            Ok(i) => {
                if let Some((_, n)) = g.get_mut(i) {
                    let Some(sum) = n.checked_add(members) else {
                        capacity_exceeded!("members holding a profile value", u32::MAX, u64::from(*n) + 1);
                    };
                    *n = sum;
                }
            }
            Err(i) => g.insert(i, (value, members)),
        }
    }

    /// Members removed from a joint value; more than hold it stops the run.
    pub fn remove(&mut self, group: usize, value: u32, members: u32) {
        let g = self.group_mut(group);
        let Ok(i) = g.binary_search_by_key(&value, |(v, _)| *v) else {
            violation!(clause = "REP.14", "members removed from a profile value nobody holds", value = value);
        };
        let Some((_, n)) = g.get_mut(i) else { return };
        let Some(rest) = n.checked_sub(members) else {
            violation!(clause = "REP.14", "more members removed from a profile value than hold it", value = value);
        };
        *n = rest;
        if rest == 0 {
            g.remove(i);
        }
    }

    /// The profile as bytes: per group, a dense histogram of a count per value, or the number of values held and
    /// each value's gap from the last with its count; every number a varint.
    #[must_use]
    pub fn encode(&self, layout: &ProfileLayout) -> Vec<u8> {
        let mut out = Vec::new();
        for (shape, g) in layout.groups.iter().zip(&self.groups) {
            if shape.dense {
                let mut held = g.iter().peekable();
                for v in 0..shape.values {
                    let n = match held.peek() {
                        Some((hv, n)) if *hv == v => {
                            let n = *n;
                            held.next();
                            n
                        }
                        _ => 0,
                    };
                    varint(&mut out, n);
                }
            } else {
                varint(&mut out, len32(g.len()));
                let mut last = 0;
                for (v, n) in g {
                    varint(&mut out, v - last);
                    varint(&mut out, *n);
                    last = *v;
                }
            }
        }
        out
    }

    /// A profile read back from its bytes.
    ///
    /// # Errors
    /// Bytes that end early, run on, or hold a value outside its group.
    pub fn decode(layout: &ProfileLayout, bytes: &[u8]) -> Result<Profile, String> {
        let mut at = 0;
        let mut groups = Vec::with_capacity(layout.groups.len());
        for shape in &layout.groups {
            let mut g = Vec::new();
            if shape.dense {
                for v in 0..shape.values {
                    let n = read(bytes, &mut at)?;
                    if n > 0 {
                        g.push((v, n));
                    }
                }
            } else {
                let held = read(bytes, &mut at)?;
                let mut last: u32 = 0;
                for i in 0..held {
                    let gap = read(bytes, &mut at)?;
                    let v = last.checked_add(gap).ok_or("a profile value past its group")?;
                    if v >= shape.values || (i > 0 && gap == 0) {
                        return Err(format!("profile value {v} out of order or outside a group of {}", shape.values));
                    }
                    g.push((v, read(bytes, &mut at)?));
                    last = v;
                }
            }
            groups.push(g);
        }
        if at != bytes.len() {
            return Err("a profile's bytes run on past its groups".to_owned());
        }
        Ok(Profile { groups })
    }
}

fn len32(n: usize) -> u32 {
    let Ok(v) = u32::try_from(n) else {
        capacity_exceeded!("values held in a profile group", u32::MAX, n);
    };
    v
}

/// A number seven bits to a byte, low bits first, each byte but the last marked.
fn varint(out: &mut Vec<u8>, mut n: u32) {
    loop {
        let low = n.to_le_bytes()[0] & !VARINT_MORE;
        n >>= VARINT_BITS;
        if n == 0 {
            out.push(low);
            return;
        }
        out.push(low | VARINT_MORE);
    }
}

fn read(bytes: &[u8], at: &mut usize) -> Result<u32, String> {
    let mut n: u64 = 0;
    let mut shift = 0;
    loop {
        let Some(b) = bytes.get(*at) else { return Err("a profile's bytes end early".to_owned()) };
        *at += 1;
        n |= u64::from(b & !VARINT_MORE) << shift;
        if b & VARINT_MORE == 0 {
            return u32::try_from(n).map_err(|_| "a profile count past its width".to_owned());
        }
        shift += VARINT_BITS;
        if shift >= u32::BITS + VARINT_BITS {
            return Err("a profile count past its width".to_owned());
        }
    }
}

/// Bytes packed into words, low byte first, the last word padded with noughts; its byte count is kept apart.
#[must_use]
pub fn to_words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks(size_of::<u64>())
        .map(|c| {
            let mut w = [0_u8; size_of::<u64>()];
            if let Some(dst) = w.get_mut(..c.len()) {
                dst.copy_from_slice(c);
            }
            u64::from_le_bytes(w)
        })
        .collect()
}

/// The first `n` bytes packed into words.
#[must_use]
pub fn from_words(words: &[u64], n: usize) -> Vec<u8> {
    let mut out: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{GroupShape, Profile, ProfileLayout, from_words, to_words};

    fn layout() -> ProfileLayout {
        ProfileLayout {
            groups: vec![
                GroupShape { role: 0, values: 12, dense: true },
                GroupShape { role: 0, values: 40_000, dense: false },
                GroupShape { role: 1, values: 3, dense: true },
            ],
        }
    }

    #[test]
    fn profile_encoding_roundtrip() {
        let l = layout();
        let mut p = Profile::empty(&l);
        for (g, v, n) in [(0, 11, 3), (0, 0, 200), (1, 39_999, 5), (1, 7, 1), (1, 300, 70_000), (2, 1, 2)] {
            p.add(&l, g, v, n);
        }
        let bytes = p.encode(&l);
        assert_eq!(Profile::decode(&l, &bytes), Ok(p.clone()));
        let words = to_words(&bytes);
        assert_eq!(Profile::decode(&l, &from_words(&words, bytes.len())), Ok(p.clone()), "through words");
        assert_eq!(
            Profile::decode(&l, &bytes[..bytes.len() - 1]).map(|_| ()),
            Err("a profile's bytes end early".to_owned())
        );
        let mut long = bytes;
        long.push(0);
        assert!(Profile::decode(&l, &long).is_err(), "bytes past the groups");
        let empty = Profile::empty(&l);
        assert_eq!(empty.encode(&l).len(), 12 + 1 + 3, "a byte per dense value and one for an empty sparse group");
        assert!(std::panic::catch_unwind(|| Profile::empty(&l).add(&l, 2, 3, 1)).is_err(), "a value past its group");
    }

    #[test]
    fn profile_sum_equals_role_count() {
        let shapes = layout();
        let mut draws = Draws::new(stream_key(Seed::new(3), "profiles"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut prof = Profile::empty(&shapes);
        let weight = 500_u32;
        for g in 0..shapes.groups.len() {
            prof.add(&shapes, g, 0, weight);
        }
        for _ in 0..5_000 {
            let g = usize::try_from(below_u64(&mut draws, 3)).unwrap();
            let values = shapes.groups[g].values;
            let held: Vec<(u32, u32)> = prof.held(g).to_vec();
            let (from, n) = held[usize::try_from(below_u64(&mut draws, phx_rand::float::len_u64(held.len()))).unwrap()];
            let moved = u32::try_from(below_u64(&mut draws, u64::from(n))).unwrap() + 1;
            let to = u32::try_from(below_u64(&mut draws, u64::from(values))).unwrap();
            prof.remove(g, from, moved);
            prof.add(&shapes, g, to, moved);
            for g in 0..3 {
                assert_eq!(prof.members(g), u64::from(weight), "each group counts its role's members");
            }
        }
        assert_eq!(Profile::decode(&shapes, &prof.encode(&shapes)), Ok(prof.clone()));
        assert!(std::panic::catch_unwind(move || prof.remove(2, 2, weight + 1)).is_err(), "more than hold a value");
    }
}
