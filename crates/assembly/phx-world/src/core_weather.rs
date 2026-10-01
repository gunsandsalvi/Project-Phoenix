//! The weather and the catastrophes on the core. Each region's day of weather is the kernel's weather store's: its
//! latents moved on from its own draws and its values kept in the store's ring, each recorded as a public event naming
//! the region; each country's day of catastrophes draws every hazard's origins over its tiles' exposure, each origin's
//! footprint and its tiles' severities recorded as an event naming the tiles and written into the day's footprint.
//! What stands on a struck tile is destroyed at its owner by its severity's share, in the goods day beside spoilage.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Cost, NATURE};
use phx_core::slots::DaySlot;
use phx_core::{NewEvent, StreamDef, WorldStreams};
use phx_geo::catastrophe::{Footprint, Struck};
use phx_geo::consts::WEATHER_CHUNK_DAYS;
use phx_geo::weather::WeatherStore;
use phx_geo::{GeoState, catastrophe};
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};
use phx_store::AddressSpace;

use crate::consts::reason::DESTROYED;
use crate::core::{Core, kind_number};
use crate::core_accounts::Line;

/// A catastrophe as the prices' reads follow it: its day and event kind, the regions it struck, each product's mark at
/// each region on its day, and the first day after it each mark rose above that.
#[derive(Clone, Debug, PartialEq, phx_macros::Saved)]
pub struct Shock {
    pub day: Day,
    pub kind: u16,
    pub regions: Vec<u32>,
    pub base: BTreeMap<(u16, u32), f64>,
    pub rose: BTreeMap<(u16, u32), Day>,
}

/// The weather's store, opened on the weather's first day; today's footprint, each struck tile with its severity in
/// permille, which the goods day destroys what stands on; and every catastrophe the prices' reads follow.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Weather {
    store: Option<WeatherStore>,
    #[saved(skip, rebuild = Weather::unstruck)]
    footprint: Option<Footprint>,
    pub shocks: Vec<Shock>,
}

impl Weather {
    /// A loaded world's footprint is opened again with its first day: a save is taken at a close, after the day's
    /// losses read it.
    fn unstruck(&mut self) -> u64 {
        self.footprint = None;
        0
    }
}

/// Each struck tile, with none yet found sited on it.
fn struck_tiles(struck: &[Struck]) -> BTreeMap<u32, Vec<PartyKey>> {
    struck.iter().map(|s| (s.tile.get(), Vec::new())).collect()
}

impl Core {
    /// The day's weather in every region and catastrophes in every country, recorded as events.
    #[clause("CHN.3", "GEO.8")]
    pub(crate) fn weather_day(&mut self, geo: &GeoState, (streams, calendar): (&WorldStreams, &Calendar), day: Day) {
        let date = calendar.date(day);
        let at = DaySlot::S3a.ordinal();
        let w = &mut self.weather;
        if w.store.is_none() {
            let store = WeatherStore::new(
                &mut AddressSpace::empty(),
                geo.regions.len(),
                geo.weather_horizon,
                WEATHER_CHUNK_DAYS,
            );
            let Ok(store) = store else {
                violation!(clause = "SET.13", "a weather store past a ring's width", regions = geo.regions.len());
            };
            w.store = Some(store);
        }
        // Every hazard may strike every land tile once a day: the most a day's footprint holds.
        let most = geo.map.tiles.len() * geo.hazards.len();
        let footprint =
            w.footprint.get_or_insert_with(|| Footprint::new(&mut AddressSpace::empty(), "GEO.footprint", most));
        footprint.clear();
        let Some(store) = w.store.as_mut() else { return };
        let draws = |r: usize| {
            let region = Subject::new(SubjectTag::Region, phx_rand::float::len_u64(r));
            streams.open_at(&phx_geo::weather::WeatherStream::DECL, region, day, at)
        };
        let _ = store.record_day(day, (&geo.regions, date.month()), draws);
        for row in store.today() {
            let region = Subject::new(SubjectTag::Region, u64::from(row.region()));
            for (v, kind) in row.values().iter().zip(&geo.weather_kinds) {
                self.happened.record(NewEvent {
                    day,
                    slot: DaySlot::S3a,
                    kind: *kind,
                    subjects: &[region],
                    details: &[(region, i64::from(*v))],
                    develops_from: Missing::Absent,
                });
            }
        }
        let days = catastrophe::days_in_year(date);
        let countries = geo.hazards.first().map_or(0, |h| h.by_country.len());
        for c in 0..countries {
            let country = Subject::new(SubjectTag::Country, phx_rand::float::len_u64(c));
            let mut d = streams.open_at(&catastrophe::CatastropheStream::DECL, country, day, at);
            for h in &geo.hazards {
                let Some(footprint) = self.weather.footprint.as_mut() else { continue };
                for e in catastrophe::hazard_day(geo, (h, c, days), &mut d, |s| footprint.push(s)) {
                    let mut regions = Vec::new();
                    for (tile, _) in &e.details {
                        let Ok(t) = u32::try_from(tile.id()) else { continue };
                        if let Missing::Present(z) = geo.zone_of(phx_id::TileId::new(t))
                            && let Missing::Present(r) = geo.zone_region(z)
                            && !regions.contains(&r)
                        {
                            regions.push(r);
                        }
                    }
                    let base = self.goods.marks.clone();
                    self.weather.shocks.push(Shock { day, kind: e.kind, regions, base, rose: BTreeMap::new() });
                    self.happened.record(NewEvent {
                        day,
                        slot: DaySlot::S3a,
                        kind: e.kind,
                        subjects: &e.subjects,
                        details: &e.details,
                        develops_from: Missing::Absent,
                    });
                }
            }
        }
    }

