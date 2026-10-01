//! `-F cells`: the ground at the design point — its tiles in cells of 100 m, the land held apart laid as parcels of a
//! few rectangles a tile and the network's segments listed by the tiles their lines cross — with a parcel found at a
//! cell, a part cut out of a parcel, a segment indexed, and the paths crossing a cell read.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_geo::cells::{CellGrid, CellId, Held, Line, Parcels, Rect, SegmentIndex};
use phx_geo::consts::PARCEL_OWNED_APART;
use phx_geo::grid::Grid;
use phx_geo::transport::SegmentId;
use phx_id::TileId;
use phx_num::Missing;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the ground is measured under.
pub const BASE: &str = "cells";

/// The tiles' side and the cells' as the register declares them, in metres.
const TILE_M: u32 = 10_000;
const CELL_M: u32 = 100;
/// A tile laid out in blocks for its parcels: rows of blocks and blocks a row, a parcel drawn within each.
const BLOCK_ROWS: u16 = 4;
const BLOCK_COLUMNS: u16 = 5;
/// How far a segment's far end lies from its near one, in cells either way: about two tiles crossed a line.
const REACH: u64 = 120;
/// Lookups, cuts and crossings read a day.
const READS: u64 = 1_000_000;
const CUTS: u64 = 10_000;
/// Room reserved past the design point's rows, so the arenas never run short during the measure.
const ROOM: u64 = 2;

const MIB: f64 = 1_048_576.0;

/// The ground: its cells, the parcels to lay, the segments' lines, and the stores they fill.
#[derive(Debug, Default)]
pub struct CellsBase {
    cells: Option<CellGrid>,
    plan: Vec<(TileId, Rect)>,
    lines: Vec<Line>,
    parcels: Option<Parcels>,
    index: Option<SegmentIndex>,
    streams: Option<Streams>,
    reserved: (u32, u32),
    folded: u64,
}

fn err(e: impl std::fmt::Display) -> FinError {
    FinError(e.to_string())
}

fn small(n: u64) -> Result<u16, FinError> {
    u16::try_from(n).map_err(err)
}

