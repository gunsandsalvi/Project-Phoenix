use libm::{expm1, log1p};
use phx_id::Day;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::{Draws, binomials_joint_at_least_one, geometric, multinomial, open_unit};

use crate::consts::{LADDER_DEN, LADDER_NUM};

/// The rung of the weight ladder at or above a weight: rungs rise by a quarter at a time, and by at least one, so a
/// landing that raises a weight within its rung needs no new candidate day; thinning absorbs the gap at the cost of
/// at most a quarter more rejected candidates.
#[clause("REP.7")]
#[must_use]
pub fn rung(weight: u32) -> u64 {
    let w = u64::from(weight);
    let mut r = 1_u64;
    while r < w {
        let next = (r * LADDER_NUM).div_ceil(LADDER_DEN);
        r = if next > r { next } else { r + 1 };
    }
    r
}

/// The chance a row's members give of at least one hit in a day: one less the product over values of each value's
/// members all escaping, `1 − Π (1 − p_v)^n_v`, computed without cancellation.
#[must_use]
pub fn chance(counts: &[u64], rates: &[f64]) -> f64 {
    let log_escape: f64 = counts.iter().zip(rates).map(|(n, p)| phx_rand::float::from_u64(*n) * log1p(-p)).sum();
    -expm1(log_escape)
}

/// The envelope a row's candidate days are drawn at: the chance of a hit if every member of the rung had the
/// largest rate the row's values reach until its envelope's validity ends.
#[clause("REP.7")]
#[must_use]
pub fn envelope(p_bar: f64, rung: u64) -> f64 {
    -expm1(phx_rand::float::from_u64(rung) * log1p(-p_bar))
}

/// What the agenda books for a row and process: its next candidate day, or the first day its rates may differ from
/// those its envelope bounds, on which the envelope is drawn again before that day is screened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Booking {
    Candidate(Day),
    Redraw(Day),
}

fn day_after(first: Day, days: u64) -> Missing<Day> {
    match u64::from(first.get()).checked_add(days).map(u32::try_from) {
        Some(Ok(d)) => Missing::Present(Day::new(d)),
        _ => Missing::Absent,
    }
}

/// The next booking from `first` on, `first` included: a candidate `geometric(π̄)` days after it, or, where the rates
/// may change before that, the day they may, `changes`. A candidate beyond every countable day and rates that
/// never change book nothing: nothing hits the row within the world's time until a visit draws afresh.
#[clause("REP.7")]
pub fn next_booking(d: &mut Draws, first: Day, pi_bar: f64, changes: Missing<Day>) -> Missing<Booking> {
    // A row that can no longer be hit waits only for the day its rates may change.
    let wait = if pi_bar > 0.0 { geometric(d, pi_bar) } else { Missing::Absent };
    let candidate = match wait {
        Missing::Present(g) => day_after(first, g),
        Missing::Absent => Missing::Absent,
    };
    match (candidate, changes) {
        (Missing::Present(c), Missing::Present(v)) if c >= v => Missing::Present(Booking::Redraw(v)),
        (Missing::Present(c), _) => Missing::Present(Booking::Candidate(c)),
        (Missing::Absent, Missing::Present(v)) => Missing::Present(Booking::Redraw(v)),
        (Missing::Absent, Missing::Absent) => Missing::Absent,
    }
}

/// The envelope's bounds checked: every value's rate within `p_bar`, every member within the rung.
fn bounded(counts: &[u64], rates: &[f64], p_bar: f64, rung: u64) {
    if counts.len() != rates.len() {
        violation!(clause = "REP.7", "profile counts and rates of different lengths", counts = counts.len());
    }
    if rates.iter().any(|p| *p > p_bar) {
        violation!(clause = "REP.7", "a rate above the envelope its candidates were drawn at");
    }
    let members: u64 = counts.iter().sum();
    if members > rung {
        capacity_exceeded!("members under a weight rung", rung, members);
    }
}

