use std::borrow::Cow;

use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{DENSE_MAX_VALUES, VARINT_BITS, VARINT_MORE};
use phx_num::Missing;

use crate::key::{KeyField, KeyRecord};
use crate::kind::Group;

/// How one profile group is kept: its role, the key field counting that role's persons in each member, how many
/// joint values it takes, and whether as a dense histogram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupShape {
    pub role: usize,
    pub per_member: Missing<KeyField>,
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
            .map(|g| GroupShape {
                role: g.role,
                per_member: g.per_member,
                values: g.values,
                dense: g.values <= DENSE_MAX_VALUES,
            })
            .collect();
        ProfileLayout { groups: shapes }
    }

    /// The persons of a group's role that `members` members of a key hold: one each, or the key's count of them.
    #[clause("REP.14", "REP.26")]
    #[must_use]
    pub fn persons(&self, group: usize, key: &KeyRecord, members: u64) -> u64 {
        let Some(shape) = self.groups.get(group) else {
            violation!(clause = "REP.32", "a profile group the kind does not hold", group = group);
        };
        match shape.per_member {
            Missing::Absent => members,
            Missing::Present(f) => {
                let Some(n) = members.checked_mul(u64::from(crate::key::read(&f, key))) else {
                    capacity_exceeded!("persons of a role", u64::MAX, members);
                };
                n
            }
        }
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
        Profile::read_from(layout, &bytes, bytes.len())
    }

    /// A profile read back from `len` bytes of a source, in place.
    ///
    /// # Errors
    /// Bytes that end early, run on, or hold a value outside its group.
    pub fn read_from<S: ByteSource>(layout: &ProfileLayout, bytes: &S, len: usize) -> Result<Profile, String> {
        let mut at = 0;
        let mut groups = Vec::with_capacity(layout.groups.len());
        for shape in &layout.groups {
            let mut g = Vec::new();
            group(shape, bytes, &mut at, Some(&mut g))?;
            groups.push(g);
        }
        if at != len {
            return Err("a profile's bytes run on past its groups".to_owned());
        }
        Ok(Profile { groups })
    }
}

/// Bytes read one at a time, from a slice or from words packed low byte first.
pub trait ByteSource {
    fn byte(&self, at: usize) -> Option<u8>;
}

impl ByteSource for &[u8] {
    fn byte(&self, at: usize) -> Option<u8> {
        self.get(at).copied()
    }
}

/// The first `len` bytes packed into words, read in place.
#[derive(Clone, Copy, Debug)]
pub struct WordBytes<'a> {
    pub words: &'a [u64],
    pub len: usize,
}

impl ByteSource for WordBytes<'_> {
    fn byte(&self, at: usize) -> Option<u8> {
        if at >= self.len {
            return None;
        }
        let w = self.words.get(at / size_of::<u64>())?;
        let shift = (at % size_of::<u64>()) * usize::try_from(u8::BITS).ok()?;
        (w >> shift).to_le_bytes().first().copied()
    }
}

/// One group's values held read from `at`, into `out` where one is given and passed over otherwise.
fn group<S: ByteSource + ?Sized>(
    shape: &GroupShape,
    bytes: &S,
    at: &mut usize,
    mut out: Option<&mut Vec<(u32, u32)>>,
) -> Result<(), String> {
    if shape.dense {
        for v in 0..shape.values {
            let n = read(bytes, at)?;
            if let Some(o) = out.as_deref_mut()
                && n > 0
            {
                o.push((v, n));
            }
        }
    } else {
        let held = read(bytes, at)?;
        if let (Some(o), Ok(n)) = (out.as_deref_mut(), usize::try_from(held)) {
            o.reserve(n);
        }
        let mut last: u32 = 0;
        for i in 0..held {
            let gap = read(bytes, at)?;
            let v = last.checked_add(gap).ok_or("a profile value past its group")?;
            if v >= shape.values || (i > 0 && gap == 0) {
                return Err(format!("profile value {v} out of order or outside a group of {}", shape.values));
            }
            let n = read(bytes, at)?;
            if let Some(o) = out.as_deref_mut() {
                o.push((v, n));
            }
            last = v;
        }
    }
    Ok(())
}

