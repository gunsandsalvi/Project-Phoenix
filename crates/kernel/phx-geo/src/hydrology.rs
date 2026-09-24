use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use phx_macros::clause;

use crate::grid::Grid;

/// A height in the flood's queue, ordered totally, ties by the cell's lot and then its place.
#[derive(Clone, Copy, Debug)]
struct Queued {
    height: f64,
    lot: u64,
    cell: usize,
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> Ordering {
        self.height.total_cmp(&other.height).then(self.lot.cmp(&other.lot)).then(self.cell.cmp(&other.cell))
    }
}

/// Where every cell drains: each land cell's receiver, the neighbour it spills to on its lowest route to an outlet;
/// the cells in the order the flood reached them, outlets first, so every receiver comes before the cells it
/// receives; and each cell's height with its depressions filled to their spill level.
#[derive(Clone, Debug, PartialEq)]
pub struct Drainage {
    pub receiver: Vec<Option<usize>>,
    pub order: Vec<usize>,
    pub filled: Vec<f64>,
}

/// The priority flood: from every outlet, the lowest cell reached so far claims its unreached neighbours, each
/// draining to it and filled to at least its height, so a depression drains over its lowest rim. Cells of one level,
/// as a filled depression's are, take their turns by lot, so water crosses a flat by a winding way rather than in the
/// straight rays of the order the flood reached them.
#[clause("GEO.10")]
#[must_use]
pub fn drainage(grid: &Grid, height: &[f64], outlet: &[bool], lot: &[u64]) -> Drainage {
    let n = grid.len();
    let mut receiver = vec![None; n];
    let mut filled = height.to_vec();
    let mut reached = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut heap = BinaryHeap::new();
    let lot_of = |cell: usize| lot.get(cell).copied().unwrap_or(u64::MAX);
    for (cell, is_outlet) in outlet.iter().enumerate() {
        if *is_outlet && let Some(h) = height.get(cell) {
            heap.push(Reverse(Queued { height: *h, lot: lot_of(cell), cell }));
            if let Some(r) = reached.get_mut(cell) {
                *r = true;
            }
        }
    }
    while let Some(Reverse(q)) = heap.pop() {
        order.push(q.cell);
        for next in grid.neighbours(grid.tile(q.cell)) {
            let i = grid.index(next);
            if reached.get(i).copied().unwrap_or(true) {
                continue;
            }
            if let Some(r) = reached.get_mut(i) {
                *r = true;
            }
            let level = filled.get(i).copied().unwrap_or(q.height);
            let spill = if level < q.height { q.height } else { level };
            if let Some(f) = filled.get_mut(i) {
                *f = spill;
            }
            if let Some(r) = receiver.get_mut(i) {
                *r = Some(q.cell);
            }
            heap.push(Reverse(Queued { height: spill, lot: lot_of(i), cell: i }));
        }
    }
    Drainage { receiver, order, filled }
}

/// Each cell's upstream count: itself and every cell that drains through it.
#[clause("GEO.10")]
#[must_use]
pub fn upstream(d: &Drainage) -> Vec<u32> {
    let mut area = vec![1_u32; d.receiver.len()];
    for cell in d.order.iter().rev() {
        let own = area.get(*cell).copied().unwrap_or(0);
        if let Some(r) = d.receiver.get(*cell).copied().flatten()
            && let Some(a) = area.get_mut(r)
        {
            *a += own;
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::{drainage, upstream};
    use crate::grid::Grid;

    #[test]
    fn a_pit_drains_over_its_lowest_rim_and_the_far_end_round_the_seam() {
        // A ring of six with the sea at the first, a pit at the third, and the far end beside the sea round the seam.
        let g = Grid { width: 6, height: 1, tile_m: 10_000 };
        let h = [0.0, 3.0, 1.0, 5.0, 6.0, 7.0];
        let outlet = [true, false, false, false, false, false];
        let d = drainage(&g, &h, &outlet, &[0; 6]);
        assert_eq!(d.receiver, vec![None, Some(0), Some(1), Some(2), Some(3), Some(0)]);
        assert_eq!(d.filled[2].to_bits(), 3.0_f64.to_bits(), "the pit filled to its rim");
        assert_eq!(upstream(&d), vec![6, 4, 3, 2, 1, 1]);
        assert_eq!(d.order[0], 0, "the outlet first, then every cell after its receiver");
    }
}
