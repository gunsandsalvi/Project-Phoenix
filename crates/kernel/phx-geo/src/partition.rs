use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use phx_id::TileId;
use phx_macros::clause;
use phx_rand::{Draws, below_u64};

use crate::grid::Grid;

/// Each tile's connected component of `mask` over the eight neighbours, numbered in order of their first tile, with
/// the components' sizes.
#[must_use]
pub fn components(grid: &Grid, mask: &[bool]) -> (Vec<Option<usize>>, Vec<u64>) {
    let mut label: Vec<Option<usize>> = vec![None; grid.len()];
    let mut sizes = Vec::new();
    for start in 0..grid.len() {
        if !mask.get(start).copied().unwrap_or(false) || label.get(start).copied().flatten().is_some() {
            continue;
        }
        let id = sizes.len();
        let mut size = 0_u64;
        let mut queue = VecDeque::from([grid.tile(start)]);
        if let Some(l) = label.get_mut(start) {
            *l = Some(id);
        }
        while let Some(t) = queue.pop_front() {
            size += 1;
            for n in grid.neighbours(t) {
                let i = grid.index(n);
                if mask.get(i).copied().unwrap_or(false) && label.get(i).copied().flatten().is_none() {
                    if let Some(l) = label.get_mut(i) {
                        *l = Some(id);
                    }
                    queue.push_back(n);
                }
            }
        }
        sizes.push(size);
    }
    (label, sizes)
}

/// Whether the tiles of `mask` join up all the way round the closed surface, east to west and north to south: a walk
/// over them that comes back to a tile it started from after crossing a seam more times one way than the other.
/// Found by walking each piece while counting the seams crossed; a tile reached again with other counts closes a way
/// round.
#[must_use]
pub fn winds(grid: &Grid, mask: &[bool]) -> (bool, bool) {
    let mut seen: Vec<Option<(i64, i64)>> = vec![None; grid.len()];
    let (mut across, mut down) = (false, false);
    for start in 0..grid.len() {
        if !mask.get(start).copied().unwrap_or(false) || seen.get(start).copied().flatten().is_some() {
            continue;
        }
        if let Some(first) = seen.get_mut(start) {
            *first = Some((0, 0));
        }
        let mut queue = VecDeque::from([start]);
        while let Some(cell) = queue.pop_front() {
            let Some((wound_x, wound_y)) = seen.get(cell).copied().flatten() else { continue };
            let tile = grid.tile(cell);
            for near in grid.neighbours(tile) {
                let other = grid.index(near);
                if !mask.get(other).copied().unwrap_or(false) {
                    continue;
                }
                let (seam_x, seam_y) = grid.seams(tile, near);
                let next = (wound_x + seam_x, wound_y + seam_y);
                match seen.get(other).copied().flatten() {
                    None => {
                        if let Some(slot) = seen.get_mut(other) {
                            *slot = Some(next);
                        }
                        queue.push_back(other);
                    }
                    Some((before_x, before_y)) => {
                        across |= before_x != next.0;
                        down |= before_y != next.1;
                    }
                }
            }
        }
    }
    (across, down)
}

/// Whole numbers in proportion to `weights` summing to `total`, by largest remainder, ties to the earlier.
#[must_use]
pub fn apportion(total: u64, weights: &[u64]) -> Vec<u64> {
    let sum: u64 = weights.iter().sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let mut out: Vec<u64> = weights.iter().map(|w| total * w / sum).collect();
    let mut rest: Vec<(u64, usize)> = weights.iter().enumerate().map(|(i, w)| (total * w % sum, i)).collect();
    rest.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let left = total - out.iter().sum::<u64>();
    for (_, i) in rest.into_iter().take(usize::try_from(left).unwrap_or(usize::MAX)) {
        if let Some(x) = out.get_mut(i) {
            *x += 1;
        }
    }
    out
}

/// The cost of stepping from one tile to a neighbour, which a split adds up from each end.
pub type StepCost<'a> = &'a dyn Fn(TileId, TileId) -> u64;

