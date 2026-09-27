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
        Leg::RateOnNotional { .. } | Leg::StepSchedule { .. } | Leg::PayableInKind { .. } | Leg::Amortising => false,
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

/// A count of members moved by a change; fewer than none is a state that cannot exist.
fn moved(count: u64, delta: i64) -> u64 {
    let Some(now) = count.checked_add_signed(delta) else {
        violation!(clause = "REG.14", "a holder leaving more members than it holds", delta = delta);
    };
    now
}

/// The members of a line side's rows by holder, to draw from by their counts: a Fenwick tree over the members each
/// holder has left, so a member is found and taken in the log of the holders.
#[clause("REP.23")]
#[derive(Clone, Debug)]
pub(crate) struct Tally {
    index: BTreeMap<PartyId, usize>,
    parties: Vec<PartyId>,
    tree: Vec<u64>,
    held: Vec<u64>,
    taken: Vec<u32>,
    left: u64,
}

impl Tally {
    /// What the tally holds in memory: its index's entries and its lists.
    pub(crate) fn bytes(&self) -> usize {
        size_of::<Tally>()
            + self.index.len() * size_of::<(PartyId, usize)>()
            + self.parties.capacity() * size_of::<PartyId>()
            + (self.tree.capacity() + self.held.capacity()) * size_of::<u64>()
            + self.taken.capacity() * size_of::<u32>()
    }

    /// A side's rows as each holder and its members.
    pub(crate) fn new(rows: &[(PartyId, u32)]) -> Tally {
        let n = rows.len();
        let mut tree = vec![0_u64; n + 1];
        for (i, (_, c)) in rows.iter().enumerate() {
            let mut j = i + 1;
            while j <= n {
                if let Some(t) = tree.get_mut(j) {
                    *t += u64::from(*c);
                }
                j += j.isolate_lowest_one();
            }
        }
        Tally {
            index: rows.iter().enumerate().map(|(i, (p, _))| (*p, i)).collect(),
            parties: rows.iter().map(|(p, _)| *p).collect(),
            tree,
            held: rows.iter().map(|(_, c)| u64::from(*c)).collect(),
            taken: vec![0; n],
            left: rows.iter().map(|(_, c)| u64::from(*c)).sum(),
        }
    }

    /// The members a holder has left to draw; the tally holds every holder of its side, so one it does not is a party
    /// that holds no row there, which cannot leave it.
    pub(crate) fn held(&self, party: PartyId) -> u64 {
        let Some(held) = self.index.get(&party).and_then(|i| self.held.get(*i)) else {
            violation!(clause = "REP.23", "members leaving a side its tally holds no row of", party = party.get());
        };
        *held
    }

    /// Every node over a holder moved by `delta`.
    fn shift(&mut self, at: usize, delta: i64) {
        let Some(held) = self.held.get_mut(at) else {
            violation!(clause = "REP.23", "a tally's holder beyond its holders", at = at);
        };
        *held = moved(*held, delta);
        let n = self.parties.len();
        let mut j = at + 1;
        while j <= n {
            if let Some(t) = self.tree.get_mut(j) {
                *t = moved(*t, delta);
            }
            j += j.isolate_lowest_one();
        }
        self.left = moved(self.left, delta);
    }

    /// A holder's members moved by a change made beside the tally's draws, so it stays the side it reads.
    pub(crate) fn adjust(&mut self, party: PartyId, delta: i64) {
        if delta != 0 {
            let Some(at) = self.index.get(&party).copied() else {
                violation!(clause = "REP.23", "a tally moving a holder it does not hold", party = party.get());
            };
            self.shift(at, delta);
        }
    }

    /// The members left to draw.
    pub(crate) fn left(&self) -> u64 {
        self.left
    }

    /// The holder holding the member at a position among those left, by the tree's descent.
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

    /// One member drawn from those left and taken; its holder.
    fn draw(&mut self, draws: &mut Draws) -> Option<PartyId> {
        if self.left == 0 {
            return None;
        }
        let at = self.find(below_u64(draws, self.left));
        self.shift(at, -1);
        if let Some(t) = self.taken.get_mut(at) {
            *t += 1;
        }
        self.parties.get(at).copied()
    }

    fn taken(&self, party: PartyId) -> u32 {
        self.index.get(&party).and_then(|i| self.taken.get(*i)).copied().unwrap_or(0)
    }

    fn all_taken(&self) -> impl Iterator<Item = (PartyId, u32)> + '_ {
        let mut all: Vec<(PartyId, u32)> =
            self.parties.iter().zip(&self.taken).filter(|(_, n)| **n > 0).map(|(p, n)| (*p, *n)).collect();
        all.sort_unstable();
        all.into_iter()
    }

    /// A taker for `count` members passed to the side's holders: one holder drawn by its members, taking them all;
    /// none when the side holds no member. A taker is not bounded by what it holds, as a buyer takes what it is sold,
    /// and nothing is taken from the tally.
    pub(crate) fn pass(&self, count: u32, draws: &mut Draws) -> Option<PartyId> {
        if count == 0 || self.left == 0 {
            return None;
        }
        self.parties.get(self.find(below_u64(draws, self.left))).copied()
    }

    /// Up to `count` members drawn from those left, one at a time; each holder drawn from, with how many, in holder
    /// order, and what remains once the side is drawn out.
    pub(crate) fn draw_many(&mut self, count: u32, draws: &mut Draws) -> (Vec<(PartyId, u32)>, u32) {
        let mut more: BTreeMap<PartyId, u32> = BTreeMap::new();
        let mut remaining = count;
        while remaining > 0 {
            let Some(p) = self.draw(draws) else { break };
            *more.entry(p).or_insert(0) += 1;
            remaining -= 1;
        }
        (more.into_iter().collect(), remaining)
    }
}

