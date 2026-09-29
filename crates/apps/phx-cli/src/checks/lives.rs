//! The persons' lives on the core: each hazard's realised hits against its declared rates over the sampled
//! households, by age class for death and illness, and the period life table each agency publishes.

use if_state::stats::{LIFE_ENTRY, LIFE_TABLE, PLACES};
use phx_world::Inspector;
use phx_world::core_rates::RateTally;

use super::{Check, Outcome};
use crate::live_check;

/// How many standard deviations of its sampling error a realised count may stand from its expectation: at this many,
/// a year's few hundred tallies all pass by chance but for about one run in a thousand.
const RATE_SIGMAS: f64 = 5.0;
/// Half a hit: how near its expectation a count with no sampling error must stand, being a whole number.
const HALF_A_HIT: f64 = 0.5;
/// The chance, one-sided, of a count at least as far from its expectation as five of its standard deviations: the
/// normal tail the exact tail below is held to.
const TAIL: f64 = 2.9e-7;
/// Expectations below this many hits are judged by the exact Poisson tail, where the normal one misleads.
const POISSON_BELOW: f64 = 30.0;
/// The processes on lives whose rates are held by age class: death and the onset of disability.
const LIFE_HAZARDS: [&str; 2] = ["DEM.death", "DEM.disability_onset"];

/// The tallies summed.
fn total<'a>(tallies: impl Iterator<Item = &'a RateTally>) -> RateTally {
    tallies.fold(RateTally::default(), |a, t| RateTally {
        expected: a.expected + t.expected,
        variance: a.variance + t.variance,
        realised: a.realised + t.realised,
    })
}

/// The chance of a Poisson count of mean `lambda` at least `n` when `n` is above the mean, at most `n` when below.
fn poisson_tail(lambda: f64, n: u64) -> f64 {
    let mut pmf = (-lambda).exp();
    let (mut below, mut at) = (0.0_f64, 0_u64);
    while at < n {
        below += pmf;
        at += 1;
        pmf *= lambda / phx_rand::float::from_u64(at);
    }
    if phx_rand::float::from_u64(n) >= lambda { 1.0 - below } else { below + pmf }
}

/// A tally within its sampling error, or why not: few expected hits are judged by the exact tail, many by the normal
/// one.
fn within(what: &str, t: &RateTally) -> Result<(), String> {
    let hits = phx_rand::float::from_u64(t.realised);
    let (lambda, variance) = (t.expected, t.variance);
    // A certain hit, or none, has no sampling error: it is exact.
    if variance <= 0.0 && (hits - lambda).abs() < HALF_A_HIT {
        return Ok(());
    }
    let off = (hits - lambda) / variance.sqrt();
    let fine = if lambda < POISSON_BELOW { poisson_tail(lambda, t.realised) >= TAIL } else { off.abs() <= RATE_SIGMAS };
    if variance > 0.0 && fine {
        return Ok(());
    }
    Err(format!("{what}: {hits:.0} hits against {lambda:.2} expected ({off:.1} sigmas)"))
}

/// Every process's realised hits over the sampled households within their sampling error of its declared rates, in
/// total and at each age and year; a process nobody could be hit by over the run is dead.
fn realised_rates(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    if processes.is_empty() {
        return Outcome::NotYet("the world keeps no process on persons");
    }
    let rates = &w.core().rates.tallies;
    for (p, (hazard, _, _)) in (0_u32..).zip(&processes) {
        let t = total(rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)).map(|(_, t)| t));
        if t.expected <= 0.0 {
            return Outcome::Fail(format!("{hazard} could hit nobody over the run"));
        }
        if let Err(e) = within(hazard, &t) {
            return Outcome::Fail(e);
        }
        for ((_, year, age), t) in rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)) {
            if let Err(e) = within(&format!("{hazard} in {year} at age {age}"), t) {
                return Outcome::Fail(e);
            }
        }
    }
    Outcome::Pass
}