/// Values over the grid's tiles that a pass sets for the few tiles it touches: starting a new pass forgets the last
/// one's without writing over the whole grid, as a split into many small parts makes many passes.
struct Marks<T> {
    pass: Vec<u32>,
    value: Vec<T>,
    now: u32,
}

impl<T: Copy + Default> Marks<T> {
    fn new(len: usize) -> Self {
        Marks { pass: vec![0; len], value: vec![T::default(); len], now: 1 }
    }

    fn forget(&mut self) {
        if self.now == u32::MAX {
            self.pass.fill(0);
            self.now = 0;
        }
        self.now += 1;
    }

    fn get(&self, i: usize) -> Option<T> {
        (self.pass.get(i) == Some(&self.now)).then(|| self.value.get(i).copied()).flatten()
    }

    fn set(&mut self, i: usize, v: T) {
        if let (Some(p), Some(x)) = (self.pass.get_mut(i), self.value.get_mut(i)) {
            *p = self.now;
            *x = v;
        }
    }
}

/// The working state of a split, kept across its passes.
struct Work<'a> {
    grid: &'a Grid,
    lot: &'a [u64],
    step: StepCost<'a>,
    member: Marks<bool>,
    piece: Marks<u32>,
    from_start: Marks<u64>,
    to_a: Marks<u64>,
    to_b: Marks<u64>,
    anchor: Marks<(u64, usize)>,
}

/// Each member's least summed step cost from `from` over the members, into `into`.
fn costs(grid: &Grid, member: &Marks<bool>, step: StepCost<'_>, from: usize, into: &mut Marks<u64>) {
    into.forget();
    into.set(from, 0);
    let mut heap = BinaryHeap::from([Reverse((0_u64, from))]);
    while let Some(Reverse((cost, i))) = heap.pop() {
        if into.get(i).is_some_and(|c| c < cost) {
            continue;
        }
        let t = grid.tile(i);
        for n in grid.neighbours(t) {
            let j = grid.index(n);
            let next = cost + step(t, n);
            if member.get(j).is_some() && into.get(j).is_none_or(|c| next < c) {
                into.set(j, next);
                heap.push(Reverse((next, j)));
            }
        }
    }
}

/// The tile of `tiles` reached at the greatest cost, ties to the least lot, then to the first place.
fn farthest(tiles: &[usize], cost: &Marks<u64>, lot: &[u64]) -> Option<usize> {
    tiles
        .iter()
        .filter_map(|i| cost.get(*i).map(|c| (c, Reverse(lot.get(*i).copied().unwrap_or(u64::MAX)), Reverse(*i))))
        .fold(None, |best, x| match best {
            Some(b) if b >= x => Some(b),
            _ => Some(x),
        })
        .map(|(_, _, Reverse(i))| i)
}

impl Work<'_> {
    /// The members' connected pieces, labelled in the order of `tiles`, and the largest piece's label.
    fn pieces(&mut self, tiles: &[usize]) -> Option<u32> {
        self.piece.forget();
        let mut largest: Option<(u64, u32)> = None;
        let mut label = 0_u32;
        for start in tiles {
            if self.piece.get(*start).is_some() {
                continue;
            }
            let mut size = 0_u64;
            self.piece.set(*start, label);
            let mut queue = VecDeque::from([*start]);
            while let Some(i) = queue.pop_front() {
                size += 1;
                for n in self.grid.neighbours(self.grid.tile(i)) {
                    let j = self.grid.index(n);
                    if self.member.get(j).is_some() && self.piece.get(j).is_none() {
                        self.piece.set(j, label);
                        queue.push_back(j);
                    }
                }
            }
            if largest.is_none_or(|(s, _)| size > s) {
                largest = Some((size, label));
            }
            label += 1;
        }
        largest.map(|(_, l)| l)
    }

    /// For every member off the largest piece, the tile of the largest piece nearest to it over the plane; each tile
    /// of the largest piece is its own. Found by spreading from the largest piece over land and sea alike, until every
    /// member off it is reached.
    fn anchors(&mut self, tiles: &[usize], largest: u32) {
        self.anchor.forget();
        let mut heap = BinaryHeap::new();
        let mut off = 0_usize;
        for i in tiles {
            if self.piece.get(*i) == Some(largest) {
                self.anchor.set(*i, (0, *i));
                heap.push(Reverse((0_u64, *i, *i)));
            } else {
                off += 1;
            }
        }
        if off == 0 {
            return;
        }
        while let Some(Reverse((d, i, source))) = heap.pop() {
            if self.anchor.get(i).is_some_and(|(best, _)| best < d) {
                continue;
            }
            if self.member.get(i).is_some() && self.piece.get(i) != Some(largest) {
                off -= 1;
                if off == 0 {
                    return;
                }
            }
            let t = self.grid.tile(i);
            for n in self.grid.neighbours(t) {
                let j = self.grid.index(n);
                let next = d + self.grid.length_m(t, 0, n, 0);
                if self.anchor.get(j).is_none_or(|(best, _)| next < best) {
                    self.anchor.set(j, (next, source));
                    heap.push(Reverse((next, j, source)));
                }
            }
        }
    }
}