/// A profile's bytes with members moved at joint values, read and written in one pass: `deltas` in (group, value)
/// order, each value's count moved by its delta; a value nobody holds any more is dropped and a new one is written in
/// its place in order.
///
/// # Errors
/// Bytes that end early, run on or hold a value outside its group; a delta outside the groups, out of order, or taking
/// more members from a value than hold it.
pub fn shifted<S: ByteSource + ?Sized>(
    layout: &ProfileLayout,
    bytes: &S,
    len: usize,
    deltas: &[(usize, u32, i64)],
) -> Result<Vec<u8>, String> {
    if deltas.windows(2).any(|w| matches!(w, [a, b] if (a.0, a.1) >= (b.0, b.1))) {
        return Err("profile deltas out of order".to_owned());
    }
    let mut at = 0;
    let mut out = Vec::with_capacity(len + deltas.len() * 2);
    let mut body = Vec::new();
    let mut rest = deltas;
    for (gi, shape) in layout.groups.iter().enumerate() {
        let here = rest.iter().take_while(|(g, _, _)| *g == gi).count();
        let (mine, later) = rest.split_at(here);
        rest = later;
        if let Some((_, v, _)) = mine.iter().find(|(_, v, _)| *v >= shape.values) {
            return Err(format!("profile value {v} outside a group of {}", shape.values));
        }
        let mut pending = mine.iter().peekable();
        if shape.dense {
            for v in 0..shape.values {
                let mut n = i64::from(read(bytes, &mut at)?);
                if let Some((_, _, d)) = pending.next_if(|(_, dv, _)| *dv == v) {
                    n += d;
                }
                varint(&mut out, moved(n)?);
            }
        } else {
            let held = read(bytes, &mut at)?;
            body.clear();
            let (mut count, mut last_in, mut last_out) = (0_u32, 0_u32, 0_u32);
            let emit = |body: &mut Vec<u8>, v: u32, n: u32, last_out: &mut u32, count: &mut u32| {
                if n > 0 {
                    varint(body, v - *last_out);
                    varint(body, n);
                    *last_out = v;
                    *count += 1;
                }
            };
            for i in 0..held {
                let gap = read(bytes, &mut at)?;
                let v = last_in.checked_add(gap).ok_or("a profile value past its group")?;
                if v >= shape.values || (i > 0 && gap == 0) {
                    return Err(format!("profile value {v} out of order or outside a group of {}", shape.values));
                }
                let n = read(bytes, &mut at)?;
                last_in = v;
                while let Some((_, dv, d)) = pending.next_if(|(_, dv, _)| *dv < v) {
                    emit(&mut body, *dv, moved(*d)?, &mut last_out, &mut count);
                }
                let mut n = i64::from(n);
                if let Some((_, _, d)) = pending.next_if(|(_, dv, _)| *dv == v) {
                    n += d;
                }
                emit(&mut body, v, moved(n)?, &mut last_out, &mut count);
            }
            for (_, dv, d) in pending {
                emit(&mut body, *dv, moved(*d)?, &mut last_out, &mut count);
            }
            varint(&mut out, count);
            out.extend_from_slice(&body);
        }
    }
    if !rest.is_empty() {
        return Err("a profile delta outside the kind's groups".to_owned());
    }
    if at != len {
        return Err("a profile's bytes run on past its groups".to_owned());
    }
    Ok(out)
}

/// Profile moves in (group, value) order, those at one value added together, as `shifted` reads them.
#[must_use]
pub fn net(mut deltas: Vec<(usize, u32, i64)>) -> Vec<(usize, u32, i64)> {
    deltas.sort_unstable_by_key(|(g, v, _)| (*g, *v));
    deltas.dedup_by(|later, first| {
        let same = (later.0, later.1) == (first.0, first.1);
        if same {
            first.2 += later.2;
        }
        same
    });
    deltas
}

/// A count after its members moved: never below nought, and within a count's width.
fn moved(n: i64) -> Result<u32, String> {
    u32::try_from(n).map_err(|_| format!("a profile value's members moved to {n}"))
}

