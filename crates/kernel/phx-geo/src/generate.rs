use std::cmp::Reverse;
use std::collections::BinaryHeap;

use phx_id::{CountryId, RegionId, TileId, ZoneId};
use phx_macros::clause;
use phx_num::{Fixed, Round, capacity_exceeded, violation};
use phx_rand::Draws;

use crate::consts::{PER_MILLE, PER_MILLE_F64, WHOLE_PERCENT};
use crate::grid::{DIRECTIONS, Grid};
use crate::hydrology::{drainage, upstream};
use crate::partition::{Partition, StepCost, components, merge_small, partition, pick_seeds};
use crate::relief::{ReliefParams, erode, raw, to_curve};
use crate::tile::{LAND, Region, Tile, WATER, Zone};

/// A terrain class: the first class whose elevation and in-tile relief ceilings a land tile keeps is its class; the
/// last class takes every tile the others leave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainClass {
    pub max_elevation_m: i16,
    pub max_relief_m: u32,
}

/// What a tile's climate class is read from: how far north it lies across the map, from 0 at the south edge to 1 at
/// the north, its elevation, and its distance to the sea.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClimateInput {
    pub north_permille: u32,
    pub elevation_m: i16,
    pub sea_distance_m: u64,
}

/// A measured distribution of heights: points in parts per thousand of the cells, and the metres at each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeightCurve {
    pub axis: Vec<i64>,
    pub metres: Vec<i64>,
}

/// What a map is generated from, every value read from the register.
#[derive(Clone, Debug, PartialEq)]
pub struct MapParams {
    pub land_tiles: u64,
    pub tile_m: u32,
    pub sea_share: f64,
    pub cells_per_tile: u32,
    pub relief: ReliefParams,
    pub land_heights: HeightCurve,
    pub sea_depths: HeightCurve,
    pub terrain: Vec<TerrainClass>,
    pub river_tiles: u32,
    pub rugged_m: u64,
    pub river_crossing_m: u64,
    pub split: Vec<u64>,
    pub regions: Vec<u64>,
    pub zones: u64,
    pub zone_min_tiles: u64,
    pub zone_max_tiles: u64,
    pub share_tolerance_per_mille: u64,
    pub partition_rounds: u64,
    pub mainland_floor_percent: u64,
    pub max_attempts: u64,
}

/// A generated map rejected by a construction condition, with the condition it failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub attempt: u64,
    pub condition: String,
}

/// The accepted map: its grid and tiles, each tile's relief within it, where each tile drains and how many tiles
/// drain through it, its zones and regions, and every attempt rejected before it.
#[clause("GEO.1", "GEO.3", "GEO.10")]
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub grid: Grid,
    pub tiles: Vec<Tile>,
    pub relief_m: Vec<u16>,
    pub drains_to: Vec<Option<TileId>>,
    pub upstream: Vec<u32>,
    pub river_tiles: u32,
    pub zones: Vec<Zone>,
    pub regions: Vec<Region>,
    pub rejections: Vec<Rejection>,
    pub attempt: u64,
}

impl Map {
    /// Whether a tile carries a river: enough land drains through it.
    #[must_use]
    pub fn is_river(&self, index: usize) -> bool {
        self.tiles.get(index).is_some_and(Tile::is_land)
            && self.upstream.get(index).is_some_and(|u| *u >= self.river_tiles)
    }
}

/// Whole numbers in proportion to `weights` summing to `total`, by largest remainder, ties to the earlier.
#[must_use]
pub fn apportion(total: u64, weights: &[u64]) -> Vec<u64> {
    let sum: u64 = weights.iter().sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let mut out: Vec<u64> = weights.iter().map(|w| total * w / sum).collect();
    let mut rest: Vec<(u64, usize)> = weights.iter().enumerate().map(|(i, w)| (total * w % sum, i)).collect();
    rest.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let left = total - out.iter().sum::<u64>();
    for (_, i) in rest.into_iter().take(usize::try_from(left).unwrap_or(usize::MAX)) {
        if let Some(x) = out.get_mut(i) {
            *x += 1;
        }
    }
    out
}

fn to_i16(x: f64) -> i16 {
    let raw = Fixed::<0>::from_f64(x, Round::HalfEven).map(Fixed::raw).ok().and_then(|r| i16::try_from(r).ok());
    let Some(v) = raw else {
        violation!(clause = "GEO.1", "an elevation beyond sixteen bits of metres");
    };
    v
}

