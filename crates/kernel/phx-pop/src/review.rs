use libm::expm1;
use phx_core::Weight;
use phx_core::calendar::Calendar;
use phx_core::calendar::period::Period;
use phx_core::consts::SHORTEST_MONTH_DAYS;
use phx_core::schedule::Phase;
use phx_core::schedule::RunsOn;
use phx_id::{CountryId, Day};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_rand::{Draws, below_u32, binomial};

use crate::consts::EXPOSURE_ONE;

/// A cell's phase within one of its review schedules: drawn once from its keyed stream for that schedule, so the
/// same whenever read and never stored, and within the period's shortest instance.
#[clause("TIME.5", "REP.21")]
pub fn phase(d: &mut Draws, period: Period) -> Phase {
    let shortest = if period.month_count() > 0 {
        u32::from(period.month_count()) * u32::from(SHORTEST_MONTH_DAYS)
    } else {
        u32::from(period.day_count())
    };
    if shortest == 0 {
        violation!(clause = "TIME.5", "a review schedule of no days");
    }
    let Some(p) = u16::try_from(below_u32(d, shortest)).ok().and_then(|o| Phase::within(period, o)) else {
        violation!(clause = "TIME.5", "a phase outside its schedule's period", days = shortest);
    };
    p
}

/// The day a surprise bearing on a decision wakes a cell for it: the next day after today its decision point runs,
/// which the agenda books for the cell's review reason.
#[clause("REP.35", "TIME.8")]
pub fn wake_day(calendar: &Calendar, country: CountryId, runs_on: RunsOn, today: Day) -> Day {
    match runs_on {
        RunsOn::Any => today.succ(),
        RunsOn::Business => calendar.on_or_after(country, today.succ()),
    }
}

/// The members' review exposure after `days` more days at a total daily rate, both in fixed point: attention is
/// constant between one visit or publication and the next, so the exposure over a span is its rate times its days.
#[clause("REP.21")]
#[must_use]
pub fn accrued(exposure: i64, per_day: i64, days: u32) -> i64 {
    let Some(e) = per_day.checked_mul(i64::from(days)).and_then(|a| exposure.checked_add(a)) else {
        capacity_exceeded!("a cell's review exposure", i64::MAX, days);
    };
    e
}

/// The chance a member reviews on a review day: one less the chance of no review over the exposure a member carries,
/// the cell's total over its weight.
#[must_use]
pub fn review_chance(exposure: i64, weight: Weight) -> f64 {
    if weight.get() == 0 {
        violation!(clause = "REP.17", "a review drawn over a cell of no members");
    }
    let per_member = phx_rand::float::from_i64(exposure) / EXPOSURE_ONE / f64::from(weight.get());
    -expm1(-per_member)
}

/// On a review day, the members of each profile value who review, drawn apart for each value from the exposure they
/// carry.
#[clause("REP.21")]
pub fn reviewers(d: &mut Draws, counts: &[u64], exposure: i64, weight: Weight, out: &mut [u64]) {
    let p = review_chance(exposure, weight);
    for (o, n) in out.iter_mut().zip(counts) {
        *o = binomial(d, *n, p);
    }
}

/// The exposure left to a cell once `reviewers` of its members have reviewed: they take their share of it, so the
/// total falls by `reviewers ÷ weight` of itself; the rest stays with the members who did not review.
#[clause("REP.21")]
#[must_use]
pub fn after_review(exposure: i64, reviewers: u64, weight: Weight) -> i64 {
    let w = u64::from(weight.get());
    if reviewers > w {
        violation!(clause = "REP.21", "more members reviewed than a cell has", reviewers = reviewers);
    }
    let share = i128::from(exposure) * i128::from(reviewers) / i128::from(w);
    let Ok(left) = i64::try_from(i128::from(exposure) - share) else {
        capacity_exceeded!("a cell's review exposure", i64::MAX, exposure);
    };
    left
}

/// The dispersion of exposure the cell's mean erases when reviewers who do not act stay in it: the stayers who
/// reviewed truly carry none and the others carry the mean before the review, but all now carry the mean after it.
/// The sum of squared differences from that mean, in the exposure's unit, for the representation's cost.
#[clause("REP.15", "REP.21")]
#[must_use]
pub fn erased_at_review(exposure: i64, weight: Weight, stayed_reviewers: u64, left: i64, stayers: u64) -> f64 {
    if stayers == 0 {
        return 0.0;
    }
    let unit = |x: i64| phx_rand::float::from_i64(x) / EXPOSURE_ONE;
    let before = unit(exposure) / f64::from(weight.get());
    let after = unit(left) / phx_rand::float::from_u64(stayers);
    let others = stayers - stayed_reviewers;
    phx_rand::float::from_u64(stayed_reviewers) * after.powi(2)
        + phx_rand::float::from_u64(others) * (before - after).powi(2)
}

