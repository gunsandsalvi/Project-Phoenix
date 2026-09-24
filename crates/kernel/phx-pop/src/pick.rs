use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Draws, Fenwick, below_u64, pick_without_replacement};

use crate::profile::{Profile, ProfileLayout};

/// One member drawn uniformly from categories holding `counts`, as its category's index: one uniform over the members,
/// found by a scan, which is a multivariate hypergeometric draw of one.
#[clause("CHN.7")]
#[must_use]
pub fn one_of(d: &mut Draws, counts: &[u64]) -> usize {
    let Some(total) = counts.iter().try_fold(0_u64, |t, c| t.checked_add(*c)) else {
        violation!(clause = "Law 7", "a population overflows");
    };
    if total == 0 {
        violation!(clause = "CHN.2", "one drawn from none");
    }
    let mut rest = below_u64(d, total);
    for (i, c) in counts.iter().enumerate() {
        if rest < *c {
            return i;
        }
        rest -= c;
    }
    violation!(clause = "CHN.2", "a draw beyond its categories", total = total)
}

/// The joint values the members a hit reaches hold in each group of their role: in the hazard's own group they are
/// the values the hit was drawn at; in the role's other groups, which are counted apart from it, they are picked
/// without replacement from each group's counts. Values within a group stay joint, since a group's value is one
/// code of all its components.
#[clause("REP.32", "REP.7")]
#[must_use]
pub fn pick_hit(
    d: &mut Draws,
    profile: &Profile,
    layout: &ProfileLayout,
    group: usize,
    hits: &[(u32, u64)],
) -> Vec<Vec<(u32, u64)>> {
    let Some(shape) = layout.groups.get(group) else {
        violation!(clause = "REP.32", "a hit on a group the kind does not declare", group = group);
    };
    let total: u64 = hits.iter().map(|(_, n)| n).sum();
    let mut out = Vec::with_capacity(layout.groups.len());
    for (g, s) in layout.groups.iter().enumerate() {
        if s.role != shape.role {
            out.push(Vec::new());
        } else if g == group {
            out.push(hits.to_vec());
        } else {
            let held = profile.held(g);
            let counts: Vec<u64> = held.iter().map(|(_, n)| u64::from(*n)).collect();
            let mut picked = vec![0_u64; counts.len()];
            pick_without_replacement(d, &counts, total, &mut picked);
            out.push(held.iter().zip(picked).filter(|(_, k)| *k > 0).map(|((v, _), k)| (*v, k)).collect());
        }
    }
    out
}

/// A victim drawn from weighted candidates — holders of a damaged class by their units, pieces of a zone by their
/// members — each in proportion to its weight.
#[clause("REP.7")]
#[must_use]
pub fn weighted(d: &mut Draws, weights: &[u64]) -> usize {
    let total: u64 = weights.iter().sum();
    if total == 0 {
        violation!(clause = "REP.7", "a victim drawn from candidates of no weight");
    }
    Fenwick::new(weights).find(below_u64(d, total))
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{pick_hit, weighted};
    use crate::profile::{GroupShape, Profile, ProfileLayout};

    fn draws(i: u32) -> Draws {
        Draws::new(stream_key(Seed::new(5), "picks"), Subject::new(SubjectTag::Party, 3), i, 0)
    }

    #[test]
    fn picks_joint_within_group() {
        let layout = ProfileLayout {
            groups: vec![
                GroupShape { role: 0, values: 6, dense: true },
                GroupShape { role: 0, values: 4, dense: true },
                GroupShape { role: 1, values: 3, dense: true },
            ],
        };
        let mut p = Profile::empty(&layout);
        for (g, v, n) in [(0, 1, 30), (0, 5, 10), (1, 0, 25), (1, 3, 15), (2, 2, 40)] {
            p.add(&layout, g, v, n);
        }
        let hits = [(1_u32, 3_u64), (5, 2)];
        let mut on_three = 0_u64;
        for i in 0..10_000 {
            let picked = pick_hit(&mut draws(i), &p, &layout, 0, &hits);
            assert_eq!(picked[0], hits, "the hit's own group keeps the values it was drawn at");
            assert_eq!(picked[1].iter().map(|(_, k)| k).sum::<u64>(), 5, "every hit member holds a value of the group");
            assert!(picked[1].iter().all(|(v, k)| u64::from(p.count(1, *v)) >= *k));
            assert!(picked[2].is_empty(), "another role's groups are not the hit's");
            on_three += picked[1].iter().filter(|(v, _)| *v == 3).map(|(_, k)| k).sum::<u64>();
        }
        let share = phx_rand::float::from_u64(on_three) / 50_000.0;
        assert!((share - 15.0 / 40.0).abs() < 0.01, "{share}");
    }

    #[test]
    fn victim_draw_weighted_by_counts() {
        let weights = [5_u64, 0, 15, 30];
        let mut drawn = [0_u64; 4];
        for i in 0..50_000 {
            drawn[weighted(&mut draws(i), &weights)] += 1;
        }
        assert_eq!(drawn[1], 0, "a holder of no units is never drawn");
        for (d, w) in drawn.iter().zip(weights) {
            let expected = 50_000.0 * phx_rand::float::from_u64(w) / 50.0;
            let tolerance = if expected > 1.0 { 4.0 * expected.sqrt() } else { 4.0 };
            assert!((phx_rand::float::from_u64(*d) - expected).abs() < tolerance, "{drawn:?}");
        }
        assert!(std::panic::catch_unwind(|| weighted(&mut draws(0), &[0, 0])).is_err());
    }
}
