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
fn furthest_behind(sizes: &[u64], targets: &[u64], frontiers: &[BinaryHeap<Reverse<(u64, u64, u32)>>]) -> Option<usize> {
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

/// The cost of stepping from one tile to a neighbour, which the growth adds up from each seed.
pub type StepCost<'a> = &'a dyn Fn(TileId, TileId) -> u64;

/// Parts grown from their seeds over the eligible tiles: the part furthest below its target claims next, taking the
/// tile of its frontier nearest its seed by summed step cost (ties by lot), until each part has its target or no
/// frontier; so fronts slow where steps cost more, and borders fall there. Tiles left over join the part that reaches
/// them first by the same costs. Returns each tile's part.
#[clause("GEO.3", "GEO.10")]
#[must_use]
pub fn grow(
    grid: &Grid,
    eligible: &[bool],
    seeds: &[TileId],
    targets: &[u64],
    lot: &[u64],
    step: StepCost<'_>,
) -> Vec<Option<usize>> {
    let mut owner: Vec<Option<usize>> = vec![None; grid.len()];
    let mut reach = vec![0_u64; grid.len()];
    let mut sizes = vec![0_u64; seeds.len()];
    let mut frontiers: Vec<BinaryHeap<Reverse<(u64, u64, u32)>>> = vec![BinaryHeap::new(); seeds.len()];
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
                    f.push(Reverse((step(*seed, n), lot_of(n), n.get())));
                }
            }
        }
    }
    loop {
        let behind = furthest_behind(&sizes, targets, &frontiers);
        let Some(part) = behind else { break };
        let Some(Reverse((cost, _, id))) = frontiers.get_mut(part).and_then(BinaryHeap::pop) else { continue };
        let t = TileId::new(id);
        if !open(&owner, t) {
            continue;
        }
        if let Some(o) = owner.get_mut(grid.index(t)) {
            *o = Some(part);
        }
        if let Some(r) = reach.get_mut(grid.index(t)) {
            *r = cost;
        }
        if let Some(s) = sizes.get_mut(part) {
            *s += 1;
        }
        for n in grid.neighbours(t) {
            if open(&owner, n)
                && let Some(f) = frontiers.get_mut(part)
            {
                f.push(Reverse((cost + step(t, n), lot_of(n), n.get())));
            }
        }
    }
    let mut spill: BinaryHeap<Reverse<(u64, u64, u32, usize)>> = BinaryHeap::new();
    for i in 0..grid.len() {
        if let Some(part) = owner.get(i).copied().flatten() {
            let t = grid.tile(i);
            let at = reach.get(i).copied().unwrap_or(0);
            for n in grid.neighbours(t) {
                if open(&owner, n) {
                    spill.push(Reverse((at + step(t, n), lot_of(n), n.get(), part)));
                }
            }
        }
    }
    while let Some(Reverse((cost, _, id, part))) = spill.pop() {
        let t = TileId::new(id);
        if !open(&owner, t) {
            continue;
        }
        if let Some(o) = owner.get_mut(grid.index(t)) {
            *o = Some(part);
        }
        for n in grid.neighbours(t) {
            if open(&owner, n) {
                spill.push(Reverse((cost + step(t, n), lot_of(n), n.get(), part)));
            }
        }
    }
    owner
}

/// Whether a tile can leave its part without cutting it: its neighbours in the part, around it, form one connected
/// piece, so every path through the tile has a way around it.
fn simple(grid: &Grid, owner: &[Option<usize>], t: TileId, part: usize) -> bool {
    let around: Vec<TileId> =
        grid.neighbours(t).filter(|n| owner.get(grid.index(*n)).copied().flatten() == Some(part)).collect();
    if around.is_empty() {
        return false;
    }
    let mut seen = vec![false; around.len()];
    let mut stack = vec![0_usize];
    if let Some(s) = seen.first_mut() {
        *s = true;
    }
    while let Some(i) = stack.pop() {
        let Some(a) = around.get(i).copied() else { continue };
        for (j, b) in around.iter().enumerate() {
            let touching = grid.neighbours(a).any(|n| n == *b);
            if touching && !seen.get(j).copied().unwrap_or(true) {
                if let Some(s) = seen.get_mut(j) {
                    *s = true;
                }
                stack.push(j);
            }
        }
    }
    seen.iter().all(|s| *s)
}

