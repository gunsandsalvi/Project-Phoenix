//! The opening's households formed from a region's persons: the pool of every person the region holds by single age
//! and sex, and households drawn from it without replacement until it is empty, so the persons' ages are the pool's
//! and the households are what those persons make.

use if_pop::{FEMALE, MALE};
use phx_num::violation;
use phx_rand::{AliasTable, Draws, below_u64, open_unit};

use crate::consts::{CHILDREN_COLUMN, MEMBER_COLUMNS, OLDER, OTHER, PARTNER_COLUMN};

/// Outcomes drawn with probability proportional to their weights.
#[derive(Debug)]
pub(crate) struct Pick<T> {
    table: AliasTable,
    outcomes: Vec<T>,
}

impl<T: Copy> Pick<T> {
    pub(crate) fn new(weighted: Vec<(T, f64)>) -> Pick<T> {
        if weighted.iter().all(|(_, w)| *w <= 0.0) {
            violation!(clause = "GEN.2", "a household draw with nothing to draw from");
        }
        let weights: Vec<f64> = weighted.iter().map(|(_, w)| *w).collect();
        Pick { table: AliasTable::new(&weights), outcomes: weighted.into_iter().map(|(t, _)| t).collect() }
    }

    pub(crate) fn draw(&self, d: &mut Draws) -> T {
        let Some(t) = self.outcomes.get(self.table.draw(d)) else {
            violation!(clause = "CHN.2", "an alias draw beyond its outcomes");
        };
        *t
    }
}

/// A total apportioned in proportion to weights by largest remainder, ties by lot: each part the whole of its exact
/// share, and one more for each of the largest remainders until the total is given.
pub(crate) fn apportion(total: u64, weights: &[f64], lot: &mut Draws) -> Vec<u64> {
    let sum: f64 = weights.iter().sum();
    if weights.iter().any(|w| w.is_nan() || *w < 0.0) || sum.is_nan() || sum <= 0.0 {
        violation!(clause = "GEN.4", "persons apportioned over weights that are no shares", total = total);
    }
    let mut parts = Vec::with_capacity(weights.len());
    let mut order: Vec<(f64, u64, usize)> = Vec::with_capacity(weights.len());
    for (i, w) in weights.iter().enumerate() {
        let exact = phx_rand::float::from_u64(total) * w / sum;
        let Some(whole) = phx_rand::float::floor_to_u64(exact) else {
            violation!(clause = "GEN.4", "an apportioned part that is no count", total = total);
        };
        parts.push(whole);
        order.push((exact - phx_rand::float::from_u64(whole), lot.next_u64(), i));
    }
    let given: u64 = parts.iter().sum();
    let Some(left) = total.checked_sub(given).and_then(|l| usize::try_from(l).ok()) else {
        violation!(clause = "GEN.4", "parts apportioned beyond their total", total = total, given = given);
    };
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, _, i) in order.into_iter().take(left) {
        if let Some(p) = parts.get_mut(i) {
            *p += 1;
        }
    }
    parts
}

/// Where a person stands in its household.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    Head,
    Partner,
    Adult,
    Child,
}

/// A person as its household is formed: its place, its age in whole years at the snapshot, and its sex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Member {
    pub place: Place,
    pub age: u32,
    pub sex: u32,
}

/// Counts by single age, drawn from by a cumulative tree so a draw over any span of ages costs its logarithm.
#[derive(Clone, Debug)]
struct Ages {
    counts: Vec<u64>,
    tree: Vec<u64>,
}

impl Ages {
    fn of(counts: Vec<u64>) -> Ages {
        let mut tree = vec![0_u64; counts.len() + 1];
        for (i, n) in counts.iter().enumerate() {
            let mut at = i + 1;
            while let Some(t) = tree.get_mut(at) {
                *t += n;
                at += at.isolate_lowest_one();
            }
        }
        Ages { counts, tree }
    }