impl FinBase for CellsBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// The design point's tiles in a square, its parcels drawn a block each across as many tiles as they fill, and
    /// its segments' lines drawn from a cell to one within a tile or so of it.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let (tiles, parcels, segments, entries) =
            (store("tiles")?, store("parcels")?, store("segments")?, store("segment_tiles")?);
        let side = (1..=tiles).find(|s| s * s >= tiles).unwrap_or(tiles);
        let side = u32::try_from(side).map_err(err)?;
        let cells = CellGrid::new(Grid { width: side, height: side, tile_m: TILE_M }, CELL_M).map_err(FinError)?;
        let mut d = streams.draws(BASE, 0, 0);
        let blocks = u64::from(BLOCK_ROWS * BLOCK_COLUMNS);
        let (across, down) = (u16::from(cells.side()) / BLOCK_COLUMNS, u16::from(cells.side()) / BLOCK_ROWS);
        let all_tiles = u64::from(side) * u64::from(side);
        for i in 0..parcels {
            let (tile, block) = (i % all_tiles, (i / all_tiles) % blocks);
            let (row, col) =
                (small(block / u64::from(BLOCK_COLUMNS))? * down, small(block % u64::from(BLOCK_COLUMNS))? * across);
            let height = u8::try_from(1 + below_u64(&mut d, u64::from(down))).map_err(err)?;
            let width = u8::try_from(1 + below_u64(&mut d, u64::from(across))).map_err(err)?;
            let origin = row * u16::from(cells.side()) + col;
            self.plan.push((TileId::new(u32::try_from(tile).map_err(err)?), Rect { origin, width, height }));
        }
        // Laid in a drawn order, so each run is kept sorted against arrivals anywhere in it.
        for i in (1..self.plan.len()).rev() {
            self.plan.swap(i, index(below_u64(&mut d, wide(i + 1)))?);
        }
        let span = u64::from(side) * u64::from(cells.side());
        let at = |n: u64| i64::try_from(n).map_err(err);
        let reach = at(REACH)?;
        for _ in 0..segments {
            let (x, y) = (at(below_u64(&mut d, span))?, at(below_u64(&mut d, span))?);
            let (dx, dy) =
                (at(below_u64(&mut d, 2 * REACH + 1))? - reach, at(below_u64(&mut d, 2 * REACH + 1))? - reach);
            self.lines.push(cells.line(cells.at(x, y), cells.at(x + dx, y + dy)));
        }
        self.reserved = (u32::try_from(parcels * ROOM).map_err(err)?, u32::try_from(entries * ROOM).map_err(err)?);
        (self.cells, self.streams) = (Some(cells), Some(*streams));
        Ok(Filled { rows: parcels })
    }

    /// The parcels laid and the segments indexed into empty stores, then the day's reads and cuts.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(cells), Some(streams)) = (self.cells, self.streams) else {
            return Err(FinError("the ground measured before its fill".to_owned()));
        };
        let mut space = AddressSpace::empty();
        let mut parcels: Parcels = Parcels::new(&mut space, cells, self.reserved.0);
        let plan = &self.plan;
        m.read(BASE, "insert", wide(plan.len()), || {
            for (i, (tile, rect)) in (0_u32..).zip(plan) {
                parcels.insert(*tile, *rect, (i, 1_000), Held(PARCEL_OWNED_APART));
            }
        });
        let mut segment_index: SegmentIndex = SegmentIndex::new(&mut space, cells, self.reserved.1);
        let lines = &self.lines;
        let entries = m.read(BASE, "index", wide(lines.len()), || {
            (0_u32..).zip(lines).map(|(i, l)| segment_index.index_segment(SegmentId::new(i), *l)).sum::<u64>()
        });
        let mut d = streams.draws(BASE, 1, crate::kept::day_of(day)?);
        let tiles = u64::from(cells.grid().width) * u64::from(cells.grid().height);
        let cell = |d: &mut phx_rand::Draws| -> Result<CellId, FinError> {
            let tile = TileId::new(u32::try_from(below_u64(d, tiles)).map_err(err)?);
            Ok(cells.cell(tile, small(below_u64(d, u64::from(cells.per_tile())))?))
        };
        let probes: Vec<CellId> = (0..READS).map(|_| cell(&mut d)).collect::<Result<_, _>>()?;
        let reader = &parcels;
        self.folded ^= m.read(BASE, "lookup", READS, || {
            let mut fold = 0_u64;
            for c in &probes {
                if let Missing::Present(p) = reader.parcel_at(*c) {
                    fold ^= u64::from(p.owner());
                }
            }
            black_box(fold)
        });
        // Every id the index lists is a line's place, so a missing line is a line to the cell itself.
        let line_of = |id: SegmentId, c: CellId| match usize::try_from(id.get()).ok().and_then(|i| lines.get(i)) {
            Some(l) => *l,
            None => cells.line(c, c),
        };
        let mut out = Vec::new();
        let ix = &segment_index;
        let read = m.read(BASE, "crossing", READS, || {
            let mut read = 0_u64;
            for c in &probes {
                read += ix.crossing(*c, |id| line_of(id, *c), &mut out);
            }
            black_box(read)
        });
        let cuts: Vec<(TileId, Rect)> = plan
            .iter()
            .take(index(CUTS)?)
            .filter(|(_, r)| r.width > 2 && r.height > 2)
            .map(|(t, r)| (*t, Rect { origin: r.origin + u16::from(cells.side()) + 1, width: 1, height: 1 }))
            .collect();
        m.read(BASE, "split", wide(cuts.len()), || {
            for (tile, part) in &cuts {
                let _ = parcels.split(*part, *tile);
            }
        });
        self.folded ^= entries ^ read;
        (self.parcels, self.index) = (Some(parcels), Some(segment_index));
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let held =
            self.parcels.as_ref().map_or(0, StoreStats::bytes) + self.index.as_ref().map_or(0, StoreStats::bytes);
        Bytes { rows: held, resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let count = |n: u64| n.to_string().parse::<f64>().ok();
        let parcels = self.parcels.as_ref().map_or(0, StoreStats::bytes);
        let entries = self.index.as_ref().map_or(0, StoreStats::rows_live);
        [
            count(self.bytes().rows).map(|b| ("mb", b / MIB)),
            count(parcels).map(|b| ("parcels_mb", b / MIB)),
            count(entries).map(|n| ("segment_tiles", n)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
