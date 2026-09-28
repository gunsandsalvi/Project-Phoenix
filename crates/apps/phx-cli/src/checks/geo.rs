//! The map's checks: the surface closed and every place in one region and country, and the map the new game
//! accepted meeting every condition its rejections name.

use phx_geo::GeoState;
use phx_geo::partition::components;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

fn country_of(geo: &GeoState, zone: phx_id::ZoneId) -> Option<(usize, phx_id::CountryId)> {
    let z = geo.map.zones.get(usize::try_from(zone.get()).ok()?)?;
    let region = usize::from(z.region.get());
    Some((region, geo.map.regions.get(region)?.country))
}

fn places(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    let countries = w.countries().len();
    let grid = &geo.map.grid;
    for i in 0..grid.len() {
        let t = grid.tile(i);
        let around: Vec<phx_id::TileId> = grid.neighbours(t).collect();
        let distinct =
            around.iter().enumerate().all(|(k, n)| *n != t && !around.get(..k).is_some_and(|a| a.contains(n)));
        if around.len() != phx_geo::grid::DIRECTIONS
            || !distinct
            || !around.iter().all(|n| grid.neighbours(*n).any(|m| m == t))
        {
            return Outcome::Fail(format!(
                "tile {i} lacks eight neighbours that each count it back: the surface is not closed"
            ));
        }
    }
    for (i, tile) in geo.map.tiles.iter().enumerate() {
        let placed = tile.zone().and_then(|z| country_of(geo, z));
        match (tile.is_land(), placed) {
            (true, Some((_, c))) if usize::from(c.get()) < countries => {}
            (false, None) => {}
            (land, _) => return Outcome::Fail(format!("tile {i} (land: {land}) lies in no region of a country")),
        }
    }
    let bare = (0..geo.map.regions.len()).find(|r| !geo.map.zones.iter().any(|z| usize::from(z.region.get()) == *r));
    bare.map_or(Outcome::Pass, |r| Outcome::Fail(format!("region {r} has no land")))
}

fn map_conditions(w: Inspector<'_>) -> Outcome {
    let (geo, p) = (w.geo(), &w.geo().params);
    let map = &geo.map;
    let listed: Vec<u64> = map.rejections.iter().map(|r| r.attempt).collect();
    if listed != (0..map.attempt).collect::<Vec<_>>() || map.rejections.iter().any(|r| r.condition.trim().is_empty()) {
        return Outcome::Fail(format!(
            "the rejections {listed:?} do not list attempts 0 to {} with conditions",
            map.attempt
        ));
    }
    let land: Vec<bool> = map.tiles.iter().map(phx_geo::tile::Tile::is_land).collect();
    if land.iter().filter(|l| **l).count() != usize::try_from(p.land_tiles).unwrap_or(usize::MAX) {
        return Outcome::Fail("the land is not its declared count".to_owned());
    }
    if let Some(z) =
        map.zones.iter().find(|z| u64::from(z.tiles) < p.zone_min_tiles || u64::from(z.tiles) > p.zone_max_tiles)
    {
        return Outcome::Fail(format!("a zone of {} tiles", z.tiles));
    }
    let tolerance = p.share_tolerance_per_mille;
    let within = |size: usize, like: u64| {
        u64::try_from(size).unwrap_or(u64::MAX).abs_diff(like) * phx_geo::consts::PER_MILLE <= like * tolerance
    };
    let (label, sizes) = components(&map.grid, &land);
    let biggest = sizes.iter().enumerate().fold((0, 0), |b, (i, s)| if *s > b.1 { (i, *s) } else { b }).0;
    let mainland: Vec<bool> = label.iter().map(|l| *l == Some(biggest)).collect();
    let on_main = |i: usize| mainland.get(i).copied().unwrap_or(false);
    let region_of = |i: usize| map.tiles.get(i).and_then(phx_geo::tile::Tile::zone).and_then(|z| country_of(geo, z));
    for r in 0..map.regions.len() {
        let mask: Vec<bool> =
            (0..map.tiles.len()).map(|i| on_main(i) && region_of(i).is_some_and(|(g, _)| g == r)).collect();
        if components(&map.grid, &mask).1.len() > 1 {
            return Outcome::Fail(format!("region {r} is in more than one piece on the mainland"));
        }
    }
    let shares = phx_geo::partition::apportion(p.land_tiles, &p.split);
    for c in 0..w.countries().len() {
        let of = |i: &usize| region_of(*i).is_some_and(|(_, k)| usize::from(k.get()) == c);
        let all = (0..map.tiles.len()).filter(of).count();
        if !within(all, shares.get(c).copied().unwrap_or(0)) {
            return Outcome::Fail(format!("country {c} holds {all} tiles, beyond the tolerance of its share"));
        }
        let regions: Vec<usize> = (0..map.regions.len())
            .filter(|r| map.regions.get(*r).is_some_and(|g| usize::from(g.country.get()) == c))
            .collect();
        let sizes: Vec<usize> = regions
            .iter()
            .map(|r| (0..map.tiles.len()).filter(|i| region_of(*i).is_some_and(|(g, _)| g == *r)).count())
            .collect();
        let like = phx_geo::partition::apportion(u64::try_from(all).unwrap_or(0), &vec![1; sizes.len()]);
        if let Some((r, size)) =
            regions.iter().zip(&sizes).zip(&like).find(|((_, s), l)| !within(**s, **l)).map(|((r, s), _)| (r, s))
        {
            return Outcome::Fail(format!(
                "region {r} of country {c} holds {size} tiles, beyond the tolerance of like size"
            ));
        }
        let main = (0..map.tiles.len()).filter(|i| on_main(*i) && of(i)).count();
        if u64::try_from(main * 100).unwrap_or(0) < p.mainland_floor_percent * u64::try_from(all).unwrap_or(0) {
            return Outcome::Fail(format!("country {c} holds {main} of {all} tiles on the mainland"));
        }
    }
    Outcome::Pass
}

pub const LC_0_11: Check = live_check! {
    id: "LC-0-11",
    title: "The surface is closed, every tile with eight neighbours that count it back; every land tile's zone is in one region and country; every region has land; every site lies in its country and region",
    from_step: "S0.13",
    check: places,
};

pub const LC_0_12: Check = live_check! {
    id: "LC-0-12",
    title: "The rejection record lists every failed attempt with its condition; the accepted map meets every condition",
    from_step: "S0.13",
    check: map_conditions,
};
