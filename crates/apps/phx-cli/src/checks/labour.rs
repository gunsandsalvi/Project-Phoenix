//! Labour: every job a contract with an employer and an employee, no person working more hours than a week has; the
//! reads of unemployment, vacancies, wages and flows; and matches as the sum of acceptances.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// The hours of a week, the most a person's jobs can buy.
const HOURS_A_WEEK: u64 = 168;

/// Each employment line's sides hold as many members, so the headcount is the count of contracts and no wage is paid
/// to nobody; and no person holds jobs of more hours than a week has.
fn contracts_and_hours(w: Inspector<'_>) -> Outcome {
    let lines = w.employment_lines();
    if lines.is_empty() {
        return Outcome::NotYet("jobs are contracts once labour is declared (S1.08)");
    }
    if let Some((line, employees, employers)) = lines.iter().find(|(_, a, b)| a != b) {
        return Outcome::Fail(format!(
            "employment line {line} counts {employees} employees against {employers} jobs at its employers"
        ));
    }
    match w.persons_over_hours(HOURS_A_WEEK) {
        0 => Outcome::Pass,
        n => Outcome::Fail(format!("{n} persons hold jobs of more hours than a week has")),
    }
}

pub const LC_1_21: super::Check = live_check! {
    id: "LC-1-21",
    title: "LAB.13: no person has more hours than a day; headcount equals contracts; no wage paid to nobody",
    from_step: "S1.08",
    check: contracts_and_hours,
};

/// The Beveridge relation, Okun's co-movement, unemployment durations, wage dispersion and job-to-job flows read
/// from the run's vacancies, applications and contracts, once employers post.
fn labour_reads(w: Inspector<'_>) -> Outcome {
    if w.labour_days().iter().all(|(_, d)| d.posted == 0) {
        return Outcome::NotYet("vacancies are posted once firms hold a price and a planned output (S1.15)");
    }
    Outcome::Pass
}

pub const LC_1_22: super::Check = live_check! {
    id: "LC-1-22",
    title: "LAB.14: the Beveridge relation, Okun's co-movement, unemployment durations, wage dispersion within \
            occupation families and job-to-job flows are reported",
    from_step: "S1.08",
    check: labour_reads,
};

/// Each day's matches are its acceptances: nothing hires but an offer accepted.
fn matches_are_acceptances(w: Inspector<'_>) -> Outcome {
    match w.labour_days().iter().find(|(_, d)| d.matches != d.acceptances) {
        Some((day, d)) => {
            Outcome::Fail(format!("day {} matched {} with {} acceptances", day.get(), d.matches, d.acceptances))
        }
        None => Outcome::Pass,
    }
}

pub const LC_1_23: super::Check = live_check! {
    id: "LC-1-23",
    title: "LAB.15: the count of matches equals the sum of acceptances; no aggregate matching function",
    from_step: "S1.08",
    check: matches_are_acceptances,
};
