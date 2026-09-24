use std::cmp::Reverse;
use std::collections::BinaryHeap;

use phx_id::{TileId, ZoneId};
use phx_macros::clause;

use crate::consts::ROUNDING_PARTS;
use crate::generate::Map;
use crate::grid::Grid;
use crate::tile::Zone;

/// Where a path may go and how high each tile stands: tiles outside `passable` are never entered.
#[derive(Clone, Copy, Debug)]
pub struct Terrain<'a> {
    pub grid: &'a Grid,
    pub elevation: &'a [i16],
    pub passable: &'a [bool],
}

impl Terrain<'_> {
    fn elev(&self, t: TileId) -> i16 {
        self.elevation.get(self.grid.index(t)).copied().unwrap_or(0)
    }

    fn open(&self, t: TileId) -> bool {
        self.passable.get(self.grid.index(t)).copied().unwrap_or(false)
    }

    fn edge(&self, a: TileId, b: TileId) -> u64 {
        self.grid.length_m(a, self.elev(a), b, self.elev(b))
    }

    /// A lower bound of any path's length to `to`: the plane distance, less the most the legs' rounding can take
    /// from a path, half a metre a leg against legs of at least a tile.
    fn bound(&self, from: TileId, to: TileId) -> u64 {
        let plane = self.grid.plane_m(from, to);
        plane - plane.div_ceil(ROUNDING_PARTS)
    }
}

/// The length of the shortest path from `from` to every tile, over passable tiles, each leg a straight line between
/// tile centres over their elevations; `None` where no path reaches.
#[clause("GEO.2")]
#[must_use]
pub fn dijkstra(t: &Terrain<'_>, from: TileId) -> Vec<Option<u64>> {
    let mut best: Vec<Option<u64>> = vec![None; t.grid.len()];
    if !t.open(from) {
        return best;
    }
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0_u64, from.get())));
    if let Some(b) = best.get_mut(t.grid.index(from)) {
        *b = Some(0);
    }
    while let Some(Reverse((d, id))) = heap.pop() {
        let at = TileId::new(id);
        if best.get(t.grid.index(at)).copied().flatten().is_some_and(|b| b < d) {
            continue;
        }
        for next in t.grid.neighbours(at).filter(|n| t.open(*n)) {
            let nd = d + t.edge(at, next);
            let slot = best.get_mut(t.grid.index(next));
            if let Some(cell) = slot
                && cell.is_none_or(|old| nd < old)
            {
                *cell = Some(nd);
                heap.push(Reverse((nd, next.get())));
            }
        }
    }
    best
}

/// The length of the shortest path between two tiles, by A* with a lower bound from the plane distance; `None` when
/// none reaches. Its value equals Dijkstra's.
#[clause("GEO.2")]
#[must_use]
pub fn path_length(t: &Terrain<'_>, from: TileId, to: TileId) -> Option<u64> {
    if !t.open(from) || !t.open(to) {
        return None;
    }
    let mut best: Vec<Option<u64>> = vec![None; t.grid.len()];
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((t.bound(from, to), 0_u64, from.get())));
    if let Some(b) = best.get_mut(t.grid.index(from)) {
        *b = Some(0);
    }
    while let Some(Reverse((_, d, id))) = heap.pop() {
        let at = TileId::new(id);
        if at == to {
            return Some(d);
        }
        if best.get(t.grid.index(at)).copied().flatten().is_some_and(|b| b < d) {
            continue;
        }
        for next in t.grid.neighbours(at).filter(|n| t.open(*n)) {
            let nd = d + t.edge(at, next);
            if let Some(cell) = best.get_mut(t.grid.index(next))
                && cell.is_none_or(|old| nd < old)
            {
                *cell = Some(nd);
                heap.push(Reverse((nd + t.bound(next, to), nd, next.get())));
            }
        }
    }
    None
}

fn slot(id: u32) -> usize {
    usize::try_from(id).unwrap_or(usize::MAX)
}

/// The length of the shortest land path between every two zones' centroids within each country, in metres; none
/// crosses a border, since crossings arrive with trade across borders.
#[clause("GEO.2")]
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct ZoneDistances {
    place: Vec<(usize, usize)>,
    countries: Vec<(usize, Vec<u32>)>,
}

/// A search's buffers: each tile's best length so far and whether it is settled, and the frontier.
struct Search {
    best: Vec<u64>,
    done: Vec<bool>,
    heap: BinaryHeap<Reverse<(u64, u32)>>,
}

