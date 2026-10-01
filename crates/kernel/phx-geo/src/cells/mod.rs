//! Cells: each tile divided into square cells of a declared side, a cell's identity its tile's times the cells a
//! tile holds plus its place within the tile, row by row. Nothing is kept per cell: its coordinates, its tile and so
//! its surface and terrain are arithmetic on its identity. Land held apart from its tile is kept as parcels, and the
//! cells a path crosses are read from its segment's line through a per-tile index.

pub mod parcels;
mod parcels_save;
pub mod segment_index;

use phx_id::TileId;
use phx_macros::{Pod, clause};
use phx_num::{capacity_exceeded, violation};

use crate::grid::Grid;

pub use parcels::{Held, Parcel, Parcels, Rect};
pub use segment_index::SegmentIndex;

/// A cell: its tile's identity times the cells a tile holds, plus its place in the tile, row by row.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Pod)]
pub struct CellId(u32);

impl CellId {
    #[must_use]
    pub const fn new(id: u32) -> CellId {
        CellId(id)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The cells over a grid: the tiles' grid and how many cells a tile's side holds.
#[clause("GEO.19")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellGrid {
    grid: Grid,
    side: u8,
    cell_m: u32,
}

impl CellGrid {
    /// A cell's side in metres.
    #[must_use]
    pub fn cell_m(&self) -> u32 {
        self.cell_m
    }
}

/// A path's line between the centres of two cells, taken the shorter way round the closed surface: its first cell's
/// column and row and the columns and rows it runs, so testing it against a cell needs no division.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    x: i64,
    y: i64,
    dx: i64,
    dy: i64,
}

/// A line walked along its longer axis: its run along that axis and across it, the bound twice a cell's offset
/// across may reach, and which axis is the longer.
struct Walk {
    along: i64,
    across: i64,
    half: i64,
    steep: bool,
}

impl Walk {
    fn of(line: Line) -> Walk {
        let steep = line.dy.abs() > line.dx.abs();
        let (along, across) = if steep { (line.dy, line.dx) } else { (line.dx, line.dy) };
        Walk { along, across, half: along.abs() + across.abs(), steep }
    }

    /// The offsets across the walk of the cells whose squares the segment touches at a step along it:
    /// |2·(along·c − across·p)| ≤ |along| + |across|, within the box between the ends.
    fn across_at(&self, p: i64) -> (i64, i64) {
        if self.along == 0 {
            return (0, 0);
        }
        let (twice, sign) = (2 * self.along.abs(), self.along.signum());
        let b = sign * 2 * self.across * p;
        let (low, high) = (ceil_div(b - self.half, twice), (b + self.half).div_euclid(twice));
        let (min, max) = if self.across < 0 { (self.across, 0) } else { (0, self.across) };
        (if low < min { min } else { low }, if high > max { max } else { high })
    }
}

fn index(n: u64) -> usize {
    match usize::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("cells", usize::MAX, n),
    }
}

impl CellGrid {
    /// The cells of a grid at a cell's side in metres: a tile's side a whole number of cells, at most 255 of them, and
    /// every cell's identity within 32 bits.
    ///
    /// # Errors
    /// A cell that does not divide its tile, a tile of more cells than a parcel's side can span, or more cells than
    /// an identity holds.
    #[clause("GEO.19")]
    #[phx_macros::opening]
    pub fn new(grid: Grid, cell_m: u32) -> Result<CellGrid, String> {
        if cell_m == 0 || !grid.tile_m.is_multiple_of(cell_m) {
            return Err(format!("a cell of {cell_m} m does not divide a tile of {} m", grid.tile_m));
        }
        let side = u8::try_from(grid.tile_m / cell_m)
            .map_err(|_| format!("a tile of {} cells a side, past a parcel's span", grid.tile_m / cell_m))?;
        let cells = u64::from(side) * u64::from(side) * u64::from(grid.width) * u64::from(grid.height);
        if cells > u64::from(u32::MAX) {
            return Err(format!("{cells} cells, past a cell's 32-bit identity"));
        }
        Ok(CellGrid { grid, side, cell_m })
    }

    /// The cells a tile's side holds.
    #[must_use]
    pub fn side(&self) -> u8 {
        self.side
    }

    /// The cells a tile holds.
    #[must_use]
    pub fn per_tile(&self) -> u32 {
        u32::from(self.side) * u32::from(self.side)
    }

