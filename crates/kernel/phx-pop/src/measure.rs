use phx_macros::clause;
use phx_store::Backing;

use crate::landing::Landed;
use crate::table::CellTable;
use crate::tolerance::Estimate;

/// What one population's table holds at a close: its cells and individuals, its members, and the members in rows of
/// weight one — cells of one member and individuals — so the share carried one by one is read beside the cell count.
#[clause("REP.15", "REP.13")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Census {
    pub cells: u64,
    pub individuals: u64,
    pub members: u64,
    pub at_weight_one: u64,
}

impl Census {
    /// The table counted row by row.
    #[must_use]
    pub fn of<B: Backing>(table: &CellTable<B>) -> Census {
        let mut c = Census::default();
        for slot in table.slots() {
            let w = u64::from(table.weight(slot).get());
            if table.hot(slot).is_individual() {
                c.individuals += 1;
            } else {
                c.cells += 1;
            }
            c.members += w;
            if w == 1 {
                c.at_weight_one += 1;
            }
        }
        c
    }

    /// The share of the population carried at weight one, or none of a population with no members.
    #[must_use]
    pub fn share_at_weight_one(&self) -> Option<f64> {
        (self.members > 0)
            .then(|| phx_rand::float::from_u64(self.at_weight_one) / phx_rand::float::from_u64(self.members))
    }
}

/// What the representation cost one kind in a day, as it is reported: per position the dispersion its joins erased
/// and the decision gap last estimated, and the day's splits, landings, new cells and occasions.
#[clause("REP.15")]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayCosts {
    pub erased: Vec<f64>,
    pub gaps: Vec<f64>,
    pub splits: u64,
    pub landings: u64,
    pub new_cells: u64,
    pub occasions: u64,
}

impl DayCosts {
    /// A day's costs for a kind of `positions` positions, nought until they are incurred.
    #[must_use]
    pub fn new(positions: usize) -> DayCosts {
        DayCosts { erased: vec![0.0; positions], gaps: vec![0.0; positions], ..DayCosts::default() }
    }

    /// Dispersion erased at one position by a pooled flow, a meeting or a review, as it is incurred.
    pub fn erase(&mut self, position: usize, amount: f64) {
        if let Some(e) = self.erased.get_mut(position) {
            *e += amount;
        }
    }

    /// A day's landings: their joins' erased dispersion, their count and the cells they made.
    pub fn landed(&mut self, l: &Landed) {
        for (i, e) in l.erased.iter().enumerate() {
            self.erase(i, *e);
        }
        self.landings += l.landings;
        self.new_cells += l.new_cells;
    }

    /// The gaps the day's estimate read, per position.
    pub fn estimated(&mut self, est: &Estimate) {
        self.gaps.clone_from(&est.gap);
    }
}

#[cfg(test)]
mod tests {
    use phx_core::Weight;

    use super::{Census, DayCosts};
    use crate::fixture::{PLAIN, Spec, Ten};
    use crate::landing::Landed;
    use crate::tolerance::Estimate;

    #[test]
    fn census_counts_members_and_weight_one() {
        let mut ten = Ten::new();
        let _ = ten.add(1, PLAIN);
        let (_, one) = ten.add(1, Spec { weight: 1, income: 100, ..PLAIN });
        let (_, ind) = ten.add(2, Spec { weight: 1, income: 100, ..PLAIN });
        ten.table.make_individual(ind);
        assert_eq!(ten.table.weight(one), Weight::new(1));
        let c = Census::of(&ten.table);
        assert_eq!(c, Census { cells: 2, individuals: 1, members: 102, at_weight_one: 2 });
        assert_eq!(c.share_at_weight_one(), Some(2.0 / 102.0));
        assert_eq!(Census::default().share_at_weight_one(), None);
    }

    #[test]
    fn day_costs_gather_what_the_day_incurred() {
        let mut costs = DayCosts::new(2);
        costs.erase(1, 4.0);
        let l = Landed { landings: 3, new_cells: 1, erased: vec![1.5, 0.5], ..Landed::default() };
        costs.landed(&l);
        costs.estimated(&Estimate { gap: vec![0.25, 0.0], pairs: vec![2, 0], sampled: 4 });
        assert_eq!(costs.erased, [1.5, 4.5]);
        assert_eq!((costs.landings, costs.new_cells), (3, 1));
        assert_eq!(costs.gaps, [0.25, 0.0]);
    }
}
