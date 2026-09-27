//! Births and leaving school: the persons counted each day are the day before's with the day's births added and its
//! deaths taken away; the age structure and fertility are reported; and every cohort leaves school on its days.

use if_pop::{CHILD, FEMALE, REGION, SEX};
use phx_rand::float::{from_u64, len_u64};
use phx_world::Inspector;
use serde_json::json;

use super::{Check, Outcome};
use crate::live_check;

/// The first age of the childbearing years the general fertility rate counts women over, and the first past them.
const CHILDBEARING: (i64, i64) = (15, 50);
/// Days in a year, on average over the calendar's cycle of leap years.
const DAYS_A_YEAR: f64 = 365.25;
/// Births are published per thousand women a year.
const PER_THOUSAND: f64 = 1_000.0;

/// The household kind's place among the population's kinds.
fn households(w: Inspector<'_>) -> Option<usize> {
    w.population().kinds.iter().position(|k| k.decl.kind == if_pop::HOUSEHOLD)
}

/// The school-leaving age of the country each region lies in.
fn leaving_ages(w: Inspector<'_>) -> Result<Vec<i64>, String> {
    w.geo()
        .map
        .regions
        .iter()
        .map(|r| {
            let age = w.register().count_in("DEM.school_leaving_age", r.country)?;
            i64::try_from(age).map_err(|e| e.to_string())
        })
        .collect()
}

/// The population by the reporting age classes and sex at the run's end, the women in the childbearing years, and the
/// births of the run.
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
        from_u64(self.births) / from_u64(self.women) * DAYS_A_YEAR / from_u64(self.days) * PER_THOUSAND
    }
}

/// The population's structure at the run's end.
#[must_use]
pub fn structure(w: Inspector<'_>) -> Option<Structure> {
    let k = households(w)?;
    let bounds = w.register().partition("DEM.age_classes").ok()?.bounds.to_vec();
    let mut classes: Vec<(i64, [u64; 2])> = bounds.iter().map(|b| (*b, [0, 0])).collect();
    let (kd, table, date) = (w.population().kinds.get(k)?, w.agent_table(k), w.date(w.today()));
    let (mut women, mut persons) = (0_u64, 0_u64);
    for slot in table.slots() {
        let h = phx_pop::explicit::household(&kd.decl, table, slot);
        for (_, p) in h.present() {
            let (age, s) = (p.age_on(date), p.attr(SEX.name)?);
            let class = bounds.partition_point(|b| *b <= age).checked_sub(1)?;
            let cell = classes.get_mut(class)?.1.get_mut(usize::try_from(s).ok()?)?;
            *cell += 1;
            persons += 1;
            if s == FEMALE && (CHILDBEARING.0..CHILDBEARING.1).contains(&age) {
                women += 1;
            }
        }
    }
    let births = w.agent_days().iter().map(|d| d.born).sum();
    Some(Structure { classes, women, persons, births, days: len_u64(w.agent_days().len()) })
}

/// The age structure and fertility as the run report publishes them.
#[must_use]
pub fn report(w: Inspector<'_>) -> serde_json::Value {
    let Some(s) = structure(w) else { return serde_json::Value::Null };
    json!({
        "age_structure": s.classes.iter().map(|(a, [f, m])| json!({"from": a, "women": f, "men": m})).collect::<Vec<_>>(),
        "persons": s.persons,
        "births": s.births,
        "general_fertility_per_thousand": s.general_fertility(),
        "school_leavers": leavers(w).iter().sum::<u64>(),
    })
}