    #[must_use]
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// The cell at a place within a tile.
    #[must_use]
    pub fn cell(&self, tile: TileId, place: u16) -> CellId {
        if u32::from(place) >= self.per_tile() {
            violation!(clause = "GEO.19", "a place past its tile's cells", place = place);
        }
        CellId(tile.get() * self.per_tile() + u32::from(place))
    }

    pub fn tile_of(&self, c: CellId) -> TileId {
        TileId::new(c.0 / self.per_tile())
    }

    /// A cell's place within its tile, row by row.
    #[must_use]
    pub fn place_of(&self, c: CellId) -> u16 {
        match u16::try_from(c.0 % self.per_tile()) {
            Ok(p) => p,
            Err(_) => capacity_exceeded!("a tile's cells", u16::MAX, c.0 % self.per_tile()),
        }
    }

    /// A cell's column and row across the whole surface.
    #[must_use]
    pub fn coords(&self, c: CellId) -> (u32, u32) {
        let (tx, ty) = self.grid.xy(self.tile_of(c));
        let place = u32::from(self.place_of(c));
        let side = u32::from(self.side);
        (tx * side + place % side, ty * side + place / side)
    }

    /// The cell at a column and row across the surface, either taken round the seams.
    #[must_use]
    pub fn at(&self, column: i64, row: i64) -> CellId {
        let side = i64::from(self.side);
        let (across, down) = (i64::from(self.grid.width) * side, i64::from(self.grid.height) * side);
        let (column, row) = (column.rem_euclid(across), row.rem_euclid(down));
        let tile = self.grid.at(narrow(column / side), narrow(row / side));
        let place = (row % side) * side + column % side;
        match u16::try_from(place) {
            Ok(p) => self.cell(tile, p),
            Err(_) => capacity_exceeded!("a tile's cells", u16::MAX, place),
        }
    }

    /// The cell at a tile's centre, where a zone's node lies.
    #[must_use]
    pub fn centre_of(&self, tile: TileId) -> CellId {
        let half = u16::from(self.side / 2);
        self.cell(tile, half * u16::from(self.side) + half)
    }

    /// The metres between two cells' centres over the surface, the shorter way round.
    #[must_use]
    pub fn distance_m(&self, a: CellId, b: CellId) -> u64 {
        let line = self.line(a, b);
        let m = u64::from(self.cell_m);
        crate::grid::rounded_sqrt((line.dx.unsigned_abs() * m).pow(2) + (line.dy.unsigned_abs() * m).pow(2))
    }

    /// The surface's columns and rows of cells.
    fn spans(&self) -> (i64, i64) {
        let side = i64::from(self.side);
        (i64::from(self.grid.width) * side, i64::from(self.grid.height) * side)
    }

    /// The line between two cells' centres, the shorter way round each seam.
    #[must_use]
    pub fn line(&self, from: CellId, to: CellId) -> Line {
        let (across, down) = self.spans();
        let ((ax, ay), (bx, by)) = (self.coords(from), self.coords(to));
        let (x, y) = (i64::from(ax), i64::from(ay));
        Line { x, y, dx: near(i64::from(bx) - x, across), dy: near(i64::from(by) - y, down) }
    }

    /// Every cell a line crosses — each cell whose square the segment between the two centres touches, corners
    /// included — in order along the line; the cells visited.
    #[clause("GEO.19")]
    pub fn cells_of(&self, line: Line, mut visit: impl FnMut(CellId)) -> u64 {
        let walk = Walk::of(line);
        let mut visited = 0_u64;
        for k in 0..=walk.along.abs() {
            let p = k * walk.along.signum();
            let (lo, hi) = walk.across_at(p);
            for c in lo..=hi {
                let (x, y) = if walk.steep { (line.x + c, line.y + p) } else { (line.x + p, line.y + c) };
                visit(self.at(x, y));
                visited += 1;
            }
        }
        visited
    }

    /// Whether a line touches a cell's square, by the same test its walk uses.
    #[must_use]
    pub fn crosses(&self, line: Line, cell: CellId) -> bool {
        let (cx, cy) = self.coords(cell);
        self.crosses_at(line, (i64::from(cx), i64::from(cy)))
    }

    /// Whether a line touches the square of the cell at a column and row, with no division.
    fn crosses_at(&self, line: Line, (cx, cy): (i64, i64)) -> bool {
        let (across, down) = self.spans();
        let (cx, cy) = (near(cx - line.x, across), near(cy - line.y, down));
        let inside = |c: i64, d: i64| if d < 0 { d <= c && c <= 0 } else { 0 <= c && c <= d };
        let cross = line.dx * cy - line.dy * cx;
        inside(cx, line.dx) && inside(cy, line.dy) && 2 * cross.abs() <= line.dx.abs() + line.dy.abs()
    }