/// The side of the square grid that holds the land at the declared sea share.
#[must_use]
pub fn side(land_tiles: u64, sea_share: f64) -> u32 {
    let tiles = phx_rand::float::from_u64(land_tiles) / (1.0 - sea_share);
    let whole = Fixed::<0>::from_f64(libm::ceil(libm::sqrt(tiles)), Round::Ceil).map(Fixed::raw);
    let Some(s) = whole.ok().and_then(|w| u32::try_from(w).ok()) else {
        capacity_exceeded!("the grid's side", u32::MAX, land_tiles);
    };
    s
}

/// Each tile's distance over the plane to the nearest water tile, in metres.
#[must_use]
pub fn sea_distance(grid: &Grid, land: &[bool]) -> Vec<u64> {
    let mut best: Vec<Option<u64>> = vec![None; grid.len()];
    let mut heap = BinaryHeap::new();
    for (i, l) in land.iter().enumerate() {
        if !l {
            heap.push(Reverse((0_u64, grid.tile(i).get())));
        }
    }
    while let Some(Reverse((d, id))) = heap.pop() {
        let t = TileId::new(id);
        let at = grid.index(t);
        if best.get(at).copied().flatten().is_some() {
            continue;
        }
        if let Some(b) = best.get_mut(at) {
            *b = Some(d);
        }
        for n in grid.neighbours(t) {
            if best.get(grid.index(n)).copied().flatten().is_none() {
                heap.push(Reverse((d + grid.length_m(t, 0, n, 0), n.get())));
            }
        }
    }
    best.into_iter().map(|b| b.unwrap_or(u64::MAX)).collect()
}

/// The tile of a set with the least summed plane distance to the others, ties to the lower identity.
fn medoid(grid: &Grid, tiles: &[TileId]) -> Option<TileId> {
    let mut best: Option<(u64, TileId)> = None;
    for t in tiles {
        let sum: u64 = tiles.iter().map(|o| grid.plane_m(*t, *o)).sum();
        if best.is_none_or(|(b, bt)| (sum, t.get()) < (b, bt.get())) {
            best = Some((sum, *t));
        }
    }
    best.map(|(_, t)| t)
}

/// A value at an index the map's own layout guarantees.
fn at<T: Copy>(values: &[T], index: usize) -> T {
    let Some(v) = values.get(index) else {
        violation!(clause = "GEO.1", "a tile read beyond the map", index = index);
    };
    *v
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The fine cells of each tile, `cells` across and down.
fn cells_of(fine: Grid, tile_grid: &Grid, cells: u32, tile: usize) -> impl Iterator<Item = usize> {
    let (tx, ty) = tile_grid.xy(tile_grid.tile(tile));
    (0..cells).flat_map(move |dy| (0..cells).map(move |dx| fine.index(fine.at(tx * cells + dx, ty * cells + dy))))
}

/// The relief: raw heights on the fine grid, the declared number of tiles of highest mean as land, rivers cutting the
/// land to the sea, then the land's cells ranked onto the measured land heights and the sea's onto the measured
/// depths; each tile's elevation the mean of its cells and its relief their range.
struct Surface {
    land: Vec<bool>,
    elevation: Vec<i16>,
    relief: Vec<u16>,
}

fn surface(p: &MapParams, grid: &Grid, draws: &mut Draws) -> Result<Surface, String> {
    let cells = p.cells_per_tile;
    let fine = Grid { width: grid.width * cells, height: grid.height * cells, tile_m: grid.tile_m / cells };
    let mut height = raw(&p.relief, &fine, draws);
    let per_tile = phx_rand::float::from_u64(u64::from(cells * cells));
    let mean = |h: &[f64], t: usize| cells_of(fine, grid, cells, t).map(|c| at(h, c)).sum::<f64>() / per_tile;
    let mut ranked: Vec<(f64, usize)> = (0..grid.len()).map(|t| (mean(&height, t), t)).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let land_count = usize::try_from(p.land_tiles).map_err(|e| e.to_string())?;
    if ranked.len() <= land_count {
        return Err("no sea at the declared sea share".to_owned());
    }
    let mut land = vec![false; grid.len()];
    for (_, t) in ranked.iter().take(land_count) {
        if let Some(l) = land.get_mut(*t) {
            *l = true;
        }
    }
    let mut fine_land = vec![false; fine.len()];
    for t in (0..grid.len()).filter(|t| at(&land, *t)) {
        for c in cells_of(fine, grid, cells, t) {
            if let Some(f) = fine_land.get_mut(c) {
                *f = true;
            }
        }
    }
    let outlet: Vec<bool> = fine_land.iter().map(|l| !l).collect();
    erode(&p.relief, &fine, &mut height, &outlet);
    to_curve(&mut height, &fine_land, &p.land_heights.axis, &p.land_heights.metres, PER_MILLE_F64);
    to_curve(&mut height, &outlet, &p.sea_depths.axis, &p.sea_depths.metres, PER_MILLE_F64);
    let mut elevation = Vec::with_capacity(grid.len());
    let mut relief = Vec::with_capacity(grid.len());
    for t in 0..grid.len() {
        let hs: Vec<f64> = cells_of(fine, grid, cells, t).map(|c| at(&height, c)).collect();
        let (lo, hi) = hs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), h| {
            (if *h < lo { *h } else { lo }, if *h > hi { *h } else { hi })
        });
        elevation.push(to_i16(hs.iter().sum::<f64>() / per_tile));
        let range = Fixed::<0>::from_f64(hi - lo, Round::HalfEven).map(Fixed::raw).ok().and_then(|r| u16::try_from(r).ok());
        let Some(r) = range else { return Err("a tile's relief beyond sixteen bits of metres".to_owned()) };
        relief.push(r);
    }
    Ok(Surface { land, elevation, relief })
}

