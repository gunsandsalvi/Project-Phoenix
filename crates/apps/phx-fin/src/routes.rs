//! `-F routes`: the network at the design point — its zones in their regions, the trunks joining the regions' market
//! zones by each mode, each region's own segments between its zones, and the parallel segments that fill the design
//! point's count — with every mode's routes between the market zones and each region's zone routes; a route read by
//! its pair, and a struck region's routes computed again. Its use part puts a working day's active pairs' flows on
//! their routes, admits the segments they carried past their capacity, and reads each pair's time at the day's loads.

use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;

use phx_geo::transport::{
    PairFlow, Route, Routes, SegmentDecl, SegmentId, SegmentRow, SegmentUse, Segments, Transport,
};
use phx_id::Day;
use phx_num::Missing;
use phx_rand::uniform::below_u64;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the network is measured under, and the one its day's use is.
pub const BASE: &str = "routes";
const USE: &str = "day_use";

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
/// What a region's own segment and a trunk carry a day, and the most trips a pair's flow carries: about as much as
/// the routes put on the busiest of them, so a few are carried past it.
const LOCAL_CAPACITY: u32 = 12_000;
const TRUNK_CAPACITY: u32 = 60_000;
const MOST_TRIPS: u64 = 200;
/// The congestion curve's form a road's is declared in: free time × (1 + a·(load/capacity)^b), a leg's free time
/// its metres at a speed in metres a second.
const BPR_A: f64 = 0.15;
const BPR_B: i32 = 4;
const SPEED_M_S: f64 = 20.0;

const MIB: f64 = 1_048_576.0;

/// The network, each region's zone routes by land mode, and the struck region's segment closed and reopened in turn.
#[derive(Debug, Default)]
pub struct RoutesBase {
    transport: Option<Transport>,
    regional: Vec<Routes>,
    markets: Vec<u16>,
    region_of: Vec<u8>,
    region_zones: u64,
    struck: Option<SegmentId>,
    flows: Vec<PairFlow>,
    used: SegmentUse,
    streams: Option<Streams>,
    today: u32,
    folded: u64,
}

fn zone(z: u64) -> Result<u16, FinError> {
    u16::try_from(z).map_err(|e| FinError(format!("zone {z}: {e}")))
}

fn decl(from: u16, to: u16, mode: u8, (metres, capacity): (u32, u32)) -> SegmentDecl {
    SegmentDecl { from, to, mode, metres, capacity, condition: 0, unit: Missing::Absent }
}

/// A leg's time in seconds at its load, by the curve's form.
fn leg_time(row: SegmentRow, load: u32) -> u64 {
    let ratio = f64::from(load) / f64::from(row.capacity());
    let seconds = f64::from(row.metres()) / SPEED_M_S * (1.0 + BPR_A * ratio.powi(BPR_B));
    match phx_rand::float::floor_to_u64(seconds) {
        Some(s) => s,
        None => phx_num::capacity_exceeded!("a leg's time", u64::MAX, row.metres()),
    }
}

/// A working day's active pairs: every pair of two market zones by each mode, the rest drawn between two zones of a
/// region by a land mode until the pairs number the design point's, each carrying a drawn number of trips; a pair
/// once, in (mode, origin, destination) order, as the travel rules' counts by pair hand them.
fn active_pairs(d: &mut phx_rand::Draws, (pairs, per): (u64, u64), markets: &[u16]) -> Result<Vec<PairFlow>, FinError> {
    let mut draw = |n: u64| below_u64(d, n);
    let small = |n: u64| u8::try_from(n).map_err(|e| FinError(e.to_string()));
    let mut chosen = BTreeSet::new();
    for mode in 0..MODES {
        for from in markets {
            for to in markets.iter().filter(|to| *to != from) {
                let _ = chosen.insert((mode, *from, *to));
            }
        }
    }
    let regions = wide(markets.len());
    while wide(chosen.len()) < pairs {
        let base = draw(regions) * per;
        let (from, to) = (zone(base + draw(per))?, zone(base + draw(per))?);
        let _ = chosen.insert((small(draw(u64::from(LAND)))?, from, to));
    }
    chosen
        .into_iter()
        .map(|(mode, from, to)| {
            let count = u32::try_from(1 + draw(MOST_TRIPS)).map_err(|e| FinError(e.to_string()))?;
            Ok(PairFlow { mode, from, to, count })
        })
        .collect()
}

