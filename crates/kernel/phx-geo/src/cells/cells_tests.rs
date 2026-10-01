//! Cells over a hand-built grid: identities and coordinates by arithmetic, parcels found, kept apart, cut and given
//! back, written by their transfer, saved and rebuilt, and paths' cells read from their segments' lines through the
//! tile index.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::TileId;
use phx_num::Missing;
use phx_store::{AddressSpace, HeapBacking, StoreStats};

use super::{CellGrid, CellId, Held, Line, Parcels, Rect, SegmentIndex};
use crate::consts::PARCEL_OWNED_APART;
use crate::grid::Grid;
use crate::transport::{Change, SegmentDecl, SegmentId, Segments};

type Heap = HeapBacking<4096>;

const OWNED: Held = Held(PARCEL_OWNED_APART);

/// Four tiles across and three down, each of 1 km in cells of 100 m: ten cells a side.
fn cells() -> CellGrid {
    CellGrid::new(Grid { width: 4, height: 3, tile_m: 1_000 }, 100).unwrap()
}

fn parcels() -> Parcels<Heap> {
    Parcels::new(&mut AddressSpace::empty(), cells(), 1 << 12)
}

fn rect(row: u16, col: u16, height: u8, width: u8) -> Rect {
    Rect { origin: row * 10 + col, width, height }
}

fn at(tile: u32, row: u16, col: u16) -> CellId {
    cells().cell(TileId::new(tile), row * 10 + col)
}

#[test]
fn cell_arithmetic_from_tile() {
    let c = cells();
    let cell = at(5, 3, 7);
    assert_eq!((c.tile_of(cell), c.place_of(cell)), (TileId::new(5), 37));
    assert_eq!(c.coords(cell), (17, 13), "tile 5 is the second across and down: ten cells a tile");
    assert_eq!(c.at(17, 13), cell);
    assert_eq!(c.at(-1, 0), c.at(39, 0), "the columns close round the seam");
    assert_eq!(c.distance_m(c.at(0, 0), c.at(39, 0)), 100, "the shorter way round");
    assert_eq!(c.distance_m(c.at(0, 0), c.at(3, 4)), 500);
    assert_eq!(c.centre_of(TileId::new(0)), c.at(5, 5));
}

#[test]
fn cell_id_fits_u32() {
    let wide = Grid { width: 2_000, height: 2_000, tile_m: 10_000 };
    assert!(CellGrid::new(wide, 100).is_err(), "four billion cells pass a 32-bit identity");
    assert!(CellGrid::new(Grid { width: 200, height: 200, tile_m: 10_000 }, 100).is_ok());
    assert!(CellGrid::new(Grid { width: 2, height: 2, tile_m: 1_000 }, 300).is_err(), "a cell that does not divide");
    assert!(CellGrid::new(Grid { width: 2, height: 2, tile_m: 30_000 }, 100).is_err(), "300 cells a side");
}

#[test]
fn parcel_lookup_in_sorted_run() {
    let mut p = parcels();
    let t = TileId::new(1);
    p.insert(t, rect(0, 0, 6, 2), (7, 600), OWNED);
    p.insert(t, rect(1, 4, 1, 3), (8, 30), OWNED);
    p.insert(t, rect(5, 5, 2, 2), (9, 40), OWNED);
    let owner = |row, col| match p.parcel_at(at(1, row, col)) {
        Missing::Present(x) => Some(x.owner()),
        Missing::Absent => None,
    };
    assert_eq!(owner(4, 1), Some(7), "a tall parcel found back past later origins");
    assert_eq!((owner(1, 6), owner(6, 6), owner(1, 7)), (Some(8), Some(9), None));
    assert_eq!(p.parcel_at(at(2, 0, 0)), Missing::Absent, "another tile's land is held with its tile");
}

#[test]
fn insert_keeps_run_sorted() {
    let mut p = parcels();
    let t = TileId::new(0);
    for (row, col) in [(5, 0), (0, 3), (9, 9), (0, 0), (2, 5)] {
        p.insert(t, rect(row, col, 1, 1), (1, 1), OWNED);
    }
    let origins: Vec<u16> = p.in_tile(t).iter().map(|x| x.rect().origin).collect();
    assert_eq!(origins, [0, 3, 25, 50, 99]);
}

#[test]
fn overlapping_parcel_refused() {
    let mut p = parcels();
    let t = TileId::new(0);
    p.insert(t, rect(2, 2, 3, 3), (1, 9), OWNED);
    let over = catch_unwind(AssertUnwindSafe(|| p.insert(t, rect(4, 4, 2, 2), (2, 9), OWNED)));
    assert!(over.is_err(), "two parcels on one cell");
    let out = catch_unwind(AssertUnwindSafe(|| p.insert(t, rect(8, 8, 1, 3), (2, 9), OWNED)));
    assert!(out.is_err(), "a parcel leaving its tile");
    let none = catch_unwind(AssertUnwindSafe(|| p.insert(t, rect(0, 0, 1, 1), (2, 9), Held(0))));
    assert!(none.is_err(), "land held apart for no reason");
    p.insert(t, rect(5, 2, 1, 3), (2, 9), OWNED);
    assert_eq!(p.len(), 2, "touching is not overlapping");
}