/// A country's land as a graph: its tiles numbered in identity order, and each one's passable neighbours with the
/// length of the leg to them, laid out one tile after another.
struct Land {
    local: Vec<Option<u32>>,
    starts: Vec<usize>,
    to: Vec<u32>,
    metres: Vec<u64>,
}

impl Land {
    fn build(grid: &Grid, elevation: &[i16], passable: &[bool]) -> Land {
        let mut local = vec![None; grid.len()];
        let mut tiles = Vec::new();
        for (i, open) in passable.iter().enumerate() {
            if *open && let Some(l) = local.get_mut(i) {
                *l = u32::try_from(tiles.len()).ok();
                tiles.push(grid.tile(i));
            }
        }
        let elev = |t: TileId| elevation.get(grid.index(t)).copied().unwrap_or(0);
        let (mut starts, mut to, mut metres) = (Vec::with_capacity(tiles.len() + 1), Vec::new(), Vec::new());
        for t in &tiles {
            starts.push(to.len());
            for n in grid.neighbours(*t) {
                if let Some(j) = local.get(grid.index(n)).copied().flatten() {
                    to.push(j);
                    metres.push(grid.length_m(*t, elev(*t), n, elev(n)));
                }
            }
        }
        starts.push(to.len());
        Land { local, starts, to, metres }
    }

    /// Dijkstra from one tile until every wanted tile is settled, its buffers reused from source to source.
    fn reach(&self, from: u32, targets: &[Option<u32>], wanted: &[bool], buf: &mut Search, out: &mut Vec<u32>) {
        let Search { best, done, heap } = buf;
        best.fill(u64::MAX);
        done.fill(false);
        heap.clear();
        let mut left = targets.iter().flatten().count();
        if let Some(b) = best.get_mut(slot(from)) {
            *b = 0;
            heap.push(Reverse((0_u64, from)));
        }
        while let Some(Reverse((dist, at))) = heap.pop() {
            let here = slot(at);
            if done.get(here).copied().unwrap_or(true) {
                continue;
            }
            if let Some(x) = done.get_mut(here) {
                *x = true;
            }
            if wanted.get(here).copied().unwrap_or(false) {
                left -= 1;
                if left == 0 {
                    break;
                }
            }
            let legs = self.starts.get(here).copied().zip(self.starts.get(here + 1).copied());
            let Some((first, end)) = legs else { continue };
            for leg in first..end {
                let (Some(next), Some(length)) = (self.to.get(leg).copied(), self.metres.get(leg).copied()) else {
                    continue;
                };
                let through = dist + length;
                if let Some(b) = best.get_mut(slot(next))
                    && through < *b
                {
                    *b = through;
                    heap.push(Reverse((through, next)));
                }
            }
        }
        out.extend(targets.iter().map(|t| {
            t.and_then(|t| best.get(slot(t)).copied())
                .filter(|m| *m != u64::MAX)
                .and_then(|m| u32::try_from(m).ok())
                .unwrap_or(u32::MAX)
        }));
    }
}

