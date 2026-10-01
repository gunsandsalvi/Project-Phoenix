//! Routes of one mode between places — the regions' market zones over the trunks between them, or a region's zones over
//! its own segments: a dense table over every (origin, destination) pair of its places, each entry a run of segment
//! identities in one arena and the route's length, found by two reads; recomputed whole, by the shortest paths over
//! the open segments joining its places, when one of them closes or opens.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use phx_id::Day;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

use super::segments::{SegmentId, SegmentRow, Segments};

/// No place: a zone that is none of the table's places, or a pair no path joins.
const NONE: u32 = u32::MAX;
/// No place of the table, as a zone's place reads.
const NO_PLACE: u16 = u16::MAX;

/// A route's run in the arena and its length; a pair no path joins holds `NONE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    offset: u32,
    len: u32,
    metres: u64,
}

const UNJOINED: Entry = Entry { offset: NONE, len: 0, metres: 0 };

/// A route: the segments it runs over, in order, and its length in metres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route<'a> {
    pub segments: &'a [u32],
    pub metres: u64,
}

/// The shortest paths' scratch, kept from one recompute to the next so a recompute allocates nothing it had before.
#[derive(Clone, Debug, Default)]
struct Scratch {
    starts: Vec<u32>,
    edges: Vec<(u32, u64, u32)>,
    best: Vec<u64>,
    via: Vec<(u32, u32)>,
    heap: BinaryHeap<Reverse<(u64, u32)>>,
    path: Vec<u32>,
}

/// One mode's routes between its places.
#[clause("FRT.2", "GEO.4")]
#[derive(Clone, Debug, Default)]
pub struct Routes {
    mode: u8,
    places: Vec<u16>,
    place_of: Vec<u16>,
    /// The segments of the mode that join two of its places, by identity, whatever their state.
    members: Vec<u32>,
    entries: Vec<Entry>,
    arena: Vec<u32>,
    scratch: Scratch,
}

impl PartialEq for Routes {
    /// Two tables alike in their routes, whatever their scratch holds.
    fn eq(&self, other: &Routes) -> bool {
        (self.mode, &self.places, &self.place_of, &self.members, &self.entries, &self.arena)
            == (other.mode, &other.places, &other.place_of, &other.members, &other.entries, &other.arena)
    }
}

impl Eq for Routes {}

fn index(n: u64) -> usize {
    match usize::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("a route table", usize::MAX, n),
    }
}

fn narrow(n: usize) -> u32 {
    match u32::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("a route table", u32::MAX, n),
    }
}