/// Each land tile's terrain class, by its elevation and its relief within it.
fn terrain(p: &MapParams, s: &Surface) -> Vec<u8> {
    s.elevation
        .iter()
        .zip(&s.relief)
        .map(|(e, r)| {
            let class = p.terrain.iter().position(|c| *e <= c.max_elevation_m && u32::from(*r) <= c.max_relief_m);
            u8::try_from(class.unwrap_or(p.terrain.len() - 1)).unwrap_or(u8::MAX)
        })
        .collect()
}

/// What a step between neighbours costs a growing country, region or zone: its length over the ground, lengthened by
/// the relief it climbs into, and by a river it crosses onto, so fronts wait at ridges and rivers and borders form
/// there. Each tile's eight steps are worked out once, as every partition takes them many times.
struct Ground<'a> {
    grid: &'a Grid,
    steps: Vec<u64>,
}

impl<'a> Ground<'a> {
    fn new(grid: &'a Grid, s: &Surface, river: &[bool], rugged_m: u64, crossing_m: u64) -> Self {
        let mut steps = vec![0; grid.len() * DIRECTIONS];
        for ia in 0..grid.len() {
            let a = grid.tile(ia);
            for b in grid.neighbours(a) {
                let ib = grid.index(b);
                let leg = grid.length_m(a, at(&s.elevation, ia), b, at(&s.elevation, ib));
                let rugged = leg * (rugged_m + u64::from(at(&s.relief, ib))) / rugged_m;
                let crossing = if at(river, ib) && !at(river, ia) { crossing_m } else { 0 };
                if let Some(d) = grid.direction(a, b)
                    && let Some(slot) = steps.get_mut(ia * DIRECTIONS + d)
                {
                    *slot = rugged + crossing;
                }
            }
        }
        Ground { grid, steps }
    }

    fn step(&self, a: TileId, b: TileId) -> u64 {
        let Some(d) = self.grid.direction(a, b) else {
            violation!(clause = "GEO.2", "a step between tiles that are not neighbours");
        };
        at(&self.steps, self.grid.index(a) * DIRECTIONS + d)
    }
}