    /// Persons below an age.
    fn below(&self, age: usize) -> u64 {
        if age > self.counts.len() {
            violation!(clause = "GEN.2", "persons counted to an age beyond the pool's", age = age);
        }
        let (mut at, mut sum) = (age, 0_u64);
        while at > 0 {
            sum += self.tree.get(at).copied().unwrap_or(0);
            at &= at - 1;
        }
        sum
    }

    fn within(&self, lo: usize, hi: usize) -> u64 {
        self.below(hi) - self.below(lo)
    }

    /// The age of the person at a place in the order of ages.
    fn at(&self, mut place: u64) -> usize {
        let (mut at, mut step) = (0_usize, self.tree.len().next_power_of_two());
        while step > 0 {
            if let Some(t) = self.tree.get(at + step).copied()
                && t <= place
            {
                at += step;
                place -= t;
            }
            step /= 2;
        }
        at
    }

    fn remove(&mut self, age: usize) {
        let Some(n) = self.counts.get_mut(age) else {
            violation!(clause = "GEN.2", "a person taken at an age the pool does not count", age = age);
        };
        let Some(left) = n.checked_sub(1) else {
            violation!(clause = "GEN.2", "a person taken from an age the pool holds none of", age = age);
        };
        *n = left;
        let mut at = age + 1;
        while let Some(t) = self.tree.get_mut(at) {
            *t -= 1;
            at += at.isolate_lowest_one();
        }
    }
}

const SEXES: [u32; 2] = [FEMALE, MALE];

/// A region's persons not yet in a household, by sex and single age.
#[derive(Clone, Debug)]
pub(crate) struct Pool {
    sexes: [Ages; 2],
}

fn sex_index(sex: u32) -> usize {
    usize::from(sex == MALE)
}

fn wide(age: u32) -> usize {
    phx_rand::float::index(u64::from(age))
}

impl Pool {
    /// The persons of each sex at each single age from nought.
    pub(crate) fn of(women: Vec<u64>, men: Vec<u64>) -> Pool {
        Pool { sexes: [Ages::of(women), Ages::of(men)] }
    }

    fn ages(&self, sex: u32) -> &Ages {
        let Some(a) = self.sexes.get(sex_index(sex)) else {
            violation!(clause = "POP.1", "a sex the pool does not hold")
        };
        a
    }

    /// The persons aged from `lo` to below `hi`, of either sex.
    pub(crate) fn within(&self, lo: u32, hi: u32) -> u64 {
        SEXES.iter().map(|s| self.ages(*s).within(wide(lo), wide(hi))).sum()
    }

    fn take_at(&mut self, sex: u32, age: usize) {
        let Some(a) = self.sexes.get_mut(sex_index(sex)) else {
            violation!(clause = "POP.1", "a sex the pool does not hold");
        };
        a.remove(age);
    }

    /// A person aged from `lo` to below `hi` of one of `sexes`, each such person alike, taken from the pool; none when
    /// it holds none.
    pub(crate) fn take(&mut self, lo: u32, hi: u32, sexes: &[u32], d: &mut Draws) -> Option<(u32, u32)> {
        let (lo, hi) = (wide(lo), wide(hi));
        let totals: Vec<u64> = sexes.iter().map(|s| self.ages(*s).within(lo, hi)).collect();
        let total: u64 = totals.iter().sum();
        if total == 0 {
            return None;
        }
        let mut place = below_u64(d, total);
        for (sex, n) in sexes.iter().zip(totals) {
            if place < n {
                let age = self.ages(*sex).at(self.ages(*sex).below(lo) + place);
                self.take_at(*sex, age);
                let Ok(age) = u32::try_from(age) else { violation!(clause = "POP.1", "an age beyond counting") };
                return Some((age, *sex));
            }
            place -= n;
        }
        violation!(clause = "CHN.2", "a draw beyond the persons it was drawn over")
    }