/// The persons who left school each day, from the events of the process that moves them.
#[must_use]
pub fn leavers(w: Inspector<'_>) -> Vec<u64> {
    let Some(kind) = w.processes().iter().find(|(h, _, _)| *h == "DEM.birthday").map(|(_, _, e)| *e) else {
        return Vec::new();
    };
    let events = w.events();
    let first = w.day_zero().get();
    let mut days = vec![0_u64; w.agent_days().len()];
    for id in 1..=len_u64(events.len()) {
        let e = events.get(id);
        if e.kind != kind {
            continue;
        }
        let at = e.day.get().checked_sub(first + 1).and_then(|a| usize::try_from(a).ok());
        if let Some(n) = at.and_then(|a| days.get_mut(a)) {
            *n += e.details.iter().map(|(_, size)| u64::try_from(*size).unwrap_or(0)).sum::<u64>();
        }
    }
    days
}

/// Each day's persons are the day before's with the day's births added and its deaths taken away.
fn persons_balance(w: Inspector<'_>) -> Outcome {
    let days = w.agent_days();
    for pair in days.windows(2) {
        let [before, d] = pair else { continue };
        if before.persons + d.born != d.persons + d.gone {
            return Outcome::Fail(format!(
                "day {}: {} persons from {} with {} born and {} gone",
                d.day.get(),
                d.persons,
                before.persons,
                d.born,
                d.gone
            ));
        }
    }
    if days.iter().all(|d| d.born == 0) {
        return Outcome::NotYet("no household has had a child in the run");
    }
    Outcome::Pass
}

/// The age structure sums to the persons counted and the fertility of the run is a number.
fn structure_reported(w: Inspector<'_>) -> Outcome {
    let Some(s) = structure(w) else { return Outcome::Fail("no age structure could be read".to_owned()) };
    let counted = w.agent_days().last().map_or(0, |d| d.persons);
    let summed: u64 = s.classes.iter().map(|(_, [f, m])| f + m).sum();
    if summed != counted || s.persons != counted {
        return Outcome::Fail(format!("the age structure holds {summed} persons of {counted} counted"));
    }
    if s.births == 0 {
        return Outcome::NotYet("fertility is read once households have children");
    }
    if !s.general_fertility().is_finite() {
        return Outcome::Fail(format!("{} births over {} women gave no fertility", s.births, s.women));
    }
    Outcome::Pass
}

/// No child is still in school past its country's school-leaving age at the run's end, and the leavers are published.
fn cohorts_leave(w: Inspector<'_>) -> Outcome {
    let Some(k) = households(w) else { return Outcome::NotYet("no households") };
    let ages = match leaving_ages(w) {
        Ok(a) => a,
        Err(e) => return Outcome::Fail(e),
    };
    let (Some(kd), table, date) = (w.population().kinds.get(k), w.agent_table(k), w.date(w.today())) else {
        return Outcome::Fail("the household kind is not kept".to_owned());
    };
    for slot in table.slots() {
        let h = phx_pop::explicit::household(&kd.decl, table, slot);
        let Some(leaving) = usize::try_from(h.attr(REGION.name)).ok().and_then(|r| ages.get(r)) else {
            return Outcome::Fail(format!("agent {} in a region of no country", table.party(slot).get()));
        };
        if let Some((_, p)) = h.present().find(|(_, p)| p.role == CHILD.name && p.age_on(date) >= *leaving) {
            return Outcome::Fail(format!(
                "agent {}: a child of {} still in school past {leaving}",
                table.party(slot).get(),
                p.age_on(date)
            ));
        }
    }
    if leavers(w).iter().all(|n| *n == 0) {
        return Outcome::NotYet("no child reached the school-leaving age in the run");
    }
    Outcome::Pass
}

pub const LC_1_35: Check = live_check! {
    id: "LC-1-35",
    title: "POP.11: the population equals births and arrivals minus deaths and departures",
    from_step: "S1.13",
    check: persons_balance,
};

pub const LC_1_36: Check = live_check! {
    id: "LC-1-36",
    title: "POP.13: age structure and fertility are reported",
    from_step: "S1.13",
    check: structure_reported,
};

pub const LC_1_49: Check = live_check! {
    id: "LC-1-49",
    title: "Every cohort reaching the school-leaving age enters the adult roles on its days, and the labour force's \
            inflow is published",
    from_step: "S1.13",
    check: cohorts_leave,
};
