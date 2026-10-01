//! The network as the map generates it, by a recorded procedure: every two regions of a country that share a border
//! are joined by each land mode over the land path between their market zones; the parts of a country no land path
//! joins, its islands, are joined by each sea mode, each part to its nearest, the shortest lanes first. Which modes run
//! on land and which by sea, and what each carries a day, are declared by the modes' places.

use std::collections::BTreeSet;

use phx_id::{Day, ZoneId};
use phx_macros::{clause, opening};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::distance::ZoneDistances;
use crate::generate::Map;
use crate::transport::{SegmentDecl, Segments, Transport};

/// What the modes are, by place: whether each runs on land, and what its segments carry a day in tonnes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modes {
    pub land: Vec<bool>,
    pub tonnes: Vec<u32>,
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

fn zone(z: ZoneId) -> u16 {
    match u16::try_from(z.get()) {
        Ok(z) => z,
        Err(_) => capacity_exceeded!("the network's zones", u16::MAX, z.get()),
    }
}

fn mode(m: usize) -> u8 {
    match u8::try_from(m) {
        Ok(m) => m,
        Err(_) => capacity_exceeded!("the network's modes", u8::MAX, m),
    }
}

/// The network over a map: each land mode between every two market zones of regions sharing a border within a
/// country, over their land path; each sea mode joining each country's parts, the shortest lane between two parts'
/// market zones first. The segments are opened in (from, to, mode, metres) order, and the routes join the regions'
/// market zones.
#[clause("GEO.4", "GEO.13")]
#[must_use]
#[opening]
pub fn generate(map: &Map, distances: &ZoneDistances, market: &[Missing<ZoneId>], modes: &Modes) -> Transport {
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
    let metres = |m: u64| match u32::try_from(m) {
        Ok(m) => m,
        Err(_) => capacity_exceeded!("a segment's length", u32::MAX, m),
    };
    let ways = |land: bool| {
        modes
            .land
            .iter()
            .zip(&modes.tonnes)
            .enumerate()
            .filter(move |(_, (l, _))| **l == land)
            .map(|(m, (_, t))| (mode(m), *t))
    };
    let mut segments = Vec::new();
    let mut parent: Vec<usize> = (0..map.regions.len()).collect();
    for &(a, b) in &touching {
        let (Some(za), Some(zb)) = (at(a), at(b)) else { continue };
        let Some(length) = distances.between(za, zb) else { continue };
        for (m, tonnes) in ways(true) {
            segments.push((zone(za), zone(zb), m, metres(length), tonnes));
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
    for (length, a, b) in lanes {
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if ra == rb {
            continue;
        }
        if let (Some(za), Some(zb)) = (at(a), at(b)) {
            for (m, tonnes) in ways(false) {
                segments.push((zone(za), zone(zb), m, metres(length), tonnes));
            }
        }
        if let Some(slot) = parent.get_mut(ra) {
            *slot = rb;
        }
    }
    segments.sort_unstable();
    let mut opened = Segments::default();
    for (from, to, mode, metres, capacity) in segments {
        let _ = opened.open(SegmentDecl { from, to, mode, metres, capacity, condition: 0, unit: Missing::Absent });
    }
    let mut places: Vec<u16> = market
        .iter()
        .filter_map(|z| match z {
            Missing::Present(z) => Some(zone(*z)),
            Missing::Absent => None,
        })
        .collect();
    places.sort_unstable();
    places.dedup();
    let Ok(zones) = u32::try_from(map.zones.len()) else {
        capacity_exceeded!("the network's zones", u32::MAX, map.zones.len());
    };
    if modes.land.len() != modes.tonnes.len() {
        violation!(clause = "GEO.4", "modes whose ground and capacity disagree in number", modes = modes.land.len());
    }
    Transport::new(opened, (&places, zones), mode(modes.land.len()), Day::new(0))
}
