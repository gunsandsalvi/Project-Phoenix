use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Draws, multivariate_hypergeometric};

/// Members of one profile value hit by several processes on one day, allocated among the processes: for each set of
/// processes, how many members were hit by exactly those. Each process's hits are drawn, in declared process order,
/// from its own stream as a hypergeometric draw over the members as the processes before it left them, so which
/// members two processes share is drawn, never assumed.
#[clause("REP.7", "REP.23")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlap {
    /// Members per set of processes, the set's bits being the processes in declared order; set nought is the
    /// members no process hit.
    pub by_set: Vec<u64>,
}

impl Overlap {
    /// The allocation of `members` among processes whose hits are `hits`, in declared order, each drawn with that
    /// process's own draws.
    #[must_use]
    pub fn allocate(members: u64, hits: &[u64], draws: &mut [Draws]) -> Overlap {
        if hits.len() != draws.len() {
            violation!(clause = "REP.7", "overlapping processes without a stream each", processes = hits.len());
        }
        let Some(sets) = u32::try_from(hits.len()).ok().and_then(|n| 1_usize.checked_shl(n)) else {
            violation!(clause = "REP.7", "too many processes overlapping on one value", processes = hits.len());
        };
        let mut by_set = vec![0_u64; sets];
        if let Some(none) = by_set.first_mut() {
            *none = members;
        }
        for (i, (k, d)) in hits.iter().zip(draws.iter_mut()).enumerate() {
            if *k > members {
                violation!(clause = "REP.7", "more members hit than a value holds", hit = *k, members = members);
            }
            let bit = 1_usize << i;
            let before: Vec<u64> = by_set.iter().take(bit).copied().collect();
            let mut taken = vec![0_u64; before.len()];
            multivariate_hypergeometric(d, &before, *k, &mut taken);
            for (set, t) in taken.iter().enumerate() {
                if let Some(from) = by_set.get_mut(set) {
                    *from -= t;
                }
                if let Some(to) = by_set.get_mut(set | bit) {
                    *to += t;
                }
            }
        }
        Overlap { by_set }
    }

    /// Members hit by a process, over every set it is in.
    #[must_use]
    pub fn hit_by(&self, process: usize) -> u64 {
        let bit = 1_usize << process;
        self.by_set.iter().enumerate().filter(|(s, _)| s & bit != 0).map(|(_, n)| n).sum()
    }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::Overlap;

    fn draws(name: &str, i: u32) -> Draws {
        Draws::new(stream_key(Seed::new(8), name), Subject::new(SubjectTag::Party, 2), i, 0)
    }

    #[test]
    fn overlap_allocation_sums() {
        let mut both = 0_u64;
        for i in 0..20_000_u32 {
            let mut d = [draws("DEM.illness", i), draws("LAB.injury", i), draws("DEM.death", i)];
            let o = Overlap::allocate(50, &[10, 5, 2], &mut d);
            assert_eq!(o.by_set.iter().sum::<u64>(), 50, "every member in one set");
            assert_eq!([o.hit_by(0), o.hit_by(1), o.hit_by(2)], [10, 5, 2], "each process's hits");
            both += o.by_set[0b011] + o.by_set[0b111];
        }
        // Ill and injured together: five of the ten ill drawn among fifty, a tenth on average of ten times five.
        let mean = phx_rand::float::from_u64(both) / 20_000.0;
        assert!((mean - 1.0).abs() < 0.03, "{mean}");
    }
}