    /// A person of one sex aged from `lo` to below `hi`, drawn in proportion to the persons of each age and the
    /// weight of that age, taken from the pool; none when no age has both.
    pub(crate) fn take_weighted(
        &mut self,
        sex: u32,
        (lo, hi): (u32, u32),
        weight: impl Fn(u32) -> f64,
        d: &mut Draws,
    ) -> Option<u32> {
        let ages = self.ages(sex);
        let Some(counts) = ages.counts.get(wide(lo)..wide(hi)) else {
            violation!(clause = "GEN.2", "persons drawn over ages beyond the pool's", age = hi);
        };
        let weights: Vec<(u32, f64)> = (lo..hi)
            .zip(counts)
            .map(|(a, n)| (a, phx_rand::float::from_u64(*n) * weight(a)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        let total: f64 = weights.iter().map(|(_, w)| w).sum();
        if weights.is_empty() {
            return None;
        }
        let mut place = open_unit(d) * total;
        let mut chosen = weights.last().map(|(a, _)| *a);
        for (a, w) in &weights {
            if place < *w {
                chosen = Some(*a);
                break;
            }
            place -= w;
        }
        let age = chosen?;
        self.take_at(sex, wide(age));
        Some(age)
    }
}

/// Whom a household type holds besides its head: a partner, children, an older relative, an adult who is no
/// relative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Type {
    pub index: usize,
    /// Whether it holds each member, by the members table's columns.
    pub holds: [bool; MEMBER_COLUMNS],
}

impl Type {
    fn has(self, column: usize) -> bool {
        let Some(h) = self.holds.get(column) else {
            violation!(clause = "GEN.2", "a household member beyond the members table's columns", column = column);
        };
        *h
    }
    pub(crate) fn partner(self) -> bool {
        self.has(PARTNER_COLUMN)
    }
    pub(crate) fn children(self) -> bool {
        self.has(CHILDREN_COLUMN)
    }
    pub(crate) fn older(self) -> bool {
        self.has(OLDER)
    }
    pub(crate) fn other(self) -> bool {
        self.has(OTHER)
    }
}

/// What households are formed by: the age of majority, one past the oldest age, the first age of old age; each
/// woman's chance of a living child at each single age under majority by her age from `first_mother`, and at each
/// single age from majority (`grown`); the chance of each whole year of a partner's age above the woman's from
/// `first_gap`; every type by its share, the types with children and those without by theirs, and the types of one
/// person and of a couple only.
#[derive(Debug)]
pub(crate) struct Rules {
    pub majority: u32,
    pub ages: u32,
    pub old_age: u32,
    pub first_mother: u32,
    pub chances: Vec<Vec<f64>>,
    pub grown: Vec<Vec<f64>>,
    pub first_gap: i64,
    pub gaps: Vec<f64>,
    pub all: Pick<Type>,
    pub with_children: Pick<Type>,
    pub without: Pick<Type>,
    pub alone: Type,
    pub pair: Type,
}

impl Rules {
    fn chance(&self, mother: u32, child: u32) -> f64 {
        let row = mother.checked_sub(self.first_mother).and_then(|r| self.chances.get(wide(r)));
        let Some(c) = row.and_then(|r| r.get(wide(child))) else {
            violation!(clause = "GEN.2", "a mother's chance of a child the kin table does not hold", age = mother);
        };
        *c
    }

    /// A mother's chance of a living child at an age from majority; none past the ages her row holds.
    fn grown_chance(&self, mother: u32, child: u32) -> f64 {
        let row = mother.checked_sub(self.first_mother).and_then(|r| self.grown.get(wide(r)));
        let at = child.checked_sub(self.majority);
        match (row, at) {
            (Some(r), Some(k)) => r.get(wide(k)).copied().unwrap_or(0.0),
            _ => violation!(
                clause = "GEN.2",
                "a mother's chance of a grown child the kin table does not hold",
                age = mother
            ),
        }
    }

    /// A mother's expected living children from majority.
    fn grown_expected(&self, mother: u32) -> f64 {
        mother.checked_sub(self.first_mother).and_then(|r| self.grown.get(wide(r))).map_or(0.0, |r| r.iter().sum())
    }

