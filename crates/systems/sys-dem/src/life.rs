//! A country's life table by Brass's relational model: its logits of survivorship are the standard's plus one level,
//! the same for both sexes, solved so that life expectancy at birth, the sexes weighted by the sex ratio at birth, is
//! the country's.

use phx_macros::clause;
use phx_num::violation;

/// Survivorship from birth to each whole age the standard gives, the first nought: each age's
/// `1 / (1 + exp(2 (standard + level)))`.
#[clause("POP.16")]
#[must_use]
pub fn survivorship(standard: &[f64], level: f64) -> Vec<f64> {
    let mut l = Vec::with_capacity(standard.len() + 1);
    l.push(1.0);
    l.extend(standard.iter().map(|y| 1.0 / (1.0 + libm::exp(2.0 * (y + level)))));
    l
}

/// The chance of dying in each year of age past the table's last, its last year's: the open age's constant hazard.
fn open_age(l: &[f64]) -> f64 {
    match l {
        [.., before, last] if *before > 0.0 => 1.0 - last / before,
        _ => violation!(clause = "POP.16", "a life table too short for its open age"),
    }
}

/// Life expectancy at birth: each year lived whole by those alive at its end and half by those who die in it, and
/// past the table's last age the years the open age's hazard leaves, `l · (1 − q) / q` beyond the half year.
#[must_use]
pub fn life_expectancy(l: &[f64]) -> f64 {
    let within: f64 = l.windows(2).map(|w| w.iter().sum::<f64>() / 2.0).sum();
    let q = open_age(l);
    let Some(last) = l.last() else { violation!(clause = "POP.16", "a life table of no ages") };
    within + last * (1.0 / q - 1.0 / 2.0)
}

/// The level at which the standards of women and men give the life expectancy asked, the men's weighted by their
/// share of births: bracketed by doubling, then halved until the bracket cannot narrow.
#[clause("POP.16")]
#[must_use]
pub fn level_for(standards: [&[f64]; 2], male_share: f64, asked: f64) -> f64 {
    let e = |level: f64| {
        let [f, m] = standards.map(|s| life_expectancy(&survivorship(s, level)));
        (1.0 - male_share) * f + male_share * m
    };
    let (mut lo, mut hi) = (-1.0_f64, 1.0_f64);
    while e(lo) < asked {
        lo *= 2.0;
        if !lo.is_finite() {
            violation!(clause = "POP.16", "a life expectancy longer than the standard's ages allow");
        }
    }
    while e(hi) > asked {
        hi *= 2.0;
        if !hi.is_finite() {
            violation!(clause = "POP.16", "a life expectancy shorter than any level gives");
        }
    }
    loop {
        let mid = f64::midpoint(lo, hi);
        if mid <= lo || mid >= hi {
            return mid;
        }
        if e(mid) > asked {
            lo = mid;
        } else {
            hi = mid;
        }
    }
}

/// The chance that one born `age` calendar years before the year dies in it: half a year at each of the two ages the
/// year spans, `1 − sqrt(l(age + 1) / l(age − 1))`, alive at the year's start; one born in the year, half a year from
/// birth, `1 − sqrt(l(1))`; past the table's last age, the open age's.
#[clause("POP.16", "REP.25")]
#[must_use]
pub fn dies_in_year(l: &[f64], age: usize) -> f64 {
    if age + 1 >= l.len() {
        return open_age(l);
    }
    let before = match age.checked_sub(1) {
        Some(a) => l.get(a),
        None => l.first(),
    };
    let Some(before) = before else { violation!(clause = "POP.16", "a life table of no ages") };
    match l.get(age + 1) {
        Some(after) if *before > 0.0 => 1.0 - libm::sqrt(after / before),
        _ => violation!(clause = "POP.16", "a life table in which no one reaches an age it gives", age = age),
    }
}

#[cfg(test)]
mod tests {
    use super::{dies_in_year, level_for, life_expectancy, survivorship};

    /// A standard whose logits rise by a tenth a year from −2.
    fn standard() -> Vec<f64> {
        (1..=100).map(|a| -2.0 + 0.1 * f64::from(a)).collect()
    }

    #[test]
    fn survivorship_starts_whole_and_falls() {
        let l = survivorship(&standard(), 0.0);
        assert_eq!((l.first(), l.len()), (Some(&1.0), 101));
        assert!(l.windows(2).all(|w| w[1] <= w[0]));
    }

    #[test]
    fn life_expectancy_counts_half_the_year_of_death_and_the_open_age() {
        // Half die each year: one and a half years within the table, and past it a quarter's 1/q − 1/2 = 1.5.
        assert!((life_expectancy(&[1.0, 0.5, 0.25]) - (0.75 + 0.375 + 0.375)).abs() < 1e-12);
    }

    #[test]
    fn the_level_found_gives_the_life_expectancy_asked() {
        let s = standard();
        for asked in [40.0, 60.0, 80.0] {
            let level = level_for([&s, &s], 0.5, asked);
            assert!((life_expectancy(&survivorship(&s, level)) - asked).abs() < 1e-9, "{asked}");
        }
    }

    #[test]
    fn a_cohorts_death_is_the_tables_over_the_ages_it_spans() {
        let l = survivorship(&standard(), 0.0);
        let q = dies_in_year(&l, 50);
        assert!((q - (1.0 - (l[51] / l[49]).sqrt())).abs() < 1e-15);
        assert!((0.0..1.0).contains(&q));
        let open = 1.0 - l[100] / l[99];
        assert!((dies_in_year(&l, 100) - open).abs() < 1e-15, "the open age's chance past the table");
        assert!((dies_in_year(&l, 120) - open).abs() < 1e-15);
        assert!((dies_in_year(&l, 0) - (1.0 - l[1].sqrt())).abs() < 1e-15, "half a year from birth");
    }
}