/// The members drawn to lose on a cleared line whose payers failed: one member at a time from the claimant rows, each
/// by the members it has left, in one sequence from the line's stream, so the first drawn are the same members
/// whatever the failures reach.
#[clause("REP.23")]
#[derive(Debug)]
pub struct Losers {
    tally: Tally,
    drawn: u64,
    draws: Draws,
}

impl Losers {
    /// What the draw holds in memory.
    #[must_use]
    pub fn bytes(&self) -> usize {
        size_of::<Losers>() + self.tally.bytes()
    }

    /// The claimant rows by party with their counts, none drawn yet.
    #[must_use]
    pub fn new(claimants: &[(PartyId, u32)], draws: Draws) -> Losers {
        Losers { tally: Tally::new(claimants), drawn: 0, draws }
    }

    /// The members of a claimant drawn so far; a party holding no claimant row has none.
    #[must_use]
    pub fn lost(&self, party: PartyId) -> u32 {
        self.tally.taken(party)
    }

    /// Every claimant with members drawn, and how many, in party order.
    pub fn all_lost(&self) -> impl Iterator<Item = (PartyId, u32)> + '_ {
        self.tally.all_taken()
    }

    /// The members drawn so far.
    #[must_use]
    pub fn drawn(&self) -> u64 {
        self.drawn
    }

    /// The sequence drawn on until `failed` members are drawn; each claimant newly drawn, with how many more.
    pub fn draw_to(&mut self, failed: u64) -> Vec<(PartyId, u32)> {
        if failed > self.drawn + self.tally.left() {
            violation!(
                clause = "REP.31",
                "more members failed on a cleared line than its claimants hold",
                failed = failed,
                held = self.drawn + self.tally.left()
            );
        }
        let mut more: BTreeMap<PartyId, u32> = BTreeMap::new();
        while self.drawn < failed {
            let Some(p) = self.tally.draw(&mut self.draws) else {
                violation!(clause = "REP.31", "a cleared line's claimants drawn out", failed = failed);
            };
            *more.entry(p).or_insert(0) += 1;
            self.drawn += 1;
        }
        more.into_iter().collect()
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

    use super::{Losers, Tally};

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(7), "REP.cleared"), Subject::new(SubjectTag::Line, 3), 11, 0)
    }

    fn seeded(t: u64) -> Draws {
        Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0)
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
        for t in 0..4000_u64 {
            let mut l = Losers::new(&rows, seeded(t));
            let _ = l.draw_to(1);
            first += l.lost(PartyId::new(1));
        }
        let share = f64::from(first) / 4000.0;
        assert!((share - 0.25).abs() < 0.03, "one member in four, drawn {share}");
    }

    #[test]
    fn leaving_draws_to_the_count_or_the_side_drawn_out() {
        let rows: Vec<(PartyId, u32)> = [(1, 8), (2, 3)].map(|(p, c)| (PartyId::new(p), c)).to_vec();
        for t in 0..64_u64 {
            let mut d = seeded(t);
            let mut tally = Tally::new(&rows);
            let (taken, rest) = tally.draw_many(4, &mut d);
            assert_eq!((taken.iter().map(|(_, k)| *k).sum::<u32>(), rest), (4, 0));
            let (taken, rest) = tally.draw_many(9, &mut d);
            assert_eq!((taken.iter().map(|(_, k)| *k).sum::<u32>(), rest), (7, 2), "what the side lacks remains");
        }
    }

    /// A remainder passed goes whole to one holder drawn by its members, and nothing is taken from the tally.
    #[test]
    fn a_passed_remainder_goes_to_one_drawn_holder() {
        let rows: Vec<(PartyId, u32)> = [(1, 0), (2, 3), (3, 2)].map(|(p, c)| (PartyId::new(p), c)).to_vec();
        for t in 0..64_u64 {
            let mut d = seeded(t);
            let tally = Tally::new(&rows);
            let taker = tally.pass(12, &mut d);
            assert!(taker.is_some_and(|p| p != PartyId::new(1)), "a holder of none takes nothing");
            assert_eq!(tally.held(PartyId::new(2)) + tally.held(PartyId::new(3)), 5, "nothing taken from the tally");
            assert_eq!(tally.pass(0, &mut d), None);
        }
        let empty = [(PartyId::new(1), 0)];
        assert_eq!(Tally::new(&empty).pass(12, &mut seeded(0)), None, "no member to take it");
    }

    /// A holder whose members left beside the draws is drawn no more, and one given members is drawn by them.
    #[test]
    fn an_adjusted_holder_is_drawn_by_what_it_holds() {
        let rows: Vec<(PartyId, u32)> = [(1, 5), (2, 5)].map(|(p, c)| (PartyId::new(p), c)).to_vec();
        for t in 0..64_u64 {
            let mut d = seeded(t);
            let mut tally = Tally::new(&rows);
            tally.adjust(PartyId::new(1), -5);
            tally.adjust(PartyId::new(2), 3);
            assert_eq!(tally.held(PartyId::new(2)), 8);
            let (taken, rest) = tally.draw_many(8, &mut d);
            assert_eq!((taken, rest), (vec![(PartyId::new(2), 8)], 0));
        }
    }
}