    /// No type of the gap falls outside the years it spans, so none of its chance lies there.
    fn gap(&self, woman: u32, man: u32) -> f64 {
        let at = usize::try_from(i64::from(man) - i64::from(woman) - self.first_gap).ok();
        at.and_then(|i| self.gaps.get(i)).copied().unwrap_or(0.0)
    }

    /// The ages of the women the chances hold, which lie within the pool's adults.
    fn mothers(&self) -> (u32, u32) {
        let end = u32::try_from(self.chances.len()).ok().and_then(|n| self.first_mother.checked_add(n));
        match end {
            Some(end) if self.first_mother >= self.majority && end <= self.ages => (self.first_mother, end),
            _ => violation!(clause = "GEN.2", "mothers' ages outside the adults the pool holds"),
        }
    }
}

/// What formed a household: its type as drawn, and whether a child in it was raised by an adult who is not its
/// mother, no woman left in the pool having been able to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Formed {
    pub kind: Type,
    pub raised: bool,
}

/// A woman and a man of the pool at the partner gap, the elder heading; the woman alone when no man is left at a gap
/// the chances hold.
fn couple(pool: &mut Pool, rules: &Rules, woman: u32, d: &mut Draws, out: &mut Vec<Member>) {
    match pool.take_weighted(MALE, (rules.majority, rules.ages), |a| rules.gap(woman, a), d) {
        Some(man) if man > woman => {
            out.push(Member { place: Place::Head, age: man, sex: MALE });
            out.push(Member { place: Place::Partner, age: woman, sex: FEMALE });
        }
        Some(man) => {
            out.push(Member { place: Place::Head, age: woman, sex: FEMALE });
            out.push(Member { place: Place::Partner, age: man, sex: MALE });
        }
        None => out.push(Member { place: Place::Head, age: woman, sex: FEMALE }),
    }
}

/// The type's relatives and other adults, while the pool holds them.
fn extras(pool: &mut Pool, rules: &Rules, t: Type, d: &mut Draws, out: &mut Vec<Member>) {
    if t.older()
        && let Some((age, sex)) = pool.take(rules.old_age, rules.ages, &SEXES, d)
    {
        out.push(Member { place: Place::Adult, age, sex });
    }
    if t.other()
        && let Some((age, sex)) = pool.take(rules.majority, rules.ages, &SEXES, d)
    {
        out.push(Member { place: Place::Adult, age, sex });
    }
}

/// A family: a child of the pool, each alike; its mother by her age's women and their chance of a child of its age;
/// her other children with her chance of each, while the pool holds one; then the type's partner and extras. A child
/// no woman left can have mothered is raised by another adult of the pool.
fn family(pool: &mut Pool, rules: &Rules, d: &mut Draws, out: &mut Vec<Member>) -> Formed {
    let Some((child, sex)) = pool.take(0, rules.majority, &SEXES, d) else {
        violation!(clause = "GEN.2", "a family formed from a pool with no child");
    };
    let t = rules.with_children.draw(d);
    let Some(mother) = pool.take_weighted(FEMALE, rules.mothers(), |m| rules.chance(m, child), d) else {
        let Some((age, s)) = pool.take(rules.majority, rules.ages, &SEXES, d) else {
            violation!(clause = "GEN.2", "a child with no adult left in its region to raise it", age = child);
        };
        out.push(Member { place: Place::Head, age, sex: s });
        out.push(Member { place: Place::Child, age: child, sex });
        return Formed { kind: t, raised: true };
    };
    if t.partner() {
        couple(pool, rules, mother, d, out);
    } else {
        out.push(Member { place: Place::Head, age: mother, sex: FEMALE });
    }
    out.push(Member { place: Place::Child, age: child, sex });
    for k in (0..rules.majority).filter(|k| *k != child) {
        if open_unit(d) < rules.chance(mother, k)
            && let Some((age, s)) = pool.take(k, k + 1, &SEXES, d)
        {
            out.push(Member { place: Place::Child, age, sex: s });
        }
    }
    extras(pool, rules, t, d, out);
    Formed { kind: t, raised: false }
}

