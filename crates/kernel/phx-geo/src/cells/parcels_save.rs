//! The parcels saved as their rows in tile order, and read back by laying each again, so their runs and directory are
//! rebuilt rather than saved.

use phx_id::TileId;
use phx_store::{Backing, LoadError, Reader, Saved, Transform, Writer};

use super::CellGrid;
use super::parcels::{Parcel, Parcels};
use crate::grid::Grid;

impl<B: Backing> Saved for Parcels<B> {
    /// The cells, the room reserved and every parcel in tile order: the runs and their directory are rebuilt.
    fn save(&self, w: &mut Writer<'_>) {
        let grid = self.cells.grid();
        for n in [grid.width, grid.height, grid.tile_m, self.cells.cell_m(), self.reserved] {
            n.save(w);
        }
        let mut all = Vec::new();
        for tile in (0_u32..).take(grid.len()) {
            all.extend_from_slice(self.in_tile(TileId::new(tile)));
        }
        w.rows(&all, Transform::Plain);
    }

    fn load(r: &mut Reader<'_>) -> Result<Parcels<B>, LoadError> {
        let mut n = [0_u32; 5];
        for v in &mut n {
            *v = u32::load(r)?;
        }
        let [width, height, tile_m, cell_m, reserved] = n;
        let grid = Grid { width, height, tile_m };
        let cells = CellGrid::new(grid, cell_m).map_err(LoadError::Invalid)?;
        let rows: Vec<Parcel> = r.rows(Transform::Plain)?;
        let mut out = Parcels::new(r.space(), cells, reserved);
        for p in rows {
            out.insert(p.tile(), p.rect(), (p.owner(), p.cost()), p.held());
        }
        Ok(out)
    }
}
