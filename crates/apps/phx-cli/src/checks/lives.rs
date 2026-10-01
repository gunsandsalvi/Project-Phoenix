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
/// The first age of the childbearing years the general fertility rate counts women over, and the first past them.
const CHILDBEARING: (i64, i64) = (15, 50);
/// Births are published per thousand women a year.
const PER_THOUSAND: f64 = 1_000.0;
/// Days in a year, on average over the calendar's cycle of leap years.
const DAYS_A_YEAR: f64 = 365.25;
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

/// No child is still in school past its country's school-leaving age at the run's end, and the leavers are published
/// as the events of the process that takes them from school.
fn cohorts_leave(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(place) = core.names.iter().position(|n| *n == "household") else {
        return Outcome::NotYet("no households on the core");
    };
    let Some(Some(persons)) = core.persons.get(place) else {
        return Outcome::NotYet("no households on the core");
    };
    let mut ages = Vec::new();
    for c in w.regions() {
        match w.register().count_in("DEM.school_leaving_age", *c).map(i64::try_from) {
            Ok(Ok(a)) => ages.push(a),
            Ok(Err(e)) => return Outcome::Fail(e.to_string()),
            Err(e) => return Outcome::Fail(e),
        }
    }
    let date = w.date(w.today());
    for slot in core.live_slots(place) {
        let region = core.household_region(slot).and_then(|r| usize::try_from(r).ok());
        let Some(leaving) = region.and_then(|r| ages.get(r)) else {
            return Outcome::Fail(format!("household slot {} in a region of no country", slot.get()));
        };
        for p in persons.of(slot) {
            let person = phx_core::person_word::PersonWord(p.word);
            if person.get(phx_core::person_word::ROLE) == if_pop::CHILD.value && person.age_on(date) >= *leaving {
                return Outcome::Fail(format!(
                    "household slot {}: a child of {} still in school past {leaving}",
                    slot.get(),
                    person.age_on(date)
                ));
            }
        }
    }
    let kinds: Vec<u16> = w.processes().iter().filter(|(h, _, _)| *h == "DEM.birthday").map(|(_, _, e)| *e).collect();
    let left: u64 =
        core.events.iter().flat_map(|(_, e)| e).filter(|e| kinds.contains(&e.kind)).map(|e| e.persons).sum();
    if left == 0 {
        return Outcome::NotYet("no child reached the school-leaving age in the run");
    }
    Outcome::Pass
}

pub const LC_1_49: Check = live_check! {
    id: "LC-1-49",
    title: "Every cohort reaching the school-leaving age enters the adult roles on its days, and the labour force's \
            inflow is published",
    from_step: "S1.13",
    check: cohorts_leave,
};

/// The population by age class and sex at the close, the women in the childbearing years, the persons counted, and the
/// births and days of the run.
pub struct Structure {
    pub classes: Vec<(i64, [u64; 2])>,
    pub women: u64,
    pub persons: u64,
    pub births: u64,
    pub days: u64,
}

impl Structure {
    /// Births a year per thousand women in the childbearing years.
    #[must_use]
    pub fn general_fertility(&self) -> f64 {
        let (births, women, days) = (self.births, self.women, self.days);
        phx_rand::float::from_u64(births) / phx_rand::float::from_u64(women) * DAYS_A_YEAR
            / phx_rand::float::from_u64(days)
            * PER_THOUSAND
    }
}

/// The population's structure at the close, read from the core's persons.
#[must_use]
pub fn structure(w: Inspector<'_>) -> Option<Structure> {
    let core = w.core();
    let place = core.names.iter().position(|n| *n == "household")?;
    let persons = core.persons.get(place)?.as_ref()?;
    let bounds = w.register().partition("DEM.age_classes").ok()?.bounds.to_vec();
    let mut classes: Vec<(i64, [u64; 2])> = bounds.iter().map(|b| (*b, [0, 0])).collect();
    let date = w.date(w.today());
    let (mut women, mut counted) = (0_u64, 0_u64);
    for slot in core.live_slots(place) {
        for p in persons.of(slot) {
            let person = phx_core::person_word::PersonWord(p.word);
            let (age, sex) = (person.age_on(date), person.get(if_pop::SEX.field));
            let class = bounds.partition_point(|b| *b <= age).checked_sub(1)?;
            *classes.get_mut(class)?.1.get_mut(usize::try_from(sex).ok()?)? += 1;
            counted += 1;
            if sex == if_pop::FEMALE && (CHILDBEARING.0..CHILDBEARING.1).contains(&age) {
                women += 1;
            }
        }
    }
    let births = core.pop_days.iter().map(|(_, d)| d.born).sum();
    Some(Structure { classes, women, persons: counted, births, days: phx_rand::float::len_u64(core.pop_days.len()) })
}

/// The age structure sums to the persons counted, and the run's fertility is a number once any child is born.
fn structure_reported(w: Inspector<'_>) -> Outcome {
    let Some(s) = structure(w) else { return Outcome::Fail("no age structure could be read".to_owned()) };
    let summed: u64 = s.classes.iter().map(|(_, [f, m])| f + m).sum();
    if summed != s.persons {
        return Outcome::Fail(format!("the age structure holds {summed} persons of {} counted", s.persons));
    }
    if s.births == 0 {
        return Outcome::NotYet("fertility is read once households have children");
    }
    if !s.general_fertility().is_finite() {
        return Outcome::Fail(format!("{} births over {} women gave no fertility", s.births, s.women));
    }
    Outcome::Pass
}

pub const LC_1_36: Check = live_check! {
    id: "LC-1-36",
    title: "POP.13: age structure and fertility are reported",
    from_step: "S1.13",
    check: structure_reported,
};

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
