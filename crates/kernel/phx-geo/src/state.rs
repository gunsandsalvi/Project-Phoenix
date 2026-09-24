use phx_core::{OpeningCtx, OpeningPhase, Purpose, Register, StreamDecl};
use phx_id::TileId;
use phx_macros::clause;
use phx_rand::{Subject, SubjectTag};

use crate::climate::{ClimateRule, RegionClimate, cell1, exp_of, scaled};
use crate::consts::MAP_MAX_ATTEMPTS;
use crate::deposits::{Deposit, draw, everywhere};
use crate::distance::ZoneDistances;
use crate::exposure::exposure;
use crate::generate::{HeightCurve, Map, MapParams, TerrainClass, generate};
use crate::relief::ReliefParams;
use crate::hazards::HAZARDS;
use crate::prims::GeoPrims;

/// The stream each map attempt draws from, subject the attempt's number.
pub const MAP_STREAM: StreamDecl =
    StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };

/// The map's step of the opening, after the setup's.
pub const MAP_PHASE: OpeningPhase = OpeningPhase(1);

/// A hazard as the day reads it: the event it records, its exposure column, and by exposure class its yearly
/// chance, spread chance and severity shapes, and each country's tiles of each class.
#[derive(Clone, Debug, PartialEq)]
pub struct HazardState {
    pub event_kind: u16,
    pub exposure: Vec<Option<u8>>,
    pub rate: Vec<f64>,
    pub spread: Vec<f64>,
    pub severity: Vec<(f64, f64)>,
    pub by_country: Vec<Vec<Vec<TileId>>>,
}

/// What GEO compiles at the opening, which only its own handlers and families are given: the map's parameters, the
/// accepted map, its zone
/// distances, the regions' climates, the hazards, the deposits, and the event kinds the weather records.
#[clause("GEO.1", "GEO.2", "GEO.6", "GEO.7")]
#[derive(Debug)]
pub struct GeoState {
    pub params: MapParams,
    pub map: Map,
    pub distances: ZoneDistances,
    pub regions: Vec<RegionClimate>,
    pub hazards: Vec<HazardState>,
    pub deposits: Vec<Deposit>,
    pub weather_kinds: Vec<u16>,
}

/// What the opening gives the map: each country's land and regions, allotted by the setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allotment {
    pub land: Vec<u64>,
    pub regions: Vec<u64>,
}

fn count(n: u64) -> Result<u32, String> {
    u32::try_from(n).map_err(|e| e.to_string())
}

fn curve(t: &phx_core::Table1) -> HeightCurve {
    HeightCurve { axis: t.axis().to_vec(), metres: t.values().to_vec() }
}

