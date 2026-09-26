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

/// A count of members moved by a change; fewer than none is a state that cannot exist.
fn moved(count: u64, delta: i64) -> u64 {
    let Some(now) = count.checked_add_signed(delta) else {
        violation!(clause = "REG.14", "a holder leaving more members than it holds", delta = delta);
    };
    now
}

/// One unit's holders in a tally: a Fenwick tree over their members left, so a member is found and taken in the log
/// of the holders.
#[derive(Clone, Debug)]
struct Class {
    unit: u32,
    parties: Vec<PartyId>,
    tree: Vec<u64>,
    held: Vec<u64>,
    taken: Vec<u32>,
    left: u64,
}

impl Class {
    fn bytes(&self) -> usize {
        size_of::<Class>()
            + self.parties.capacity() * size_of::<PartyId>()
            + (self.tree.capacity() + self.held.capacity()) * size_of::<u64>()
            + self.taken.capacity() * size_of::<u32>()
    }

    fn new(unit: u32, rows: &[(PartyId, u32)]) -> Class {
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
        Class {
            unit,
            parties: rows.iter().map(|(p, _)| *p).collect(),
            tree,
            held: rows.iter().map(|(_, c)| u64::from(*c)).collect(),
            taken: vec![0; n],
            left: rows.iter().map(|(_, c)| u64::from(*c)).sum(),
        }
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

    /// A holder's members moved by `delta`, as a change the tally did not draw: more contracts to it, or fewer.
    fn adjust(&mut self, at: usize, delta: i64) {
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

    /// A holder's whole unit taken; an agent holding less than its multiplicity is a state that cannot exist.
    fn take(&mut self, at: usize) {
        let Some(held) = self.held.get_mut(at).filter(|h| **h >= u64::from(self.unit)) else {
            violation!(clause = "REP.9", "an agent's row holding less than its multiplicity", unit = self.unit);
        };
        *held -= u64::from(self.unit);
        let n = self.parties.len();
        let mut j = at + 1;
        while j <= n {
            if let Some(t) = self.tree.get_mut(j) {
                *t -= u64::from(self.unit);
            }
            j += j.isolate_lowest_one();
        }
        if let Some(l) = self.taken.get_mut(at) {
            *l += self.unit;
        }
        self.left -= u64::from(self.unit);
    }
}

/// The members of a line side's rows by holder, grouped by each holder's unit — its multiplicity, one for an
/// individual — to draw from by their counts: a member is drawn and its holder gives its whole unit, so an agent's
/// twins stay alike.
#[clause("REP.23", "REP.9")]
#[derive(Clone, Debug)]
pub(crate) struct Tally {
    index: BTreeMap<PartyId, (usize, usize)>,
    classes: Vec<Class>,
}

impl Tally {
    /// What the tally holds in memory: its index's entries and its classes with their lists.
    pub(crate) fn bytes(&self) -> usize {
        self.index.len() * size_of::<(PartyId, (usize, usize))>() + self.classes.iter().map(Class::bytes).sum::<usize>()
    }

    /// A side's rows as each holder, its members and its unit.
    pub(crate) fn new(rows: &[(PartyId, u32, u32)]) -> Tally {
        let mut by_unit: BTreeMap<u32, Vec<(PartyId, u32)>> = BTreeMap::new();
        for (p, c, u) in rows {
            if *u == 0 {
                violation!(clause = "REP.3", "a holder of no multiplicity", party = p.get());
            }
            by_unit.entry(*u).or_default().push((*p, *c));
        }
        let classes: Vec<Class> = by_unit.iter().map(|(u, r)| Class::new(*u, r)).collect();
        let index = classes
            .iter()
            .enumerate()
            .flat_map(|(ci, c)| c.parties.iter().enumerate().map(move |(i, p)| (*p, (ci, i))))
            .collect();
        Tally { index, classes }
    }

    /// The members a holder has left to draw; the tally holds every holder of its side, so one it does not is a party
    /// that holds no row there, which cannot leave it.
    pub(crate) fn held(&self, party: PartyId) -> u64 {
        let Some(held) = self.index.get(&party).and_then(|(ci, i)| self.classes.get(*ci).and_then(|c| c.held.get(*i)))
        else {
            violation!(clause = "REP.23", "members leaving a side its tally holds no row of", party = party.get());
        };
        *held
    }

    /// A holder's members moved by a change made beside the tally's draws, so it stays the side it reads.
    pub(crate) fn adjust(&mut self, party: PartyId, delta: i64) {
        if delta == 0 {
            return;
        }
        let Some((c, at)) = self.index.get(&party).and_then(|(ci, i)| self.classes.get_mut(*ci).map(|c| (c, *i)))
        else {
            violation!(clause = "REP.23", "a tally moving a holder it does not hold", party = party.get());
        };
        c.adjust(at, delta);
    }

    /// The members left to draw.
    pub(crate) fn left(&self) -> u64 {
        self.classes.iter().map(|c| c.left).sum()
    }

    /// One member drawn from those left in holders whose unit is at most `most`, by the position the draws give among
    /// them in unit and holder order, and its holder's unit taken; the holder and the members taken.
    fn draw(&mut self, most: u32, draws: &mut Draws) -> Option<(PartyId, u32)> {
        let eligible: u64 = self.classes.iter().filter(|c| c.unit <= most).map(|c| c.left).sum();
        if eligible == 0 {
            return None;
        }
        let mut target = below_u64(draws, eligible);
        for c in self.classes.iter_mut().filter(|c| c.unit <= most) {
            if target < c.left {
                let at = c.find(target);
                c.take(at);
                return c.parties.get(at).map(|p| (*p, c.unit));
            }
            target -= c.left;
        }
        None
    }

    fn taken(&self, party: PartyId) -> u32 {
        self.index
            .get(&party)
            .and_then(|(ci, i)| self.classes.get(*ci).and_then(|c| c.taken.get(*i)))
            .copied()
            .unwrap_or(0)
    }

    fn all_taken(&self) -> impl Iterator<Item = (PartyId, u32)> + '_ {
        let mut all: Vec<(PartyId, u32)> = self
            .classes
            .iter()
            .flat_map(|c| c.parties.iter().zip(&c.taken).filter(|(_, n)| **n > 0).map(|(p, n)| (*p, *n)))
            .collect();
        all.sort_unstable();
        all.into_iter()
    }

    /// One member drawn among the holders of one unit, and its holder's unit taken.
    fn draw_like(&mut self, unit: u32, draws: &mut Draws) -> Option<(PartyId, u32)> {
        let c = self.classes.iter_mut().find(|c| c.unit == unit && c.left >= u64::from(unit))?;
        let at = c.find(below_u64(draws, c.left));
        c.take(at);
        c.parties.get(at).map(|p| (*p, c.unit))
    }

    /// Up to `count` members drawn from those left, like with like: among the holders of the leaving party's unit
    /// `like` while they hold what remains, else among the holders whose unit fits in what remains; each holder drawn
    /// from, with how many, in holder order, and what remains once no holder's unit fits.
    pub(crate) fn draw_many(&mut self, count: u32, like: u32, draws: &mut Draws) -> (Vec<(PartyId, u32)>, u32) {
        let mut more: BTreeMap<PartyId, u32> = BTreeMap::new();
        let mut remaining = count;
        while remaining > 0 {
            let alike = if remaining >= like { self.draw_like(like, draws) } else { None };
            let Some((p, k)) = alike.or_else(|| self.draw(remaining, draws)) else { break };
            *more.entry(p).or_insert(0) += k;
            remaining -= k;
        }
        (more.into_iter().collect(), remaining)
    }
}

/// The members drawn to lose on a cleared line whose payers failed: one member at a time from the claimant rows, each
/// by the members it has left, its holder losing its whole unit, in one sequence from the line's stream, so the first
/// drawn are the same members whatever the failures reach; the drawn may pass the failed by less than a unit.
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

