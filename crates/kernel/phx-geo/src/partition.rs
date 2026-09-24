use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use phx_id::TileId;
use phx_macros::clause;
use phx_rand::{Draws, below_u64};

use crate::grid::{DIRECTIONS, Grid, rounded_sqrt};

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

/// The cost of stepping from one tile to a neighbour, which the growth adds up from each seed.
pub type StepCost<'a> = &'a dyn Fn(TileId, TileId) -> u64;

/// What a partition is grown over: the tiles its fronts may cross, the tiles that take the nearest part when no front
/// reaches them, and each part's seed and target.
#[derive(Clone, Copy, Debug)]
pub struct Partition<'a> {
    pub eligible: &'a [bool],
    pub joiners: &'a [bool],
    pub seeds: &'a [TileId],
    pub targets: &'a [u64],
}

/// Each tile claimed by the part that reaches it at least cost, every part holding its seed and starting from it with
/// its own handicap; ties by lot. Every tile's claim comes through a neighbour of the same part, so each part is one
/// piece.
fn claim(
    grid: &Grid,
    eligible: &[bool],
    seeds: &[TileId],
    handicap: &[u64],
    lot: &[u64],
    step: StepCost<'_>,
) -> (Vec<Option<usize>>, Vec<Option<u64>>) {
    let mut owner: Vec<Option<usize>> = vec![None; grid.len()];
    let mut reach: Vec<Option<u64>> = vec![None; grid.len()];
    let lot_of = |t: TileId| lot.get(grid.index(t)).copied().unwrap_or(u64::MAX);
    let mut heap: BinaryHeap<Reverse<(u64, u64, u32, usize)>> = BinaryHeap::new();
    for (part, seed) in seeds.iter().enumerate() {
        if eligible.get(grid.index(*seed)).copied().unwrap_or(false)
            && let Some(o) = owner.get_mut(grid.index(*seed))
        {
            *o = Some(part);
        }
    }
    let open = |owner: &[Option<usize>], heap: &mut BinaryHeap<_>, t: TileId, cost: u64, part: usize| {
        for n in grid.neighbours(t) {
            let j = grid.index(n);
            if eligible.get(j).copied().unwrap_or(false) && owner.get(j).copied().flatten().is_none() {
                heap.push(Reverse((cost + step(t, n), lot_of(n), n.get(), part)));
            }
        }
    };
    for (part, (seed, h)) in seeds.iter().zip(handicap).enumerate() {
        if owner.get(grid.index(*seed)).copied().flatten() == Some(part) {
            if let Some(r) = reach.get_mut(grid.index(*seed)) {
                *r = Some(*h);
            }
            open(&owner, &mut heap, *seed, *h, part);
        }
    }
    while let Some(Reverse((cost, _, id, part))) = heap.pop() {
        let t = TileId::new(id);
        let Some(slot) = owner.get_mut(grid.index(t)) else { continue };
        if slot.is_some() {
            continue;
        }
        *slot = Some(part);
        if let Some(r) = reach.get_mut(grid.index(t)) {
            *r = Some(cost);
        }
        open(&owner, &mut heap, t, cost, part);
    }
    (owner, reach)
}

/// Whether a tile can leave its part without cutting it: its neighbours in the part, around it, form one connected
/// piece, so every path through the tile has a way around it.
fn simple(grid: &Grid, owner: &[Option<usize>], t: TileId, part: usize) -> bool {
    let mut around = [TileId::new(0); DIRECTIONS];
    let mut count = 0;
    for n in grid.neighbours(t) {
        if owner.get(grid.index(n)).copied().flatten() == Some(part)
            && let Some(slot) = around.get_mut(count)
        {
            *slot = n;
            count += 1;
        }
    }
    let Some(around) = around.get(..count) else { return false };
    if around.is_empty() {
        return false;
    }
    let mut seen = 1_u16;
    let mut stack = [0_usize; DIRECTIONS];
    let mut top = 1;
    while top > 0 {
        top -= 1;
        let Some(a) = stack.get(top).and_then(|i| around.get(*i)).copied() else { continue };
        for (j, b) in around.iter().enumerate() {
            let bit = 1_u16 << j;
            if seen & bit == 0 && grid.direction(a, *b).is_some() {
                seen |= bit;
                if let Some(slot) = stack.get_mut(top) {
                    *slot = j;
                    top += 1;
                }
            }
        }
    }
    usize::try_from(seen.count_ones()).is_ok_and(|n| n == count)
}

