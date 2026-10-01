//! `-F routes`: the network at the design point — its zones in their regions, the trunks joining the regions' market
//! zones by each mode, each region's own segments between its zones, and the parallel segments that fill the design
//! point's count — with every mode's routes between the market zones and each region's zone routes; a route read by
//! its pair, and a struck region's routes computed again.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_geo::transport::{Routes, SegmentDecl, SegmentId, Segments, Transport};
use phx_id::Day;
use phx_num::Missing;
use phx_rand::uniform::below_u64;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the network is measured under.
pub const BASE: &str = "routes";

/// The trunks' modes — road, rail, sea — and the land modes a region's own segments carry.
const MODES: u8 = 3;
const LAND: u8 = 2;
/// A region's zones laid out in rows of this many, each joined to the next in its row and to the one below.
const ROW: u64 = 8;
/// A segment's length between neighbouring zones, and the most a parallel segment's detour adds, in metres.
const NEIGHBOUR_M: u32 = 4_000;
const DETOUR_M: u64 = 6_000;
/// A trunk's length between neighbouring regions' market zones, in metres.
const TRUNK_M: u32 = 40_000;
/// Route reads a day, and the day the network opens on.
const READS: u64 = 1_000_000;
const OPENED: u32 = 1;

const MIB: f64 = 1_048_576.0;

/// The network, each region's zone routes by land mode, and the struck region's segment closed and reopened in turn.
#[derive(Debug, Default)]
pub struct RoutesBase {
    transport: Option<Transport>,
    regional: Vec<Routes>,
    markets: Vec<u16>,
    region_zones: u64,
    struck: Option<SegmentId>,
    streams: Option<Streams>,
    today: u32,
    folded: u64,
}

fn zone(z: u64) -> Result<u16, FinError> {
    u16::try_from(z).map_err(|e| FinError(format!("zone {z}: {e}")))
}

fn decl(from: u16, to: u16, mode: u8, metres: u32) -> SegmentDecl {
    SegmentDecl { from, to, mode, metres, capacity: 1, condition: 0, unit: Missing::Absent }
}

impl FinBase for RoutesBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The design point's network: each region's zones in rows, its land segments between neighbours, the trunks
    /// between neighbouring regions' market zones by every mode, and parallel segments to the design point's count.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let (zones, regions, wanted) = (store("zones")?, store("regions")?, store("segments")?);
        let per = zones / regions;
        let mut s = Segments::default();
        let mut local = Vec::new();
        for r in 0..regions {
            for i in 0..per {
                let z = r * per + i;
                for n in [i + 1, i + ROW] {
                    if n < per && (n != i + 1 || n % ROW != 0) {
                        local.push((zone(z)?, zone(r * per + n)?));
                    }
                }
            }
        }
        for (a, b) in &local {
            for mode in 0..LAND {
                let _ = s.open(decl(*a, *b, mode, NEIGHBOUR_M));
            }
        }
        let side = (1..=regions).find(|k| k * k >= regions).unwrap_or(regions);
        let markets: Vec<u16> = (0..regions).map(|r| zone(r * per)).collect::<Result<_, _>>()?;
        for r in 0..regions {
            for n in [r + 1, r + side] {
                if n < regions && (n != r + 1 || n % side != 0) {
                    let (Some(a), Some(b)) = (markets.get(index(r)?), markets.get(index(n)?)) else { continue };
                    for mode in 0..MODES {
                        let _ = s.open(decl(*a, *b, mode, TRUNK_M));
                    }
                }
            }
        }
        let mut d = streams.draws(BASE, 0, 0);
        while wide(s.len()) < wanted {
            let Some(&(a, b)) = local.get(index(below_u64(&mut d, wide(local.len())))?) else { break };
            let metres =
                NEIGHBOUR_M + u32::try_from(below_u64(&mut d, DETOUR_M)).map_err(|e| FinError(e.to_string()))?;
            let mode = u8::try_from(below_u64(&mut d, u64::from(LAND))).map_err(|e| FinError(e.to_string()))?;
            let _ = s.open(decl(a, b, mode, metres));
        }
        let mut changes = Vec::new();
        s.take_changes(&mut changes);
        let zones_u32 = u32::try_from(zones).map_err(|e| FinError(e.to_string()))?;
        let today = Day::new(OPENED);
        for r in 0..regions {
            let own: Vec<u16> = (r * per..(r + 1) * per).map(zone).collect::<Result<_, _>>()?;
            for mode in 0..LAND {
                let mut routes = Routes::new(mode, &own, zones_u32);
                routes.admit_all(&s);
                routes.recompute(&s, today);
                self.regional.push(routes);
            }
        }
        self.struck = s.iter().next().map(|(id, _)| id);
        self.transport = Some(Transport::new(s, (&markets, zones_u32), MODES, today));
        (self.markets, self.region_zones, self.streams, self.today) = (markets, per, Some(*streams), OPENED);
        Ok(Filled { rows: wanted })
    }

    /// The day's route reads between drawn market zones, then a struck region's segment closed and its routes, and
    /// the trunks', computed again.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(net), Some(streams), Some(struck)) = (self.transport.as_mut(), self.streams, self.struck) else {
            return Err(FinError("the network measured before its fill".to_owned()));
        };
        let mut draws = streams.draws(BASE, u64::from(self.today), crate::kept::day_of(day)?);
        let markets = &self.markets;
        let mut pick = || markets.get(usize::try_from(below_u64(&mut draws, wide(markets.len()))).ok()?).copied();
        let pairs: Vec<(u16, u16)> = (0..READS).filter_map(|_| Some((pick()?, pick()?))).collect();
        let reader = &*net;
        self.folded ^= m.read(BASE, "route", READS, || {
            let mut fold = 0_u64;
            for (from, to) in &pairs {
                if let Missing::Present(r) = reader.route(0, *from, *to) {
                    fold ^= r.metres;
                }
            }
            black_box(fold)
        });
        let today = Day::new(self.today);
        let regional = &mut self.regional;
        self.folded ^= m.read(BASE, "recompute", 1, || {
            let mut visited = net.close(struck, today.succ(), today);
            for routes in regional.iter_mut().take(usize::from(LAND)) {
                visited += routes.recompute(&net.segments, today);
            }
            black_box(visited + net.reopen(struck, today))
        });
        self.today += 1;
        black_box(self.region_zones);
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let regional: usize = self.regional.iter().map(Routes::bytes).sum();
        let network = self.transport.as_ref().map_or(0, phx_store::StoreStats::bytes);
        Bytes { rows: network + wide(regional), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let bytes = self.bytes().rows.to_string().parse::<f64>().ok();
        bytes.map(|b| vec![("mb", b / MIB)]).unwrap_or_default()
    }
}
