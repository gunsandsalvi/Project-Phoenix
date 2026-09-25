//! An agent's next hit of a process, drawn ahead: the day's chance of any hit over its persons' own chances, a
//! geometric wait at it until the next day any person's rate may change, and on a hit day the persons it reached.

use phx_id::Day;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Draws, binomials_joint_at_least_one, geometric};

/// What an agent's booking for a process is: a hit on a day, a redraw on the day its rate may change, or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Booking {
    Hit(Day),
    Redraw(Day),
    Never,
}

/// The chance a day holds at least one of an agent's persons' hits, `1 − Π(1 − qᵢ)`, computed without cancellation.
#[clause("REP.7", "CHN.2")]
#[must_use]
pub fn any_hit(qs: &[f64]) -> f64 {
    let mut none = 0.0_f64;
    for q in qs {
        if !(0.0..=1.0).contains(q) {
            violation!(clause = "CHN.2", "a daily chance outside a probability");
        }
        if *q >= 1.0 {
            return 1.0;
        }
        none += libm::log1p(-q);
    }
    -libm::expm1(none)
}

/// The next booking from `from`, the first day that may hold a hit, at a daily chance `p` of any hit that holds until
/// `change`, the first day it may differ: a hit before `change`, or a redraw on it. Days are independent, so a redraw
/// starts afresh with nothing owed from before it.
#[clause("REP.7", "CHN.4")]
pub fn next_booking(d: &mut Draws, p: f64, from: Day, change: Missing<Day>) -> Booking {
    let redraw = match change {
        Missing::Present(c) if c <= from => violation!(clause = "REP.7", "a rate's change on or before its first day"),
        Missing::Present(c) => Booking::Redraw(c),
        Missing::Absent => Booking::Never,
    };
    if p <= 0.0 {
        return redraw;
    }
    let Missing::Present(wait) = geometric(d, p) else { return redraw };
    let hit = u64::from(from.get()).checked_add(wait).and_then(|h| u32::try_from(h).ok());
    match (hit, change) {
        (Some(h), Missing::Present(c)) if h < c.get() => Booking::Hit(Day::new(h)),
        (Some(h), Missing::Absent) => Booking::Hit(Day::new(h)),
        _ => redraw,
    }
}

/// The persons a hit reached, by their places: each by its own chance, conditioned on at least one.
#[clause("REP.7", "CHN.7")]
pub fn reached(d: &mut Draws, qs: &[f64], out: &mut Vec<usize>) {
    out.clear();
    let ones = vec![1_u64; qs.len()];
    let mut hits = vec![0_u64; qs.len()];
    binomials_joint_at_least_one(d, &ones, qs, &mut hits);
    out.extend(hits.iter().enumerate().filter(|(_, h)| **h > 0).map(|(i, _)| i));
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Booking, any_hit, next_booking, reached};

    fn draws(i: u64) -> Draws {
        Draws::new(stream_key(Seed::new(9), "t"), Subject::new(SubjectTag::Party, i), 1, 3)
    }

    #[test]
    fn any_hit_is_one_less_the_chance_of_none() {
        let p = any_hit(&[0.1, 0.2]);
        assert!((p - (1.0 - 0.9 * 0.8)).abs() < 1e-15);
        assert!(any_hit(&[]).abs() < f64::EPSILON);
        assert!((any_hit(&[0.3, 1.0]) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn waits_are_geometric_and_a_change_ends_them() {
        let p = 0.01;
        let (mut total, mut n) = (0_u64, 0_u64);
        for i in 0..20_000 {
            if let Booking::Hit(h) = next_booking(&mut draws(i), p, Day::new(100), Missing::Absent) {
                total += u64::from(h.get() - 100);
                n += 1;
            }
        }
        let mean = phx_rand::float::from_u64(total) / phx_rand::float::from_u64(n);
        assert!((mean - (1.0 - p) / p).abs() < 3.0, "mean wait {mean}");
        let mut redraws = 0;
        for i in 0..20_000 {
            match next_booking(&mut draws(i), p, Day::new(100), Missing::Present(Day::new(110))) {
                Booking::Hit(h) => assert!(h.get() < 110),
                Booking::Redraw(c) => {
                    assert_eq!(c, Day::new(110));
                    redraws += 1;
                }
                Booking::Never => panic!("a change is a redraw"),
            }
        }
        let expected = 20_000.0 * 0.99_f64.powi(10);
        assert!((f64::from(redraws) - expected).abs() < 4.0 * expected.sqrt(), "{redraws} redraws");
        assert_eq!(next_booking(&mut draws(1), 0.0, Day::new(5), Missing::Absent), Booking::Never);
    }

    #[test]
    fn a_hit_reaches_at_least_one_person() {
        let mut out = Vec::new();
        let mut firsts = 0;
        for i in 0..10_000 {
            reached(&mut draws(i), &[0.001, 0.001], &mut out);
            assert!(!out.is_empty() && out.len() <= 2);
            firsts += usize::from(out.contains(&0));
        }
        assert!((4_700..5_300).contains(&firsts), "{firsts} of the hits reach the first person");
    }
}