/// A family of grown children: a mother by her age's women and her expected living children from majority, the
/// type's partner, each of her grown children the pool still holds with her chance of a living child of its age,
/// then the type's extras; a mother none of whose grown children is left heads the household of her type without
/// children. None when no woman left can have grown children.
fn grown_family(pool: &mut Pool, rules: &Rules, t: Type, d: &mut Draws, out: &mut Vec<Member>) -> Option<Formed> {
    let mother = pool.take_weighted(FEMALE, rules.mothers(), |m| rules.grown_expected(m), d)?;
    let mut children = Vec::new();
    for k in rules.majority..rules.ages {
        if open_unit(d) < rules.grown_chance(mother, k)
            && let Some((age, sex)) = pool.take(k, k + 1, &SEXES, d)
        {
            children.push(Member { place: Place::Adult, age, sex });
        }
    }
    if t.partner() {
        couple(pool, rules, mother, d, out);
    } else {
        out.push(Member { place: Place::Head, age: mother, sex: FEMALE });
    }
    if children.is_empty() {
        return Some(Formed { kind: if t.partner() { rules.pair } else { rules.alone }, raised: false });
    }
    out.extend(children);
    extras(pool, rules, t, d, out);
    Some(Formed { kind: t, raised: false })
}

/// A household of adults: its type among all by their shares — one with children formed around grown children, and
/// among those without when no woman left can have any — its head a woman at the partner gap from a man when the type
/// holds a partner, else any adult, and its extras.
fn adults(pool: &mut Pool, rules: &Rules, d: &mut Draws, out: &mut Vec<Member>) -> Formed {
    let mut t = rules.all.draw(d);
    if t.children() {
        if let Some(formed) = grown_family(pool, rules, t, d, out) {
            return formed;
        }
        t = rules.without.draw(d);
    }
    let woman = if t.partner() { pool.take(rules.majority, rules.ages, &[FEMALE], d) } else { None };
    if let Some((w, _)) = woman {
        couple(pool, rules, w, d, out);
    } else {
        let Some((age, sex)) = pool.take(rules.majority, rules.ages, &SEXES, d) else {
            violation!(clause = "GEN.2", "a household of adults formed from a pool with none");
        };
        out.push(Member { place: Place::Head, age, sex });
    }
    extras(pool, rules, t, d, out);
    Formed { kind: t, raised: false }
}