/// Whether part `a`'s size is further over its target than part `b`'s, as shares of their targets.
fn further_over(sizes: &[u64], targets: &[u64], a: usize, b: usize) -> bool {
    let share = |p: usize| (u128::from(sizes.get(p).copied().unwrap_or(0)), u128::from(targets.get(p).copied().unwrap_or(1)));
    let ((sa, ta), (sb, tb)) = (share(a), share(b));
    sa * tb > sb * ta
}

/// The last tiles to the targets, where handicaps cannot reach them because a part takes a whole peninsula with its
/// neck: the part furthest over its target gives to its neighbour furthest under, as many tiles as bring their shares
/// level, each the donor's tile the taker would reach at least extra cost over the donor's own, never the donor's seed
/// nor a tile it would be cut at; so borders move as the costs' level lines do. Each transfer narrows the spread of
/// shares, so it ends when every part is within the tolerance or none can give.
fn finish(
    grid: &Grid,
    p: &Partition<'_>,
    owner: &mut [Option<usize>],
    reach: &mut [Option<u64>],
    tolerance_per_mille: u64,
    lot: &[u64],
    step: StepCost<'_>,
) -> bool {
    let parts = p.targets.len();
    let mut sizes = vec![0_u64; parts];
    for part in owner.iter().flatten() {
        if let Some(s) = sizes.get_mut(*part) {
            *s += 1;
        }
    }
    let mut stuck = vec![false; parts];
    loop {
        if spread(&sizes, p.targets) <= tolerance_per_mille {
            return true;
        }
        let donor = (0..parts)
            .filter(|d| !stuck.get(*d).copied().unwrap_or(true))
            .fold(None, |best: Option<usize>, d| match best {
                Some(b) if !further_over(&sizes, p.targets, d, b) => Some(b),
                _ => Some(d),
            });
        let Some(donor) = donor else { return false };
        let mut plus_one = sizes.clone();
        let mut taker: Option<usize> = None;
        for i in (0..grid.len()).filter(|i| owner.get(*i).copied().flatten() == Some(donor)) {
            for n in grid.neighbours(grid.tile(i)) {
                let Some(q) = owner.get(grid.index(n)).copied().flatten() else { continue };
                if let Some(s) = plus_one.get_mut(q) {
                    *s = sizes.get(q).copied().unwrap_or(0) + 1;
                }
                if q != donor
                    && further_over(&sizes, p.targets, donor, q)
                    && !further_over(&plus_one, p.targets, q, donor)
                    && taker.is_none_or(|t| further_over(&sizes, p.targets, t, q))
                {
                    taker = Some(q);
                }
            }
        }
        let Some(taker) = taker else {
            if let Some(s) = stuck.get_mut(donor) {
                *s = true;
            }
            continue;
        };
        let (sd, td) = (u128::from(sizes.get(donor).copied().unwrap_or(0)), u128::from(p.targets.get(donor).copied().unwrap_or(1)));
        let (st, tt) = (u128::from(sizes.get(taker).copied().unwrap_or(0)), u128::from(p.targets.get(taker).copied().unwrap_or(1)));
        // The tiles that bring the two shares level: (sd - k) / td = (st + k) / tt.
        let level = u64::try_from((sd * tt).saturating_sub(st * td) / (td + tt)).unwrap_or(u64::MAX);
        let want = if level == 0 { 1 } else { level };
        let lot_of = |i: usize| lot.get(i).copied().unwrap_or(u64::MAX);
        let via = |owner: &[Option<usize>], reach: &[Option<u64>], t: TileId| {
            grid.neighbours(t)
                .filter(|n| owner.get(grid.index(*n)).copied().flatten() == Some(taker))
                .filter_map(|n| reach.get(grid.index(n)).copied().flatten().map(|r| r + step(n, t)))
                .fold(None, |a: Option<u64>, c| Some(a.map_or(c, |a| if c < a { c } else { a })))
        };
        let margin = |owner: &[Option<usize>], reach: &[Option<u64>], i: usize| {
            let own = reach.get(i).copied().flatten()?;
            via(owner, reach, grid.tile(i)).map(|v| i128::from(v) - i128::from(own))
        };
        let mut border: BinaryHeap<Reverse<(i128, u64, usize)>> = BinaryHeap::new();
        for i in (0..grid.len()).filter(|i| owner.get(*i).copied().flatten() == Some(donor)) {
            if let Some(m) = margin(owner, reach, i) {
                border.push(Reverse((m, lot_of(i), i)));
            }
        }
        let mut moved = 0_u64;
        while moved < want {
            let Some(Reverse((_, _, i))) = border.pop() else { break };
            let t = grid.tile(i);
            if owner.get(i).copied().flatten() != Some(donor) || p.seeds.contains(&t) || !simple(grid, owner, t, donor) {
                continue;
            }
            let cost = via(owner, reach, t);
            if let Some(o) = owner.get_mut(i) {
                *o = Some(taker);
            }
            if let Some(r) = reach.get_mut(i) {
                *r = cost;
            }
            moved += 1;
            for n in grid.neighbours(t) {
                let j = grid.index(n);
                if owner.get(j).copied().flatten() == Some(donor)
                    && let Some(m) = margin(owner, reach, j)
                {
                    border.push(Reverse((m, lot_of(j), j)));
                }
            }
        }
        if let Some(s) = sizes.get_mut(donor) {
            *s -= moved;
        }
        if let Some(s) = sizes.get_mut(taker) {
            *s += moved;
        }
        if moved == 0 {
            if let Some(s) = stuck.get_mut(donor) {
                *s = true;
            }
        } else {
            stuck.fill(false);
        }
    }
}