#[cfg(test)]
mod tests {
    use libm::log1p;
    use phx_core::Weight;
    use phx_core::calendar::Calendar;
    use phx_core::calendar::rules::{CountryRules, WeekendRule};
    use phx_core::schedule::RunsOn;
    use phx_id::{CountryId, Date, Weekday};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, binomial, stream_key};

    use super::{accrued, after_review, erased_at_review, review_chance, reviewers, wake_day};
    use crate::consts::EXPOSURE_ONE;

    fn draws(tag: &str, i: u32) -> Draws {
        Draws::new(stream_key(Seed::new(3), tag), Subject::new(SubjectTag::Party, 1), i, 0)
    }

    /// A daily attention rate in fixed point: the −ln(1 − a) of a daily review chance `a`.
    fn rate(a: f64) -> i64 {
        phx_rand::float::floor_to_i64((-log1p(-a) * EXPOSURE_ONE).round()).unwrap()
    }

    #[test]
    fn a_surprise_wakes_the_next_day_its_point_runs() {
        let rules =
            CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
        let c = CountryId::new(0);
        let calendar = Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(c, rules)], 2026).unwrap();
        let day = |d| calendar.day(Date::new(2026, 9, d).unwrap()).unwrap();
        assert_eq!(wake_day(&calendar, c, RunsOn::Business, day(25)), day(28), "a Friday's surprise wakes on Monday");
        assert_eq!(wake_day(&calendar, c, RunsOn::Any, day(25)), day(26));
        assert_eq!(wake_day(&calendar, c, RunsOn::Business, day(24)), day(25));
    }

    #[test]
    fn review_count_first_day_exact() {
        // Thirty days at a constant attention of 1% a day, over a cell of 200 in two values.
        let (a, days, weight) = (0.01, 30, Weight::new(200));
        let exposure = accrued(0, rate(a) * 200, days);
        let counts = [120_u64, 80];
        let trials = 40_000_u32;
        let mut cell = [0_u64; 2];
        let mut member = [0_u64; 2];
        for t in 0..trials {
            let mut out = [0; 2];
            reviewers(&mut draws("cell", t), &counts, exposure, weight, &mut out);
            let mut d = draws("members", t);
            // Each member reviewing on the day with the chance its own thirty days give.
            let own = 1.0 - (1.0 - a).powi(i32::try_from(days).unwrap());
            let per: Vec<u64> = counts.iter().map(|n| binomial(&mut d, *n, own)).collect();
            for v in 0..2 {
                cell[v] += out[v];
                member[v] += per[v];
            }
        }
        for v in 0..2 {
            let (c, m) = (phx_rand::float::from_u64(cell[v]), phx_rand::float::from_u64(member[v]));
            assert!((c - m).abs() < 4.0 * (c + m).sqrt(), "value {v}: {c} against {m}");
        }
        assert!((review_chance(exposure, weight) - (1.0 - (1.0 - a).powi(30))).abs() < 1e-6);
    }

    #[test]
    fn review_count_bias_measured() {
        // A cell whose members review monthly at 2% a day, carried by its mean after each review, against the same
        // members each carrying its own exposure: the mean over-counts, the concavity of 1 − e^(−x).
        let (a, weight) = (0.02, 400_u64);
        let per_day = rate(a);
        let (mut total, mut own): (i64, Vec<i64>) = (0, vec![0; 400]);
        let (mut by_mean, mut by_member) = (0.0_f64, 0.0_f64);
        for month in 0..24_u32 {
            total = accrued(total, per_day * 400, 30);
            for e in &mut own {
                *e = accrued(*e, per_day, 30);
            }
            let p_mean = review_chance(total, Weight::new(400));
            by_mean += p_mean * phx_rand::float::from_u64(weight);
            let mut d = draws("bias", month);
            let mut k = 0_u64;
            for e in &mut own {
                let p = -libm::expm1(-phx_rand::float::from_i64(*e) / EXPOSURE_ONE);
                by_member += p;
                if phx_rand::open_unit(&mut d) < p {
                    *e = 0;
                    k += 1;
                }
            }
            total = after_review(total, k, Weight::new(400));
        }
        let bias = by_mean / by_member - 1.0;
        assert!(bias >= 0.0, "the mean over-counts reviews: {bias}");
        assert!(bias < 0.25, "and by a measured share: {bias}");
    }

    #[test]
    fn exposure_leaves_with_reviewers() {
        let w = Weight::new(10);
        assert_eq!(after_review(1_000, 3, w), 700);
        assert_eq!(after_review(1_000, 0, w), 1_000);
        assert_eq!(after_review(1_000, 10, w), 0, "all review, none is left");
        assert!(std::panic::catch_unwind(|| after_review(1_000, 11, w)).is_err());
        // Three reviewers of ten stay: they truly carry none, the seven others their mean of before.
        let one = phx_rand::float::floor_to_i64(EXPOSURE_ONE).unwrap();
        let erased = erased_at_review(10 * one, w, 3, 7 * one, 10);
        let (before, after) = (1.0, 0.7);
        assert!((erased - (3.0 * after * after + 7.0 * (before - after) * (before - after))).abs() < 1e-9);
    }
}
