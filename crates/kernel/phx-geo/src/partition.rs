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

/// `count` distinct tiles picked uniformly from `among`, in the order drawn.
#[must_use]
pub fn pick_seeds(among: &[TileId], count: usize, d: &mut Draws) -> Vec<TileId> {
    let mut pool = among.to_vec();
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let n = u64::try_from(pool.len()).unwrap_or(0);
        if n == 0 {
            break;
        }
        let Ok(i) = usize::try_from(below_u64(d, n)) else { break };
        out.push(pool.swap_remove(i));
    }
    out
}

/// The part furthest below its target, as a share of the target, among those below it with a frontier left; ties
/// go to the earlier part.
fn furthest_behind(sizes: &[u64], targets: &[u64], frontiers: &[BinaryHeap<Reverse<(u64, u32)>>]) -> Option<usize> {
    let mut best: Option<(usize, u128, u128)> = None;
    for (part, ((size, target), frontier)) in sizes.iter().zip(targets).zip(frontiers).enumerate() {
        if size >= target || frontier.is_empty() {
            continue;
        }
        let (s, t) = (u128::from(*size), u128::from(*target));
        if best.is_none_or(|(_, bs, bt)| s * bt < bs * t) {
            best = Some((part, s, t));
        }
    }
    best.map(|(part, _, _)| part)
}

/// Parts grown from their seeds over the eligible tiles: the part furthest below its target claims next, taking the
/// tile of its frontier with the smallest lot, until each part has its target or no frontier; tiles left over join
/// the part that reaches them first, by the same lots. Returns each tile's part.
#[clause("GEO.3", "GEO.10")]
#[must_use]
pub fn grow(grid: &Grid, eligible: &[bool], seeds: &[TileId], targets: &[u64], lot: &[u64]) -> Vec<Option<usize>> {
    let mut owner: Vec<Option<usize>> = vec![None; grid.len()];
    let mut sizes = vec![0_u64; seeds.len()];
    let mut frontiers: Vec<BinaryHeap<Reverse<(u64, u32)>>> = vec![BinaryHeap::new(); seeds.len()];
    let lot_of = |t: TileId| lot.get(grid.index(t)).copied().unwrap_or(u64::MAX);
    let open = |owner: &[Option<usize>], t: TileId| {
        eligible.get(grid.index(t)).copied().unwrap_or(false) && owner.get(grid.index(t)).copied().flatten().is_none()
    };
    for (part, seed) in seeds.iter().enumerate() {
        if open(&owner, *seed) {
            if let Some(o) = owner.get_mut(grid.index(*seed)) {
                *o = Some(part);
            }
            if let Some(s) = sizes.get_mut(part) {
                *s = 1;
            }
            for n in grid.neighbours(*seed) {
                if let Some(f) = frontiers.get_mut(part) {
                    f.push(Reverse((lot_of(n), n.get())));
                }
            }
        }
    }
    loop {
        let behind = furthest_behind(&sizes, targets, &frontiers);
        let Some(part) = behind else { break };
        let Some(Reverse((_, id))) = frontiers.get_mut(part).and_then(BinaryHeap::pop) else { continue };
        let t = TileId::new(id);
        if !open(&owner, t) {
            continue;
        }
        if let Some(o) = owner.get_mut(grid.index(t)) {
            *o = Some(part);
        }
        if let Some(s) = sizes.get_mut(part) {
            *s += 1;
        }
        for n in grid.neighbours(t) {
            if open(&owner, n)
                && let Some(f) = frontiers.get_mut(part)
            {
                f.push(Reverse((lot_of(n), n.get())));
            }
        }
    }
    let mut spill: BinaryHeap<Reverse<(u64, u32, usize)>> = BinaryHeap::new();
    for i in 0..grid.len() {
        if let Some(part) = owner.get(i).copied().flatten() {
            for n in grid.neighbours(grid.tile(i)) {
                if open(&owner, n) {
                    spill.push(Reverse((lot_of(n), n.get(), part)));
                }
            }
        }
    }
    while let Some(Reverse((_, id, part))) = spill.pop() {
        let t = TileId::new(id);
        if !open(&owner, t) {
            continue;
        }
        if let Some(o) = owner.get_mut(grid.index(t)) {
            *o = Some(part);
        }
        for n in grid.neighbours(t) {
            if open(&owner, n) {
                spill.push(Reverse((lot_of(n), n.get(), part)));
            }
        }
    }
    owner
}

