//! `-F weather`: the weather's store at the design point — its regions' days of weather kept over the horizon — with a
//! day's weather drawn and kept for every region, and a strike's footprint spread over the map's tiles and written into
//! the day's footprint buffer.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_geo::catastrophe::{Footprint, Struck, footprint};
use phx_geo::climate::{Marginal, RegionClimate};
use phx_geo::grid::Grid;
use phx_geo::weather::WeatherStore;
use phx_id::{Day, TileId};
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::wide;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the weather is measured under.
pub const BASE: &str = "weather";

/// The horizon the history is kept to, two years, and the days a ring chunk holds.
const HORIZON: u32 = 730;
const CHUNK_DAYS: u32 = 32;
/// Strikes a day, the chance a strike spreads to a neighbouring tile, and the most tiles a footprint holds.
const STRIKES: u64 = 25;
const SPREAD: f64 = 0.2;
const MOST_STRUCK: usize = 1_000_000;
const SEVERITY: u32 = 500;
/// A region's climate at the design point: a temperate month's marginals and each variable's persistence.
const PERSISTENCE: f64 = 0.7;
const MONTH: u8 = 1;

/// The store, the regions' climates, the map's grid and the day reached.
#[derive(Debug, Default)]
pub struct WeatherBase {
    store: Option<WeatherStore>,
    footprint: Option<Footprint>,
    climates: Vec<RegionClimate>,
    grid: Option<Grid>,
    streams: Option<Streams>,
    today: u32,
    folded: u64,
}

fn err(e: impl std::fmt::Display) -> FinError {
    FinError(e.to_string())
}

fn climate() -> RegionClimate {
    let month = vec![
        Marginal::Normal { mean: 12.0, sd: 4.0 },
        Marginal::WetGamma { dry: 0.6, shape: 0.8, scale: 6.0 },
        Marginal::Weibull { shape: 2.0, scale: 5.0 },
        Marginal::Beta { a: 2.0, b: 3.0 },
    ];
    RegionClimate { months: vec![month; 12], persistence: vec![PERSISTENCE; 4] }
}

impl WeatherBase {
    fn day_of_weather(&mut self, day: Day) -> Result<u64, FinError> {
        let (Some(store), Some(streams)) = (self.store.as_mut(), self.streams) else {
            return Err(FinError("the weather measured before its fill".to_owned()));
        };
        let draws = |r: usize| streams.draws(BASE, wide(r), day.get());
        Ok(store.record_day(day, (&self.climates, MONTH), draws))
    }
}

impl FinBase for WeatherBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The design point's regions, each a temperate climate, and two years of their weather drawn into the store.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let (regions, tiles) = (store("regions")?, store("tiles")?);
        let side = u32::try_from((1..=tiles).find(|s| s * s >= tiles).unwrap_or(tiles)).map_err(err)?;
        let regions = crate::kept::index(regions)?;
        let mut space = AddressSpace::empty();
        self.store = Some(WeatherStore::new(&mut space, regions, HORIZON, CHUNK_DAYS).map_err(FinError)?);
        self.footprint = Some(Footprint::new(&mut space, "GEO.footprint", MOST_STRUCK));
        self.climates = vec![climate(); regions];
        self.grid = Some(Grid { width: side, height: side, tile_m: 10_000 });
        self.streams = Some(*streams);
        let mut rows = 0;
        for d in 1..=HORIZON {
            rows += self.day_of_weather(Day::new(d))?;
        }
        self.today = HORIZON;
        Ok(Filled { rows })
    }

    /// The next day's weather in every region, and a day's strikes spread into the footprint.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        self.today += 1;
        let today = Day::new(self.today);
        let regions = wide(self.climates.len());
        let mut out = Ok(0);
        self.folded ^= m.read(BASE, "region_day", regions, || {
            out = self.day_of_weather(today);
            black_box(regions)
        });
        let _ = out?;
        let (Some(grid), Some(footprint_buf), Some(streams)) = (self.grid, self.footprint.as_mut(), self.streams)
        else {
            return Err(FinError("the weather measured before its fill".to_owned()));
        };
        let exposure = vec![Some(0_u8); grid.len()];
        let mut d = streams.draws(BASE, wide(self.climates.len()), crate::kept::day_of(day)?);
        let origins: Vec<TileId> = (0..STRIKES)
            .map(|_| u32::try_from(below_u64(&mut d, wide(grid.len()))).map(TileId::new).map_err(err))
            .collect::<Result<_, _>>()?;
        footprint_buf.clear();
        self.folded ^= m.read(BASE, "footprint", STRIKES, || {
            for origin in &origins {
                for t in footprint(&grid, &exposure, &[SPREAD], *origin, &mut d) {
                    footprint_buf.push(Struck { tile: t, severity: SEVERITY });
                }
            }
            black_box(wide(footprint_buf.len()))
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.store.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let struck = self.footprint.as_ref().map_or(0, |f| wide(f.len()));
        struck.to_string().parse::<f64>().map(|n| vec![("struck_tiles", n)]).unwrap_or_default()
    }
}