/// The countries: grown over the mainland from seeds on it and sized to their shares of all the land, the islands
/// joining the nearest; refused when a country's land strays from its share by more than the tolerance, or holds less
/// of its land on the mainland than the declared floor.
fn countries(
    p: &MapParams,
    grid: &Grid,
    land: &[bool],
    lot: &[u64],
    step: StepCost<'_>,
    draws: &mut Draws,
) -> Result<(Vec<Option<usize>>, Vec<bool>), String> {
    let (label, sizes) = components(grid, land);
    let mainland = sizes.iter().enumerate().fold(0, |best, (i, s)| if *s > at(&sizes, best) { i } else { best });
    let on_mainland: Vec<bool> = label.iter().map(|l| *l == Some(mainland)).collect();
    let seeds = pick_seeds(&tiles_of(grid, &on_mainland), p.split.len(), draws);
    let targets = apportion(p.land_tiles, &p.split);
    let over = Partition { eligible: &on_mainland, joiners: land, seeds: &seeds, targets: &targets };
    let (owner, within) = partition(grid, &over, p.share_tolerance_per_mille, p.partition_rounds, lot, step);
    if !within {
        return Err("the countries' land strays from their shares beyond the tolerance".to_owned());
    }
    for c in 0..p.split.len() {
        let all = count(owner.iter().filter(|o| **o == Some(c)).count());
        let main = count(owner.iter().zip(&on_mainland).filter(|(o, m)| **o == Some(c) && **m).count());
        if main * WHOLE_PERCENT < p.mainland_floor_percent * all {
            return Err(format!("country {c} holds {main} of its {all} land tiles on the mainland"));
        }
    }
    Ok((owner, on_mainland))
}

/// The tiles of `mask`, in identity order.
fn tiles_of(grid: &Grid, mask: &[bool]) -> Vec<TileId> {
    mask.iter().zip(0..).filter(|(m, _)| **m).map(|(_, i)| grid.tile(i)).collect()
}

/// Where a part's seeds are drawn: its mainland tiles, so no part starts stranded on an island, or all its tiles when
/// none of them is on the mainland.
fn seedable(grid: &Grid, mask: &[bool], on_mainland: &[bool]) -> Vec<TileId> {
    let main: Vec<bool> = mask.iter().zip(on_mainland).map(|(a, b)| *a && *b).collect();
    let tiles = tiles_of(grid, &main);
    if tiles.is_empty() { tiles_of(grid, mask) } else { tiles }
}

/// Regions and zones, grown by the ground's step costs within each country and region: regions seeded on the
/// mainland, of like size and balanced to it within the tolerance, and zones below the least size merging into their
/// smallest neighbour, or the nearest zone from an island; refused when a region strays from its share, is in two
/// pieces on the mainland, or a zone falls outside its declared size.
struct Places {
    regions: Vec<Region>,
    zones: Vec<Zone>,
    zone_of: Vec<Option<ZoneId>>,
}

#[allow(clippy::too_many_arguments)]
fn places(
    p: &MapParams,
    grid: &Grid,
    owner: &[Option<usize>],
    on_mainland: &[bool],
    lot: &[u64],
    step: StepCost<'_>,
    draws: &mut Draws,
) -> Result<Places, String> {
    let zones_by_country = apportion(p.zones, &p.split);
    let mut out = Places { regions: Vec::new(), zones: Vec::new(), zone_of: vec![None; grid.len()] };
    for c in 0..p.split.len() {
        let in_country: Vec<bool> = owner.iter().map(|o| *o == Some(c)).collect();
        let tiles = tiles_of(grid, &in_country);
        let regions = usize::try_from(p.regions.get(c).copied().unwrap_or(0)).map_err(|e| e.to_string())?;
        let targets = apportion(count(tiles.len()), &vec![1; regions]);
        let seeds = pick_seeds(&seedable(grid, &in_country, on_mainland), regions, draws);
        let over = Partition { eligible: &in_country, joiners: &in_country, seeds: &seeds, targets: &targets };
        let (region, within) = partition(grid, &over, p.share_tolerance_per_mille, p.partition_rounds, lot, step);
        if !within {
            return Err(format!("country {c}'s regions stray from like size beyond the tolerance"));
        }
        let sizes: Vec<u64> = (0..regions).map(|r| count(region.iter().filter(|o| **o == Some(r)).count())).collect();
        let zone_counts = apportion(zones_by_country.get(c).copied().unwrap_or(0), &sizes);
        for r in 0..regions {
            let in_region: Vec<bool> = region.iter().map(|o| *o == Some(r)).collect();
            let on_main: Vec<bool> = in_region.iter().zip(on_mainland).map(|(a, b)| *a && *b).collect();
            if components(grid, &on_main).1.len() > 1 {
                return Err(format!("region {r} of country {c} is in more than one piece on the mainland"));
            }
            let region_id = RegionId::new(u16::try_from(out.regions.len()).map_err(|e| e.to_string())?);
            out.regions.push(Region { country: CountryId::new(u8::try_from(c).map_err(|e| e.to_string())?) });
            let r_tiles = tiles_of(grid, &in_region);
            let zones = usize::try_from(zone_counts.get(r).copied().unwrap_or(0)).map_err(|e| e.to_string())?;
            let z_targets = apportion(count(r_tiles.len()), &vec![1; zones]);
            let z_seeds = pick_seeds(&r_tiles, zones, draws);
            let over = Partition { eligible: &in_region, joiners: &in_region, seeds: &z_seeds, targets: &z_targets };
            let (mut zone, _) = partition(grid, &over, p.share_tolerance_per_mille, p.partition_rounds, lot, step);
            merge_small(grid, &mut zone, zones, p.zone_min_tiles);
            for z in 0..zones {
                let z_tiles = tiles_of(grid, &zone.iter().map(|o| *o == Some(z)).collect::<Vec<_>>());
                let size = count(z_tiles.len());
                if size == 0 {
                    continue;
                }
                if size < p.zone_min_tiles || size > p.zone_max_tiles {
                    return Err(format!(
                        "a zone of {size} tiles, outside {} to {}",
                        p.zone_min_tiles, p.zone_max_tiles
                    ));
                }
                let id = ZoneId::new(u32::try_from(out.zones.len()).map_err(|e| e.to_string())?);
                for t in &z_tiles {
                    if let Some(slot) = out.zone_of.get_mut(grid.index(*t)) {
                        *slot = Some(id);
                    }
                }
                let centroid = medoid(grid, &z_tiles).ok_or("an empty zone")?;
                out.zones.push(Zone {
                    region: region_id,
                    centroid,
                    tiles: u32::try_from(size).map_err(|e| e.to_string())?,
                });
            }
        }
    }
    Ok(out)
}

