use phx_id::TileId;
use phx_macros::clause;
use phx_num::capacity_exceeded;

/// The world's grid of square tiles over a closed surface: it wraps east to west and north to south, so it has no
/// edge. A tile's identity is its row-major place, its coordinates derive from it, and its neighbours are the eight
/// around it, across the seams as anywhere else.
#[clause("GEO.1", "GEO.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub tile_m: u32,
}

/// The eight neighbours' offsets, in one fixed order.
const AROUND: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// How many neighbours a tile has.
pub const DIRECTIONS: usize = AROUND.len();

/// A coordinate moved by a step of -1, 0 or 1 along an axis of `span` tiles, round the seam.
fn wrap(at: u32, by: i64, span: u32) -> u32 {
    let moved = (i64::from(at) + by).rem_euclid(i64::from(span));
    u32::try_from(moved).unwrap_or(at)
}

/// The shorter way between two coordinates along an axis of `span` units that closes on itself.
fn around(a: u64, b: u64, span: u64) -> u64 {
    let direct = a.abs_diff(b);
    let other = span - direct;
    if other < direct { other } else { direct }
}

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

    /// The tiles around a tile, in one fixed order, across the seams. On a grid under three tiles across, where a
    /// step round the seam comes back to the tile or to one already given, that step is left out.
    pub fn neighbours(&self, t: TileId) -> impl Iterator<Item = TileId> + '_ {
        let (x, y) = self.xy(t);
        let thin = self.width <= 2 || self.height <= 2;
        let mut seen = [t; DIRECTIONS];
        let mut count = 0;
        AROUND.iter().filter_map(move |(dx, dy)| {
            let n = self.at(wrap(x, *dx, self.width), wrap(y, *dy, self.height));
            if !thin {
                return Some(n);
            }
            if n == t || seen.get(..count).is_some_and(|s| s.contains(&n)) {
                return None;
            }
            if let Some(slot) = seen.get_mut(count) {
                *slot = n;
                count += 1;
            }
            Some(n)
        })
    }

    /// Which of the eight directions leads from `a` to its neighbour `b`, if `b` is one.
    #[must_use]
    pub fn direction(&self, a: TileId, b: TileId) -> Option<usize> {
        let (x, y) = self.xy(a);
        AROUND.iter().position(|(dx, dy)| self.at(wrap(x, *dx, self.width), wrap(y, *dy, self.height)) == b)
    }

    /// The seams a step from `a` to its neighbour `b` crosses, east and south counted as one, west and north as minus
    /// one, across and down.
    #[must_use]
    pub fn seams(&self, a: TileId, b: TileId) -> (i64, i64) {
        let ((ax, ay), (bx, by)) = (self.xy(a), self.xy(b));
        let cross = |from: u32, to: u32| {
            let jump = i64::from(to) - i64::from(from);
            i64::from(jump < -1) - i64::from(jump > 1)
        };
        (cross(ax, bx), cross(ay, by))
    }

    /// A tile's centre, in metres from the first tile's corner.
    #[must_use]
    pub fn centre_m(&self, t: TileId) -> (u64, u64) {
        let (x, y) = self.xy(t);
        let half = u64::from(self.tile_m) / 2;
        (u64::from(x) * u64::from(self.tile_m) + half, u64::from(y) * u64::from(self.tile_m) + half)
    }

    /// The metres across and down between two tiles' centres, each the shorter way round.
    fn apart_m(&self, a: TileId, b: TileId) -> (u64, u64) {
        let ((ax, ay), (bx, by)) = (self.centre_m(a), self.centre_m(b));
        let tile = u64::from(self.tile_m);
        (around(ax, bx, u64::from(self.width) * tile), around(ay, by, u64::from(self.height) * tile))
    }

    /// The straight length between two tiles' centres over their elevations, the shorter way round, in whole metres,
    /// rounded to the nearest.
    #[must_use]
    pub fn length_m(&self, a: TileId, a_elev: i16, b: TileId, b_elev: i16) -> u64 {
        let (dx, dy) = self.apart_m(a, b);
        let dz = u64::from((i32::from(a_elev) - i32::from(b_elev)).unsigned_abs());
        rounded_sqrt(dx.pow(2) + dy.pow(2) + dz.pow(2))
    }

    /// The straight length between two tiles' centres over the surface, the shorter way round, a lower bound of any
    /// path between them.
    #[must_use]
    pub fn plane_m(&self, a: TileId, b: TileId) -> u64 {
        let (dx, dy) = self.apart_m(a, b);
        rounded_sqrt(dx.pow(2) + dy.pow(2))
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
    fn the_grid_closes_on_itself() {
        let g = Grid { width: 4, height: 3, tile_m: 10_000 };
        assert_eq!((g.len(), g.xy(TileId::new(6)), g.at(2, 1)), (12, (2, 1), TileId::new(6)));
        assert_eq!(
            g.neighbours(TileId::new(0)).map(TileId::get).collect::<Vec<_>>(),
            vec![11, 8, 9, 3, 1, 7, 4, 5],
            "the corner's neighbours lie across both seams"
        );
        assert!((0..12).all(|i| g.neighbours(TileId::new(i)).count() == 8), "every tile has eight");
        assert!(
            (0..12).all(|i| g.neighbours(TileId::new(i)).all(|n| g.neighbours(n).any(|m| m == TileId::new(i)))),
            "neighbours are mutual"
        );
        assert_eq!(g.direction(TileId::new(0), TileId::new(11)), Some(0), "up and left, across both seams");
        assert_eq!(g.direction(TileId::new(0), TileId::new(2)), None, "two across is not a neighbour");
        assert_eq!(g.seams(TileId::new(0), TileId::new(11)), (-1, -1), "west and north across both seams");
        assert_eq!(g.seams(TileId::new(3), TileId::new(0)), (1, 0), "east across one");
        assert_eq!(g.seams(TileId::new(5), TileId::new(6)), (0, 0));
        assert_eq!(g.plane_m(TileId::new(0), TileId::new(3)), 10_000, "the short way round");
        assert_eq!(g.plane_m(TileId::new(0), TileId::new(2)), 20_000, "either way is as long");
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(8), 0), 10_000, "across the north-south seam");
        let thin = Grid { width: 5, height: 1, tile_m: 10_000 };
        assert_eq!(
            thin.neighbours(TileId::new(0)).map(TileId::get).collect::<Vec<_>>(),
            vec![4, 1],
            "on a grid one tile high, the steps that come back are left out"
        );
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(1), 0), 10_000);
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(5), 0), 14_142);
        assert_eq!(g.length_m(TileId::new(0), 0, TileId::new(1), 1_000), 10_050);
        assert_eq!((rounded_sqrt(8), rounded_sqrt(12), rounded_sqrt(13)), (3, 3, 4));
    }
}
