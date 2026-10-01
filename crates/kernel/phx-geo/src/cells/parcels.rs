//! Land held apart from its tile: one row per parcel, a rectangle of a tile's cells, kept in a run per tile sorted by
//! its origin, so a cell's parcel is a binary search and a short walk back. Parcels in a tile never overlap; a part cut
//! out of one leaves the rest as whole rectangles, its cost shared by cells with the residue named.

use phx_id::TileId;
use phx_macros::{Pod, clause};
use phx_num::apportion::{Residue, Ties, apportion};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{
    AddressSpace, Backing, CellListRef, CellLists, ChunkArena, ListRef, Region, StoreStats, SystemBacking,
};

use super::{CellGrid, CellId, tile_index};
use crate::consts::PARCEL_SHORT_RUN;

/// Why land is held apart from its tile, as declared flags; a parcel held for no reason is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held(pub u16);

/// A rectangle of a tile's cells: its first cell's place, row by row, and its width and height in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub origin: u16,
    pub width: u8,
    pub height: u8,
}

/// A parcel: its cost, its owner, its tile, its rectangle and why it is held apart.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Parcel {
    cost: i64,
    owner: u32,
    tile: u32,
    origin: u16,
    why: u16,
    width: u8,
    height: u8,
    pad: u16,
}

impl Parcel {
    #[must_use]
    pub fn cost(&self) -> i64 {
        self.cost
    }

    #[must_use]
    pub fn owner(&self) -> u32 {
        self.owner
    }

    pub fn tile(&self) -> TileId {
        TileId::new(self.tile)
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        Rect { origin: self.origin, width: self.width, height: self.height }
    }

    #[must_use]
    pub fn held(&self) -> Held {
        Held(self.why)
    }