/// The map's parameters, read from the register and the allotment.
///
/// # Errors
/// A value beyond its field's width, or terrain tables of unequal lengths.
pub fn params(p: &GeoPrims, r: &Register, a: &Allotment) -> Result<MapParams, String> {
    let (elevation, relief) = (p.terrain_elevation.shared(r), p.terrain_relief.shared(r));
    if elevation.axis() != relief.axis() {
        return Err("the terrain classes' elevation and relief tables list different classes".to_owned());
    }
    let terrain = elevation
        .axis()
        .iter()
        .map(|c| {
            Ok(TerrainClass {
                max_elevation_m: i16::try_from(cell1(elevation, *c)).map_err(|e| e.to_string())?,
                max_relief_m: u32::try_from(cell1(relief, *c)).map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let relief_params = ReliefParams {
        base_cells: count(p.base_cells.shared(r).get())?,
        octaves: u8::try_from(p.octaves.shared(r).get()).map_err(|e| e.to_string())?,
        roughness: p.roughness.shared(r).to_f64(),
        falloff: p.falloff.shared(r).to_f64(),
        plates: count(p.plates.shared(r).get())?,
        belt: p.plate_belt.shared(r).to_f64(),
        plate_weight: p.plate_weight.shared(r).to_f64(),
        mountain_weight: p.mountain_weight.shared(r).to_f64(),
        warp: p.warp.shared(r).to_f64(),
        erosion_passes: count(p.erosion_passes.shared(r).get())?,
        erosion_rate: p.erosion_rate.shared(r).to_f64(),
        area_exponent: p.area_exponent.shared(r).to_f64(),
    };
    Ok(MapParams {
        land_tiles: a.land.iter().sum(),
        tile_m: count(p.tile_m.shared(r).get())?,
        sea_share: p.sea_share.shared(r).to_f64(),
        cells_per_tile: count(p.relief_cells.shared(r).get())?,
        relief: relief_params,
        land_heights: curve(p.land_heights.shared(r)),
        sea_depths: curve(p.sea_depths.shared(r)),
        terrain,
        river_tiles: count(p.river_tiles.shared(r).get())?,
        rugged_m: p.rugged_m.shared(r).get(),
        river_crossing_m: p.river_crossing_m.shared(r).get(),
        split: a.land.clone(),
        regions: a.regions.clone(),
        zones: p.zones.shared(r).get(),
        zone_min_tiles: p.zone_min_tiles.shared(r).get(),
        zone_max_tiles: p.zone_max_tiles.shared(r).get(),
        share_tolerance_per_mille: p.share_tolerance.shared(r).get(),
        mainland_floor_percent: p.mainland_floor.shared(r).get(),
        max_attempts: MAP_MAX_ATTEMPTS,
    })
}

/// A hazard's tables read by exposure class, which the rate table's axis lists as 0, 1, 2 and so on.
fn hazard(
    p: &GeoPrims,
    r: &Register,
    map: &Map,
    index: usize,
    column: Vec<Option<u8>>,
    event_kind: u16,
) -> Result<HazardState, String> {
    let (Some(tables), Some(spec)) = (p.hazards.get(index), HAZARDS.get(index)) else {
        return Err(format!("hazard {index} has no tables"));
    };
    let [_, _, _, rate_decl, spread_decl, a_decl, b_decl] = spec.tables;
    let rate_table = tables.rate.shared(r);
    let classes = rate_table.axis();
    if classes.iter().zip(0_i64..).any(|(c, i)| *c != i) {
        return Err(format!("`{}`'s exposure classes are not numbered from 0 in turn", spec.decl.name));
    }
    let by_class = |t: &phx_core::Table1, decl| classes.iter().map(|c| scaled(cell1(t, *c), exp_of(decl))).collect();
    let rate: Vec<f64> = by_class(rate_table, rate_decl);
    let spread = by_class(tables.spread.shared(r), spread_decl);
    let first: Vec<f64> = by_class(tables.severity_a.shared(r), a_decl);
    let second: Vec<f64> = by_class(tables.severity_b.shared(r), b_decl);
    let countries =
        map.regions.iter().map(|g| usize::from(g.country.get()) + 1).fold(0, |m, c| if c > m { c } else { m });
    let mut by_country = vec![vec![Vec::new(); classes.len()]; countries];
    for (i, class) in column.iter().enumerate() {
        let Some(class) = class else { continue };
        let tile = map.grid.tile(i);
        let country = map
            .tiles
            .get(i)
            .and_then(crate::tile::Tile::zone)
            .and_then(|z| map.zones.get(usize::try_from(z.get()).ok()?))
            .and_then(|z| map.regions.get(usize::from(z.region.get())))
            .map(|g| usize::from(g.country.get()))
            .ok_or_else(|| format!("land tile {} lies in no country", tile.get()))?;
        let list = by_country
            .get_mut(country)
            .and_then(|c: &mut Vec<Vec<TileId>>| c.get_mut(usize::from(*class)))
            .ok_or_else(|| format!("`{}` exposure class {class} has no rate", spec.decl.name))?;
        list.push(tile);
    }
    Ok(HazardState {
        event_kind,
        exposure: column,
        rate,
        spread,
        severity: first.into_iter().zip(second).collect(),
        by_country,
    })
}

impl GeoState {
    /// The map generated from the seed, in the map's phase of the opening, and everything read from it; every refusal
    /// of the data at once.
    ///
    /// # Errors
    /// A resource declared on every terrain, tables that disagree, or an event kind never declared.
    pub fn build(
        p: &GeoPrims,
        r: &Register,
        a: &Allotment,
        ctx: &OpeningCtx<'_>,
        event_kind: &dyn Fn(&str) -> Option<u16>,
    ) -> Result<GeoState, Vec<String>> {
        let one = |e: String| vec![e];
        let mut errors = everywhere(p, r);
        let map_params = params(p, r, a).map_err(one)?;
        let side = crate::generate::side(map_params.land_tiles, map_params.sea_share);
        let rule = ClimateRule::read(p, r, u64::from(side) * u64::from(map_params.tile_m)).map_err(one)?;
        let map = generate(&map_params, &|input| rule.class(input), &|attempt| {
            ctx.draws(&MAP_STREAM, Subject::new(SubjectTag::World, attempt))
        });
        let kind = |name: &str| event_kind(name).ok_or_else(|| format!("event kind `{name}` is never declared"));
        let mut hazards = Vec::new();
        for (i, (column, spec)) in exposure(p, r, &map).into_iter().zip(HAZARDS).enumerate() {
            match kind(spec.event.name).and_then(|k| hazard(p, r, &map, i, column, k)) {
                Ok(h) => hazards.push(h),
                Err(e) => errors.push(e),
            }
        }
        let weather_kinds: Vec<u16> = crate::weather::VARIABLES
            .iter()
            .filter_map(|v| kind(v.event.name).map_err(|e| errors.push(e)).ok())
            .collect();
        if !errors.is_empty() {
            return Err(errors);
        }
        let deposits = draw(p, r, &map, ctx);
        let regions = crate::climate::regions(p, r, &map);
        let distances = ZoneDistances::measure(&map);
        Ok(GeoState { params: map_params, map, distances, regions, hazards, deposits, weather_kinds })
    }

    /// Bytes the map holds: tiles, exposure columns, distances and deposits.
    #[must_use]
    pub fn bytes(&self) -> usize {
        let tiles = self.map.tiles.len() * size_of::<crate::tile::Tile>();
        let exposure: usize = self.hazards.iter().map(|h| h.exposure.len() * size_of::<Option<u8>>()).sum();
        tiles + exposure + self.distances.bytes() + self.deposits.len() * size_of::<Deposit>()
    }
}