    /// The claimant rows by party with their counts and units, none drawn yet.
    #[must_use]
    pub fn new(claimants: &[(PartyId, u32, u32)], draws: Draws) -> Losers {
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

    /// The sequence drawn on until at least `failed` members are drawn; each claimant newly drawn, with how many more.
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
            let Some((p, k)) = self.tally.draw(u32::MAX, &mut self.draws) else {
                violation!(clause = "REP.31", "a cleared line's claimants drawn out", failed = failed);
            };
            *more.entry(p).or_insert(0) += k;
            self.drawn += u64::from(k);
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

    use super::Losers;

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(7), "REP.cleared"), Subject::new(SubjectTag::Line, 3), 11, 0)
    }

    fn claimants() -> Vec<(PartyId, u32, u32)> {
        [(1, 4), (2, 1), (3, 0), (4, 9), (5, 2)].map(|(p, c)| (PartyId::new(p), c, 1)).to_vec()
    }

    #[test]
    fn the_first_drawn_are_the_same_whatever_the_count() {
        let mut stepped = Losers::new(&claimants(), draws());
        for n in [1, 3, 3, 7, 12] {
            let _ = stepped.draw_to(n);
        }
        let mut once = Losers::new(&claimants(), draws());
        let _ = once.draw_to(12);
        let lost = |l: &Losers| claimants().iter().map(|(p, _, _)| l.lost(*p)).collect::<Vec<_>>();
        assert_eq!(lost(&stepped), lost(&once));
    }

    #[test]
    fn the_drawn_sum_to_the_failed_within_each_row() {
        let mut l = Losers::new(&claimants(), draws());
        let more = l.draw_to(10);
        assert_eq!(more.iter().map(|(_, k)| u64::from(*k)).sum::<u64>(), 10);
        for (p, c, _) in claimants() {
            assert!(l.lost(p) <= c, "a row loses no more members than it holds");
        }
        assert_eq!(l.lost(PartyId::new(3)), 0, "a row of no members is never drawn");
        let _ = l.draw_to(16);
        for (p, c, _) in claimants() {
            assert_eq!(l.lost(p), c, "every member drawn once all failed");
        }
    }

    #[test]
    fn each_member_is_as_likely_to_lose() {
        let rows: Vec<(PartyId, u32, u32)> = [(1, 1), (2, 3)].map(|(p, c)| (PartyId::new(p), c, 1)).to_vec();
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

    #[test]
    fn an_agent_loses_its_whole_unit() {
        let rows: Vec<(PartyId, u32, u32)> = [(1, 3, 1), (2, 12, 4)].map(|(p, c, u)| (PartyId::new(p), c, u)).to_vec();
        for t in 0..64_u64 {
            let d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut l = Losers::new(&rows, d);
            let _ = l.draw_to(5);
            assert_eq!(l.lost(PartyId::new(2)) % 4, 0, "an agent's twins lose alike");
            assert!(l.drawn() >= 5 && l.drawn() < 5 + 4, "passes the failed by less than a unit");
        }
    }

    /// The player's one member and its donor's 169 leave a line whose other side holds a seated counterpart's one
    /// and 169 beside agents of 170: each leaving finds its like, whatever the draws.
    #[test]
    fn leaving_draws_like_with_like() {
        let rows: Vec<(PartyId, u32, u32)> =
            [(1, 2, 1), (2, 338, 169), (3, 3400, 170)].map(|(p, c, u)| (PartyId::new(p), c, u)).to_vec();
        for t in 0..64_u64 {
            let mut d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut tally = super::Tally::new(&rows);
            assert_eq!(tally.draw_many(1, 1, &mut d), (vec![(PartyId::new(1), 1)], 0), "the player's like");
            assert_eq!(tally.draw_many(169, 169, &mut d), (vec![(PartyId::new(2), 169)], 0), "the donor's like");
            assert_eq!(tally.draw_many(340, 170, &mut d), (vec![(PartyId::new(3), 340)], 0), "an agent's like");
        }
    }

    #[test]
    fn leaving_takes_whole_units_to_the_count() {
        let rows: Vec<(PartyId, u32, u32)> =
            [(1, 8, 1), (2, 8, 4), (3, 4, 4)].map(|(p, c, u)| (PartyId::new(p), c, u)).to_vec();
        for t in 0..64_u64 {
            let mut d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut tally = super::Tally::new(&rows);
            let (taken, rest) = tally.draw_many(4, 1, &mut d);
            assert_eq!(rest, 0);
            assert_eq!(taken.iter().map(|(_, k)| *k).sum::<u32>(), 4);
            for (p, k) in taken {
                if p != PartyId::new(1) {
                    assert_eq!(k % 4, 0, "an agent gives its whole unit");
                }
            }
        }
    }

    /// A party of one leaving 509 members against a side of agents of 170 alone: two agents' units leave with it,
    /// and the 169 no unit fits remain.
    #[test]
    fn what_no_unit_fits_remains() {
        let rows: Vec<(PartyId, u32, u32)> =
            [(1, 1700, 170), (2, 3400, 170)].map(|(p, c, u)| (PartyId::new(p), c, u)).to_vec();
        for t in 0..64_u64 {
            let mut d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut tally = super::Tally::new(&rows);
            let (taken, rest) = tally.draw_many(509, 1, &mut d);
            assert_eq!(taken.iter().map(|(_, k)| *k).sum::<u32>(), 340);
            assert_eq!(rest, 169);
        }
    }

    /// A holder whose members left beside the draws is drawn no more, and one given members is drawn by them.
    #[test]
    fn an_adjusted_holder_is_drawn_by_what_it_holds() {
        let rows: Vec<(PartyId, u32, u32)> = [(1, 5, 1), (2, 5, 1)].map(|(p, c, u)| (PartyId::new(p), c, u)).to_vec();
        for t in 0..64_u64 {
            let mut d = Draws::new(stream_key(Seed::new(t), "REP.cleared"), Subject::new(SubjectTag::Line, 1), 1, 0);
            let mut tally = super::Tally::new(&rows);
            tally.adjust(PartyId::new(1), -5);
            tally.adjust(PartyId::new(2), 3);
            assert_eq!(tally.held(PartyId::new(2)), 8);
            let (taken, rest) = tally.draw_many(8, 1, &mut d);
            assert_eq!((taken, rest), (vec![(PartyId::new(2), 8)], 0));
        }
    }
}
