use phx_core::Register;
use phx_macros::clause;
use phx_num::violation;

use crate::climate::cell2;
use crate::generate::{Map, sea_distance};
use crate::prims::GeoPrims;

/// Each tile's exposure class to each declared hazard, one column per hazard, none on water: read from its terrain
/// and climate class, by the coastal table within the declared distance of the sea and the inland one beyond it.
#[clause("GEO.7")]
#[must_use]
pub fn exposure(p: &GeoPrims, r: &Register, map: &Map) -> Vec<Vec<Option<u8>>> {
    let land: Vec<bool> = map.tiles.iter().map(crate::tile::Tile::is_land).collect();
    let to_sea = sea_distance(&map.grid, &land);
    let coast_m = p.coast_m.shared(r).get();
    p.hazards
        .iter()
        .map(|h| {
            map.tiles
                .iter()
                .zip(&to_sea)
                .map(|(t, d)| {
                    if !t.is_land() {
                        return None;
                    }
                    let table = if *d <= coast_m { h.coastal.shared(r) } else { h.inland.shared(r) };
                    let class = cell2(table, i64::from(t.terrain), i64::from(t.climate));
                    let Ok(c) = u8::try_from(class) else {
                        violation!(clause = "GEO.7", "an exposure class beyond a byte", class = class);
                    };
                    Some(c)
                })
                .collect()
        })
        .collect()
}