/// The tiles of `members` shared among parts in proportion to `weights`, by halving: the parts are cut in two groups,
/// the tiles are ranked by how much nearer they lie to one end of the land than to the other, by summed step cost, and
/// cut at the first group's share; each group is then halved the same way. The ends are the tile furthest from one
/// drawn on the largest piece, and the tile furthest from that, so each cut crosses the land's long way. Along a
/// tile's route to either end its rank never moves away from that end's, so each part holds together, and its border
/// lies along a line of equal costs, bent by the ridges and rivers that make steps dear. Tiles of equal rank, such as
/// a peninsula's, which reach both ends only through its neck, are never parted: the cut falls at the edge of such a
/// group nearest the share, so a count is exact unless the cut would split one. A tile off the largest piece ranks
/// with the tile of it nearest over the plane, so an island goes with the coast it faces. Returns each tile's part.
#[clause("GEO.3", "GEO.10")]
#[must_use]
pub fn split(
    grid: &Grid,
    members: &[bool],
    weights: &[u64],
    lot: &[u64],
    step: StepCost<'_>,
    d: &mut Draws,
) -> Vec<Option<usize>> {
    let mut work = Work {
        grid,
        lot,
        step,
        member: Marks::new(grid.len()),
        piece: Marks::new(grid.len()),
        from_start: Marks::new(grid.len()),
        to_a: Marks::new(grid.len()),
        to_b: Marks::new(grid.len()),
        anchor: Marks::new(grid.len()),
    };
    let tiles: Vec<usize> = (0..grid.len()).filter(|i| members.get(*i).copied().unwrap_or(false)).collect();
    let mut owner = vec![None; grid.len()];
    divide(&mut work, &tiles, 0, weights, d, &mut owner);
    owner
}

fn divide(
    work: &mut Work<'_>,
    tiles: &[usize],
    first: usize,
    weights: &[u64],
    draws: &mut Draws,
    owner: &mut [Option<usize>],
) {
    if weights.len() < 2 {
        if !weights.is_empty() {
            for i in tiles {
                if let Some(o) = owner.get_mut(*i) {
                    *o = Some(first);
                }
            }
        }
        return;
    }
    let (left, right) = weights.split_at(weights.len() / 2);
    let total = u64::try_from(tiles.len()).unwrap_or(u64::MAX);
    let to_left = apportion(total, &[left.iter().sum(), right.iter().sum()]).first().copied().unwrap_or(0);
    work.member.forget();
    for i in tiles {
        work.member.set(*i, true);
    }
    let ranked = match work.pieces(tiles) {
        Some(largest) => rank(work, tiles, largest, draws),
        None => Vec::new(),
    };
    let cut = nearest_edge(&ranked, to_left);
    let tiles: Vec<usize> = ranked.iter().map(|(_, i)| *i).collect();
    let (near, far) = tiles.split_at_checked(cut).unwrap_or((&tiles, &[]));
    divide(work, near, first, left, draws, owner);
    divide(work, far, first + left.len(), right, draws, owner);
}