/// Every tile of `joiners` with no owner takes, component by component, the owner of the owned tile nearest to it
/// over the plane, ties broken by the owned tile's lot.
#[clause("GEO.3")]
pub fn join_nearest(grid: &Grid, owner: &mut [Option<usize>], joiners: &[bool], lot: &[u64]) {
    let mut nearest: Vec<Option<(u64, u64, usize)>> = vec![None; grid.len()];
    let mut heap: BinaryHeap<Reverse<(u64, u64, u32, usize)>> = BinaryHeap::new();
    for i in 0..grid.len() {
        if let Some(part) = owner.get(i).copied().flatten() {
            let l = lot.get(i).copied().unwrap_or(u64::MAX);
            heap.push(Reverse((0, l, grid.tile(i).get(), part)));
        }
    }
    while let Some(Reverse((d, l, id, part))) = heap.pop() {
        let t = TileId::new(id);
        let i = grid.index(t);
        if nearest.get(i).copied().flatten().is_some() {
            continue;
        }
        if let Some(n) = nearest.get_mut(i) {
            *n = Some((d, l, part));
        }
        for n in grid.neighbours(t) {
            if nearest.get(grid.index(n)).copied().flatten().is_none() {
                heap.push(Reverse((d + grid.length_m(t, 0, n, 0), l, n.get(), part)));
            }
        }
    }
    let unowned: Vec<bool> = (0..grid.len())
        .map(|i| joiners.get(i).copied().unwrap_or(false) && owner.get(i).copied().flatten().is_none())
        .collect();
    let (label, sizes) = components(grid, &unowned);
    let mut choice: Vec<Option<(u64, u64, usize)>> = vec![None; sizes.len()];
    for i in 0..grid.len() {
        if let (Some(c), Some(n)) = (label.get(i).copied().flatten(), nearest.get(i).copied().flatten())
            && let Some(slot) = choice.get_mut(c)
            && slot.is_none_or(|old| (n.0, n.1) < (old.0, old.1))
        {
            *slot = Some(n);
        }
    }
    for i in 0..grid.len() {
        if let Some(c) = label.get(i).copied().flatten()
            && let Some((_, _, part)) = choice.get(c).copied().flatten()
            && let Some(o) = owner.get_mut(i)
        {
            *o = Some(part);
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::TileId;

    use super::{components, grow, join_nearest};
    use crate::grid::Grid;

    fn lot(n: usize) -> Vec<u64> {
        (0..n).map(|i| u64::try_from((i * 7919) % 1009).unwrap()).collect()
    }

    fn connected(g: &Grid, owner: &[Option<usize>], part: usize) -> bool {
        let mask: Vec<bool> = owner.iter().map(|o| *o == Some(part)).collect();
        components(g, &mask).1.len() == 1
    }

    #[test]
    fn partition_reaches_shares_and_bounds() {
        let g = Grid { width: 12, height: 10, tile_m: 10_000 };
        let eligible: Vec<bool> = (0..g.len()).map(|i| !(40..44).contains(&i) && i % 12 != 11).collect();
        let land = eligible.iter().filter(|e| **e).count();
        let seeds = [TileId::new(0), TileId::new(65), TileId::new(118)];
        let targets = [50_u64, 30, u64::try_from(land).unwrap() - 80];
        let owner = grow(&g, &eligible, &seeds, &targets, &lot(g.len()));
        let count = |p: usize| owner.iter().filter(|o| **o == Some(p)).count();
        assert_eq!((count(0), count(1), count(2)), (50, 30, land - 80), "each part reaches its target");
        assert!((0..3).all(|p| connected(&g, &owner, p)), "each part is one piece");
        assert!(owner.iter().zip(&eligible).all(|(o, e)| o.is_some() == *e), "every eligible tile, and no other");
    }

    #[test]
    fn islands_join_nearest_country() {
        let g = Grid { width: 10, height: 4, tile_m: 10_000 };
        let mut land = vec![false; g.len()];
        for y in 0..4 {
            for x in 0..4 {
                land[g.index(g.at(x, y))] = true;
            }
        }
        land[g.index(g.at(8, 0))] = true;
        land[g.index(g.at(8, 3))] = true;
        land[g.index(g.at(9, 3))] = true;
        let mut owner: Vec<Option<usize>> =
            (0..g.len()).map(|i| land[i].then(|| usize::from(g.xy(g.tile(i)).1 >= 2))).collect();
        for i in [g.index(g.at(8, 0)), g.index(g.at(8, 3)), g.index(g.at(9, 3))] {
            owner[i] = None;
        }
        join_nearest(&g, &mut owner, &land, &lot(g.len()));
        assert_eq!(owner[g.index(g.at(8, 0))], Some(0), "the north island joins the north country");
        assert_eq!(
            (owner[g.index(g.at(8, 3))], owner[g.index(g.at(9, 3))]),
            (Some(1), Some(1)),
            "one island, one owner"
        );
    }
}
