//! The weather and the catastrophes on the core. Each region's day of weather moves its variables' latents on by
//! their persistence from its own draws and records each value as a public event naming the region; each country's
//! day of catastrophes draws every hazard's origins over its tiles' exposure, each origin's footprint and its tiles'
//! severities recorded as an event naming the tiles. What stands on a struck tile is destroyed at its owner by its
//! severity's share, in the goods day beside spoilage.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Cost, NATURE};
use phx_core::{NewEvent, StreamDef, Streams, SubStep};
use phx_geo::weather::{Latents, VARIABLES};
use phx_geo::{GeoState, catastrophe};
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::{Fixed, Missing, Round, violation};
use phx_rand::{Subject, SubjectTag};

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

/// The weather's latents by region, today's struck tiles, each with its severity in permille, which the goods day
/// destroys what stands on, and every catastrophe the prices' reads follow.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Weather {
    pub latents: Vec<Latents>,
    pub struck: Vec<(u32, i64)>,
    pub shocks: Vec<Shock>,
}

/// A size in its event's units, whole.
fn whole(v: f64) -> i64 {
    match Fixed::<0>::from_f64(v, Round::HalfEven) {
        Ok(f) => f.raw(),
        Err(_) => violation!(clause = "CHN.3", "a weather value beyond its width"),
    }
}

/// Each struck tile, with none yet found sited on it.
fn struck_tiles(struck: &[(u32, i64)]) -> BTreeMap<u32, Vec<PartyKey>> {
    struck.iter().map(|(t, _)| (*t, Vec::new())).collect()
}

impl Core {
    /// The day's weather in every region and catastrophes in every country, recorded as events.
    #[clause("CHN.3", "GEO.8")]
    pub(crate) fn weather_day(&mut self, geo: &GeoState, (streams, calendar): (&Streams, &Calendar), day: Day) {
        let date = calendar.date(day);
        let at = SubStep::S3a.ordinal();
        if self.weather.latents.len() < geo.regions.len() {
            self.weather.latents.resize(geo.regions.len(), [Missing::Absent; VARIABLES.len()]);
        }
        for (r, climate) in geo.regions.iter().enumerate() {
            let region = Subject::new(SubjectTag::Region, phx_rand::float::len_u64(r));
            let mut d = streams.open(&phx_geo::weather::WeatherStream::DECL, region, day, at);
            let Some(latents) = self.weather.latents.get_mut(r) else { continue };
            let values = phx_geo::weather::region_day(climate, date.month(), latents, &mut d);
            for (v, kind) in values.iter().zip(&geo.weather_kinds) {
                let Missing::Present(v) = v else { continue };
                self.happened.record(NewEvent {
                    day,
                    substep: SubStep::S3a,
                    kind: *kind,
                    subjects: &[region],
                    details: &[(region, whole(*v))],
                    develops_from: Missing::Absent,
                });
            }
        }
        let days = catastrophe::days_in_year(date);
        let countries = geo.hazards.first().map_or(0, |h| h.by_country.len());
        for c in 0..countries {
            let country = Subject::new(SubjectTag::Country, phx_rand::float::len_u64(c));
            let mut d = streams.open(&catastrophe::CatastropheStream::DECL, country, day, at);
            for h in &geo.hazards {
                for e in catastrophe::hazard_day(geo, h, c, days, &mut d) {
                    let mut regions = Vec::new();
                    for (tile, share) in &e.details {
                        let Ok(t) = u32::try_from(tile.id()) else { continue };
                        self.weather.struck.push((t, *share));
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
                        substep: SubStep::S3a,
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
        let struck = std::mem::take(&mut self.weather.struck);
        let Some(firm) = self.bound.kinds.firm else { return 0 };
        if struck.is_empty() {
            return 0;
        }
        let per_mille = i64::try_from(phx_geo::consts::PER_MILLE).unwrap_or(i64::MAX);
        // The firms sited on each struck tile, found in one pass over the firms.
        let mut sited: BTreeMap<u32, Vec<PartyKey>> = struck_tiles(&struck);
        if let Some(store) = self.kinds.get(firm) {
            for s in store.parties.live_slots() {
                let site = store.record(s).get(crate::consts::firm::SITE).map(|w| w.get());
                if let Some(Missing::Present(t)) = site
                    && let Some(on) = u32::try_from(t).ok().and_then(|t| sited.get_mut(&t))
                {
                    on.push(PartyKey::new(kind_number(firm), s));
                }
            }
        }
        let mut destroyed = 0;
        for (tile, share) in struck {
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
