use phx_id::TileId;
use phx_macros::clause;
use phx_num::capacity_exceeded;

/// The world's rectangle of square tiles on one planar projection in metres: a tile's identity is its row-major
/// place, its coordinates derive from it, and its neighbours are the eight around it.
#[clause("GEO.1", "GEO.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub tile_m: u32,
}

/// The eight neighbours' offsets, in one fixed order.
const AROUND: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

impl Grid {
    #[must_use]
    pub fn len(&self) -> usize {
        let Ok(n) = usize::try_from(u64::from(self.width) * u64::from(self.height)) else {
            capacity_exceeded!("tiles", usize::MAX, u64::from(self.width) * u64::from(self.height));
        };
        n
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn xy(&self, t: TileId) -> (u32, u32) {
        (t.get() % self.width, t.get() / self.width)
    }

    pub fn at(&self, x: u32, y: u32) -> TileId {
        TileId::new(y * self.width + x)
    }

    #[must_use]
    pub fn index(&self, t: TileId) -> usize {
        let Ok(i) = usize::try_from(t.get()) else {
            capacity_exceeded!("tile index", usize::MAX, t.get());
        };
        i
    }

    pub fn tile(&self, index: usize) -> TileId {
        let Ok(i) = u32::try_from(index) else {
            capacity_exceeded!("tile identity", u32::MAX, index);
        };
        TileId::new(i)
    }

    /// The tiles around a tile, in one fixed order, those off the grid left out.
    pub fn neighbours(&self, t: TileId) -> impl Iterator<Item = TileId> + '_ {
        let (x, y) = self.xy(t);
        AROUND.iter().filter_map(move |(dx, dy)| {
            let nx = u32::try_from(i64::from(x) + dx).ok().filter(|v| *v < self.width)?;
            let ny = u32::try_from(i64::from(y) + dy).ok().filter(|v| *v < self.height)?;
            Some(self.at(nx, ny))
        })
    }

    /// A tile's centre, in metres from the grid's corner.
    #[must_use]
    pub fn centre_m(&self, t: TileId) -> (u64, u64) {
        let (x, y) = self.xy(t);
        let half = u64::from(self.tile_m) / 2;
        (u64::from(x) * u64::from(self.tile_m) + half, u64::from(y) * u64::from(self.tile_m) + half)
    }

    /// The straight length between two tiles' centres over their elevations, in whole metres, rounded to the nearest.
    #[must_use]
    pub fn length_m(&self, a: TileId, a_elev: i16, b: TileId, b_elev: i16) -> u64 {
        let (ax, ay) = self.centre_m(a);
        let (bx, by) = self.centre_m(b);
        let dz = u64::from((i32::from(a_elev) - i32::from(b_elev)).unsigned_abs());
        rounded_sqrt(ax.abs_diff(bx).pow(2) + ay.abs_diff(by).pow(2) + dz.pow(2))
    }

    /// The straight length between two tiles' centres over the plane, a lower bound of any path between them.
    #[must_use]
    pub fn plane_m(&self, a: TileId, b: TileId) -> u64 {
        let (ax, ay) = self.centre_m(a);
        let (bx, by) = self.centre_m(b);
        rounded_sqrt(ax.abs_diff(bx).pow(2) + ay.abs_diff(by).pow(2))
    }
}

/// The square root rounded to the nearest whole number, exactly: `s` or `s + 1`, where `s` is the root rounded down,
/// by whether `n` passes `(s + ½)²`.
#[must_use]
pub fn rounded_sqrt(n: u64) -> u64 {
    let s = n.isqrt();
    if n - s * s > s { s + 1 } else { s }
}

#[cfg(test)]
mod tests {
    use phx_id::TileId;

    use super::{Grid, rounded_sqrt};

    #[test]
    fn grid_places_and_neighbours() {
        let g = Grid { width: 4, height: 3, tile_m: 10_000 };
        assert_eq!((g.len(), g.xy(TileId::new(6)), g.at(2, 1)), (12, (2, 1), TileId::new(6)));
        assert_eq!(g.neighbours(TileId::new(0)).map(TileId::get).collect::<Vec<_>>(), vec![1, 4, 5]);
        assert_eq!(g.neighbours(TileId::new(5)).count(), 8);
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(1), 0), 10_000);
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(5), 0), 14_142);
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(1), 1_000), 10_050);
        assert_eq!((rounded_sqrt(8), rounded_sqrt(12), rounded_sqrt(13)), (3, 3, 4));
    }
}