    /// The cells it covers.
    #[must_use]
    pub fn cells(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// Every parcel, a run a tile: each tile's run reference and its tallest parcel's height, the runs' rows, and the
/// scratch their compaction copies through.
#[clause("GEO.5", "GEO.19")]
#[derive(Debug)]
pub struct Parcels<B: Backing = SystemBacking> {
    pub(super) cells: CellGrid,
    runs: Vec<CellListRef>,
    tallest: Vec<u8>,
    arena: ChunkArena<B, Parcel>,
    scratch: Region<Parcel, B>,
    pub(super) reserved: u32,
    rows: u64,
}

/// A rectangle's rows and columns within its tile: its first row and column, and the ones past it.
fn span(side: u8, r: Rect) -> (u16, u16, u16, u16) {
    let side = u16::from(side);
    let (row, col) = (r.origin / side, r.origin % side);
    (row, col, row + u16::from(r.height), col + u16::from(r.width))
}

fn rows(n: u32) -> usize {
    match usize::try_from(n) {
        Ok(n) => n,
        Err(_) => capacity_exceeded!("parcels", usize::MAX, n),
    }
}

fn overlap(side: u8, a: Rect, b: Rect) -> bool {
    let ((ar, ac, ar2, ac2), (br, bc, br2, bc2)) = (span(side, a), span(side, b));
    ar < br2 && br < ar2 && ac < bc2 && bc < ac2
}

fn holds(side: u8, r: Rect, place: u16) -> bool {
    let (row, col, row2, col2) = span(side, r);
    let (pr, pc) = (place / u16::from(side), place % u16::from(side));
    row <= pr && pr < row2 && col <= pc && pc < col2
}

impl<B: Backing> Parcels<B> {
    /// No parcel yet over a grid's cells, room reserved for `reserved` rows.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, cells: CellGrid, reserved: u32) -> Parcels<B> {
        let tiles = cells.grid().len();
        Parcels {
            cells,
            runs: vec![CellListRef::EMPTY; tiles],
            tallest: vec![0; tiles],
            arena: ChunkArena::new(space, reserved),
            scratch: Region::reserve(space, rows(reserved)),
            reserved,
            rows: 0,
        }
    }

    fn run(&self, tile: TileId) -> ListRef {
        match self.runs.get(tile_index(tile)) {
            Some(r) => self.arena.resolve(tile.get(), *r),
            None => violation!(clause = "GEO.19", "a tile past the map", tile = tile.get()),
        }
    }

    fn set_run(&mut self, tile: TileId, list: ListRef) {
        let Some(r) = self.runs.get_mut(tile_index(tile)) else {
            violation!(clause = "GEO.19", "a tile past the map", tile = tile.get());
        };
        self.arena.store(tile.get(), r, list);
    }

    /// A tile's parcels, in their origins' order.
    #[must_use]
    pub fn in_tile(&self, tile: TileId) -> &[Parcel] {
        self.arena.read(self.run(tile))
    }

    /// Land held apart: a rectangle of a tile's cells, its owner, its cost and why. A rectangle leaving its tile,
    /// overlapping a parcel, or held for no reason stops the run.
    #[clause("GEO.5", "GEO.19")]
    pub fn insert(&mut self, tile: TileId, rect: Rect, (owner, cost): (u32, i64), why: Held) {
        let side = self.cells.side();
        let (_, _, row2, col2) = span(side, rect);
        if rect.width == 0 || rect.height == 0 || row2 > u16::from(side) || col2 > u16::from(side) {
            violation!(clause = "GEO.19", "a parcel leaving its tile", tile = tile.get(), origin = rect.origin);
        }
        if why.0 == 0 {
            violation!(clause = "GEO.5", "land held apart for no reason", tile = tile.get());
        }
        if self.in_tile(tile).iter().any(|p| overlap(side, p.rect(), rect)) {
            violation!(clause = "GEO.19", "a parcel over another", tile = tile.get(), origin = rect.origin);
        }
        self.lay(tile, rect, (owner, cost), why);
    }

    /// A parcel's row placed in its tile's run at its origin's place, its rectangle already known to be free.
    fn lay(&mut self, tile: TileId, rect: Rect, (owner, cost): (u32, i64), why: Held) {
        let row = Parcel {
            cost,
            owner,
            tile: tile.get(),
            origin: rect.origin,
            why: why.0,
            width: rect.width,
            height: rect.height,
            pad: 0,
        };
        let mut list = self.run(tile);
        let at = self.arena.read(list).partition_point(|p| p.origin < rect.origin);
        self.arena.append(&mut list, &[row]);
        if let Some(tail) = self.arena.read_mut(list).get_mut(at..) {
            tail.rotate_right(1);
        }
        self.set_run(tile, list);
        if let Some(t) = self.tallest.get_mut(tile_index(tile))
            && *t < rect.height
        {
            *t = rect.height;
        }
        self.rows += 1;
        self.compact_if_due();
    }

    /// The parcel covering a cell, or `Missing` where its land is held with its tile.
    #[clause("GEO.19")]
    pub fn parcel_at(&self, cell: CellId) -> Missing<Parcel> {
        match self.find(cell) {
            Some((_, p)) => Missing::Present(p),
            None => Missing::Absent,
        }
    }

    /// The parcel covering a cell and its place in its tile's run: the last origin at or before the cell, then back
    /// while a parcel that tall could still reach the cell's row.
    fn find(&self, cell: CellId) -> Option<(usize, Parcel)> {
        let tile = self.cells.tile_of(cell);
        let place = self.cells.place_of(cell);
        let side = self.cells.side();
        let run = self.in_tile(tile);
        let row = place / u16::from(side);
        // A short run is read through in order, which its few cache lines stream for; a long one is searched.
        if run.len() <= PARCEL_SHORT_RUN {
            return (0..)
                .zip(run)
                .take_while(|(_, p)| p.origin <= place)
                .find(|(_, p)| holds(side, p.rect(), place))
                .map(|(i, p)| (i, *p));
        }
        let Some(tallest) = self.tallest.get(tile_index(tile)).map(|t| u16::from(*t)) else {
            violation!(clause = "GEO.19", "a tile past the map", tile = tile.get());
        };
        let mut at = run.partition_point(|p| p.origin <= place);
        while at > 0 {
            at -= 1;
            let p = run.get(at)?;
            if p.origin / u16::from(side) + tallest <= row {
                return None;
            }
            if holds(side, p.rect(), place) {
                return Some((at, *p));
            }
        }
        None
    }

    fn remove_at(&mut self, tile: TileId, at: usize) {
        let mut list = self.run(tile);
        let Ok(at) = u32::try_from(at) else { capacity_exceeded!("a tile's parcels", u32::MAX, at) };
        self.arena.remove(&mut list, at, 1);
        self.set_run(tile, list);
        self.rows -= 1;
    }

    /// The parcel covering a cell given back to its tile, its land no longer held apart; what it was.
    #[clause("GEO.5")]
    pub fn release(&mut self, cell: CellId) -> Parcel {
        let Some((at, p)) = self.find(cell) else {
            violation!(clause = "GEO.5", "a release of land not held apart", cell = cell.get());
        };
        self.remove_at(p.tile(), at);
        p
    }

    /// The owner and cost of the parcel covering a cell set by its holding's transfer, their one writer.
    #[clause("GEO.5")]
    pub fn transfer(&mut self, cell: CellId, owner: u32, cost: i64) {
        let Some((at, p)) = self.find(cell) else {
            violation!(clause = "GEO.5", "a transfer of land not held apart", cell = cell.get());
        };
        let list = self.run(p.tile());
        if let Some(row) = self.arena.read_mut(list).get_mut(at) {
            (row.owner, row.cost) = (owner, cost);
        }
    }

    /// A part cut out of the parcel that covers it, by guillotine cuts: the rows above and below it across the
    /// parcel's width, and the columns left and right of it across its rows, each its own parcel with the same owner
    /// and reason, the cost shared by cells, the lower pieces' remainders first. The part, as it now stands.
    #[clause("GEO.5", "GEO.19")]
    pub fn split(&mut self, part: Rect, tile: TileId) -> Parcel {
        let side = self.cells.side();
        let Some((at, whole)) = self.find(self.cells.cell(tile, part.origin)) else {
            violation!(clause = "GEO.19", "a part of land not held apart", tile = tile.get(), origin = part.origin);
        };
        let (wr, wc, wr2, wc2) = span(side, whole.rect());
        let (pr, pc, pr2, pc2) = span(side, part);
        if part.width == 0 || part.height == 0 || pr2 > wr2 || pc2 > wc2 {
            violation!(clause = "GEO.19", "a part reaching past its parcel", tile = tile.get(), origin = part.origin);
        }
        let s = u16::from(side);
        let rect = |row: u16, col: u16, height: u16, width: u16| {
            let (Ok(height), Ok(width)) = (u8::try_from(height), u8::try_from(width)) else {
                capacity_exceeded!("a parcel's side", u8::MAX, height);
            };
            Rect { origin: row * s + col, width, height }
        };
        let mut pieces = [part, part, part, part, part];
        let mut n = 1;
        let mut add = |r: Rect| {
            if r.width > 0
                && r.height > 0
                && let Some(slot) = pieces.get_mut(n)
            {
                *slot = r;
                n += 1;
            }
        };
        add(rect(wr, wc, pr - wr, wc2 - wc));
        add(rect(pr2, wc, wr2 - pr2, wc2 - wc));
        add(rect(pr, wc, pr2 - pr, pc - wc));
        add(rect(pr, pc2, pr2 - pr, wc2 - pc2));
        let mut weights = [0_u64; 5];
        let mut costs = [0_i64; 5];
        for (w, r) in weights.iter_mut().zip(&pieces) {
            *w = u64::from(r.width) * u64::from(r.height);
        }
        let (Some(pieces), Some(weights), Some(costs)) = (pieces.get(..n), weights.get(..n), costs.get_mut(..n)) else {
            violation!(clause = "GEO.19", "a cut into more pieces than a rectangle leaves", pieces = n);
        };
        apportion(whole.cost, weights, Residue::LargestRemainder { ties: Ties::Order }, costs);
        // The pieces tile the parcel they replace, so none can lie over another parcel: the part takes the whole's
        // row, the rest join the run, and the run is put back in its origins' order.
        let row = |r: &Rect, cost: i64| Parcel { cost, origin: r.origin, width: r.width, height: r.height, ..whole };
        let mut rows = [whole; 4];
        for (slot, (r, cost)) in rows.iter_mut().zip(pieces.iter().zip(costs.iter().copied()).skip(1)) {
            *slot = row(r, cost);
        }
        let mut list = self.run(tile);
        if let (Some(first), Some(cost)) = (self.arena.read_mut(list).get_mut(at), costs.first()) {
            *first = row(&part, *cost);
        }
        let Some(others) = rows.get(..n - 1) else {
            violation!(clause = "GEO.19", "a cut into more pieces than a rectangle leaves", pieces = n);
        };
        self.arena.append(&mut list, others);
        self.arena.read_mut(list).sort_unstable_by_key(|p| p.origin);
        self.set_run(tile, list);
        self.rows += others.iter().map(|_| 1).sum::<u64>();
        self.compact_if_due();
        match self.parcel_at(self.cells.cell(tile, part.origin)) {
            Missing::Present(p) => p,
            Missing::Absent => violation!(clause = "GEO.19", "a part lost in its cut", tile = tile.get()),
        }
    }

    fn compact_if_due(&mut self) {
        if self.arena.needs_compaction() {
            let mut lists = CellLists { first_owner: 0, refs: &mut self.runs };
            self.arena.compact(&mut lists, &mut self.scratch);
        }
    }

    /// The parcels held.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.rows
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// The bytes its directory and rows hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.runs.capacity() * size_of::<CellListRef>() + self.tallest.capacity() + self.arena.bytes_committed()
    }
}

impl<B: Backing> PartialEq for Parcels<B> {
    /// Two stores alike in their cells and every tile's parcels, however their rows lie in their arenas.
    fn eq(&self, other: &Parcels<B>) -> bool {
        self.cells == other.cells
            && self.runs.len() == other.runs.len()
            && (0_u32..).zip(&self.runs).all(|(t, _)| self.in_tile(TileId::new(t)) == other.in_tile(TileId::new(t)))
    }
}

impl<B: Backing> StoreStats for Parcels<B> {
    fn rows_live(&self) -> u64 {
        self.rows
    }

    fn rows_ever(&self) -> u64 {
        self.rows
    }

    fn bytes(&self) -> u64 {
        u64::try_from(Parcels::bytes(self)).unwrap_or(u64::MAX)
    }
}