/// The next household of the pool into `out`: a family while the pool holds a child, then households of adults;
/// none once the pool is empty. Every person it takes leaves the pool.
pub(crate) fn household(pool: &mut Pool, rules: &Rules, d: &mut Draws, out: &mut Vec<Member>) -> Option<Formed> {
    out.clear();
    if pool.within(0, rules.majority) > 0 {
        Some(family(pool, rules, d, out))
    } else if pool.within(rules.majority, rules.ages) > 0 {
        Some(adults(pool, rules, d, out))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use if_pop::{FEMALE, MALE};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Ages, Pick, Place, Pool, Rules, Type, household};

    fn draws(i: u64) -> Draws {
        Draws::new(stream_key(Seed::new(3), "compose"), Subject::new(SubjectTag::World, i), 0, 0)
    }

    const COUPLE_KIDS: Type = Type { index: 0, holds: [true, true, false, false] };
    const LONE_PARENT: Type = Type { index: 1, holds: [false, true, false, false] };
    const EXTENDED: Type = Type { index: 2, holds: [true, true, true, false] };
    const ONE: Type = Type { index: 3, holds: [false, false, false, false] };
    const COUPLE: Type = Type { index: 4, holds: [true, false, false, false] };
    const SHARED: Type = Type { index: 5, holds: [false, false, false, true] };

    /// Ages to 90, majority at 18, old age at 65; women have a child at each age with a chance of 0.08 while they are
    /// 18 years older, a minor only while they are under 50; partners two years older, give or take two.
    fn rules() -> Rules {
        let chance = |m: u32, k: u32| if m >= k + 18 { 0.08 } else { 0.0 };
        let chances =
            (18..91_u32).map(|m| (0..18_u32).map(|k| if m < 50 { chance(m, k) } else { 0.0 }).collect()).collect();
        let grown = (18..91_u32).map(|m| (18..91_u32).map(|k| chance(m, k)).collect()).collect();
        let types =
            [(COUPLE_KIDS, 0.3), (LONE_PARENT, 0.1), (EXTENDED, 0.1), (ONE, 0.2), (COUPLE, 0.25), (SHARED, 0.05)];
        Rules {
            majority: 18,
            ages: 91,
            old_age: 65,
            first_mother: 18,
            chances,
            grown,
            first_gap: 0,
            gaps: vec![0.1, 0.2, 0.4, 0.2, 0.1],
            all: Pick::new(types.to_vec()),
            with_children: Pick::new(types.iter().copied().filter(|(t, _)| t.children()).collect()),
            without: Pick::new(types.iter().copied().filter(|(t, _)| !t.children()).collect()),
            alone: ONE,
            pair: COUPLE,
        }
    }

    fn pool(people: u64) -> (Pool, Vec<u64>, Vec<u64>) {
        let women: Vec<u64> = (0..91_u64).map(|a| people * (100 - a) / 5000).collect();
        let men: Vec<u64> = (0..91_u64).map(|a| people * (96 - a) / 5000).collect();
        (Pool::of(women.clone(), men.clone()), women, men)
    }

    #[test]
    fn apportioned_parts_sum_to_the_total_and_follow_the_weights() {
        let parts = super::apportion(10, &[1.0, 1.0, 1.0], &mut draws(1));
        assert_eq!(parts.iter().sum::<u64>(), 10);
        assert!(parts.iter().all(|p| *p == 3 || *p == 4));
        assert_eq!(super::apportion(7, &[0.5, 0.25, 0.25, 0.0], &mut draws(2)), [3, 2, 2, 0]);
        assert!(std::panic::catch_unwind(|| super::apportion(3, &[0.0, 0.0], &mut draws(3))).is_err());
    }

    #[test]
    fn the_tree_draws_each_person_once() {
        let mut ages = Ages::of(vec![2, 0, 3, 1]);
        assert_eq!((ages.below(2), ages.within(2, 4), ages.within(0, 4)), (2, 4, 6));
        let placed: Vec<usize> = (0..6).map(|p| ages.at(p)).collect();
        assert_eq!(placed, [0, 0, 2, 2, 2, 3], "each place in the order of ages");
        ages.remove(2);
        assert_eq!((ages.within(2, 3), ages.at(2), ages.at(4)), (2, 2, 3));
    }

    /// Every person of the pool is in exactly one household, at its own age and sex; every household has a head,
    /// at most one partner, and its children under majority; a child's mother is at least eighteen years its elder
    /// unless another adult raised it.
    #[test]
    fn every_person_of_the_pool_is_in_one_household() {
        let rules = rules();
        for seed in 0..4 {
            let (mut pool, women, men) = pool(20_000);
            let (mut got_w, mut got_m) = (vec![0_u64; 91], vec![0_u64; 91]);
            let (mut out, mut d, mut raised) = (Vec::new(), draws(seed), 0);
            while let Some(formed) = household(&mut pool, &rules, &mut d, &mut out) {
                raised += u32::from(formed.raised);
                assert_eq!(out.iter().filter(|m| m.place == Place::Head).count(), 1);
                assert!(out.iter().filter(|m| m.place == Place::Partner).count() <= 1);
                assert!(out.iter().all(|m| (m.place == Place::Child) == (m.age < 18)));
                let head = out.iter().find(|m| m.place == Place::Head).unwrap();
                let mut children: Vec<u32> = out.iter().filter(|m| m.place == Place::Child).map(|m| m.age).collect();
                children.sort_unstable();
                if let Some(child) = children.last()
                    && !formed.raised
                {
                    let mut mothers = out.iter().filter(|m| m.sex == FEMALE && m.place != Place::Child);
                    assert!(mothers.any(|m| m.age >= child + 18), "a mother of age to have her children");
                }
                assert!(head.age >= 18);
                for m in &out {
                    let got = if m.sex == MALE { &mut got_m } else { &mut got_w };
                    got[usize::try_from(m.age).unwrap()] += 1;
                }
            }
            assert_eq!((got_w, got_m), (women, men), "the pool's persons, each once");
            assert!(raised < 50, "a child raised by another adult is rare where women abound: {raised}");
        }
    }

    /// A pool of adults alone forms families around grown children: each grown child at home is at least eighteen
    /// years younger than a woman of its household, and every person of the pool is in one household.
    #[test]
    fn grown_children_live_with_their_mothers() {
        let rules = rules();
        let women: Vec<u64> = (0..91_u64).map(|a| if a >= 18 { 40 } else { 0 }).collect();
        let men = women.clone();
        let mut pool = Pool::of(women, men);
        let (mut out, mut d, mut families, mut persons) = (Vec::new(), draws(11), 0, 0);
        while let Some(formed) = household(&mut pool, &rules, &mut d, &mut out) {
            persons += out.len();
            let grown: Vec<u32> =
                out.iter().filter(|m| m.place == Place::Adult && formed.kind.children()).map(|m| m.age).collect();
            if let Some(youngest) = grown.iter().copied().reduce(|a, b| if b < a { b } else { a }) {
                families += 1;
                assert!(out.iter().any(|m| m.sex == FEMALE && m.age >= youngest + 18), "a mother of age to have them");
            }
            assert!(!formed.kind.children() || out.len() >= 2, "a type with children holds one");
        }
        assert_eq!(persons, 2 * 73 * 40);
        assert!(families > 100, "types with children form around grown children: {families}");
    }

    /// Children who outnumber the women who can have mothered them are raised by other adults, and the pool still
    /// ends empty.
    #[test]
    fn children_without_mothers_are_raised_by_adults() {
        let rules = rules();
        let mut women = vec![0_u64; 91];
        let mut men = vec![0_u64; 91];
        women[5] = 20;
        men[6] = 20;
        women[70] = 30;
        men[30] = 20;
        women[25] = 5;
        let mut pool = Pool::of(women, men);
        let (mut out, mut d, mut raised, mut persons) = (Vec::new(), draws(9), 0, 0);
        while let Some(formed) = household(&mut pool, &rules, &mut d, &mut out) {
            raised += u32::from(formed.raised);
            persons += out.len();
        }
        assert_eq!(persons, 95);
        assert!(raised >= 30, "few women of age, many children: {raised}");
        let mut orphans = Pool::of(vec![3, 0, 0], vec![0, 0, 0]);
        let three = Rules { majority: 1, ages: 3, ..rules };
        assert!(
            std::panic::catch_unwind(move || household(&mut orphans, &three, &mut draws(1), &mut Vec::new())).is_err()
        );
    }

    /// A partner's age above the woman's follows the gap's chances.
    #[test]
    fn partners_take_the_gap() {
        let rules = rules();
        let mut gaps = [0_u32; 5];
        for i in 0..2_000 {
            let mut women = vec![0_u64; 91];
            women[30] = 1;
            let men: Vec<u64> = (0..91).map(|a| if (30..35).contains(&a) { 100 } else { 0 }).collect();
            let mut pool = Pool::of(women, men);
            let mut out = Vec::new();
            super::couple(&mut pool, &rules, 30, &mut draws(i), &mut out);
            let man = out.iter().find(|m| m.sex == MALE).unwrap();
            gaps[usize::try_from(man.age - 30).unwrap()] += 1;
        }
        let expected = [200, 400, 800, 400, 200];
        for (g, e) in gaps.iter().zip(expected) {
            assert!(g.abs_diff(e) < 90, "gap counts {gaps:?} against {expected:?}");
        }
    }
}
