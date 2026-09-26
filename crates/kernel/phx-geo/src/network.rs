//! The transport network: segments between regions' market zones, generated with the map by a recorded procedure.
//! Every two regions of a country that share a border are joined by each land mode over the land path between their
//! market zones; the parts of a country no land path joins, its islands, are joined by sea lanes, each part to its
//! nearest, the shortest lanes first. Routes are the shortest paths of one mode over the segments.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use phx_id::ZoneId;
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::distance::ZoneDistances;
use crate::generate::Map;

/// The modes the network carries, by place: land modes join regions that share a border, the sea mode the parts of a
/// country no land path joins.
pub const ROAD: u16 = 0;
pub const RAIL: u16 = 1;
pub const SEA: u16 = 2;
/// The land modes, which run over land paths between market zones.
pub const LAND: [u16; 2] = [ROAD, RAIL];

/// A segment between two zones, of a mode, its length in metres and what it carries a day in tonnes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct Segment {
    pub from: ZoneId,
    pub to: ZoneId,
    pub mode: u16,
    pub metres: u64,
    pub tonnes: u64,
}

/// The network as generated with the map.
#[clause("GEO.4")]
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Network {
    pub segments: Vec<Segment>,
}

/// A route: the segments one mode runs over from one zone to another, in order, and its length in metres.
#[clause("FRT.2")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub mode: u16,
    pub segments: Vec<usize>,
    pub metres: u64,
}

/// The root of a part in a forest of parts.
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while let Some(&p) = parent.get(i) {
        if p == i {
            break;
        }
        let up = parent.get(p).copied().unwrap_or(p);
        if let Some(slot) = parent.get_mut(i) {
            *slot = up;
        }
        i = p;
    }
    i
}

/// The network over a map: each land mode between every two market zones of regions sharing a border within a
/// country, over their land path; sea lanes joining each country's parts, the shortest lane between two parts' market
/// zones first. Each mode carries what `tonnes` gives it a day.
#[clause("GEO.4", "GEO.13")]
#[must_use]
pub fn generate(map: &Map, distances: &ZoneDistances, market: &[Missing<ZoneId>], tonnes: &[u64]) -> Network {
    let region_of = |z: ZoneId| map.zones.get(usize::try_from(z.get()).ok()?).map(|z| usize::from(z.region.get()));
    let mut touching: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (i, tile) in map.tiles.iter().enumerate() {
        let Some(z) = tile.zone() else { continue };
        let Some(r) = region_of(z) else { continue };
        for n in map.grid.neighbours(map.grid.tile(i)) {
            let Some(other) = map.tiles.get(map.grid.index(n)).and_then(crate::tile::Tile::zone) else { continue };
            if let Some(q) = region_of(other).filter(|q| *q != r) {
                touching.insert(if r < q { (r, q) } else { (q, r) });
            }
        }
    }
    let at = |r: usize| match market.get(r) {
        Some(Missing::Present(z)) => Some(*z),
        _ => None,
    };
    let capacity = |mode: u16| match tonnes.get(usize::from(mode)) {
        Some(t) => *t,
        None => violation!(clause = "GEO.4", "a mode with no declared capacity", mode = mode),
    };
    let mut segments = Vec::new();
    let mut parent: Vec<usize> = (0..map.regions.len()).collect();
    for &(a, b) in &touching {
        let (Some(za), Some(zb)) = (at(a), at(b)) else { continue };
        let Some(metres) = distances.between(za, zb) else { continue };
        for mode in LAND {
            segments.push(Segment { from: za, to: zb, mode, metres, tonnes: capacity(mode) });
        }
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if let Some(slot) = parent.get_mut(ra) {
            *slot = rb;
        }
    }
    // The shortest lanes between parts of a country join them first, until each country is one part.
    let centroid = |z: ZoneId| map.zones.get(usize::try_from(z.get()).ok()?).map(|z| z.centroid);
    let mut lanes: Vec<(u64, usize, usize)> = Vec::new();
    for a in 0..map.regions.len() {
        for b in a + 1..map.regions.len() {
            let same = map.regions.get(a).map(|r| r.country) == map.regions.get(b).map(|r| r.country);
            let (Some(za), Some(zb)) = (at(a), at(b)) else { continue };
            let (Some(ca), Some(cb)) = (centroid(za), centroid(zb)) else { continue };
            if same && root(&mut parent, a) != root(&mut parent, b) {
                lanes.push((map.grid.plane_m(ca, cb), a, b));
            }
        }
    }
    lanes.sort_unstable();
    for (metres, a, b) in lanes {
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if ra == rb {
            continue;
        }
        if let (Some(za), Some(zb)) = (at(a), at(b)) {
            segments.push(Segment { from: za, to: zb, mode: SEA, metres, tonnes: capacity(SEA) });
        }
        if let Some(slot) = parent.get_mut(ra) {
            *slot = rb;
        }
    }
    segments.sort_unstable();
    Network { segments }
}

