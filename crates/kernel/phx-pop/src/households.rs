//! The persons a process reached in a cell, grouped into the households they belong to.

use std::collections::BTreeMap;

use phx_core::HouseholdHit;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Draws, below_u64};

/// The persons reached in a cell of `members` households of `per_member` persons each of the role, by joint value,
/// grouped into households. Each person in turn falls in a household not yet reached with chance proportional to the
/// persons those hold, or in one already reached with chance proportional to its persons not yet reached: drawing
/// persons without replacement, which is how the hits were drawn. Households that lost the same persons form one group,
/// in the order of their values.
#[clause("REP.26", "REP.14")]
#[must_use]
pub fn households_hit(d: &mut Draws, members: u64, per_member: u64, by_value: &[(u32, u64)]) -> Vec<HouseholdHit> {
    let reached: u64 = by_value.iter().map(|(_, n)| n).sum();
    if members.checked_mul(per_member).is_none_or(|all| reached > all) {
        violation!(clause = "REP.14", "more persons reached than a cell holds", reached = reached);
    }
    let mut households: Vec<Vec<u32>> = Vec::new();
    for (v, n) in by_value {
        for _ in 0..*n {
            if per_member == 1 {
                households.push(vec![*v]);
                continue;
            }
            let fresh = (members - phx_rand::float::len_u64(households.len())) * per_member;
            let taken: u64 =
                households.iter().map(|h| per_member - phx_rand::float::len_u64(h.len())).sum::<u64>() + fresh;
            let mut at = below_u64(d, taken);
            if at < fresh {
                households.push(vec![*v]);
                continue;
            }
            at -= fresh;
            let found = households.iter_mut().find(|h| {
                let left = per_member - phx_rand::float::len_u64(h.len());
                if at < left {
                    return true;
                }
                at -= left;
                false
            });
            let Some(h) = found else {
                violation!(clause = "REP.14", "a person reached in no household", value = *v);
            };
            h.push(*v);
        }
    }
    let mut groups: BTreeMap<Vec<(u32, u32)>, u64> = BTreeMap::new();
    for mut h in households {
        h.sort_unstable();
        let mut persons: Vec<(u32, u32)> = Vec::new();
        for v in h {
            match persons.last_mut() {
                Some((last, n)) if *last == v => *n += 1,
                _ => persons.push((v, 1)),
            }
        }
        *groups.entry(persons).or_default() += 1;
    }
    groups.into_iter().map(|(persons, households)| HouseholdHit { households, persons }).collect()
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::households_hit;

    fn draws(n: u64) -> Draws {
        Draws::new(stream_key(Seed::new(n), "households"), Subject::new(SubjectTag::World, n), 0, 0)
    }

    #[test]
    fn one_person_each_is_one_household_each() {
        let got = households_hit(&mut draws(1), 10, 1, &[(3, 2), (5, 1)]);
        let flat: Vec<(u64, Vec<(u32, u32)>)> = got.into_iter().map(|h| (h.households, h.persons)).collect();
        assert_eq!(flat, vec![(2, vec![(3, 1)]), (1, vec![(5, 1)])]);
    }

    #[test]
    fn every_person_is_placed_and_no_household_overfills() {
        for seed in 0..200 {
            let got = households_hit(&mut draws(seed), 3, 2, &[(1, 3), (2, 2)]);
            let persons: u64 =
                got.iter().map(|h| h.households * h.persons.iter().map(|(_, n)| u64::from(*n)).sum::<u64>()).sum();
            assert_eq!(persons, 5, "every person reached is in a household");
            assert!(got.iter().all(|h| h.persons.iter().map(|(_, n)| n).sum::<u32>() <= 2), "none holds more than two");
            assert!(got.iter().map(|h| h.households).sum::<u64>() <= 3, "no more households than the cell has");
        }
    }

    /// Two persons reached among two households of two persons each share a household with chance one in three, as
    /// drawing them without replacement gives.
    #[test]
    fn two_reached_share_a_household_at_the_urns_rate() {
        let (mut shared, trials) = (0_u32, 30_000_u32);
        for seed in 0..u64::from(trials) {
            let got = households_hit(&mut draws(seed), 2, 2, &[(0, 2)]);
            if got.iter().any(|h| h.persons == vec![(0, 2)]) {
                shared += 1;
            }
        }
        let rate = f64::from(shared) / f64::from(trials);
        assert!((rate - 1.0 / 3.0).abs() < 0.012, "shared {rate}");
    }
}
