use std::cmp::Reverse;
use std::collections::BinaryHeap;

use phx_id::{CountryId, RegionId, TileId, ZoneId};
use phx_macros::clause;
use phx_num::{Fixed, Round, capacity_exceeded, violation};
use phx_rand::Draws;

use crate::consts::{HALF, PER_MILLE, WHOLE_PERCENT};
use crate::grid::Grid;
use crate::noise::{fractal, octaves};
use crate::partition::{components, grow, join_nearest, pick_seeds};
use crate::tile::{LAND, Region, Tile, WATER, Zone};

/// A terrain class: the first class whose elevation and slope ceilings a land tile keeps is its class; the last
/// class takes every tile the others leave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainClass {
    pub max_elevation_m: i16,
    pub max_slope_permille: u32,
}

/// What a tile's climate class is read from: how far north it lies across the map, from 0 at the south edge to 1 at
/// the north, its elevation, and its distance to the sea.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClimateInput {
    pub north_permille: u32,
    pub elevation_m: i16,
    pub sea_distance_m: u64,
}

/// What a map is generated from, every value read from the register.
#[derive(Clone, Debug, PartialEq)]
pub struct MapParams {
    pub land_tiles: u64,
    pub tile_m: u32,
    pub sea_share: f64,
    pub base_cells: u32,
    pub octaves: u8,
    pub roughness: f64,
    pub falloff: f64,
    pub max_elevation_m: f64,
    pub max_depth_m: f64,
    pub terrain: Vec<TerrainClass>,
    pub split: Vec<u64>,
    pub regions: Vec<u64>,
    pub zones: u64,
    pub zone_min_tiles: u64,
    pub zone_max_tiles: u64,
    pub mainland_floor_percent: u64,
    pub max_attempts: u64,
}

/// A generated map rejected by a construction condition, with the condition it failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub attempt: u64,
    pub condition: String,
}

/// The accepted map: its grid and tiles, its zones and regions, and every attempt rejected before it.
#[clause("GEO.1", "GEO.3", "GEO.10")]
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub grid: Grid,
    pub tiles: Vec<Tile>,
    pub zones: Vec<Zone>,
    pub regions: Vec<Region>,
    pub rejections: Vec<Rejection>,
    pub attempt: u64,
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

/// The height field: the fractal noise, less a falloff from the centre that puts the sea at the edges.
fn heights(p: &MapParams, grid: &Grid, draws: &mut Draws) -> Vec<f64> {
    let layers = octaves(p.base_cells, p.base_cells, p.octaves, draws);
    let span = f64::from(grid.width);
    (0..grid.len())
        .map(|index| {
            let (x, y) = grid.xy(grid.tile(index));
            let (across, down) = ((f64::from(x) + HALF) / span, (f64::from(y) + HALF) / span);
            let (du, dv) = (across - HALF, down - HALF);
            fractal(&layers, p.roughness, across, down) - p.falloff * (du * du + dv * dv)
        })
        .collect()
}

/// The declared number of highest tiles as land, and the elevations: land scaled from the sea level to the highest
/// point, water from the sea level to the deepest.
fn surface(p: &MapParams, height: &[f64]) -> Result<(Vec<bool>, Vec<i16>), String> {
    let mut ranked: Vec<(f64, usize)> = height.iter().copied().zip(0..).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let land_count = usize::try_from(p.land_tiles).map_err(|e| e.to_string())?;
    let mut land = vec![false; height.len()];
    for (_, index) in ranked.iter().take(land_count) {
        if let Some(l) = land.get_mut(*index) {
            *l = true;
        }
    }
    let level = ranked.get(land_count).map(|r| r.0).ok_or("no sea at the declared sea share")?;
    let top = ranked.first().map(|r| r.0).ok_or("an empty map")?;
    let bottom = ranked.last().map(|r| r.0).ok_or("an empty map")?;
    let elevation = height
        .iter()
        .zip(&land)
        .map(|(h, is_land)| {
            if *is_land {
                to_i16((h - level) / (top - level) * p.max_elevation_m)
            } else {
                to_i16(-(level - h) / (level - bottom) * p.max_depth_m)
            }
        })
        .collect();
    Ok((land, elevation))
}