impl RoutesBase {
    /// The bytes the network and its routes hold, and those its day's use does.
    fn held(&self) -> (u64, u64) {
        let regional: usize = self.regional.iter().map(Routes::bytes).sum();
        let network = self.transport.as_ref().map_or(0, phx_store::StoreStats::bytes);
        (network + wide(regional), phx_store::StoreStats::bytes(&self.used))
    }

    /// The day's flows put on their routes, the segments past their capacity admitted, and every pair's time read.
    fn use_day(&mut self, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(net), Some(streams)) = (self.transport.as_ref(), self.streams) else {
            return Err(FinError("the network measured before its fill".to_owned()));
        };
        let (regional, region_of) = (&self.regional, &self.region_of);
        let route = |f: &PairFlow| -> Missing<Route<'_>> {
            let (a, b) = (region_of.get(usize::from(f.from)), region_of.get(usize::from(f.to)));
            match (a, b) {
                (Some(a), Some(b)) if a == b && f.mode < LAND => {
                    match regional.get(usize::from(*a) * usize::from(LAND) + usize::from(f.mode)) {
                        Some(r) => r.route(f.from, f.to),
                        None => Missing::Absent,
                    }
                }
                _ => net.route(f.mode, f.from, f.to),
            }
        };
        let (flows, used, today) = (&self.flows, &mut self.used, Day::new(self.today));
        let legs: u64 = flows
            .iter()
            .map(|f| match route(f) {
                Missing::Present(r) => wide(r.segments.len()),
                Missing::Absent => 0,
            })
            .sum();
        // The pairs' routes read alone, the share of the adds and times that is the route table's and not the loads'.
        let routed = m.read(USE, "pair_route", legs, || {
            let mut fold = 0_u32;
            for f in flows {
                if let Missing::Present(r) = route(f) {
                    fold = r.segments.iter().fold(fold, |a, s| a ^ s);
                }
            }
            black_box(fold)
        });
        let added = m.read(USE, "load", legs, || used.add_flows(&net.segments, flows, route, today));
        let mut lot = streams.draws(USE, u64::from(self.today), 0);
        let read =
            m.read(USE, "admit", wide(flows.len()), || used.admit(&net.segments, flows, route, (&mut lot, today)));
        let time = m.read(USE, "pair_time", legs, || {
            let mut fold = 0_u64;
            for f in flows {
                if let Missing::Present(r) = route(f) {
                    fold ^= used.time_at(&net.segments, r, today, leg_time);
                }
            }
            black_box(fold)
        });
        self.folded ^= added ^ read ^ time ^ u64::from(routed);
        Ok(())
    }
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
                let _ = s.open(decl(*a, *b, mode, (NEIGHBOUR_M, LOCAL_CAPACITY)));
            }
        }
        let side = (1..=regions).find(|k| k * k >= regions).unwrap_or(regions);
        let markets: Vec<u16> = (0..regions).map(|r| zone(r * per)).collect::<Result<_, _>>()?;
        for r in 0..regions {
            for n in [r + 1, r + side] {
                if n < regions && (n != r + 1 || n % side != 0) {
                    let (Some(a), Some(b)) = (markets.get(index(r)?), markets.get(index(n)?)) else { continue };
                    for mode in 0..MODES {
                        let _ = s.open(decl(*a, *b, mode, (TRUNK_M, TRUNK_CAPACITY)));
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
            let _ = s.open(decl(a, b, mode, (metres, LOCAL_CAPACITY)));
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
        self.flows = active_pairs(&mut d, (store("active_pairs")?, per), &markets)?;
        self.struck = s.iter().next().map(|(id, _)| id);
        self.transport = Some(Transport::new(s, (&markets, zones_u32), MODES, today));
        self.region_of =
            (0..zones).map(|z| u8::try_from(z / per)).collect::<Result<_, _>>().map_err(|e| FinError(e.to_string()))?;
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
        self.use_day(m)?;
        self.today += 1;
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let (network, used) = self.held();
        Bytes { rows: network + used, resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let (network, used) = self.held();
        let (loaded, over) = self.used.loaded();
        let count = |n: u64| n.to_string().parse::<f64>().ok();
        [
            count(network).map(|b| ("mb", b / MIB)),
            count(used).map(|b| ("use_mb", b / MIB)),
            count(wide(loaded)).map(|n| ("segments_loaded", n)),
            count(wide(over)).map(|n| ("segments_over", n)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
