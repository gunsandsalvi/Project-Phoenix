use phx_core::{Event, annual_to_daily};
use phx_geo::audit::DEPOSIT_TABLE;
use phx_geo::audit::facts::{Extracted, Opening, Remaining};
use phx_geo::partition::components;
use phx_geo::weather::VARIABLES;
use phx_geo::{GeoState, hazards::HAZARDS};
use phx_id::{Date, Slot};
use phx_num::Missing;
use phx_rand::float::from_u64;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Standard errors a realised mean or count may stray from its declared value before the check fails: at 6.1, the
/// chance a correct mechanism fails any of the run's checks is below one in a hundred million.
const Z: f64 = 6.1;

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

fn events_of(w: Inspector<'_>, name: &str) -> Vec<Event> {
    let Some(kind) = w.event_kinds().iter().position(|k| k.name == name) else { return Vec::new() };
    (1..=w.events().len())
        .filter_map(|id| u64::try_from(id).ok())
        .map(|id| w.events().get(id))
        .filter(|e| usize::from(e.kind) == kind)
        .collect()
}

fn weather_within(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    for (v, var) in VARIABLES.iter().enumerate() {
        let events = events_of(w, var.event.name);
        for (r, climate) in geo.regions.iter().enumerate() {
            for month in 1..=12_u8 {
                let values: Vec<f64> = events
                    .iter()
                    .filter(|e| w.date(e.day).month() == month)
                    .flat_map(|e| e.details.iter())
                    .filter(|(s, _)| {
                        phx_rand::Subject::from_raw(*s).is_some_and(|s| {
                            s.tag() == phx_rand::SubjectTag::Region && s.id() == u64::try_from(r).unwrap_or(u64::MAX)
                        })
                    })
                    .map(|(_, size)| phx_rand::float::from_i64(*size) / var.units_per)
                    .collect();
                if values.is_empty() {
                    continue;
                }
                let (Some(m), Some(phi)) = (climate.month(month).get(v), climate.persistence.get(v).copied()) else {
                    return Outcome::Fail(format!("variable {v} undeclared"));
                };
                let (mean, var_x) = m.moments();
                let n = phx_rand::float::len_u64(values.len());
                let observed = values.iter().sum::<f64>() / from_u64(n);
                let variance = var_x / from_u64(n) * (1.0 + phi) / (1.0 - phi);
                let gap = observed - mean;
                if gap * gap > Z * Z * variance {
                    return Outcome::Fail(format!(
                        "region {r}, {}, month {month}: mean {observed:.3} against {mean:.3} (variance {variance:.5})",
                        var.event.name
                    ));
                }
            }
        }
    }
    Outcome::Pass
}

fn days_in_year(date: Date) -> u32 {
    (1..=12).filter_map(|m| Date::days_in_month(date.year(), m)).map(u32::from).sum()
}

fn catastrophe_rates(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    let days: Vec<Date> = {
        let mut d = w.day_zero().succ();
        let mut out = Vec::new();
        while d <= w.today() {
            out.push(w.date(d));
            d = d.succ();
        }
        out
    };
    for (h, spec) in geo.hazards.iter().zip(HAZARDS) {
        let expected: f64 = days
            .iter()
            .map(|date| {
                h.by_country
                    .iter()
                    .flat_map(|classes| classes.iter().zip(&h.rate))
                    .map(|(tiles, rate)| {
                        phx_rand::float::from_u64(phx_rand::float::len_u64(tiles.len()))
                            * annual_to_daily(*rate, days_in_year(*date))
                    })
                    .sum::<f64>()
            })
            .sum();
        let observed = from_u64(phx_rand::float::len_u64(events_of(w, spec.event.name).len()));
        let gap = observed - expected;
        if gap * gap > Z * Z * expected {
            return Outcome::Fail(format!("{}: {observed} events against {expected:.2} expected", spec.event.name));
        }
    }
    Outcome::Pass
}

fn deposits_balance(w: Inspector<'_>) -> Outcome {
    use phx_core::FactDef;
    let Some(table) = w.table(DEPOSIT_TABLE) else {
        return Outcome::Fail("the world keeps no deposit table".to_owned());
    };
    for row in 0..table.rows() {
        let slot = Slot::new(row);
        let read = |name| match table.value(name, slot) {
            Missing::Present(v) => Some(i128::from(v)),
            Missing::Absent => None,
        };
        let quantities = (read(Opening::ITEM.name), read(Extracted::ITEM.name), read(Remaining::ITEM.name));
        let (Some(opening), Some(extracted), Some(remaining)) = quantities else {
            return Outcome::Fail(format!("deposit {row} has a missing quantity"));
        };
        if extracted + remaining != opening {
            return Outcome::Fail(format!(
                "deposit {row}: {extracted} extracted and {remaining} remaining of {opening}"
            ));
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

pub const LC_0_13: Check = live_check! {
    id: "LC-0-13",
    title: "Each region's realised weather is within z = 6.1 of its declared climate for the month, adjusted for persistence",
    from_step: "S0.13",
    check: weather_within,
};

pub const LC_0_14: Check = live_check! {
    id: "LC-0-14",
    title: "Catastrophe frequencies per hazard are within z = 6.1 of their declared rates over the run",
    from_step: "S0.13",
    check: catastrophe_rates,
};

pub const LC_0_15: Check = live_check! {
    id: "LC-0-15",
    title: "For every finite deposit, extracted plus remaining equals its opening quantity",
    from_step: "S0.13",
    check: deposits_balance,
};