/// Parts brought within `tolerance_per_mille` of their targets, one border tile at a time: the part furthest over its
/// target gives to the neighbouring part furthest under, so long as the taker stays below the giver's share, the tile
/// of least lot among those it can give without being cut, never its seed. Every move narrows the spread of shares,
/// so it ends: when every part is within the tolerance, or no such tile is left. Returns whether every part is within
/// it.
#[clause("GEO.3")]
pub fn balance(
    grid: &Grid,
    owner: &mut [Option<usize>],
    seeds: &[TileId],
    targets: &[u64],
    tolerance_per_mille: u64,
    lot: &[u64],
) -> bool {
    let parts = targets.len();
    let mut sizes = vec![0_u64; parts];
    for part in owner.iter().flatten() {
        if let Some(s) = sizes.get_mut(*part) {
            *s += 1;
        }
    }
    let per_mille = crate::consts::PER_MILLE;
    let within = |size: u64, target: u64| {
        size * per_mille <= target * (per_mille + tolerance_per_mille)
            && size * per_mille + target * tolerance_per_mille >= target * per_mille
    };
    // Each part's share of its target, compared as fractions.
    let over = |a: (u64, u64), b: (u64, u64)| u128::from(a.0) * u128::from(b.1) > u128::from(b.0) * u128::from(a.1);
    let mut stuck = vec![false; parts];
    loop {
        if sizes.iter().zip(targets).all(|(s, t)| within(*s, *t)) {
            return true;
        }
        let donor = (0..parts)
            .filter(|p| !stuck.get(*p).copied().unwrap_or(true))
            .fold(None, |best: Option<usize>, p| {
                let here = (sizes.get(p).copied().unwrap_or(0), targets.get(p).copied().unwrap_or(1));
                match best {
                    Some(b) if !over(here, (sizes.get(b).copied().unwrap_or(0), targets.get(b).copied().unwrap_or(1))) => {
                        Some(b)
                    }
                    _ => Some(p),
                }
            });
        let Some(donor) = donor else { return false };
        let share = |p: usize| (sizes.get(p).copied().unwrap_or(0), targets.get(p).copied().unwrap_or(1));
        let mut best: Option<((u64, u64), u64, usize, usize)> = None;
        for i in 0..grid.len() {
            if owner.get(i).copied().flatten() != Some(donor) || seeds.contains(&grid.tile(i)) {
                continue;
            }
            let t = grid.tile(i);
            for n in grid.neighbours(t) {
                let Some(taker) = owner.get(grid.index(n)).copied().flatten() else { continue };
                let (taken, target) = share(taker);
                if taker == donor || !over(share(donor), (taken + 1, target)) {
                    continue;
                }
                let l = lot.get(i).copied().unwrap_or(u64::MAX);
                let better = match best {
                    None => true,
                    Some((s, bl, _, _)) => over(s, share(taker)) || (share(taker) == s && l < bl),
                };
                if better && simple(grid, owner, t, donor) {
                    best = Some((share(taker), l, i, taker));
                }
            }
        }
        let Some((_, _, i, taker)) = best else {
            if let Some(s) = stuck.get_mut(donor) {
                *s = true;
            }
            continue;
        };
        if let Some(o) = owner.get_mut(i) {
            *o = Some(taker);
        }
        if let Some(s) = sizes.get_mut(donor) {
            *s -= 1;
        }
        if let Some(s) = sizes.get_mut(taker) {
            *s += 1;
        }
        stuck.fill(false);
    }
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

/// The smallest part with some tiles and fewer than `min`, ties to the earlier.
fn smallest_below(sizes: &[u64], min: u64) -> Option<usize> {
    let mut best: Option<(usize, u64)> = None;
    for (part, size) in sizes.iter().enumerate() {
        if *size > 0 && *size < min && best.is_none_or(|(_, b)| *size < b) {
            best = Some((part, *size));
        }
    }
    best.map(|(part, _)| part)
}

/// The part owning the tile nearest over the plane to any tile of `part`, ties to the lower tile identity.
fn nearest_other(grid: &Grid, owner: &[Option<usize>], part: usize) -> Option<usize> {
    let mine: Vec<TileId> =
        (0..grid.len()).filter(|i| owner.get(*i).copied().flatten() == Some(part)).map(|i| grid.tile(i)).collect();
    let mut best: Option<(u64, u32, usize)> = None;
    for i in 0..grid.len() {
        let Some(other) = owner.get(i).copied().flatten().filter(|o| *o != part) else { continue };
        let t = grid.tile(i);
        for m in &mine {
            let d = grid.plane_m(*m, t);
            if best.is_none_or(|b| (d, t.get()) < (b.0, b.1)) {
                best = Some((d, t.get(), other));
            }
        }
    }
    best.map(|(_, _, other)| other)
}

/// Parts below `min` tiles merge, the smallest first, into their smallest neighbouring part, ties to the earlier, or,
/// with no neighbour, as an island's, into the part nearest over the plane; until none is below it.
#[clause("GEO.3")]
pub fn merge_small(grid: &Grid, owner: &mut [Option<usize>], parts: usize, min: u64) {
    loop {
        let mut sizes = vec![0_u64; parts];
        for part in owner.iter().flatten() {
            if let Some(s) = sizes.get_mut(*part) {
                *s += 1;
            }
        }
        let Some(small) = smallest_below(&sizes, min) else { return };
        let mut into: Option<(u64, usize)> = None;
        for i in 0..grid.len() {
            if owner.get(i).copied().flatten() != Some(small) {
                continue;
            }
            for n in grid.neighbours(grid.tile(i)) {
                if let Some(other) = owner.get(grid.index(n)).copied().flatten()
                    && other != small
                {
                    let size = sizes.get(other).copied().unwrap_or(u64::MAX);
                    if into.is_none_or(|b| (size, other) < b) {
                        into = Some((size, other));
                    }
                }
            }
        }
        let Some(target) = into.map(|(_, t)| t).or_else(|| nearest_other(grid, owner, small)) else { return };
        for o in owner.iter_mut() {
            if *o == Some(small) {
                *o = Some(target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::TileId;

    use super::{balance, components, grow, join_nearest, merge_small};
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
        let owner = grow(&g, &eligible, &seeds, &targets, &lot(g.len()), &|_, _| 1);
        let count = |p: usize| owner.iter().filter(|o| **o == Some(p)).count();
        assert_eq!((count(0), count(1), count(2)), (50, 30, land - 80), "each part reaches its target");
        assert!((0..3).all(|p| connected(&g, &owner, p)), "each part is one piece");
        assert!(owner.iter().zip(&eligible).all(|(o, e)| o.is_some() == *e), "every eligible tile, and no other");
    }

    #[test]
    fn balance_moves_border_tiles_without_cutting() {
        let g = Grid { width: 10, height: 6, tile_m: 10_000 };
        let mut owner: Vec<Option<usize>> = (0..g.len()).map(|i| Some(usize::from(i % 10 >= 7))).collect();
        let seeds = [TileId::new(0), TileId::new(9)];
        let ok = balance(&g, &mut owner, &seeds, &[30, 30], 0, &lot(g.len()));
        let count = |p: usize| owner.iter().filter(|o| **o == Some(p)).count();
        assert!(ok && count(0) == 30 && count(1) == 30, "balanced exactly: {} {}", count(0), count(1));
        assert!((0..2).all(|p| connected(&g, &owner, p)), "neither part is cut");
    }

    #[test]
    fn growth_stops_at_costly_ground() {
        let g = Grid { width: 9, height: 1, tile_m: 10_000 };
        let eligible = vec![true; 9];
        let ridge = |_: TileId, b: TileId| if b.get() == 3 || b.get() == 4 { 100 } else { 1 };
        let owner = grow(&g, &eligible, &[TileId::new(0), TileId::new(8)], &[5, 4], &lot(9), &ridge);
        assert_eq!(owner.iter().filter(|o| **o == Some(0)).count(), 5);
        assert_eq!(owner[5], Some(1), "the far part reaches past the ridge while the near one waits at it");
    }

    #[test]
    fn small_parts_merge_into_their_smallest_neighbour() {
        let g = Grid { width: 6, height: 1, tile_m: 10_000 };
        let mut owner = vec![Some(0), Some(0), Some(0), Some(1), Some(2), Some(2)];
        merge_small(&g, &mut owner, 3, 2);
        assert_eq!(owner, vec![Some(0), Some(0), Some(0), Some(2), Some(2), Some(2)], "into the smaller neighbour");
        let mut island = vec![Some(0), None, Some(1), Some(1), None, None];
        merge_small(&g, &mut island, 2, 2);
        assert_eq!(island[0], Some(1), "an island's part joins the nearest");
        let mut only = vec![Some(0), None, None, None, None, None];
        merge_small(&g, &mut only, 1, 2);
        assert_eq!(only[0], Some(0), "a lone part has nowhere to go, for the condition to judge");
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