/// One attempt: the map, or the condition it fails.
fn attempt(p: &MapParams, climate: &dyn Fn(ClimateInput) -> u8, draws: &mut Draws) -> Result<Map, String> {
    let side = side(p.land_tiles, p.sea_share);
    let grid = Grid { width: side, height: side, tile_m: p.tile_m };
    let s = surface(p, &grid, draws)?;
    let lot: Vec<u64> = (0..grid.len()).map(|_| draws.next_u64()).collect();
    let terrain = terrain(p, &s);
    let heights: Vec<f64> = s.elevation.iter().map(|e| f64::from(*e)).collect();
    let outlet: Vec<bool> = s.land.iter().map(|l| !l).collect();
    let routes = drainage(&grid, &heights, &outlet);
    let upstream = upstream(&routes);
    let river: Vec<bool> =
        upstream.iter().zip(&s.land).map(|(u, l)| *l && *u >= p.river_tiles).collect();
    let ground = Ground::new(&grid, &s, &river, p.rugged_m, p.river_crossing_m);
    let step = |a: TileId, b: TileId| ground.step(a, b);
    let to_sea = sea_distance(&grid, &s.land);
    let (owner, on_mainland) = countries(p, &grid, &s.land, &lot, &step, draws)?;
    let places = places(p, &grid, &owner, &on_mainland, &lot, &step, draws)?;
    let rows = u64::from(side - 1);
    let tiles = (0..grid.len())
        .map(|index| {
            let (_, y) = grid.xy(grid.tile(index));
            let north = u32::try_from(u64::from(side - 1 - y) * PER_MILLE / rows).unwrap_or(0);
            let elevation_m = at(&s.elevation, index);
            let input = ClimateInput { north_permille: north, elevation_m, sea_distance_m: at(&to_sea, index) };
            let surface = if at(&s.land, index) { LAND } else { WATER };
            Tile::new(elevation_m, surface, at(&terrain, index), climate(input), at(&places.zone_of, index))
        })
        .collect();
    let drains_to = routes.receiver.iter().map(|r| r.map(|i| grid.tile(i))).collect();
    Ok(Map {
        grid,
        tiles,
        relief_m: s.relief,
        drains_to,
        upstream,
        river_tiles: p.river_tiles,
        zones: places.zones,
        regions: places.regions,
        rejections: Vec::new(),
        attempt: 0,
    })
}

/// The map from the seed: attempt after attempt from the map's stream, each rejected one recorded with the
/// condition it failed, never nudged; the first to meet every condition is the map. `draws` gives each attempt's.
#[clause("GEO.10", "GEO.17")]
#[must_use]
pub fn generate(p: &MapParams, climate: &dyn Fn(ClimateInput) -> u8, draws: &dyn Fn(u64) -> Draws) -> Map {
    let mut rejections = Vec::new();
    for n in 0..p.max_attempts {
        let mut d = draws(n);
        match attempt(p, climate, &mut d) {
            Ok(mut map) => {
                map.attempt = n;
                map.rejections = rejections;
                return map;
            }
            Err(condition) => rejections.push(Rejection { attempt: n, condition }),
        }
    }
    capacity_exceeded!("map attempts", p.max_attempts, p.max_attempts);
}