#[test]
fn split_keeps_cells_and_cost() {
    let mut p = parcels();
    let t = TileId::new(3);
    p.insert(t, rect(1, 1, 6, 5), (4, 1_003), OWNED);
    let part = p.split(rect(3, 2, 2, 2), t);
    assert_eq!((part.rect(), part.owner()), (rect(3, 2, 2, 2), 4));
    let run = p.in_tile(t);
    assert_eq!(run.len(), 5, "the part and the four rectangles round it");
    assert_eq!(run.iter().map(super::Parcel::cells).sum::<u64>(), 30, "every cell kept");
    assert_eq!(run.iter().map(super::Parcel::cost).sum::<i64>(), 1_003, "the cost kept exactly");
    assert_eq!(part.cost(), 134, "four cells of thirty: 133.73, its remainder among the three largest");
    let corner = p.split(rect(1, 1, 2, 5), t);
    assert_eq!(corner.rect(), rect(1, 1, 2, 5), "a part on the edge leaves fewer pieces");
    assert_eq!(p.in_tile(t).len(), 5);
}

#[test]
fn release_when_not_held_apart() {
    let mut p = parcels();
    let t = TileId::new(2);
    p.insert(t, rect(0, 0, 2, 2), (1, 5), OWNED);
    let gone = p.release(at(2, 1, 1));
    assert_eq!((gone.owner(), p.len(), p.parcel_at(at(2, 0, 0))), (1, 0, Missing::Absent));
    assert!(catch_unwind(AssertUnwindSafe(|| p.release(at(2, 0, 0)))).is_err(), "land not held apart");
}

#[test]
fn empty_tile_costs_nothing() {
    let p = parcels();
    assert_eq!(p.in_tile(TileId::new(5)), &[]);
    assert_eq!(p.len(), 0);
    let directory = 12 * (size_of::<phx_store::CellListRef>() + 1);
    assert_eq!(StoreStats::bytes(&p), u64::try_from(directory).unwrap(), "a tile with nothing held: its directory");
}

#[test]
fn rows_bounded_by_holdings() {
    let mut p = parcels();
    let t = TileId::new(0);
    for i in 0..10 {
        p.insert(t, rect(i, 0, 1, 10), (i.into(), 1), OWNED);
    }
    assert_eq!(p.len(), 10, "a row per parcel");
    for i in 0..10 {
        let _ = p.release(at(0, i, 5));
    }
    assert_eq!((p.len(), p.in_tile(t).len()), (0, 0), "and none once their land is held with its tile");
}

#[test]
fn parcel_written_by_transfer_only() {
    let mut p = parcels();
    let t = TileId::new(0);
    p.insert(t, rect(0, 0, 2, 2), (1, 50), OWNED);
    p.transfer(at(0, 1, 1), 2, 70);
    let Missing::Present(x) = p.parcel_at(at(0, 0, 0)) else { panic!("held") };
    assert_eq!((x.owner(), x.cost()), (2, 70), "the holding's transfer writes its owner and cost");
    let _ = p.split(rect(0, 0, 1, 2), t);
    let Missing::Present(y) = p.parcel_at(at(0, 1, 0)) else { panic!("held") };
    assert_eq!(y.owner(), 2, "a cut keeps the owner: only a transfer moves it");
}

#[test]
fn cells_roundtrip() {
    let mut p = parcels();
    p.insert(TileId::new(0), rect(0, 0, 2, 2), (1, 50), OWNED);
    p.insert(TileId::new(7), rect(4, 4, 3, 1), (2, -9), OWNED);
    let _ = p.split(rect(0, 0, 1, 1), TileId::new(0));
    let (back, _) = phx_store::roundtrip(&p).unwrap();
    assert_eq!(back, p, "the parcels saved, the runs and directory rebuilt");
}

/// Three segments from tile 0's centre: along its row of tiles, down its column, and diagonally to tile 5's centre.
fn segments() -> (Segments, [Line; 3]) {
    let c = cells();
    let mut s = Segments::default();
    let lines = [c.line(c.at(5, 5), c.at(25, 5)), c.line(c.at(5, 5), c.at(5, 15)), c.line(c.at(5, 5), c.at(15, 15))];
    for _ in lines {
        let _ = s.open(SegmentDecl {
            from: 0,
            to: 1,
            mode: 0,
            metres: 1,
            capacity: 1,
            condition: 0,
            unit: Missing::Absent,
        });
    }
    (s, lines)
}