impl Network {
    /// The shortest route of one mode from one zone to another over the segments, either way along each, the lower
    /// zone first among equals; none where the mode joins them by no path.
    #[clause("FRT.2")]
    #[must_use]
    pub fn route(&self, mode: u16, from: ZoneId, to: ZoneId) -> Option<Route> {
        let mut edges: BTreeMap<ZoneId, Vec<(ZoneId, u64, usize)>> = BTreeMap::new();
        for (i, s) in self.segments.iter().enumerate().filter(|(_, s)| s.mode == mode) {
            edges.entry(s.from).or_default().push((s.to, s.metres, i));
            edges.entry(s.to).or_default().push((s.from, s.metres, i));
        }
        let mut best: BTreeMap<ZoneId, (u64, Option<(ZoneId, usize)>)> = BTreeMap::new();
        best.insert(from, (0, None));
        let mut heap = BinaryHeap::new();
        heap.push(core::cmp::Reverse((0_u64, from)));
        while let Some(core::cmp::Reverse((d, z))) = heap.pop() {
            if z == to {
                break;
            }
            if best.get(&z).is_some_and(|(b, _)| *b < d) {
                continue;
            }
            for &(n, m, i) in edges.get(&z).map_or(&[][..], Vec::as_slice) {
                let next = d + m;
                if best.get(&n).is_none_or(|(b, _)| next < *b) {
                    best.insert(n, (next, Some((z, i))));
                    heap.push(core::cmp::Reverse((next, n)));
                }
            }
        }
        let (metres, _) = *best.get(&to)?;
        let mut segments = Vec::new();
        let mut at = to;
        while let Some(&(_, Some((prev, i)))) = best.get(&at) {
            segments.push(i);
            at = prev;
        }
        segments.reverse();
        Some(Route { mode, segments, metres })
    }

    /// Bytes the segments hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.segments.len() * core::mem::size_of::<Segment>()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::ZoneId;

    use super::{Network, ROAD, SEA, Segment};

    fn seg(a: u32, b: u32, mode: u16, metres: u64) -> Segment {
        Segment { from: ZoneId::new(a), to: ZoneId::new(b), mode, metres, tonnes: 100 }
    }

    #[test]
    fn a_route_is_the_shortest_path_of_its_mode() {
        let n = Network {
            segments: vec![seg(0, 1, ROAD, 10), seg(1, 2, ROAD, 10), seg(0, 2, ROAD, 25), seg(2, 3, SEA, 5)],
        };
        let r = n.route(ROAD, ZoneId::new(0), ZoneId::new(2)).expect("a road joins them");
        assert_eq!((r.metres, r.segments.len()), (20, 2));
        let back = n.route(ROAD, ZoneId::new(2), ZoneId::new(0)).expect("either way along a segment");
        assert_eq!(back.metres, 20);
        assert!(n.route(ROAD, ZoneId::new(0), ZoneId::new(3)).is_none(), "no road reaches the island");
        assert_eq!(n.route(SEA, ZoneId::new(2), ZoneId::new(3)).map(|r| r.metres), Some(5));
        assert_eq!(n.route(ROAD, ZoneId::new(1), ZoneId::new(1)).map(|r| r.metres), Some(0));
    }
}
