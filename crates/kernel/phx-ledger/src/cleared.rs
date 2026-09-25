use std::collections::BTreeMap;

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_rand::{Draws, below_u64};

use crate::algebra::Leg;

/// Whether a leg's amount is per contract, so a row's due is it times the row's count, rather than reckoned on the
/// row's balance, which is its total already.
#[must_use]
pub fn per_contract(leg: &Leg) -> bool {
    match leg {
        Leg::FixedAmount(_)
        | Leg::Principal { .. }
        | Leg::PerTime { .. }
        | Leg::Contingent { .. }
        | Leg::Delivery(_) => true,
        Leg::RateOnNotional { .. } | Leg::StepSchedule { .. } | Leg::PayableInKind { .. } => false,
        Leg::Indexed { leg, .. } => per_contract(leg),
        Leg::Elective { legs, .. } => {
            let all = legs.iter().all(per_contract);
            if !all && legs.iter().any(per_contract) {
                violation!(clause = "REG.5", "an elective leg paying both per contract and on the balance");
            }
            all
        }
    }
}

/// An amount per contract times a count of contracts.
pub(crate) fn times(per: i64, count: u32) -> i64 {
    let Some(x) = per.checked_mul(i64::from(count)) else {
        capacity_exceeded!("a row's dues", i64::MAX, count);
    };
    x
}

/// The members drawn to lose on a cleared line whose payers failed: one member at a time from the claimant rows, each
/// by the members it has left, in one sequence from the line's stream, so the first n drawn are the same members
/// whatever n the failures reach.
#[clause("REP.23")]
#[derive(Debug)]
pub struct Losers {
    index: BTreeMap<PartyId, usize>,
    parties: Vec<PartyId>,
    tree: Vec<u64>,
    lost: Vec<u32>,
    left: u64,
    drawn: u64,
    draws: Draws,
}

impl Losers {
    /// The claimant rows by party with their counts, none drawn yet.
    #[must_use]
    pub fn new(claimants: &[(PartyId, u32)], draws: Draws) -> Losers {
        let n = claimants.len();
        let mut tree = vec![0_u64; n + 1];
        for (i, (_, c)) in claimants.iter().enumerate() {
            let mut j = i + 1;
            while j <= n {
                if let Some(t) = tree.get_mut(j) {
                    *t += u64::from(*c);
                }
                j += j.isolate_lowest_one();
            }
        }
        Losers {
            index: claimants.iter().enumerate().map(|(i, (p, _))| (*p, i)).collect(),
            parties: claimants.iter().map(|(p, _)| *p).collect(),
            tree,
            lost: vec![0; n],
            left: claimants.iter().map(|(_, c)| u64::from(*c)).sum(),
            drawn: 0,
            draws,
        }
    }

    /// The members of a claimant drawn so far; a party holding no claimant row has none.
    #[must_use]
    pub fn lost(&self, party: PartyId) -> u32 {
        self.index.get(&party).and_then(|i| self.lost.get(*i)).copied().unwrap_or(0)
    }

    /// Every claimant with members drawn, and how many.
    pub fn all_lost(&self) -> impl Iterator<Item = (PartyId, u32)> + '_ {
        self.parties.iter().zip(&self.lost).filter(|(_, n)| **n > 0).map(|(p, n)| (*p, *n))
    }

    /// The members drawn so far.
    #[must_use]
    pub fn drawn(&self) -> u64 {
        self.drawn
    }

    /// The sequence drawn on until `failed` members are drawn; each claimant newly drawn, with how many more.
    pub fn draw_to(&mut self, failed: u64) -> Vec<(PartyId, u32)> {
        if failed > self.drawn + self.left {
            violation!(
                clause = "REP.31",
                "more members failed on a cleared line than its claimants hold",
                failed = failed,
                held = self.drawn + self.left
            );
        }
        let mut more: BTreeMap<usize, u32> = BTreeMap::new();
        while self.drawn < failed {
            let u = below_u64(&mut self.draws, self.left);
            let at = self.find(u);
            self.take(at);
            *more.entry(at).or_insert(0) += 1;
        }
        more.into_iter().filter_map(|(i, k)| self.parties.get(i).map(|p| (*p, k))).collect()
    }

    /// The claimant holding the member at a position among those left, by the tree's descent.
    fn find(&self, mut target: u64) -> usize {
        let n = self.parties.len();
        let mut at = 0_usize;
        let mut step = n.checked_next_power_of_two().unwrap_or(n);
        while step > 0 {
            let next = at + step;
            if let Some(t) = self.tree.get(next).filter(|_| next <= n)
                && *t <= target
            {
                target -= *t;
                at = next;
            }
            step /= 2;
        }
        at
    }

    fn take(&mut self, at: usize) {
        let n = self.parties.len();
        let mut j = at + 1;
        while j <= n {
            if let Some(t) = self.tree.get_mut(j) {
                *t -= 1;
            }
            j += j.isolate_lowest_one();
        }
        if let Some(l) = self.lost.get_mut(at) {
            *l += 1;
        }
        self.left -= 1;
        self.drawn += 1;
    }
}

/// A line's draws in the kernel's tests.
#[cfg(test)]
pub(crate) fn test_draws(line: phx_id::LineId) -> Draws {
    use phx_rand::{Seed, Subject, SubjectTag, stream_key};
    Draws::new(stream_key(Seed::new(1), "REP.cleared"), Subject::new(SubjectTag::Line, u64::from(line.get())), 0, 0)
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::Losers;

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(7), "REP.cleared"), Subject::new(SubjectTag::Line, 3), 11, 0)
    }

    fn claimants() -> Vec<(PartyId, u32)> {
        [(1, 4), (2, 1), (3, 0), (4, 9), (5, 2)].map(|(p, c)| (PartyId::new(p), c)).to_vec()
    }

    #[test]
    fn the_first_drawn_are_the_same_whatever_the_count() {
        let mut stepped = Losers::new(&claimants(), draws());
        for n in [1, 3, 3, 7, 12] {
            let _ = stepped.draw_to(n);
        }
        let mut once = Losers::new(&claimants(), draws());
        let _ = once.draw_to(12);
        let lost = |l: &Losers| claimants().iter().map(|(p, _)| l.lost(*p)).collect::<Vec<_>>();
        assert_eq!(lost(&stepped), lost(&once));
    }

    #[test]
    fn the_drawn_sum_to_the_failed_within_each_row() {
        let mut l = Losers::new(&claimants(), draws());
        let more = l.draw_to(10);
        assert_eq!(more.iter().map(|(_, k)| u64::from(*k)).sum::<u64>(), 10);
        for (p, c) in claimants() {
            assert!(l.lost(p) <= c, "a row loses no more members than it holds");
        }
        assert_eq!(l.lost(PartyId::new(3)), 0, "a row of no members is never drawn");
        let _ = l.draw_to(16);
        for (p, c) in claimants() {
            assert_eq!(l.lost(p), c, "every member drawn once all failed");
        }
    }

    #[test]
    fn each_member_is_as_likely_to_lose() {
        let rows: Vec<(PartyId, u32)> = [(1, 1), (2, 3)].map(|(p, c)| (PartyId::new(p), c)).to_vec();
        let mut first = 0_u32;
        let trials = 4000_u64;
        for t in 0..trials {
            let d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut l = Losers::new(&rows, d);
            let _ = l.draw_to(1);
            first += l.lost(PartyId::new(1));
        }
        let share = f64::from(first) / 4000.0;
        assert!((share - 0.25).abs() < 0.03, "one member in four, drawn {share}");
    }
}
