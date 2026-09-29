//! The map's checks: the surface closed and every place in one region and country, the map the new game accepted
//! meeting every condition its rejections name, the weather within its climate, the catastrophes at their rates with
//! their losses clustered, and a drought's place pricing first.

use phx_core::{Event, annual_to_daily};
use phx_geo::GeoState;
use phx_geo::hazards::HAZARDS;
use phx_geo::partition::components;
use phx_geo::weather::VARIABLES;
use phx_id::Date;
use phx_num::Missing;
use phx_rand::float::from_u64;
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

/// Standard errors a realised mean or count may stray from its declared value before the check fails: at 6.1, the
/// chance a correct mechanism fails any of the run's checks is below one in a hundred million.
const Z: f64 = 6.1;

/// Every event of a kind, by its name.
fn events_of(w: Inspector<'_>, name: &str) -> Vec<Event> {
    let Some(kind) = w.event_kinds().iter().position(|k| *k == name) else { return Vec::new() };
    (1..=w.events().len())
        .filter_map(|id| u64::try_from(id).ok())
        .map(|id| w.events().get(id))
        .filter(|e| usize::from(e.kind) == kind)
        .collect()
}

/// Each region's mean of each weather variable in each month is within `Z` standard errors of its climate's, the
/// error widened by the variable's persistence.
fn weather_within(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    let mut read = 0_u64;
    for (v, var) in VARIABLES.iter().enumerate() {
        let events = events_of(w, var.event.name);
        for (r, climate) in geo.regions.iter().enumerate() {
            for month in 1..=phx_geo::consts::MONTHS_U8 {
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
                read += n;
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
    if read == 0 { Outcome::NotYet("the run recorded no weather") } else { Outcome::Pass }
}

/// Every hazard's events number what its tiles' daily chances over the run's days expect, within `Z` standard errors;
/// then its losses cluster.
fn catastrophe_rates(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    let mut days: Vec<Date> = Vec::new();
    let mut d = w.day_zero().succ();
    while d <= w.today() {
        days.push(w.date(d));
        d = d.succ();
    }
    if days.is_empty() {
        return Outcome::NotYet("the run closed no day");
    }
    for (h, spec) in geo.hazards.iter().zip(HAZARDS) {
        let expected: f64 = days
            .iter()
            .map(|date| {
                h.by_country
                    .iter()
                    .flat_map(|classes| classes.iter().zip(&h.rate))
                    .map(|(tiles, rate)| {
                        from_u64(phx_rand::float::len_u64(tiles.len()))
                            * annual_to_daily(*rate, phx_geo::catastrophe::days_in_year(*date))
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
    losses_cluster(w, &days)
}

/// A catastrophe's losses cluster in place and time: their shares over the regions and months of the run are more
/// concentrated than the same losses spread evenly over them, their Herfindahl index above one over the cells.
fn losses_cluster(w: Inspector<'_>, days: &[Date]) -> Outcome {
    let geo = w.geo();
    let mut cells: std::collections::BTreeMap<(u32, i32, u8), u64> = std::collections::BTreeMap::new();
    for spec in HAZARDS {
        for e in events_of(w, spec.event.name) {
            let date = w.date(e.day);
            for (subject, share) in &e.details {
                let tile = phx_rand::Subject::from_raw(*subject).and_then(|s| u32::try_from(s.id()).ok());
                let region = tile.map(phx_id::TileId::new).map(|t| match geo.zone_of(t) {
                    Missing::Present(z) => geo.zone_region(z),
                    Missing::Absent => Missing::Absent,
                });
                let (Some(Missing::Present(region)), Ok(share)) = (region, u64::try_from(*share)) else {
                    return Outcome::Fail(format!("{}: a loss on no region's land", spec.event.name));
                };
                *cells.entry((region, date.year(), date.month())).or_insert(0) += share;
            }
        }
    }
    let total: u64 = cells.values().sum();
    if total == 0 {
        return Outcome::NotYet("no catastrophe destroyed anything in the run");
    }
    let months: std::collections::BTreeSet<(i32, u8)> = days.iter().map(|d| (d.year(), d.month())).collect();
    let cells_run = from_u64(phx_rand::float::len_u64(months.len() * geo.map.regions.len()));
    let index: f64 = cells.values().map(|l| (from_u64(*l) / from_u64(total)).powi(2)).sum();
    if index * cells_run > 1.0 {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("catastrophe losses spread evenly: an index of {index:.4} over {cells_run} cells"))
    }
}

/// For every drought and every product whose mark rose after it at a region it struck, whether no region it did not
/// strike saw the product's mark rise sooner: the check passes when the struck place rose first in most such cases.
fn drought_prices_first(w: Inspector<'_>) -> Outcome {
    let Some(drought) = w.event_kinds().iter().position(|k| *k == "GEO.drought") else {
        return Outcome::Fail("the world declares no drought".to_owned());
    };
    let (mut first, mut cases) = (0_u32, 0_u32);
    for shock in w.core().weather.shocks.iter().filter(|s| usize::from(s.kind) == drought) {
        for ((product, region), there) in shock.rose.iter().filter(|((_, r), _)| shock.regions.contains(r)) {
            let earlier = shock
                .rose
                .iter()
                .any(|((p, r), d)| p == product && !shock.regions.contains(r) && d < there && r != region);
            cases += 1;
            if !earlier {
                first += 1;
            }
        }
    }
    if cases == 0 {
        return Outcome::NotYet("no drought struck a region whose marks rose after it in the run");
    }
    if first * 2 <= cases {
        return Outcome::Fail(format!("the struck place's price rose first in {first} of {cases} cases"));
    }
    Outcome::Pass
}

/// Every finite deposit has given and holds, together, what it opened with.
fn deposits_balance(w: Inspector<'_>) -> Outcome {
    let d = &w.core().deposits;
    for (i, ((open, left), given)) in d.opening.iter().zip(&d.remaining).zip(&d.extracted).enumerate() {
        match (open, left) {
            (Some(open), Some(left)) if given + left != *open => {
                return Outcome::Fail(format!("deposit {i}: {given} extracted and {left} remaining of {open}"));
            }
            (Some(_), None) | (None, Some(_)) => {
                return Outcome::Fail(format!("deposit {i} is finite on one side of its record only"));
            }
            _ => {}
        }
    }
    if d.opening.is_empty() { Outcome::Fail("the map holds no deposit".to_owned()) } else { Outcome::Pass }
}

/// The deposits family found nothing at any close, and some deposit was worked.
fn deposits_clean(w: Inspector<'_>) -> Outcome {
    if let Some(f) = w.findings().iter().find(|f| f.family == "deposits") {
        return Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail));
    }
    match deposits_balance(w) {
        Outcome::Pass if w.core().deposits.extracted.iter().all(|e| *e == 0) => {
            Outcome::NotYet("no deposit was worked in the run")
        }
        other => other,
    }
}

pub const LC_0_15: Check = live_check! {
    id: "LC-0-15",
    title: "For every finite deposit, extracted plus remaining equals its opening quantity",
    from_step: "S0.13",
    check: deposits_balance,
};

pub const LC_1_14: Check = live_check! {
    id: "LC-1-14",
    title: "for every finite deposit, extracted plus remaining equals its opening quantity: the family of deposits \
            (GEO.12) is clean",
    from_step: "S1.05",
    check: deposits_clean,
};

pub const LC_0_13: Check = live_check! {
    id: "LC-0-13",
    title: "Each region's realised weather is within z = 6.1 of its declared climate for the month, adjusted for persistence",
    from_step: "S0.13",
    check: weather_within,
};

pub const LC_0_14: Check = live_check! {
    id: "LC-0-14",
    title: "Catastrophe frequencies per hazard are within z = 6.1 of their declared rates over the run, and their losses cluster in place and time",
    from_step: "S0.13",
    check: catastrophe_rates,
};

pub const LC_1_46: Check = live_check! {
    id: "LC-1-46",
    title: "when a drought strikes one place, the price there rises before prices elsewhere (GDS.9)",
    from_step: "S1.05",
    check: drought_prices_first,
};