/// Realised mortality and illness per age class within their sampling error of the declared tables, each year.
fn life_rates_by_class(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    let lives: Vec<(u32, &str)> = (0_u32..)
        .zip(&processes)
        .filter(|(_, (h, _, _))| LIFE_HAZARDS.contains(h))
        .map(|(p, (h, _, _))| (p, *h))
        .collect();
    if lives.len() != LIFE_HAZARDS.len() {
        return Outcome::Fail("death or the onset of disability is not a process on persons".to_owned());
    }
    let bounds = match w.register().partition("DEM.age_classes") {
        Ok(p) => p.bounds.clone(),
        Err(e) => return Outcome::Fail(e),
    };
    let class = |age: i64| bounds.partition_point(|b| *b <= age);
    let mut by_class: std::collections::BTreeMap<(u32, u32, usize), RateTally> = std::collections::BTreeMap::new();
    for ((p, year, age), t) in &w.core().rates.tallies {
        if !lives.iter().any(|(q, _)| q == p) {
            continue;
        }
        let slot = by_class.entry((*p, *year, class(i64::from(*age)))).or_default();
        *slot = total([*slot, *t].iter());
    }
    if by_class.is_empty() {
        return Outcome::NotYet("no sampled household was measured");
    }
    for ((p, year, c), t) in &by_class {
        let hazard = lives.iter().find(|(q, _)| q == p).map_or("", |(_, h)| h);
        if let Err(e) = within(&format!("{hazard} in {year} at age class {c}"), t) {
            return Outcome::Fail(e);
        }
    }
    Outcome::Pass
}

/// Every published life table's rates are its events over its exposures, and every country's table was published
/// on its law's day for every month closed that was due.
fn life_table_traced(w: Inspector<'_>) -> Outcome {
    let Ok(series) = u8::try_from(LIFE_TABLE) else { return Outcome::Fail("no life table series".to_owned()) };
    let stats = &w.core().stats;
    let tables: Vec<_> = stats.published.iter().filter(|r| r.series == series).collect();
    if tables.is_empty() {
        return Outcome::NotYet("the first period life table follows its period's end and its lag");
    }
    let Some(places) = PLACES
        .get(LIFE_TABLE)
        .and_then(|p| i64::try_from(phx_num::price::pow10(*p)).ok())
        .map(phx_rand::float::from_i64)
    else {
        return Outcome::Fail("no places for the life table".to_owned());
    };
    for t in &tables {
        if t.values.len() % LIFE_ENTRY != 0 {
            return Outcome::Fail(format!("period {}'s life table is not whole entries", t.period));
        }
        for e in t.values.chunks(LIFE_ENTRY) {
            let [_, _, _, events, exposure, rate] = e else { continue };
            let (events, exposure) = (phx_rand::float::from_i64(*events), phx_rand::float::from_i64(*exposure));
            let Some(expected) = sys_sta::rate(events, exposure) else {
                return Outcome::Fail(format!("period {}: a rate published over no exposure", t.period));
            };
            let published = phx_rand::float::from_i64(*rate) / places;
            // Half the last place the rate is published to, and as much again for the float's own rounding.
            if (published - expected).abs() > 1.0 / places {
                return Outcome::Fail(format!(
                    "period {}: a rate of {published} where its events and exposure give {expected}",
                    t.period
                ));
            }
        }
    }
    let calendar = w.calendar();
    let first = tables.iter().fold(u32::MAX, |m, t| if t.period < m { t.period } else { m });
    let now = phx_world::core_stats::period_of(calendar, w.today());
    for country in tables.iter().map(|t| t.country).collect::<std::collections::BTreeSet<_>>() {
        for period in first..now {
            let due = stats.due(calendar, (country, LIFE_TABLE, period));
            let found = tables.iter().any(|t| (t.country, t.period, t.vintage) == (country, period, 0));
            if due.is_some_and(|d| d <= w.today()) && !found {
                return Outcome::Fail(format!(
                    "country {country}'s life table for period {period} was due and not published"
                ));
            }
        }
    }
    Outcome::Pass
}

pub const LC_0_39: Check = live_check! {
    id: "LC-0-39",
    title: "Every hazard's realised hit rate is within its sampling error of its declared rate",
    from_step: "S0.22",
    check: realised_rates,
};

pub const LC_0_54: Check = live_check! {
    id: "LC-0-54",
    title: "Realised mortality and illness per age class match their declared tables within sampling error",
    from_step: "S0.25",
    check: life_rates_by_class,
};

pub const LC_1_51: Check = live_check! {
    id: "LC-1-51",
    title: "STA.1: the period life table is published on its calendar, each rate traceable to the sampled events and \
            exposures it came from",
    from_step: "S1.14",
    check: life_table_traced,
};

#[cfg(test)]
mod tests {
    use super::poisson_tail;

    #[test]
    fn the_poisson_tail_is_exact() {
        assert!((poisson_tail(0.025, 1) - (1.0 - (-0.025_f64).exp())).abs() < 1e-12);
        assert!((poisson_tail(2.0, 0) - (-2.0_f64).exp()).abs() < 1e-12);
        let at_most_one = (-3.0_f64).exp() * (1.0 + 3.0);
        assert!((poisson_tail(3.0, 1) - at_most_one).abs() < 1e-12);
    }
}