/// Where to cut a ranked list so that no two tiles of equal rank part: the edge between ranks nearest `share`, ties
/// to the earlier.
fn nearest_edge(ranked: &[(i128, usize)], share: u64) -> usize {
    let mut best: Option<(u64, usize)> = None;
    for at in 0..=ranked.len() {
        let is_edge =
            at == 0 || at == ranked.len() || ranked.get(at - 1).map(|(r, _)| *r) != ranked.get(at).map(|(r, _)| *r);
        let off = u64::try_from(at).unwrap_or(u64::MAX).abs_diff(share);
        if is_edge && best.is_none_or(|(b, _)| off < b) {
            best = Some((off, at));
        }
    }
    best.map_or(0, |(_, at)| at)
}

/// The members ranked from the first end to the second, each with its rank: how much nearer the first end it lies.
fn rank(work: &mut Work<'_>, tiles: &[usize], largest: u32, draws: &mut Draws) -> Vec<(i128, usize)> {
    let main: Vec<usize> = tiles.iter().copied().filter(|i| work.piece.get(*i) == Some(largest)).collect();
    let on_main = u64::try_from(main.len()).unwrap_or(0);
    let Some(start) = usize::try_from(below_u64(draws, on_main)).ok().and_then(|k| main.get(k).copied()) else {
        return Vec::new();
    };
    costs(work.grid, &work.member, work.step, start, &mut work.from_start);
    let Some(first_end) = farthest(&main, &work.from_start, work.lot) else { return Vec::new() };
    costs(work.grid, &work.member, work.step, first_end, &mut work.to_a);
    let Some(second_end) = farthest(&main, &work.to_a, work.lot) else { return Vec::new() };
    costs(work.grid, &work.member, work.step, second_end, &mut work.to_b);
    work.anchors(tiles, largest);
    let mut keyed: Vec<(i128, u64, usize)> = tiles
        .iter()
        .filter_map(|i| {
            let (_, anchor) = work.anchor.get(*i)?;
            let (to_a, to_b) = (work.to_a.get(anchor)?, work.to_b.get(anchor)?);
            Some((i128::from(to_a) - i128::from(to_b), work.lot.get(*i).copied().unwrap_or(u64::MAX), *i))
        })
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(r, _, i)| (r, i)).collect()
}

#[cfg(test)]
mod tests {
    use phx_core::{Purpose, StreamDecl, Streams};
    use phx_id::{Day, TileId};
    use phx_rand::{Draws, Seed, Subject, SubjectTag};

    use super::{apportion, components, split, winds};
    use crate::grid::Grid;

    const MAP: StreamDecl = StreamDecl { name: "GEO.map", purpose: Purpose::Opening, keyed: false, clause: "GEO.10" };

    fn draws(seed: u64) -> Draws {
        let streams = Streams::new(Seed::new(seed), &[MAP]).unwrap();
        streams.open(&MAP, Subject::new(SubjectTag::World, 0), Day::new(0), 0)
    }

    fn lot(n: usize) -> Vec<u64> {
        (0..n).map(|i| u64::try_from((i * 7919) % 1009).unwrap()).collect()
    }

    /// Steps of unlike cost, as over real ground.
    fn ground(_: TileId, b: TileId) -> u64 {
        10_000 + u64::from(b.get() * 7919 % 997)
    }

    fn count(owner: &[Option<usize>], part: usize) -> u64 {
        u64::try_from(owner.iter().filter(|o| **o == Some(part)).count()).unwrap()
    }