/// A candidate day: accepted with the chance the row's members give today over the envelope's, and then the members
/// hit per value, jointly conditioned on at least one, into `out`. So a row is hit on a day with exactly its day's
/// chance, and its hits are exactly the daily binomial counts.
#[clause("REP.7", "CHN.7")]
pub fn candidate(d: &mut Draws, counts: &[u64], rates: &[f64], p_bar: f64, rung: u64, out: &mut [u64]) -> bool {
    bounded(counts, rates, p_bar, rung);
    // A row whose values reach no rate today is not hit, whatever its envelope, which may be none; a ratio past one
    // only by rounding accepts, as the chance it stands for is the envelope itself.
    let today = chance(counts, rates);
    if today <= 0.0 || open_unit(d) >= today / envelope(p_bar, rung) {
        out.fill(0);
        return false;
    }
    binomials_joint_at_least_one(d, counts, rates, out);
    true
}

/// A candidate day of a hazard with several outcomes: each value's rate is the sum of its outcomes' rates; the
/// members hit per value are drawn as by [`candidate`], then split among the outcomes in proportion to their rates.
/// `out` holds a count per (value, outcome), values first.
#[clause("REP.7")]
pub fn candidate_outcomes(
    d: &mut Draws,
    counts: &[u64],
    rates: &[&[f64]],
    p_bar: f64,
    rung: u64,
    out: &mut [u64],
) -> bool {
    let totals: Vec<f64> = rates.iter().map(|r| r.iter().sum()).collect();
    let mut hit = vec![0_u64; counts.len()];
    if !candidate(d, counts, &totals, p_bar, rung, &mut hit) {
        out.fill(0);
        return false;
    }
    let outcomes = rates.first().map_or(0, |r| r.len());
    if out.len() != counts.len() * outcomes {
        violation!(clause = "REP.7", "an outcome table of another size than values by outcomes", size = out.len());
    }
    for (v, ((n, r), total)) in hit.iter().zip(rates).zip(&totals).enumerate() {
        let probs: Vec<f64> = r.iter().map(|p| p / total).collect();
        let Some(row) = out.get_mut(v * outcomes..(v + 1) * outcomes) else { continue };
        if *n == 0 {
            row.fill(0);
        } else {
            multinomial(d, *n, &probs, row);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, binomial, stream_key};

    use super::{Booking, candidate, candidate_outcomes, chance, envelope, next_booking, rung};

    fn draws(tag: &str, day: u32) -> Draws {
        Draws::new(stream_key(Seed::new(11), tag), Subject::new(SubjectTag::Party, 7), day, 0)
    }

    /// Histograms of daily hits per value, the last bin gathering every larger count.
    const BINS: usize = 4;

    fn bin(n: u64) -> usize {
        usize::try_from(n).map_or(BINS - 1, |n| if n < BINS { n } else { BINS - 1 })
    }

    /// Pearson's statistic of two histograms of equal totals, over bins both expect at least ten in.
    fn chi2(a: &[[u64; BINS]], b: &[[u64; BINS]]) -> (f64, usize) {
        let mut stat = 0.0;
        let mut dof = 0;
        for (ra, rb) in a.iter().zip(b) {
            for (x, y) in ra.iter().zip(rb) {
                let (x, y) = (phx_rand::float::from_u64(*x), phx_rand::float::from_u64(*y));
                if x + y >= 20.0 {
                    stat += (x - y).powi(2) / (x + y);
                    dof += 1;
                }
            }
        }
        (stat, dof)
    }

    /// The scheme over `days` days: candidate days drawn ahead at the envelope, redrawn where its validity ends,
    /// thinned on each candidate; `rates_on(day)` gives each value's rate that day and `counts_on(day)` its members.
    fn scheme(
        tag: &str,
        days: u32,
        p_bar_on: &dyn Fn(u32) -> (f64, Missing<Day>),
        rates_on: &dyn Fn(u32) -> Vec<f64>,
        counts_on: &dyn Fn(u32) -> Vec<u64>,
        weight_rung: u64,
    ) -> Vec<[u64; BINS]> {
        let mut hist = vec![[0_u64; BINS]; counts_on(0).len()];
        let mut day = 0_u32;
        let mut hits_days = 0_u64;
        let (mut p_bar, mut valid) = p_bar_on(0);
        let mut next = next_booking(&mut draws(tag, 0), Day::new(0), envelope(p_bar, weight_rung), valid);
        while let Missing::Present(b) = next {
            match b {
                Booking::Redraw(at) => {
                    (p_bar, valid) = p_bar_on(at.get());
                    day = at.get();
                    // The redraw is drawn apart from the candidate that may fall on its own day.
                    next = next_booking(
                        &mut draws(&format!("{tag}-redraw"), day),
                        at,
                        envelope(p_bar, weight_rung),
                        valid,
                    );
                }
                Booking::Candidate(at) => {
                    if at.get() >= days {
                        break;
                    }
                    day = at.get();
                    let mut d = draws(tag, day);
                    let counts = counts_on(day);
                    let mut out = vec![0; counts.len()];
                    if candidate(&mut d, &counts, &rates_on(day), p_bar, weight_rung, &mut out) {
                        hits_days += 1;
                        for (h, n) in hist.iter_mut().zip(&out) {
                            h[bin(*n)] += 1;
                        }
                    }
                    next = next_booking(&mut d, at.succ(), envelope(p_bar, weight_rung), valid);
                }
            }
        }
        let _ = (day, hits_days);
        hist
    }

    /// The daily binomials over the same days, counting only days with a hit so both histograms count the same
    /// thing.
    fn daily(
        tag: &str,
        days: u32,
        rates_on: &dyn Fn(u32) -> Vec<f64>,
        counts_on: &dyn Fn(u32) -> Vec<u64>,
    ) -> Vec<[u64; BINS]> {
        let mut hist = vec![[0_u64; BINS]; counts_on(0).len()];
        for day in 0..days {
            let mut d = draws(tag, day);
            let out: Vec<u64> =
                counts_on(day).iter().zip(rates_on(day)).map(|(n, p)| binomial(&mut d, *n, p)).collect();
            if out.iter().any(|n| *n > 0) {
                for (h, n) in hist.iter_mut().zip(&out) {
                    h[bin(*n)] += 1;
                }
            }
        }
        hist
    }

    /// Chi-square critical values at the 0.1% level, by degrees of freedom less one.
    fn critical(dof: usize) -> f64 {
        const CRIT: [f64; 12] = [10.8, 13.8, 16.3, 18.5, 20.5, 22.5, 24.3, 26.1, 27.9, 29.6, 31.3, 32.9];
        let at = if dof == 0 { 0 } else { dof - 1 };
        CRIT[if at < CRIT.len() { at } else { CRIT.len() - 1 }]
    }

    const DAYS: u32 = 1_000_000;

    #[test]
    fn a_row_with_no_envelope_books_only_its_redraw() {
        let mut d = draws("none", 2);
        let v = Missing::Present(Day::new(40));
        assert_eq!(next_booking(&mut d, Day::new(3), 0.0, v), Missing::Present(Booking::Redraw(Day::new(40))));
        assert_eq!(next_booking(&mut d, Day::new(3), 0.0, Missing::Absent), Missing::Absent);
    }

    #[test]
    fn a_row_with_no_chance_today_is_not_hit() {
        let mut out = [7_u64; 2];
        let mut d = draws("none", 1);
        assert!(!candidate(&mut d, &[3, 4], &[0.0, 0.0], 0.0, 8, &mut out));
        assert_eq!(out, [0, 0]);
    }

    #[test]
    fn envelope_scheme_matches_daily_binomial() {
        let counts = |_: u32| vec![100, 50, 20];
        let constant = |_: u32| vec![0.0004, 0.0012, 0.003];
        let p_bar = |_: u32| (0.003, Missing::Absent);
        let (stat, dof) = chi2(
            &scheme("env-constant", DAYS, &p_bar, &constant, &counts, rung(170)),
            &daily("daily-constant", DAYS, &constant, &counts),
        );
        assert!(stat < critical(dof), "constant rates: χ² = {stat:.1} over {dof} bins");
        let falling = |day: u32| {
            let f = 1.0 - f64::from(day) / f64::from(DAYS) * 0.8;
            vec![0.0004 * f, 0.0012 * f, 0.003 * f]
        };
        let (stat, dof) = chi2(
            &scheme("env-falling", DAYS, &p_bar, &falling, &counts, rung(170)),
            &daily("daily-falling", DAYS, &falling, &counts),
        );
        assert!(stat < critical(dof), "falling rates: χ² = {stat:.1} over {dof} bins");
    }

    #[test]
    fn redraw_at_validity_window_end_is_exact() {
        // The rates triple at the year's turn, past the first envelope; the candidates drawn at it end there.
        let turn = DAYS / 2;
        let counts = |_: u32| vec![120, 50];
        let rates = move |day: u32| if day < turn { vec![0.0005, 0.001] } else { vec![0.0015, 0.003] };
        let p_bar = move |day: u32| {
            if day < turn { (0.001, Missing::Present(Day::new(turn))) } else { (0.003, Missing::Absent) }
        };
        let (stat, dof) = chi2(
            &scheme("env-window", DAYS, &p_bar, &rates, &counts, rung(170)),
            &daily("daily-window", DAYS, &rates, &counts),
        );
        assert!(stat < critical(dof), "a window's end: χ² = {stat:.1} over {dof} bins");
    }

    #[test]
    fn weight_ladder_thinning_exact() {
        // The weight rises from 119 to 148 within its rung, with no new candidate drawn.
        assert_eq!(rung(119), 148);
        let counts = |day: u32| if day < DAYS / 3 { vec![70, 49] } else { vec![88, 60] };
        let rates = |_: u32| vec![0.001, 0.002];
        let p_bar = |_: u32| (0.002, Missing::Absent);
        let (stat, dof) = chi2(
            &scheme("env-ladder", DAYS, &p_bar, &rates, &counts, rung(119)),
            &daily("daily-ladder", DAYS, &rates, &counts),
        );
        assert!(stat < critical(dof), "a weight within its rung: χ² = {stat:.1} over {dof} bins");
        let ladder: Vec<u64> = [1, 2, 3, 4, 5, 6, 7, 9, 12, 15, 19].into_iter().map(rung).collect();
        assert_eq!(ladder, [1, 2, 3, 4, 5, 7, 7, 9, 12, 15, 19]);
    }

    #[test]
    fn multi_outcome_conditioned_on_a_hit() {
        let (counts, rates): (Vec<u64>, [&[f64]; 2]) = (vec![60, 40], [&[0.01, 0.03], &[0.02, 0.02]]);
        let p_bar = 0.04;
        let mut per_outcome = [0_u64; 4];
        let mut accepted = 0_u64;
        let trials = 200_000_u32;
        for i in 0..trials {
            let mut out = [0_u64; 4];
            if candidate_outcomes(&mut draws("outcomes", i), &counts, &rates, p_bar, 100, &mut out) {
                accepted += 1;
                assert!(out.iter().sum::<u64>() >= 1, "an accepted day hits at least one");
                for (a, o) in per_outcome.iter_mut().zip(out) {
                    *a += o;
                }
            }
        }
        // Accepted as often as a hit comes; outcomes in proportion to their rates.
        let pi = chance(&counts, &[0.04, 0.04]) / envelope(p_bar, 100);
        let accepted_share = phx_rand::float::from_u64(accepted) / f64::from(trials);
        assert!((accepted_share - pi).abs() < 0.01, "{accepted_share} against {pi}");
        let ratio = |a: u64, b: u64| phx_rand::float::from_u64(a) / phx_rand::float::from_u64(b);
        assert!((ratio(per_outcome[1], per_outcome[0]) - 3.0).abs() < 0.15, "{per_outcome:?}");
        assert!((ratio(per_outcome[3], per_outcome[2]) - 1.0).abs() < 0.1, "{per_outcome:?}");
    }

    #[test]
    fn the_envelope_bounds_its_candidates() {
        let mut out = [0; 2];
        let d = &mut draws("bounds", 0);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| candidate(
                d,
                &[5, 5],
                &[0.1, 0.3],
                0.2,
                12,
                &mut out
            )))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| candidate(
                d,
                &[9, 9],
                &[0.1, 0.1],
                0.2,
                12,
                &mut out
            )))
            .is_err()
        );
    }
}