impl Routes {
    /// A mode's table over its places among `zones` zones, each pair unjoined until recomputed.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(mode: u8, places: &[u16], zones: u32) -> Routes {
        let mut place_of = vec![NO_PLACE; index(u64::from(zones))];
        for (i, z) in places.iter().enumerate() {
            let Some(place) = u16::try_from(i).ok().filter(|p| *p != NO_PLACE) else {
                capacity_exceeded!("a route table's places", NO_PLACE, i);
            };
            match place_of.get_mut(usize::from(*z)) {
                Some(p) => *p = place,
                None => violation!(clause = "GEO.4", "a place past the map's zones", zone = *z),
            }
        }
        let pairs = places.len() * places.len();
        Routes {
            mode,
            places: places.to_vec(),
            place_of,
            members: Vec::new(),
            entries: vec![UNJOINED; pairs],
            arena: Vec::new(),
            scratch: Scratch::default(),
        }
    }

    /// Every segment of the mode joining two of its places made one of the table's, once at its making.
    pub fn admit_all(&mut self, segments: &Segments) {
        for (id, row) in segments.iter() {
            self.admit(id, row);
        }
        self.members.shrink_to_fit();
    }

    /// A segment made one of the table's if it is of its mode and joins two of its places.
    pub fn admit(&mut self, id: SegmentId, row: SegmentRow) {
        let placed = |z: u16| self.place_of.get(usize::from(z)).is_some_and(|p| *p != NO_PLACE);
        let (a, b) = row.ends();
        if row.mode() == self.mode && placed(a) && placed(b) {
            self.members.push(id.get());
        }
    }

    /// The route from one place to another, or `Missing` where no open path of the mode joins them; a zone that is
    /// none of the table's places stops the run.
    pub fn route(&self, from: u16, to: u16) -> Missing<Route<'_>> {
        let place = |z: u16| match self.place_of.get(usize::from(z)) {
            Some(p) if *p != NO_PLACE => usize::from(*p),
            _ => violation!(clause = "FRT.2", "a route asked of a zone the table does not hold", zone = z),
        };
        let at = place(from) * self.places.len() + place(to);
        match self.entries.get(at) {
            Some(e) if e.offset != NONE => {
                let start = index(u64::from(e.offset));
                match self.arena.get(start..start + index(u64::from(e.len))) {
                    Some(segments) => Missing::Present(Route { segments, metres: e.metres }),
                    None => violation!(clause = "FRT.2", "a route past its arena", from = from, to = to),
                }
            }
            Some(_) => Missing::Absent,
            None => violation!(clause = "FRT.2", "a route past its table", from = from, to = to),
        }
    }

    /// Every route recomputed over the mode's segments open on a day that join two of the table's places: from each
    /// place, the shortest path to every other over them, either way along each, the lower zone first among equals.
    /// It returns the segment rows it visited.
    #[clause("FRT.2", "FRT.7", "GEO.8")]
    pub fn recompute(&mut self, segments: &Segments, day: Day) -> u64 {
        let zones = self.place_of.len();
        let s = &mut self.scratch;
        s.starts.clear();
        s.starts.resize(zones + 1, 0);
        let members = &self.members;
        let open = || members.iter().map(|m| (*m, segments.row(SegmentId::new(*m)))).filter(|(_, r)| r.open_on(day));
        for (_, r) in open() {
            let (a, b) = r.ends();
            for z in [a, b] {
                if let Some(n) = s.starts.get_mut(usize::from(z) + 1) {
                    *n += 1;
                }
            }
        }
        let mut total = 0;
        for n in &mut s.starts {
            total += *n;
            *n = total;
        }
        s.edges.clear();
        s.edges.resize(index(u64::from(total)), (0, 0, 0));
        s.path.clear();
        s.path.extend(s.starts.iter().copied());
        for (id, r) in open() {
            let (a, b) = r.ends();
            for (z, other) in [(a, b), (b, a)] {
                let Some(next) = s.path.get_mut(usize::from(z)) else { continue };
                if let Some(e) = s.edges.get_mut(index(u64::from(*next))) {
                    *e = (u32::from(other), u64::from(r.metres()), id);
                }
                *next += 1;
            }
        }
        self.arena.clear();
        let n = self.places.len();
        for (i, from) in self.places.iter().enumerate() {
            shortest(s, *from, zones);
            for (j, to) in self.places.iter().enumerate() {
                let entry = trace(s, &mut self.arena, *to);
                if let Some(e) = self.entries.get_mut(i * n + j) {
                    *e = entry;
                }
            }
        }
        // Each member is read twice: once to count the zones' edges, once to lay them.
        match u64::try_from(self.members.len()).ok().and_then(|m| m.checked_mul(2)) {
            Some(rows) => rows,
            None => capacity_exceeded!("a route table", u64::MAX, self.members.len()),
        }
    }

    /// The pairs the table holds and the bytes its table and arena hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.entries.capacity() * size_of::<Entry>()
            + (self.arena.capacity() + self.members.capacity()) * size_of::<u32>()
            + self.place_of.capacity() * size_of::<u16>()
    }
}

/// The shortest distances from one zone over the scratch's edges, each zone's best and the edge it was reached by.
fn shortest(s: &mut Scratch, from: u16, zones: usize) {
    s.best.clear();
    s.best.resize(zones, u64::MAX);
    s.via.clear();
    s.via.resize(zones, (NONE, NONE));
    s.heap.clear();
    if let Some(b) = s.best.get_mut(usize::from(from)) {
        *b = 0;
    }
    s.heap.push(Reverse((0, u32::from(from))));
    while let Some(Reverse((d, z))) = s.heap.pop() {
        let zi = index(u64::from(z));
        if s.best.get(zi).is_some_and(|b| *b < d) {
            continue;
        }
        let Some(&[lo, hi]) = s.starts.get(zi..zi + 2) else {
            violation!(clause = "FRT.2", "a zone past the network's edges", zone = z);
        };
        for e in lo..hi {
            let Some(&(to, metres, seg)) = s.edges.get(index(u64::from(e))) else { continue };
            let next = d + metres;
            let ti = index(u64::from(to));
            if s.best.get(ti).is_some_and(|b| next < *b) {
                if let Some(b) = s.best.get_mut(ti) {
                    *b = next;
                }
                if let Some(v) = s.via.get_mut(ti) {
                    *v = (z, seg);
                }
                s.heap.push(Reverse((next, to)));
            }
        }
    }
}

/// The route to a zone from the last `shortest`'s origin, its segments appended to the arena in order.
fn trace(s: &mut Scratch, arena: &mut Vec<u32>, to: u16) -> Entry {
    let Some(&metres) = s.best.get(usize::from(to)).filter(|b| **b != u64::MAX) else { return UNJOINED };
    s.path.clear();
    let mut at = u32::from(to);
    while let Some(&(prev, seg)) = s.via.get(index(u64::from(at))).filter(|(p, _)| *p != NONE) {
        s.path.push(seg);
        at = prev;
    }
    let offset = narrow(arena.len());
    arena.extend(s.path.iter().rev());
    Entry { offset, len: narrow(s.path.len()), metres }
}