#[test]
fn path_cells_from_segment_line() {
    let c = cells();
    let mut seen = Vec::new();
    let line = c.line(c.at(0, 0), c.at(4, 2));
    let n = c.cells_of(line, |x| seen.push(c.coords(x)));
    assert_eq!(n, 7);
    assert_eq!(seen, [(0, 0), (1, 0), (1, 1), (2, 1), (3, 1), (3, 2), (4, 2)], "every cell the segment touches");
    assert!(seen.iter().all(|(x, y)| c.crosses(line, c.at(i64::from(*x), i64::from(*y)))), "walk and test agree");
    assert!(!c.crosses(line, c.at(2, 0)) && !c.crosses(line, c.at(4, 1)), "cells off the line");
    let corner = c.line(c.at(0, 0), c.at(2, 2));
    assert!(c.crosses(corner, c.at(1, 0)), "a cell the diagonal touches at a corner");
    let mut tiles = Vec::new();
    c.tiles_of(c.line(c.at(5, 5), c.at(25, 5)), &mut tiles);
    assert_eq!(tiles, [TileId::new(0), TileId::new(1), TileId::new(2)]);
    c.tiles_of(c.line(c.at(5, 5), c.at(35, 5)), &mut tiles);
    assert_eq!(tiles, [TileId::new(0), TileId::new(3)], "the shorter way, round the seam");
}

#[test]
fn crossing_reads_tile_index() {
    let c = cells();
    let (s, lines) = segments();
    let mut ix: SegmentIndex<Heap> = SegmentIndex::new(&mut AddressSpace::empty(), c, 1 << 12);
    let line_of = |id: SegmentId| *lines.get(usize::try_from(id.get()).unwrap()).unwrap();
    let _ = ix.rebuild(&s, line_of);
    assert_eq!(ix.in_tile(TileId::new(0)), &[0, 1, 2]);
    assert_eq!(ix.in_tile(TileId::new(1)), &[0, 2], "the row's line, and the diagonal touching its corner");
    assert_eq!(ix.in_tile(TileId::new(4)), &[1, 2], "the column's line, and the diagonal touching its corner");
    let mut out = Vec::new();
    let read = ix.crossing(c.at(12, 5), line_of, &mut out);
    assert_eq!((read, out.clone()), (2, vec![SegmentId::new(0)]), "the tile's two read, the one touching kept");
    let _ = ix.crossing(c.at(7, 7), line_of, &mut out);
    assert_eq!(out, [SegmentId::new(2)], "the diagonal's cell, not the others'");
    let _ = ix.crossing(c.at(7, 9), line_of, &mut out);
    assert!(out.is_empty(), "a cell no line touches");
}

#[test]
fn segment_index_rebuilt_equal() {
    let c = cells();
    let (mut s, lines) = segments();
    let line_of = |id: SegmentId| *lines.get(usize::try_from(id.get()).unwrap()).unwrap();
    let mut fed: SegmentIndex<Heap> = SegmentIndex::new(&mut AddressSpace::empty(), c, 1 << 12);
    let mut changes = Vec::new();
    s.take_changes(&mut changes);
    let _ = fed.follow(&changes, line_of);
    s.remove(SegmentId::new(1));
    changes.clear();
    s.take_changes(&mut changes);
    assert_eq!(changes, [(SegmentId::new(1), Change::Removed)]);
    let _ = fed.follow(&changes, line_of);
    let mut rebuilt: SegmentIndex<Heap> = SegmentIndex::new(&mut AddressSpace::empty(), c, 1 << 12);
    let _ = rebuilt.rebuild(&s, line_of);
    for t in 0..12 {
        assert_eq!(fed.in_tile(TileId::new(t)), rebuilt.in_tile(TileId::new(t)), "tile {t}");
    }
    assert_eq!(StoreStats::rows_live(&fed), StoreStats::rows_live(&rebuilt));
}

#[test]
fn parcel_lookup_in_long_run() {
    // Past a short run's length a tile's parcels are searched: a tall parcel found back past many later origins.
    let mut p = parcels();
    let t = TileId::new(6);
    p.insert(t, rect(0, 0, 10, 1), (1, 1), OWNED);
    for row in 0..10 {
        for col in 1..5 {
            p.insert(t, rect(row, col, 1, 1), (2, 1), OWNED);
        }
    }
    assert!(p.in_tile(t).len() > 32);
    let Missing::Present(tall) = p.parcel_at(at(6, 9, 0)) else { panic!("held") };
    assert_eq!(tall.owner(), 1);
    let Missing::Present(small) = p.parcel_at(at(6, 9, 3)) else { panic!("held") };
    assert_eq!(small.owner(), 2);
    assert_eq!(p.parcel_at(at(6, 9, 7)), Missing::Absent);
}
