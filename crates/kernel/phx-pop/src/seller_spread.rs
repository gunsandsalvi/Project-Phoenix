use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_rand::{Draws, below_u64, multinomial, multivariate_hypergeometric};

/// A seller cell's sales since its last review spread over its members: the units each sold, how many units of the
/// purchases none could fill, and how many phases the spread took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spread {
    pub sold: Vec<u64>,
    pub unfilled: u64,
    pub phases: u64,
}

fn len(n: usize) -> u64 {
    phx_rand::float::len_u64(n)
}

fn at(i: u64) -> usize {
    let Ok(i) = usize::try_from(i) else {
        capacity_exceeded!("members of a seller cell", usize::MAX, i);
    };
    i
}

/// The members holding units, and the fewest units any of them holds.
fn active(units: &[u64]) -> (Vec<usize>, u64) {
    let members: Vec<usize> = units.iter().enumerate().filter(|(_, u)| **u > 0).map(|(i, _)| i).collect();
    let fewest = members.iter().filter_map(|i| units.get(*i)).fold(u64::MAX, |f, u| if *u < f { *u } else { f });
    (members, fewest)
}

/// One purchase reaching a member drawn uniformly among those with units left; a member that cannot fill it sells what
/// it holds, and the rest chooses again among the others. Returns the units no member could fill.
fn one_by_lot(d: &mut Draws, units: &mut [u64], sold: &mut [u64], quantity: u64) -> u64 {
    let mut rest = quantity;
    while rest > 0 {
        let (members, _) = active(units);
        if members.is_empty() {
            return rest;
        }
        let Some(m) = members.get(at(below_u64(d, len(members.len())))).copied() else { return rest };
        let (Some(u), Some(s)) = (units.get_mut(m), sold.get_mut(m)) else { return rest };
        let taken = if *u >= rest { rest } else { *u };
        *u -= taken;
        *s += taken;
        rest -= taken;
    }
    0
}