impl ZoneDistances {
    /// Every country's zones measured from each of their centroids over the country's own land.
    #[must_use]
    pub fn measure(map: &Map) -> ZoneDistances {
        let country_of = |z: &Zone| map.regions.get(usize::from(z.region.get())).map(|r| usize::from(r.country.get()));
        let countries =
            map.regions.iter().map(|r| usize::from(r.country.get()) + 1).fold(0, |a, b| if b > a { b } else { a });
        let mut place = vec![(0, 0); map.zones.len()];
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); countries];
        for (z, zone) in map.zones.iter().enumerate() {
            if let Some(c) = country_of(zone)
                && let Some(list) = members.get_mut(c)
            {
                if let Some(slot) = place.get_mut(z) {
                    *slot = (c, list.len());
                }
                list.push(z);
            }
        }
        let elevation: Vec<i16> = map.tiles.iter().map(|t| t.elevation_m).collect();
        let mut out = Vec::with_capacity(countries);
        for (c, zones) in members.iter().enumerate() {
            let passable: Vec<bool> = map
                .tiles
                .iter()
                .map(|t| t.zone().and_then(|z| map.zones.get(slot(z.get()))).and_then(country_of) == Some(c))
                .collect();
            let land = Land::build(&map.grid, &elevation, &passable);
            let centroids: Vec<Option<u32>> = zones
                .iter()
                .map(|z| map.zones.get(*z).and_then(|z| land.local.get(map.grid.index(z.centroid)).copied().flatten()))
                .collect();
            let tiles = land.local.iter().flatten().count();
            let mut wanted = vec![false; tiles];
            for t in centroids.iter().flatten() {
                if let Some(w) = wanted.get_mut(slot(*t)) {
                    *w = true;
                }
            }
            let mut buf = Search { best: vec![u64::MAX; tiles], done: vec![false; tiles], heap: BinaryHeap::new() };
            let mut metres = Vec::with_capacity(zones.len() * zones.len());
            for from in &centroids {
                match from {
                    Some(f) => land.reach(*f, &centroids, &wanted, &mut buf, &mut metres),
                    None => metres.extend(centroids.iter().map(|_| u32::MAX)),
                }
            }
            out.push((zones.len(), metres));
        }
        ZoneDistances { place, countries: out }
    }

    /// The path length between two zones' centroids; `None` across a border or where no land path joins them.
    #[must_use]
    pub fn between(&self, a: ZoneId, b: ZoneId) -> Option<u64> {
        let (ca, ia) = *self.place.get(slot(a.get()))?;
        let (cb, ib) = *self.place.get(slot(b.get()))?;
        if ca != cb {
            return None;
        }
        let (n, metres) = self.countries.get(ca)?;
        metres.get(ia * n + ib).copied().filter(|m| *m != u32::MAX).map(u64::from)
    }

    /// Bytes the distances hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.countries.iter().map(|(_, m)| m.len() * size_of::<u32>()).sum::<usize>()
            + self.place.len() * size_of::<(usize, usize)>()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::TileId;

    use super::{Terrain, dijkstra, path_length};
    use crate::grid::Grid;

    #[test]
    fn dijkstra_matches_brute_force() {
        let grid = Grid { width: 10, height: 10, tile_m: 10_000 };
        let size = grid.len();
        let elevation: Vec<i16> = (0..size).map(|i| i16::try_from((i * 37) % 900).unwrap()).collect();
        let passable: Vec<bool> = (0..size).map(|i| i % 7 != 3).collect();
        let terrain = Terrain { grid: &grid, elevation: &elevation, passable: &passable };
        let inf = u64::MAX / 4;
        let mut d = vec![vec![inf; size]; size];
        for i in 0..size {
            d[i][i] = if passable[i] { 0 } else { inf };
            for j in grid.neighbours(grid.tile(i)) {
                let j = grid.index(j);
                if passable[i] && passable[j] {
                    d[i][j] = grid.length_m(grid.tile(i), elevation[i], grid.tile(j), elevation[j]);
                }
            }
        }
        for k in 0..size {
            for i in 0..size {
                for j in 0..size {
                    if d[i][k] + d[k][j] < d[i][j] {
                        d[i][j] = d[i][k] + d[k][j];
                    }
                }
            }
        }
        for from in [0, 17, 55, 99] {
            let got = dijkstra(&terrain, TileId::new(from));
            for (to, found) in got.iter().enumerate() {
                let want = d[usize::try_from(from).unwrap()][to];
                assert_eq!(*found, (want < inf).then_some(want), "from {from} to {to}");
                let a = path_length(&terrain, TileId::new(from), TileId::new(u32::try_from(to).unwrap()));
                assert_eq!(a, *found, "A* from {from} to {to}");
            }
        }
    }

    #[test]
    fn zone_distances_stay_within_countries() {
        use phx_core::{Purpose, StreamDecl, Streams};
        use phx_id::{Day, ZoneId};
        use phx_rand::{Seed, Subject, SubjectTag};

        use super::ZoneDistances;
        use crate::generate::generate;
        use crate::generate::tests::small;

        const MAP: StreamDecl =
            StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };
        let p = small(400, vec![60, 40], vec![3, 3], 20);
        let streams = Streams::new(Seed::new(3), &[MAP]).unwrap();
        let map = generate(&p, &|_| 0, &|a| streams.open(&MAP, Subject::new(SubjectTag::World, a), Day::new(0), 0));
        let d = ZoneDistances::measure(&map);
        let country = |z: usize| map.regions[usize::from(map.zones[z].region.get())].country;
        for a in 0..map.zones.len() {
            let za = ZoneId::new(u32::try_from(a).unwrap());
            assert_eq!(d.between(za, za), Some(0));
            for b in 0..map.zones.len() {
                let zb = ZoneId::new(u32::try_from(b).unwrap());
                if country(a) != country(b) {
                    assert_eq!(d.between(za, zb), None, "no distance crosses a border");
                } else if let Some(m) = d.between(za, zb) {
                    assert_eq!(Some(m), d.between(zb, za), "symmetric");
                    assert!(m >= map.grid.plane_m(map.zones[a].centroid, map.zones[b].centroid));
                }
            }
        }
    }
}