#[cfg(test)]
pub(crate) mod tests {
    use phx_core::{Purpose, StreamDecl, Streams};
    use phx_id::Day;
    use phx_rand::{Draws, Seed, Subject, SubjectTag};

    use super::{HeightCurve, MapParams, TerrainClass, apportion, generate};
    use crate::relief::ReliefParams;
    use crate::partition::components;

    const MAP: StreamDecl = StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };

    /// A small map's parameters, for tests of what the generator guarantees on any map.
    pub(crate) fn small(land_tiles: u64, split: Vec<u64>, regions: Vec<u64>, zones: u64) -> MapParams {
        MapParams {
            land_tiles,
            tile_m: 10_000,
            sea_share: 0.4,
            cells_per_tile: 3,
            relief: ReliefParams {
                base_cells: 3,
                octaves: 4,
                roughness: 0.5,
                falloff: 2.0,
                plates: 6,
                belt: 0.05,
                plate_weight: 0.3,
                mountain_weight: 0.5,
                warp: 0.1,
                erosion_passes: 2,
                erosion_rate: 0.01,
                area_exponent: 0.5,
            },
            land_heights: HeightCurve { axis: vec![0, 500, 1000], metres: vec![0, 300, 3000] },
            sea_depths: HeightCurve { axis: vec![0, 1000], metres: vec![-4000, -10] },
            terrain: vec![
                TerrainClass { max_elevation_m: 200, max_relief_m: 100 },
                TerrainClass { max_elevation_m: 1_500, max_relief_m: 600 },
                TerrainClass { max_elevation_m: i16::MAX, max_relief_m: u32::MAX },
            ],
            river_tiles: 10,
            rugged_m: 500,
            river_crossing_m: 5_000,
            split,
            regions,
            zones,
            zone_min_tiles: 4,
            zone_max_tiles: 60,
            share_tolerance_per_mille: 100,
            partition_rounds: 32,
            mainland_floor_percent: 60,
            max_attempts: 50,
        }
    }

    #[test]
    fn apportion_by_largest_remainder() {
        assert_eq!(apportion(25, &[50, 30, 20]), vec![13, 7, 5]);
        assert_eq!(apportion(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(apportion(7, &[0, 0]), vec![0, 0]);
    }

    #[test]
    fn a_small_map_meets_its_conditions() {
        let p = small(900, vec![50, 30, 20], vec![5, 3, 3], 45);
        let streams = Streams::new(Seed::new(7), &[MAP]).unwrap();
        let draws = |a: u64| -> Draws { streams.open(&MAP, Subject::new(SubjectTag::World, a), Day::new(0), 0) };
        let map = generate(&p, &|c| u8::from(c.elevation_m > 1_000), &draws);
        let land = map.tiles.iter().filter(|t| t.is_land()).count();
        assert_eq!(land, 900, "land is the declared count");
        assert!(
            map.tiles.iter().all(|t| t.is_land() == t.zone().is_some()),
            "every land tile, and only land, has a zone"
        );
        assert!(map.regions.len() == 11 && map.zones.len() <= 45, "zones merge, never split");
        let plains = map.tiles.iter().filter(|t| t.is_land() && t.terrain == 0).count();
        assert!(plains > 0, "some land is plain");
        assert!(map.upstream.iter().any(|u| *u >= 10), "water gathers into rivers");
        for (z, zone) in map.zones.iter().enumerate() {
            let mask: Vec<bool> = map
                .tiles
                .iter()
                .map(|t| t.zone().map(phx_id::ZoneId::get) == Some(u32::try_from(z).unwrap()))
                .collect();
            assert!((p.zone_min_tiles..=p.zone_max_tiles).contains(&u64::from(zone.tiles)));
            assert!(mask[map.grid.index(zone.centroid)], "a zone's centroid is its own tile");
            assert!(!components(&map.grid, &mask).1.is_empty(), "zone {z} holds land");
        }
        let again = generate(&p, &|c| u8::from(c.elevation_m > 1_000), &draws);
        assert_eq!(map, again, "the same seed gives the same map");
    }
}