/// The capacity-respecting spread on a seller cell's review day: each purchase — the units a buyer bought in one meeting — reaches a
/// member drawn uniformly from those with units left, as if the purchases arrived one by one in a uniformly random
/// order. It is done in phases, exact in distribution: while the fewest units any active member holds, `u`, is at
/// least the largest purchase left, `q`, a phase takes `⌊u ÷ q⌋` purchases, drawn from the counts per quantity, and
/// deals each quantity's over the active members by an equal-probability multinomial, since none can run out within
/// it; otherwise one purchase, drawn by lot, reaches one member. Members at nought leave before the next phase.
/// `units` are the members' units; `purchases` the counts of purchases per quantity.
#[clause("REP.22")]
#[must_use]
pub fn spread(d: &mut Draws, units: &[u64], purchases: &[(u64, u64)]) -> Spread {
    if purchases.iter().any(|(quantity, count)| *quantity == 0 && *count > 0) {
        violation!(clause = "REP.22", "a purchase of no units");
    }
    let mut left = units.to_vec();
    let mut sold = vec![0_u64; units.len()];
    let mut counts: Vec<u64> = purchases.iter().map(|(_, n)| *n).collect();
    let (mut unfilled, mut phases) = (0_u64, 0_u64);
    loop {
        let remaining: u64 = counts.iter().sum();
        if remaining == 0 {
            break;
        }
        let (members, fewest) = active(&left);
        if members.is_empty() {
            unfilled += purchases.iter().zip(&counts).map(|((quantity, _), count)| quantity * count).sum::<u64>();
            break;
        }
        phases += 1;
        let largest = purchases
            .iter()
            .zip(&counts)
            .filter(|(_, n)| **n > 0)
            .fold(0, |l, ((q, _), _)| if *q > l { *q } else { l });
        if fewest >= largest {
            let take = fewest / largest;
            let take = if take < remaining { take } else { remaining };
            let mut drawn = vec![0_u64; counts.len()];
            multivariate_hypergeometric(d, &counts, take, &mut drawn);
            let even = vec![1.0 / phx_rand::float::from_u64(len(members.len())); members.len()];
            for (j, taken) in drawn.iter().enumerate() {
                let (Some((q, _)), Some(c)) = (purchases.get(j), counts.get_mut(j)) else { continue };
                *c -= taken;
                if *taken == 0 {
                    continue;
                }
                let mut dealt = vec![0_u64; members.len()];
                multinomial(d, *taken, &even, &mut dealt);
                for (m, got) in members.iter().zip(dealt) {
                    let (Some(held), Some(gone)) = (left.get_mut(*m), sold.get_mut(*m)) else { continue };
                    *held -= q * got;
                    *gone += q * got;
                }
            }
        } else {
            let mut one = vec![0_u64; counts.len()];
            multivariate_hypergeometric(d, &counts, 1, &mut one);
            let Some(j) = one.iter().position(|drawn| *drawn == 1) else {
                violation!(clause = "REP.22", "a purchase drawn by lot from none left");
            };
            let (Some((quantity, _)), Some(count)) = (purchases.get(j), counts.get_mut(j)) else { break };
            *count -= 1;
            unfilled += one_by_lot(d, &mut left, &mut sold, *quantity);
        }
    }
    Spread { sold, unfilled, phases }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, below_u64};

    use super::spread;
    use crate::fixture::draws;

    /// The same purchases arriving one by one in a uniformly random order, each reaching a uniform member with units
    /// left, a member that cannot fill one selling what it holds and the rest choosing again.
    fn sequential(d: &mut Draws, units: &[u64], purchases: &[(u64, u64)]) -> Vec<u64> {
        let mut all: Vec<u64> = purchases.iter().flat_map(|(q, n)| (0..*n).map(move |_| *q)).collect();
        for i in (1..all.len()).rev() {
            let j = usize::try_from(below_u64(d, u64::try_from(i + 1).unwrap())).unwrap();
            all.swap(i, j);
        }
        let mut left = units.to_vec();
        let mut sold = vec![0_u64; units.len()];
        for q in all {
            let mut rest = q;
            while rest > 0 {
                let active: Vec<usize> = (0..left.len()).filter(|m| left[*m] > 0).collect();
                if active.is_empty() {
                    break;
                }
                let m = active[usize::try_from(below_u64(d, u64::try_from(active.len()).unwrap())).unwrap()];
                let taken = if left[m] >= rest { rest } else { left[m] };
                left[m] -= taken;
                sold[m] += taken;
                rest -= taken;
            }
        }
        sold
    }

    #[test]
    fn seller_spread_never_exceeds_units() {
        for i in 0..2_000 {
            let mut d = draws("REP.seller_spread", i);
            let units: Vec<u64> = (0..7).map(|_| below_u64(&mut d, 12)).collect();
            let purchases = [(1, below_u64(&mut d, 20)), (3, below_u64(&mut d, 8)), (5, below_u64(&mut d, 4))];
            let s = spread(&mut d, &units, &purchases);
            assert!(s.sold.iter().zip(&units).all(|(s, u)| s <= u), "{units:?} {s:?}");
            let demand: u64 = purchases.iter().map(|(q, n)| q * n).sum();
            assert_eq!(s.sold.iter().sum::<u64>() + s.unfilled, demand, "every unit bought is sold or left unfilled");
            let supply: u64 = units.iter().sum();
            assert!(
                s.unfilled == 0 || s.sold.iter().sum::<u64>() == supply,
                "units are left unfilled only when none remain"
            );
        }
    }

    /// The chi-square statistic of two samples' counts over the same categories.
    fn chi_square(a: &[u64], b: &[u64]) -> f64 {
        let (na, nb) = (a.iter().sum::<u64>(), b.iter().sum::<u64>());
        let f = phx_rand::float::from_u64;
        a.iter()
            .zip(b)
            .filter(|(x, y)| **x + **y > 0)
            .map(|(x, y)| {
                let pooled = f(*x + *y) / f(na + nb);
                let (ea, eb) = (pooled * f(na), pooled * f(nb));
                (f(*x) - ea).powi(2) / ea + (f(*y) - eb).powi(2) / eb
            })
            .sum()
    }

    #[test]
    fn seller_spread_matches_uniform_sequential_purchases() {
        // Enough units for every purchase, and too few, so that members run out.
        for (units, purchases) in [
            (vec![3_u64, 5, 8, 12], vec![(1_u64, 6_u64), (2, 3), (4, 2)]),
            (vec![2, 3, 4], vec![(1, 4), (2, 3), (3, 1)]),
        ] {
            let trials = 20_000;
            let top = usize::try_from(units[0]).unwrap() + 1;
            let (mut phased, mut one_by_one) = (vec![0_u64; top], vec![0_u64; top]);
            let (mut phases_total, mut sold_total) = (0_u64, [0_u64; 2]);
            for i in 0..trials {
                let s = spread(&mut draws("REP.seller_spread", i), &units, &purchases);
                phased[usize::try_from(s.sold[0]).unwrap()] += 1;
                phases_total += s.phases;
                sold_total[0] += s.sold.iter().sum::<u64>();
                let q = sequential(&mut draws("sequential", i), &units, &purchases);
                one_by_one[usize::try_from(q[0]).unwrap()] += 1;
                sold_total[1] += q.iter().sum::<u64>();
            }
            // Degrees of freedom are at most twelve; 40 lies beyond the 0.9999 quantile of any of them.
            let chi = chi_square(&phased, &one_by_one);
            assert!(chi < 40.0, "{chi}: {phased:?} against {one_by_one:?}");
            assert_eq!(sold_total[0], sold_total[1], "the same units sold in all");
            assert!(phases_total < u64::from(trials) * 20);
        }
    }
}
