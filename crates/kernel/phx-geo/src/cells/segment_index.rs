//! The segments by tile: each tile's list of the segments whose line crosses it, in identity order, fed as segments
//! open and are removed and rebuilt at load from the open segments, so a cell's paths are found from its tile's few
//! segments and no path's cells are ever stored.

use phx_id::TileId;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::{
    AddressSpace, Backing, CellListRef, CellLists, ChunkArena, ListRef, Region, StoreStats, SystemBacking,
};

use super::{CellGrid, CellId, Line, tile_index};
use crate::transport::{Change, SegmentId, Segments};

/// Each tile's segment list and the lists' ids, the tiles a line crosses as a kept buffer, and the scratch the lists'
/// compaction copies through.
#[clause("GEO.19")]
#[derive(Debug)]
pub struct SegmentIndex<B: Backing = SystemBacking> {
    cells: CellGrid,
    lists: Vec<CellListRef>,
    arena: ChunkArena<B, u32>,
    scratch: Region<u32, B>,
    tiles: Vec<TileId>,
    entries: u64,
}

fn words(n: u32) -> usize {
    match usize::try_from(n) {
        Ok(n) => n,
        Err(_) => capacity_exceeded!("the segment index", usize::MAX, n),
    }
}

impl<B: Backing> SegmentIndex<B> {
    /// No segment indexed yet over a grid's tiles, room reserved for `reserved` entries.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, cells: CellGrid, reserved: u32) -> SegmentIndex<B> {
        SegmentIndex {
            cells,
            lists: vec![CellListRef::EMPTY; cells.grid().len()],
            arena: ChunkArena::new(space, reserved),
            scratch: Region::reserve(space, words(reserved)),
            tiles: Vec::new(),
            entries: 0,
        }
    }

    fn list(&self, tile: TileId) -> ListRef {
        match self.lists.get(tile_index(tile)) {
            Some(r) => self.arena.resolve(tile.get(), *r),
            None => violation!(clause = "GEO.19", "a tile past the map", tile = tile.get()),
        }
    }

    fn set(&mut self, tile: TileId, list: ListRef) {
        let Some(r) = self.lists.get_mut(tile_index(tile)) else {
            violation!(clause = "GEO.19", "a tile past the map", tile = tile.get());
        };
        self.arena.store(tile.get(), r, list);
    }

    /// The segments whose line crosses a tile, in identity order.
    #[must_use]
    pub fn in_tile(&self, tile: TileId) -> &[u32] {
        self.arena.read(self.list(tile))
    }

    /// A segment listed at every tile its line crosses; listing it twice stops the run. The entries written.
    #[clause("GEO.19")]
    pub fn index_segment(&mut self, id: SegmentId, line: Line) -> u64 {
        let mut tiles = std::mem::take(&mut self.tiles);
        self.cells.tiles_of(line, &mut tiles);
        for tile in &tiles {
            let mut list = self.list(*tile);
            let Err(at) = self.arena.read(list).binary_search(&id.get()) else {
                violation!(clause = "GEO.19", "a segment indexed twice", segment = id.get());
            };
            self.arena.append(&mut list, &[id.get()]);
            if let Some(tail) = self.arena.read_mut(list).get_mut(at..) {
                tail.rotate_right(1);
            }
            self.set(*tile, list);
        }
        let written = u64::try_from(tiles.len()).unwrap_or(u64::MAX);
        self.entries += written;
        self.tiles = tiles;
        self.compact_if_due();
        written
    }

    /// A segment taken off every tile its line crosses; one not listed stops the run. The entries removed.
    #[clause("GEO.19")]
    pub fn unindex_segment(&mut self, id: SegmentId, line: Line) -> u64 {
        let mut tiles = std::mem::take(&mut self.tiles);
        self.cells.tiles_of(line, &mut tiles);
        for tile in &tiles {
            let mut list = self.list(*tile);
            let Ok(at) = self.arena.read(list).binary_search(&id.get()) else {
                violation!(clause = "GEO.19", "a segment taken off a tile it was never on", segment = id.get());
            };
            let Ok(at) = u32::try_from(at) else { capacity_exceeded!("a tile's segments", u32::MAX, at) };
            self.arena.remove(&mut list, at, 1);
            self.set(*tile, list);
        }
        let removed = u64::try_from(tiles.len()).unwrap_or(u64::MAX);
        self.entries -= removed;
        self.tiles = tiles;
        removed
    }

    /// The segments' openings and removals, as their log hands them, followed in the same apply.
    pub fn follow(&mut self, changes: &[(SegmentId, Change)], line_of: impl Fn(SegmentId) -> Line) -> u64 {
        let mut written = 0;
        for (id, change) in changes {
            written += match change {
                Change::Opened => self.index_segment(*id, line_of(*id)),
                Change::Removed => self.unindex_segment(*id, line_of(*id)),
            };
        }
        written
    }

    /// Every list emptied and every segment not removed listed again in identity order, as a load rebuilds it.
    #[phx_macros::opening]
    pub fn rebuild(&mut self, segments: &Segments, line_of: impl Fn(SegmentId) -> Line) -> u64 {
        for (tile, r) in (0_u32..).zip(self.lists.iter_mut()) {
            let mut list = self.arena.resolve(tile, *r);
            self.arena.clear(&mut list);
            self.arena.store(tile, r, list);
        }
        self.entries = 0;
        let mut written = 0;
        for (id, row) in segments.iter() {
            if !row.removed() {
                written += self.index_segment(id, line_of(id));
            }
        }
        written
    }

    /// The segments whose line crosses a cell, from its tile's list, each tested against the cell; the segments
    /// read.
    #[clause("GEO.19")]
    pub fn crossing(&self, cell: CellId, line_of: impl Fn(SegmentId) -> Line, out: &mut Vec<SegmentId>) -> u64 {
        out.clear();
        let listed = self.in_tile(self.cells.tile_of(cell));
        let point = self.cells.point(cell);
        for id in listed {
            let id = SegmentId::new(*id);
            if self.cells.crosses_point(line_of(id), point) {
                out.push(id);
            }
        }
        u64::try_from(listed.len()).unwrap_or(u64::MAX)
    }

    fn compact_if_due(&mut self) {
        if self.arena.needs_compaction() {
            let mut lists = CellLists { first_owner: 0, refs: &mut self.lists };
            self.arena.compact(&mut lists, &mut self.scratch);
        }
    }

    /// The bytes its directory and lists hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.lists.capacity() * size_of::<CellListRef>() + self.arena.bytes_committed()
    }
}

impl<B: Backing> StoreStats for SegmentIndex<B> {
    fn rows_live(&self) -> u64 {
        self.entries
    }

    fn rows_ever(&self) -> u64 {
        self.entries
    }

    fn bytes(&self) -> u64 {
        u64::try_from(SegmentIndex::bytes(self)).unwrap_or(u64::MAX)
    }
}