    /// The tiles a line crosses, each once, in identity order, into a buffer the caller keeps: the walk taken a tile's
    /// width at a time along its longer axis, the cells it touches across within each stretch read at the stretch's
    /// two ends, the walk being monotone.
    pub fn tiles_of(&self, line: Line, out: &mut Vec<TileId>) {
        out.clear();
        let walk = Walk::of(line);
        let side = i64::from(self.side);
        let (start, origin_across) = if walk.steep { (line.y, line.x) } else { (line.x, line.y) };
        let (sign, end) = (walk.along.signum(), walk.along.abs());
        let mut k = 0;
        while k <= end {
            let a = start + k * sign;
            let room = if sign < 0 { a.rem_euclid(side) } else { side - 1 - a.rem_euclid(side) };
            let last = if k + room > end { end } else { k + room };
            let ((lo0, hi0), (lo1, hi1)) = (walk.across_at(k * sign), walk.across_at(last * sign));
            let (lo, hi) = (if lo0 < lo1 { lo0 } else { lo1 }, if hi0 > hi1 { hi0 } else { hi1 });
            let block = a.div_euclid(side);
            for row in (origin_across + lo).div_euclid(side)..=(origin_across + hi).div_euclid(side) {
                let (tx, ty) = if walk.steep { (row, block) } else { (block, row) };
                out.push(self.tile_at(tx, ty));
            }
            k = last + 1;
        }
        out.sort_unstable();
        out.dedup();
    }

    /// The tile at a column and row of tiles, either taken round the seams.
    fn tile_at(&self, tx: i64, ty: i64) -> TileId {
        let (w, h) = (i64::from(self.grid.width), i64::from(self.grid.height));
        self.grid.at(narrow(tx.rem_euclid(w)), narrow(ty.rem_euclid(h)))
    }

    /// A cell's column and row, signed, for the tests that compare it with lines.
    #[must_use]
    pub fn point(&self, cell: CellId) -> (i64, i64) {
        let (x, y) = self.coords(cell);
        (i64::from(x), i64::from(y))
    }

    /// Whether a line touches the cell at a column and row already read.
    #[must_use]
    pub fn crosses_point(&self, line: Line, point: (i64, i64)) -> bool {
        self.crosses_at(line, point)
    }
}

/// A gap of less than a whole span along an axis of `span` cells that closes on itself, taken the shorter way:
/// within half the span either way.
fn near(gap: i64, span: i64) -> i64 {
    let gap = if gap < 0 { gap + span } else { gap };
    if 2 * gap > span { gap - span } else { gap }
}

/// The least whole number at or above a fraction over a positive denominator.
fn ceil_div(n: i64, d: i64) -> i64 {
    n.div_euclid(d) + i64::from(n.rem_euclid(d) != 0)
}

fn narrow(n: i64) -> u32 {
    match u32::try_from(n) {
        Ok(n) => n,
        Err(_) => capacity_exceeded!("cells", u32::MAX, n),
    }
}

/// A segment's line: from its first zone's node to its second's, each the centre cell of the zone's centroid tile.
#[must_use]
pub fn segment_line(cells: &CellGrid, map: &crate::generate::Map, row: crate::transport::SegmentRow) -> Line {
    let node = |z: u16| match map.zones.get(usize::from(z)) {
        Some(zone) => cells.centre_of(zone.centroid),
        None => violation!(clause = "GEO.19", "a segment's zone past the map", zone = z),
    };
    let (a, b) = row.ends();
    cells.line(node(a), node(b))
}

/// Whether a cell is held: a named unit's site, a parcel covering it or a path crossing it — the three reads, never a
/// row of the cell's own.
#[clause("GEO.19")]
pub fn held<B: phx_store::Backing>(
    cell: CellId,
    (parcels, index): (&Parcels<B>, &SegmentIndex<B>),
    (site, line_of): (impl Fn(CellId) -> bool, impl Fn(crate::transport::SegmentId) -> Line),
    buf: &mut Vec<crate::transport::SegmentId>,
) -> bool {
    if site(cell) || matches!(parcels.parcel_at(cell), phx_num::Missing::Present(_)) {
        return true;
    }
    let _ = index.crossing(cell, line_of, buf);
    !buf.is_empty()
}

/// The places a tile's cells hold, as an index into a per-tile table.
#[must_use]
pub fn tile_index(tile: TileId) -> usize {
    index(u64::from(tile.get()))
}

#[cfg(test)]
#[path = "cells_tests.rs"]
mod tests;