/// One group's values held, read from a profile's bytes without decoding the groups around it.
///
/// # Errors
/// Bytes that end early or hold a value outside its group.
pub fn read_group<S: ByteSource + ?Sized>(
    layout: &ProfileLayout,
    bytes: &S,
    which: usize,
) -> Result<Vec<(u32, u32)>, String> {
    let mut at = 0;
    for shape in layout.groups.iter().take(which) {
        group(shape, bytes, &mut at, None)?;
    }
    let Some(shape) = layout.groups.get(which) else {
        return Err(format!("profile group {which} outside the kind's groups"));
    };
    let mut out = Vec::new();
    group(shape, bytes, &mut at, Some(&mut out))?;
    Ok(out)
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

fn read<S: ByteSource + ?Sized>(bytes: &S, at: &mut usize) -> Result<u32, String> {
    let mut n: u64 = 0;
    let mut shift = 0;
    loop {
        let Some(b) = bytes.byte(*at) else { return Err("a profile's bytes end early".to_owned()) };
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

/// The first `len` bytes of words packed low byte first: the words' own memory where the machine keeps a word's low byte
/// first, so a profile is read in place; a copy otherwise.
#[must_use]
pub fn packed_bytes(words: &[u64], len: usize) -> Cow<'_, [u8]> {
    if cfg!(target_endian = "little")
        && let Some(b) = phx_store::as_bytes(words).get(..len)
    {
        return Cow::Borrowed(b);
    }
    Cow::Owned(from_words(words, len))
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
                GroupShape { role: 0, per_member: phx_num::Missing::Absent, values: 12, dense: true },
                GroupShape { role: 0, per_member: phx_num::Missing::Absent, values: 40_000, dense: false },
                GroupShape { role: 1, per_member: phx_num::Missing::Absent, values: 3, dense: true },
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
        let in_place = super::WordBytes { words: &words, len: bytes.len() };
        for g in 0..3 {
            assert_eq!(super::read_group(&l, &in_place, g).as_deref(), Ok(p.held(g)), "group {g} read alone");
        }
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

    #[test]
    fn shifted_equals_decode_edit_encode() {
        let l = layout();
        let mut d = Draws::new(stream_key(Seed::new(9), "profile.shift"), Subject::new(SubjectTag::World, 0), 0, 0);
        for _ in 0..200 {
            let mut p = Profile::empty(&l);
            for (g, shape) in l.groups.iter().enumerate() {
                for _ in 0..below_u64(&mut d, 12) {
                    let v = u32::try_from(below_u64(&mut d, u64::from(shape.values))).unwrap();
                    p.add(&l, g, v, u32::try_from(below_u64(&mut d, 300)).unwrap() + 1);
                }
            }
            let mut deltas = Vec::new();
            let mut edited = p.clone();
            for (g, shape) in l.groups.iter().enumerate() {
                for v in 0..shape.values {
                    if below_u64(&mut d, 4) != 0 {
                        continue;
                    }
                    let held = i64::from(p.count(g, v));
                    // Members added, or taken away up to all that hold the value.
                    let taken = i64::try_from(below_u64(&mut d, u64::try_from(held).unwrap() + 1)).unwrap();
                    let delta = if below_u64(&mut d, 2) == 0 { taken } else { -taken };
                    if delta == 0 {
                        continue;
                    }
                    if delta > 0 {
                        edited.add(&l, g, v, u32::try_from(delta).unwrap());
                    } else {
                        edited.remove(g, v, u32::try_from(-delta).unwrap());
                    }
                    deltas.push((g, v, delta));
                }
            }
            let bytes = p.encode(&l);
            let got = super::shifted(&l, &bytes.as_slice(), bytes.len(), &deltas).unwrap();
            assert_eq!(got, edited.encode(&l));
        }
        let p = Profile::empty(&l);
        let bytes = p.encode(&l);
        assert!(super::shifted(&l, &bytes.as_slice(), bytes.len(), &[(1, 3, -1)]).is_err(), "members nobody holds");
        assert!(super::shifted(&l, &bytes.as_slice(), bytes.len(), &[(1, 3, 1), (0, 2, 1)]).is_err(), "out of order");
    }
}