    /// Each followed catastrophe's marks that rose today above their mark on its day, the day recorded once.
    #[clause("GDS.11")]
    pub(crate) fn note_rises(&mut self, day: Day, printed: &[((u16, u32), f64)]) {
        for shock in self.weather.shocks.iter_mut().filter(|s| s.day < day) {
            for (key, mark) in printed {
                if !shock.rose.contains_key(key) && shock.base.get(key).is_some_and(|b| mark > b) {
                    shock.rose.insert(*key, day);
                }
            }
        }
    }

    /// What stands on each tile struck today destroyed at its owner by the tile's severity: each free unit of every
    /// good a firm sited there holds, the share in permille, whole units, lost to nature at its cost.
    #[clause("GEO.8", "GDS.9", "GDS.10")]
    pub(crate) fn destroy(&mut self, day: Day, moved: &mut Vec<Flow>) -> i64 {
        let Some(firm) = self.bound.kinds.firm else { return 0 };
        let struck = self.weather.footprint.as_ref().map_or(&[][..], Footprint::as_slice);
        if struck.is_empty() {
            return 0;
        }
        let per_mille = i64::try_from(phx_geo::consts::PER_MILLE).unwrap_or(i64::MAX);
        // The firms sited on each struck tile, found in one pass over the firms.
        let mut sited: BTreeMap<u32, Vec<PartyKey>> = struck_tiles(struck);
        if let Some(store) = self.kinds.get(firm) {
            for s in self.directory.live_slots(kind_number(firm)) {
                let site = store.record(s).get(crate::consts::firm::SITE).map(|w| w.get());
                if let Some(Missing::Present(t)) = site
                    && let Some(on) = u32::try_from(t).ok().and_then(|t| sited.get_mut(&t))
                {
                    on.push(PartyKey::new(kind_number(firm), s));
                }
            }
        }
        let mut destroyed = 0;
        let count = struck.len();
        for i in 0..count {
            let Some(&Struck { tile, severity }) = self.weather.footprint.as_ref().and_then(|f| f.as_slice().get(i))
            else {
                continue;
            };
            let (tile, share) = (tile.get(), i64::from(severity));
            for owner in sited.get(&tile).cloned().unwrap_or_default() {
                let held: Vec<(u16, i64)> = self.goods.stocks.holdings(owner).map(|h| (h.unit, h.free())).collect();
                for (unit, free) in held {
                    let lost = free * share / per_mille;
                    if lost <= 0 {
                        continue;
                    }
                    let flow = Flow {
                        payer: owner,
                        payee: NATURE,
                        amount: lost,
                        source: tile,
                        denomination: Denom::units(unit),
                        reason: DESTROYED,
                        order: 0,
                    };
                    let cost = self.move_goods(flow, Cost::Carried, day, moved);
                    self.recognise(owner, Line::GoodsLost, cost.unwrap_or(0));
                    destroyed += lost;
                }
            }
        }
        destroyed
    }
}

#[cfg(test)]
mod tests {
    use phx_geo::catastrophe::Struck;
    use phx_id::TileId;

    use super::struck_tiles;

    #[test]
    fn empty_footprint_destroys_nothing() {
        assert!(struck_tiles(&[]).is_empty(), "a day no catastrophe struck sites nothing to destroy");
        let one = struck_tiles(&[
            Struck { tile: TileId::new(4), severity: 300 },
            Struck { tile: TileId::new(4), severity: 9 },
        ]);
        assert_eq!(one.keys().copied().collect::<Vec<_>>(), [4], "a tile struck twice is looked for once");
    }
}