/// The furthest any part strays from its target, in parts per thousand of the target.
fn spread(sizes: &[u64], targets: &[u64]) -> u64 {
    sizes
        .iter()
        .zip(targets)
        .map(|(s, t)| match (*s, *t) {
            (0, 0) => 0,
            (_, 0) => u64::MAX,
            _ => s.abs_diff(*t) * crate::consts::PER_MILLE / t,
        })
        .fold(0, |a, b| if b > a { b } else { a })
}

/// Parts grown over the ground by summed step cost, so fronts slow where steps cost more and borders fall at ridges
/// and rivers; then sized by handicaps. Each round every part's handicap moves by the cost of the rows of tiles it
/// has too many or too few, each row the mean step over the ground. Rounds end when every part is within
/// `tolerance_per_mille` of its target, or after `rounds`; the round with the least spread is finished tile by tile.
/// Returns each tile's part, and whether every part is within the tolerance.
#[clause("GEO.3", "GEO.10")]
#[must_use]
pub fn partition(
    grid: &Grid,
    p: &Partition<'_>,
    tolerance_per_mille: u64,
    rounds: u64,
    lot: &[u64],
    step: StepCost<'_>,
) -> (Vec<Option<usize>>, bool) {
    let parts = p.targets.len();
    let mut handicap = vec![0_i128; parts];
    let mut last = vec![0_i128; parts];
    let mut gain = vec![1_i128; parts];
    let mut best: Option<(u64, Vec<Option<usize>>, Vec<Option<u64>>)> = None;
    // A row of tiles costs the mean step over the ground, not across the borders, which sit on its costliest steps.
    let (mut total, mut pairs) = (0_u128, 0_u128);
    for i in (0..grid.len()).filter(|i| p.eligible.get(*i).copied().unwrap_or(false)) {
        let t = grid.tile(i);
        for n in grid.neighbours(t).filter(|n| p.eligible.get(grid.index(*n)).copied().unwrap_or(false)) {
            total += u128::from(step(t, n));
            pairs += 1;
        }
    }
    let mean = u64::try_from(if pairs == 0 { 0 } else { total / pairs }).unwrap_or(u64::MAX);
    // The tiles no front reaches join the part whose coast faces them as the parts first grow, and keep it: each
    // island moves whole, so letting it change hands as the borders move would make the sizes jump.
    let (first, _) = claim(grid, p.eligible, p.seeds, &vec![0; parts], lot, step);
    let mut joined = first.clone();
    join_nearest(grid, &mut joined, p.joiners, lot);
    let joined: Vec<(usize, usize)> = first
        .iter()
        .zip(&joined)
        .enumerate()
        .filter_map(|(i, (before, after))| if before.is_none() { after.map(|part| (i, part)) } else { None })
        .collect();
    for _ in 0..rounds {
        let least = handicap.iter().copied().fold(None, |a: Option<i128>, h| Some(a.map_or(h, |a| if h < a { h } else { a })));
        let base: Vec<u64> =
            handicap.iter().map(|h| u64::try_from(*h - least.unwrap_or(0)).unwrap_or(u64::MAX)).collect();
        let (mut owner, reach) = claim(grid, p.eligible, p.seeds, &base, lot, step);
        for (i, part) in &joined {
            if let Some(o) = owner.get_mut(*i) {
                *o = Some(*part);
            }
        }
        let mut sizes = vec![0_u64; parts];
        for part in owner.iter().flatten() {
            if let Some(s) = sizes.get_mut(*part) {
                *s += 1;
            }
        }
        let worst = spread(&sizes, p.targets);
        if best.as_ref().is_none_or(|(b, _, _)| worst < *b) {
            best = Some((worst, owner, reach));
        }
        if worst <= tolerance_per_mille {
            break;
        }
        for part in 0..parts {
            let (Some(size), Some(target)) = (sizes.get(part), p.targets.get(part)) else { continue };
            let excess = i128::from(*size) - i128::from(*target);
            // A part that overshot moves by half as much from then on, so the handicaps settle rather than swing.
            if let (Some(l), Some(g)) = (last.get_mut(part), gain.get_mut(part)) {
                if excess.signum() * l.signum() < 0 {
                    *g *= 2;
                }
                *l = excess;
            }
            // Over steps of like cost a part grows as a square, a row on every side at a time: from its size to its
            // target is half the difference of their sides in rows, each row the mean step.
            let side = |tiles: u64| i128::from(rounded_sqrt(tiles * mean * mean));
            if let (Some(h), Some(g)) = (handicap.get_mut(part), gain.get(part)) {
                let by = (side(*size) - side(*target)) / (2 * g);
                *h += if by == 0 { excess.signum() } else { by };
            }
        }
    }
    match best {
        Some((worst, owner, _)) if worst <= tolerance_per_mille => (owner, true),
        Some((_, mut owner, mut reach)) => {
            let within = finish(grid, p, &mut owner, &mut reach, tolerance_per_mille, lot, step);
            (owner, within)
        }
        None => (vec![None; grid.len()], false),
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

    use super::{Partition, components, join_nearest, merge_small, partition};
    use crate::grid::Grid;

    fn lot(n: usize) -> Vec<u64> {
        (0..n).map(|i| u64::try_from((i * 7919) % 1009).unwrap()).collect()
    }

    fn connected(g: &Grid, owner: &[Option<usize>], part: usize) -> bool {
        let mask: Vec<bool> = owner.iter().map(|o| *o == Some(part)).collect();
        components(g, &mask).1.len() == 1
    }

    #[test]
    fn partition_reaches_targets_in_one_piece_each() {
        let g = Grid { width: 12, height: 10, tile_m: 10_000 };
        let eligible: Vec<bool> = (0..g.len()).map(|i| !(40..44).contains(&i) && i % 12 != 11).collect();
        let land = u64::try_from(eligible.iter().filter(|e| **e).count()).unwrap();
        let seeds = [TileId::new(0), TileId::new(65), TileId::new(118)];
        let targets = [50_u64, 30, land - 80];
        let over = Partition { eligible: &eligible, joiners: &eligible, seeds: &seeds, targets: &targets };
        let ground = |_: TileId, b: TileId| 10_000 + u64::from(b.get() * 7919 % 997);
        let (owner, within) = partition(&g, &over, 50, 48, &lot(g.len()), &ground);
        let count = |p: usize| u64::try_from(owner.iter().filter(|o| **o == Some(p)).count()).unwrap();
        assert!(within, "within 5%: {} {} {}", count(0), count(1), count(2));
        assert!((0..3).all(|p| connected(&g, &owner, p)), "each part is one piece");
        assert!(owner.iter().zip(&eligible).all(|(o, e)| o.is_some() == *e), "every eligible tile, and no other");
    }

    #[test]
    fn borders_fall_at_costly_ground() {
        let g = Grid { width: 9, height: 1, tile_m: 10_000 };
        let eligible = vec![true; 9];
        let ridge = |a: TileId, b: TileId| if a.get().min(b.get()) == 3 && a.get().max(b.get()) == 4 { 100 } else { 1 };
        let seeds = [TileId::new(0), TileId::new(8)];
        let over = Partition { eligible: &eligible, joiners: &eligible, seeds: &seeds, targets: &[5, 4] };
        let (owner, within) = partition(&g, &over, 250, 48, &lot(9), &ridge);
        assert!(within);
        assert_eq!(owner[3], Some(0), "the west part stops at the ridge");
        assert_eq!(owner[4], Some(1), "and the east part holds the far side, though it is further from its seed");
    }

    #[test]
    fn a_part_with_no_room_is_refused() {
        let g = Grid { width: 6, height: 1, tile_m: 10_000 };
        let eligible = vec![true, true, false, true, true, true];
        let seeds = [TileId::new(0), TileId::new(5)];
        let over = Partition { eligible: &eligible, joiners: &eligible, seeds: &seeds, targets: &[3, 2] };
        let (owner, within) = partition(&g, &over, 0, 48, &lot(6), &|_, _| 1);
        assert!(!within, "the western piece holds two tiles, never three");
        assert_eq!(owner[1], Some(0));
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
