use std::collections::{BTreeSet, VecDeque};

use phx_core::{EventIntent, annual_to_daily, declare_stream};
use phx_id::{Date, TileId};
use phx_macros::clause;
use phx_num::{Fixed, Round, violation};
use phx_rand::{Draws, Subject, SubjectTag, accept, below_u64, beta, binomial};

use crate::consts::PER_MILLE_F64;
use crate::grid::Grid;
use crate::state::{GeoState, HazardState};

declare_stream! { pub CatastropheStream = "GEO.catastrophe" { family: World, purpose: Catastrophe, keyed: false, clause: "CHN.3" } }

/// `k` distinct places among `n`, uniformly, by Floyd's method: each draw from a range one wider than the last, a
/// place already taken giving way to the range's new top.
#[must_use]
pub fn distinct_picks(d: &mut Draws, n: u64, k: u64) -> Vec<u64> {
    let Some(start) = n.checked_sub(k) else {
        violation!(clause = "GEO.8", "more picks than places", places = n, picks = k);
    };
    let mut taken = BTreeSet::new();
    let mut order = Vec::new();
    for top in start..n {
        let pick = below_u64(d, top + 1);
        let chosen = if taken.contains(&pick) { top } else { pick };
        taken.insert(chosen);
        order.push(chosen);
    }
    order
}

/// A catastrophe's footprint from its origin: breadth first over the eight neighbours in tile-id order, each land
/// tile not yet tried joining with the hazard's spread chance for its own exposure class.
#[clause("GEO.8")]
#[must_use]
pub fn footprint(grid: &Grid, exposure: &[Option<u8>], spread: &[f64], origin: TileId, d: &mut Draws) -> Vec<TileId> {
    let mut tried = BTreeSet::from([origin.get()]);
    let mut struck = vec![origin];
    let mut queue = VecDeque::from([origin]);
    while let Some(t) = queue.pop_front() {
        let mut around: Vec<TileId> = grid.neighbours(t).collect();
        around.sort_unstable_by_key(|n| n.get());
        for n in around {
            if !tried.insert(n.get()) {
                continue;
            }
            let Some(class) = exposure.get(grid.index(n)).copied().flatten() else { continue };
            let Some(chance) = spread.get(usize::from(class)).copied() else {
                violation!(clause = "GEO.8", "an exposure class with no spread chance", class = class);
            };
            if accept(d, chance) {
                struck.push(n);
                queue.push_back(n);
            }
        }
    }
    struck
}

/// Days in a date's year, over which a yearly chance compounds.
#[must_use]
pub fn days_in_year(date: Date) -> u32 {
    (1..=crate::consts::MONTHS_U8).filter_map(|m| Date::days_in_month(date.year(), m)).map(u32::from).sum()
}

/// A struck tile and its severity in permille, as the day's footprint holds it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct Struck {
    pub tile: TileId,
    pub severity: u32,
}

/// The day's footprint: every tile the day's catastrophes struck with its severity, a day buffer the day's losses
/// read and the next day clears.
pub type Footprint<B = phx_store::SystemBacking> = phx_store::DayBuf<Struck, B>;

/// One hazard's day in one country: per exposure class, the count of origins a binomial over the class's tiles at
/// its daily chance, the origins uniform among them, and each origin's footprint with its tiles' severities, each
/// struck tile also handed to the day's footprint as it is drawn.
#[must_use]
pub fn hazard_day(
    geo: &GeoState,
    (h, country, days): (&HazardState, usize, u32),
    d: &mut Draws,
    mut strike: impl FnMut(Struck),
) -> Vec<EventIntent> {
    let mut out = Vec::new();
    let Some(classes) = h.by_country.get(country) else {
        violation!(clause = "GEO.8", "a country the map does not have", country = country);
    };
    for (tiles, rate) in classes.iter().zip(&h.rate) {
        let places = phx_rand::float::len_u64(tiles.len());
        let origins = binomial(d, places, annual_to_daily(*rate, days));
        for pick in distinct_picks(d, places, origins) {
            let Some(origin) = tiles.get(phx_rand::float::index(pick)).copied() else { continue };
            let struck = footprint(&geo.map.grid, &h.exposure, &h.spread, origin, d);
            let mut details = Vec::with_capacity(struck.len());
            for t in &struck {
                let class = h.exposure.get(geo.map.grid.index(*t)).copied().flatten().map_or(usize::MAX, usize::from);
                let Some((first, second)) = h.severity.get(class).copied() else {
                    violation!(clause = "GEO.8", "an exposure class with no severity", tile = t.get());
                };
                let Ok(share) = Fixed::<0>::from_f64(beta(d, first, second) * PER_MILLE_F64, Round::HalfEven) else {
                    violation!(clause = "GEO.8", "a severity beyond its width");
                };
                let Ok(severity) = u32::try_from(share.raw()) else {
                    violation!(clause = "GEO.8", "a severity below none", tile = t.get());
                };
                strike(Struck { tile: *t, severity });
                details.push((Subject::new(SubjectTag::Tile, u64::from(t.get())), share.raw()));
            }
            let subjects = details.iter().map(|(s, _)| *s).collect();
            out.push(EventIntent { kind: h.event_kind, subjects, details });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_core::{StreamDef, WorldStreams};
    use phx_id::{Day, TileId};
    use phx_rand::{Seed, Subject, SubjectTag, binomial};

    use super::{CatastropheStream, distinct_picks, footprint};
    use crate::grid::Grid;
    use crate::partition::components;

    fn draws(i: u64) -> phx_rand::Draws {
        let streams = WorldStreams::new(Seed::new(9), &[CatastropheStream::DECL]).unwrap();
        streams.open(&CatastropheStream::DECL, Subject::new(SubjectTag::Country, i), Day::new(1), 0)
    }

    #[test]
    fn footprint_connected() {
        let g = Grid { width: 20, height: 20, tile_m: 10_000 };
        let exposure: Vec<Option<u8>> = (0..g.len()).map(|i| (i % 20 != 10).then_some(u8::from(i % 3 == 0))).collect();
        for seed in 0..50 {
            let origin = TileId::new(205);
            let struck = footprint(&g, &exposure, &[0.3, 0.6], origin, &mut draws(seed));
            let mut mask = vec![false; g.len()];
            for t in &struck {
                mask[g.index(*t)] = true;
            }
            assert_eq!(components(&g, &mask).1.len(), 1, "one piece");
            assert!(struck[0] == origin && struck.iter().all(|t| exposure[g.index(*t)].is_some()), "on land");
            let mut ids: Vec<u32> = struck.iter().map(|t| t.get()).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), struck.len(), "each tile struck once");
        }
    }

    #[test]
    fn origin_counts_by_exposure_class_exact() {
        let mut d = draws(1);
        assert_eq!((binomial(&mut d, 500, 0.0), binomial(&mut d, 500, 1.0)), (0, 500), "no chance, every tile");
        for k in [0, 1, 7, 40] {
            let picks = distinct_picks(&mut d, 40, k);
            let mut sorted = picks.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!((picks.len(), sorted.len()), (usize::try_from(k).unwrap(), picks.len()), "{k} distinct picks");
            assert!(picks.iter().all(|p| *p < 40));
        }
        let mut first = [0_u32; 10];
        for i in 0..20_000 {
            first[usize::try_from(distinct_picks(&mut draws(i), 10, 1)[0]).unwrap()] += 1;
        }
        assert!(first.iter().all(|c| (1_800..2_200).contains(c)), "uniform over the class: {first:?}");
    }
}