/// Each tile's terrain class, by its elevation and its steepest slope to a neighbour, in parts per thousand.
fn terrain(p: &MapParams, grid: &Grid, elevation: &[i16]) -> Vec<u8> {
    (0..grid.len())
        .map(|index| {
            let t = grid.tile(index);
            let here = i32::from(at(elevation, index));
            let steepest = grid
                .neighbours(t)
                .map(|nb| {
                    let rise = u64::from((here - i32::from(at(elevation, grid.index(nb)))).unsigned_abs());
                    rise * PER_MILLE / grid.plane_m(t, nb)
                })
                .fold(0, |a, b| if b > a { b } else { a });
            let slope = u32::try_from(steepest).unwrap_or(u32::MAX);
            let class =
                p.terrain.iter().position(|c| here <= i32::from(c.max_elevation_m) && slope <= c.max_slope_permille);
            u8::try_from(class.unwrap_or(p.terrain.len() - 1)).unwrap_or(u8::MAX)
        })
        .collect()
}

/// The countries: grown on the mainland to their shares of it, the islands joining the nearest; refused when a
/// country holds less of its land on the mainland than the declared floor.
fn countries(
    p: &MapParams,
    grid: &Grid,
    land: &[bool],
    lot: &[u64],
    draws: &mut Draws,
) -> Result<(Vec<Option<usize>>, Vec<bool>), String> {
    let (label, sizes) = components(grid, land);
    let mainland = sizes.iter().enumerate().fold(0, |best, (i, s)| if *s > at(&sizes, best) { i } else { best });
    let on_mainland: Vec<bool> = label.iter().map(|l| *l == Some(mainland)).collect();
    let tiles: Vec<TileId> = on_mainland.iter().zip(0..).filter(|(m, _)| **m).map(|(_, i)| grid.tile(i)).collect();
    let targets = apportion(count(tiles.len()), &p.split);
    let seeds = pick_seeds(&tiles, p.split.len(), draws);
    let mut owner = grow(grid, &on_mainland, &seeds, &targets, lot);
    join_nearest(grid, &mut owner, land, lot);
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

/// Regions and zones, grown within each country and region of like size, those left over joining the nearest;
/// refused when a region is in two pieces on the mainland or a zone falls outside its declared size.
struct Places {
    regions: Vec<Region>,
    zones: Vec<Zone>,
    zone_of: Vec<Option<ZoneId>>,
}

fn places(
    p: &MapParams,
    grid: &Grid,
    owner: &[Option<usize>],
    on_mainland: &[bool],
    lot: &[u64],
    draws: &mut Draws,
) -> Result<Places, String> {
    let zones_by_country = apportion(p.zones, &p.split);
    let mut out = Places { regions: Vec::new(), zones: Vec::new(), zone_of: vec![None; grid.len()] };
    for c in 0..p.split.len() {
        let in_country: Vec<bool> = owner.iter().map(|o| *o == Some(c)).collect();
        let tiles = tiles_of(grid, &in_country);
        let regions = usize::try_from(p.regions.get(c).copied().unwrap_or(0)).map_err(|e| e.to_string())?;
        let targets = apportion(count(tiles.len()), &vec![1; regions]);
        let seeds = pick_seeds(&tiles, regions, draws);
        let mut region = grow(grid, &in_country, &seeds, &targets, lot);
        join_nearest(grid, &mut region, &in_country, lot);
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
            let mut zone = grow(grid, &in_region, &z_seeds, &z_targets, lot);
            join_nearest(grid, &mut zone, &in_region, lot);
            for z in 0..zones {
                let z_tiles = tiles_of(grid, &zone.iter().map(|o| *o == Some(z)).collect::<Vec<_>>());
                let size = count(z_tiles.len());
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
    let height = heights(p, &grid, draws);
    let lot: Vec<u64> = (0..grid.len()).map(|_| draws.next_u64()).collect();
    let (land, elevation) = surface(p, &height)?;
    let terrain = terrain(p, &grid, &elevation);
    let to_sea = sea_distance(&grid, &land);
    let (owner, on_mainland) = countries(p, &grid, &land, &lot, draws)?;
    let places = places(p, &grid, &owner, &on_mainland, &lot, draws)?;
    let rows = u64::from(side - 1);
    let tiles = (0..grid.len())
        .map(|index| {
            let (_, y) = grid.xy(grid.tile(index));
            let north = u32::try_from(u64::from(side - 1 - y) * PER_MILLE / rows).unwrap_or(0);
            let elevation_m = at(&elevation, index);
            let input = ClimateInput { north_permille: north, elevation_m, sea_distance_m: at(&to_sea, index) };
            let surface = if at(&land, index) { LAND } else { WATER };
            Tile::new(elevation_m, surface, at(&terrain, index), climate(input), at(&places.zone_of, index))
        })
        .collect();
    Ok(Map { grid, tiles, zones: places.zones, regions: places.regions, rejections: Vec::new(), attempt: 0 })
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
mod tests {
    use phx_core::{Purpose, StreamDecl, Streams};
    use phx_id::Day;
    use phx_rand::{Draws, Seed, Subject, SubjectTag};

    use super::{MapParams, TerrainClass, apportion, generate};
    use crate::partition::components;

    const MAP: StreamDecl = StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };

    #[test]
    fn apportion_by_largest_remainder() {
        assert_eq!(apportion(25, &[50, 30, 20]), vec![13, 7, 5]);
        assert_eq!(apportion(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(apportion(7, &[0, 0]), vec![0, 0]);
    }

    #[test]
    fn a_small_map_meets_its_conditions() {
        let p = MapParams {
            land_tiles: 900,
            tile_m: 10_000,
            sea_share: 0.4,
            base_cells: 3,
            octaves: 4,
            roughness: 0.5,
            falloff: 2.0,
            max_elevation_m: 3_000.0,
            max_depth_m: 4_000.0,
            terrain: vec![
                TerrainClass { max_elevation_m: 300, max_slope_permille: 20 },
                TerrainClass { max_elevation_m: 1_500, max_slope_permille: 80 },
                TerrainClass { max_elevation_m: i16::MAX, max_slope_permille: u32::MAX },
            ],
            split: vec![50, 30, 20],
            regions: vec![5, 3, 3],
            zones: 45,
            zone_min_tiles: 5,
            zone_max_tiles: 60,
            mainland_floor_percent: 80,
            max_attempts: 50,
        };
        let streams = Streams::new(Seed::new(7), &[MAP]).unwrap();
        let draws = |a: u64| -> Draws { streams.open(&MAP, Subject::new(SubjectTag::World, a), Day::new(0), 0) };
        let map = generate(&p, &|c| u8::from(c.elevation_m > 1_000), &draws);
        let land = map.tiles.iter().filter(|t| t.is_land()).count();
        assert_eq!(land, 900, "land is the declared count");
        assert!(
            map.tiles.iter().all(|t| t.is_land() == t.zone().is_some()),
            "every land tile, and only land, has a zone"
        );
        assert_eq!((map.regions.len(), map.zones.len()), (11, 45));
        for (z, zone) in map.zones.iter().enumerate() {
            let mask: Vec<bool> = map
                .tiles
                .iter()
                .map(|t| t.zone().map(phx_id::ZoneId::get) == Some(u32::try_from(z).unwrap()))
                .collect();
            assert!((5..=60).contains(&u64::from(zone.tiles)));
            assert!(mask[map.grid.index(zone.centroid)], "a zone's centroid is its own tile");
            assert!(!components(&map.grid, &mask).1.is_empty(), "zone {z} holds land");
        }
        let again = generate(&p, &|c| u8::from(c.elevation_m > 1_000), &draws);
        assert_eq!(map, again, "the same seed gives the same map");
    }
}