    #[test]
    fn apportion_by_largest_remainder() {
        assert_eq!(apportion(25, &[50, 30, 20]), vec![13, 7, 5]);
        assert_eq!(apportion(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(apportion(7, &[0, 0]), vec![0, 0]);
    }

    #[test]
    fn parts_hold_their_shares_each_in_one_piece() {
        let g = Grid { width: 30, height: 20, tile_m: 10_000 };
        let members: Vec<bool> = (0..g.len()).map(|i| i % 30 != 29 && !(200..210).contains(&i)).collect();
        let land = u64::try_from(members.iter().filter(|m| **m).count()).unwrap();
        let shares = [5_u64, 3, 2, 2, 1];
        let whole: u64 = shares.iter().sum();
        // Each of the three halvings on the way to a part rounds its count by less than a tile.
        let halvings = 3;
        for seed in 1..6 {
            let owner = split(&g, &members, &shares, &lot(g.len()), &ground, &mut draws(seed));
            for (part, share) in shares.iter().enumerate() {
                let off = (count(&owner, part) * whole).abs_diff(land * share);
                assert!(off < halvings * whole, "part {part} holds its share (seed {seed})");
                let mask: Vec<bool> = owner.iter().map(|o| *o == Some(part)).collect();
                assert_eq!(components(&g, &mask).1.len(), 1, "part {part} is one piece (seed {seed})");
            }
            assert!(owner.iter().zip(&members).all(|(o, m)| o.is_some() == *m), "every member, and no other");
        }
    }

    #[test]
    fn a_band_goes_round_and_a_blob_does_not() {
        let g = Grid { width: 8, height: 6, tile_m: 10_000 };
        let band: Vec<bool> = (0..g.len()).map(|i| g.xy(g.tile(i)).1 == 2).collect();
        assert_eq!(winds(&g, &band), (true, false), "a row all the way across goes round east to west");
        let column: Vec<bool> = (0..g.len()).map(|i| g.xy(g.tile(i)).0 == 7).collect();
        assert_eq!(winds(&g, &column), (false, true), "a column at the seam goes round north to south");
        let blob: Vec<bool> = (0..g.len()).map(|i| g.xy(g.tile(i)).0 < 3 && g.xy(g.tile(i)).1 < 3).collect();
        assert_eq!(winds(&g, &blob), (false, false));
        let broken: Vec<bool> = (0..g.len()).map(|i| g.xy(g.tile(i)).1 == 2 && g.xy(g.tile(i)).0 != 4).collect();
        assert_eq!(winds(&g, &broken), (false, false), "a row with a gap does not");
    }

    #[test]
    fn peninsulas_stay_whole() {
        // A comb: a spine three tiles deep with a tooth one tile wide on every other column, each reaching the spine
        // through one neck.
        let g = Grid { width: 24, height: 12, tile_m: 10_000 };
        let members: Vec<bool> = (0..g.len())
            .map(|i| {
                let (x, y) = g.xy(g.tile(i));
                x < 21 && (y < 3 || (y < 9 && x % 2 == 0))
            })
            .collect();
        for seed in 1..20 {
            let owner = split(&g, &members, &[1, 1, 1, 1], &lot(g.len()), &ground, &mut draws(seed));
            for part in 0..4 {
                let mask: Vec<bool> = owner.iter().map(|o| *o == Some(part)).collect();
                assert_eq!(components(&g, &mask).1.len(), 1, "part {part} is one piece (seed {seed})");
            }
        }
    }

    #[test]
    fn an_island_goes_with_the_coast_it_faces() {
        // Wide enough that the island lies nearer the east coast than the west one round the seam.
        let g = Grid { width: 20, height: 6, tile_m: 10_000 };
        let mut members = vec![false; g.len()];
        for y in 0..6 {
            for x in 0..10 {
                members[g.index(g.at(x, y))] = true;
            }
        }
        let island = [g.index(g.at(13, 0)), g.index(g.at(14, 0)), g.index(g.at(13, 1))];
        for i in island {
            members[i] = true;
        }
        for seed in 1..6 {
            let owner = split(&g, &members, &[1, 1], &lot(g.len()), &ground, &mut draws(seed));
            let coast = owner[g.index(g.at(9, 0))];
            assert!(island.iter().all(|i| owner[*i] == coast), "the island with its facing coast (seed {seed})");
        }
    }
}
