//! The setup's countries as the opening reads them.

use phx_core::OpeningCountry;
use phx_geo::GeoState;
use phx_id::{CountryId, TileId};

use crate::consts::WHOLE;
use crate::opening::newgame::NewGame;

/// Each country as the opening's contributions read it: its people, the setup's split of the world's persons; its
/// derived values; and its land tiles, where its parties are sited.
#[must_use]
pub fn countries(game: &NewGame, geo: &GeoState, persons: u64, units: &[u64]) -> Vec<OpeningCountry> {
    let map = &geo.map;
    let region_of = |i: usize| {
        map.tiles
            .get(i)
            .and_then(phx_geo::tile::Tile::zone)
            .and_then(|z| map.zones.get(usize::try_from(z.get()).ok()?))
            .map(|z| z.region.get())
            .and_then(|r| map.regions.get(usize::from(r)).map(|x| (u32::from(r), x.country)))
    };
    let mut sites: Vec<Vec<TileId>> = vec![Vec::new(); game.countries.len()];
    let mut regions: Vec<Vec<(u32, Vec<TileId>)>> = vec![Vec::new(); game.countries.len()];
    for i in 0..map.tiles.len() {
        if let Some((r, c)) = region_of(i)
            && let (Some(s), Some(rs)) = (sites.get_mut(usize::from(c.get())), regions.get_mut(usize::from(c.get())))
        {
            let tile = map.grid.tile(i);
            s.push(tile);
            match rs.iter_mut().find(|(id, _)| *id == r) {
                Some((_, land)) => land.push(tile),
                None => rs.push((r, vec![tile])),
            }
        }
    }
    for rs in &mut regions {
        rs.sort_by_key(|(r, _)| *r);
    }
    game.countries
        .iter()
        .zip(&game.setup.split)
        .zip(sites.into_iter().zip(regions))
        .zip(0_u8..)
        .map(|(((country, share), (sites, regions)), i)| {
            let people = persons * share / WHOLE;
            let per_head = country.derived.iter().find(|(n, _)| n == "GEN.gdp_per_head").map(|(_, v)| *v);
            let unit = units.get(usize::from(i)).copied().map(phx_rand::float::from_u64);
            let (Some(per_head), Some(unit)) = (per_head, unit) else {
                phx_num::violation!(clause = "GEN.15", "a country with no GDP per head or no unit", country = i);
            };
            OpeningCountry {
                id: CountryId::new(i),
                people,
                gdp: phx_rand::float::from_u64(people) * per_head * unit,
                derived: country.derived.clone(),
                sites,
                regions,
            }
        })
        .collect()
}
