//! Outlooks on the core: every method's own outlook of each public series, never one shared, formed before it is
//! read; and the firms' stances, whose shares move as the heuristics' records do.

use phx_num::Missing;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Each series' outlooks of its next print by every method that has formed one.
fn outlooks(w: Inspector<'_>) -> Vec<Vec<f64>> {
    w.core()
        .goods
        .outlooks
        .series
        .values()
        .map(|s| {
            s.methods
                .iter()
                .flat_map(|v| v.outlook.iter())
                .filter_map(|o| match o {
                    Missing::Present(x) => Some(*x),
                    Missing::Absent => None,
                })
                .collect::<Vec<f64>>()
        })
        .filter(|o| o.len() > 1)
        .collect()
}

/// Some series' methods disagree: their outlooks are not all one.
fn disagree(w: Inspector<'_>) -> Outcome {
    let series = outlooks(w);
    if series.is_empty() {
        return Outcome::NotYet("no public series has two methods' outlooks in the run");
    }
    if series.iter().any(|o| o.iter().any(|x| Some(x) != o.first())) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("every method's outlook agrees on each of {} series", series.len()))
    }
}

/// Every series' outlooks were formed on or before today.
fn formed_before_use(w: Inspector<'_>) -> Outcome {
    let series = &w.core().goods.outlooks.series;
    if series.is_empty() {
        return Outcome::NotYet("no public series printed in the run");
    }
    match series.iter().find(|(_, s)| s.day > w.today()) {
        Some(((p, r), s)) => {
            Outcome::Fail(format!("product {p} at region {r}: its outlooks formed on day {}, after today", s.day.get()))
        }
        None => Outcome::Pass,
    }
}

/// The firms' shares by heuristic are kept each day and move over the run as their stances are reconsidered.
fn shares_move(w: Inspector<'_>) -> Outcome {
    let days = &w.core().goods.outlooks.days;
    let (Some(first), Some(last)) = (days.first(), days.last()) else {
        return Outcome::NotYet("no day's stances were counted in the run");
    };
    if first.by_heuristic.iter().sum::<u64>() == 0 {
        return Outcome::NotYet("no firm held a stance in the run");
    }
    if days.iter().any(|d| d.by_heuristic != first.by_heuristic) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!(
            "the firms' heuristics stood at {:?} on every day from day {} to {}",
            first.by_heuristic, first.day, last.day
        ))
    }
}

pub const LC_1_01: Check = live_check! {
    id: "LC-1-01",
    title: "outlooks disagree: their dispersion per variable is reported, positive where methods or histories differ, and wider after a large surprise",
    from_step: "S1.01",
    check: disagree,
};

pub const LC_1_02: Check = live_check! {
    id: "LC-1-02",
    title: "no outlook is formed after the stage that uses it",
    from_step: "S1.01",
    check: formed_before_use,
};

pub const LC_1_03: Check = live_check! {
    id: "LC-1-03",
    title: "heuristic shares per series are reported and move over the run, their lead over price swings published",
    from_step: "S1.01",
    check: shares_move,
};

/// Every households' outlook of a published series was formed on a day that series was published in its country.
fn read_when_published(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let series = &core.stats.outlooks.series;
    if series.is_empty() {
        return Outcome::NotYet("no statistic's change was published in the run");
    }
    let published = |(s, c): (u16, u32), day: phx_id::Day| {
        core.stats.published.iter().any(|r| u16::from(r.series) == s && u32::from(r.country) == c && r.published == day)
    };
    match series.iter().find(|(k, s)| !published(**k, s.day) || s.day > w.today()) {
        Some(((s, c), v)) => Outcome::Fail(format!(
            "series {s} of country {c}: the households' outlooks formed on day {}, no day it was published",
            v.day.get()
        )),
        None => Outcome::Pass,
    }
}

pub const LC_1_38: Check = live_check! {
    id: "LC-1-38",
    title: "STA.4: no party read a statistic before its publication day",
    from_step: "S1.14",
    check: read_when_published,
};

/// Each method's mean lag behind the turning points of the series it forecasts, in prints, by memory type and
/// heuristic, over the firms' series and the households'.
#[must_use]
pub fn lags(w: Inspector<'_>) -> std::collections::BTreeMap<(usize, usize), (u64, u64)> {
    let core = w.core();
    let mut out = core.goods.outlooks.lags.clone();
    for (k, (sum, n)) in &core.stats.outlooks.lags {
        let e = out.entry(*k).or_insert((0, 0));
        *e = (e.0 + sum, e.1 + n);
    }
    out
}

/// Methods lag the turning points by their memory and heuristic: their mean lags are not all one.
fn lags_differ(w: Inspector<'_>) -> Outcome {
    let lags = lags(w);
    let means: Vec<(u64, u64)> = lags.values().filter(|(_, n)| *n > 0).copied().collect();
    let Some(first) = means.first().copied() else {
        return Outcome::NotYet("no series turned and was followed in the run");
    };
    // Two means are one where their cross products are.
    if means.iter().any(|(s, n)| u128::from(*s) * u128::from(first.1) != u128::from(first.0) * u128::from(*n)) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("every method of {} lags the turns alike", means.len()))
    }
}

pub const LC_1_43: Check = live_check! {
    id: "LC-1-43",
    title: "each method's lag behind each turning point of a published series, by memory type and heuristic mix",
    from_step: "S1.01",
    check: lags_differ,
};

/// The surprised firms' first price changes after their surprises, in the surprises' order of size, cut into classes of
/// equal count: each class's mean surprise over what was expected, its mean days to the change, and its count.
#[must_use]
pub fn responses_by_size(w: Inspector<'_>, classes: usize) -> Vec<(f64, f64, usize)> {
    let mut r = w.core().goods.outlooks.responses.clone();
    r.sort_by(|a, b| a.0.total_cmp(&b.0));
    if r.is_empty() || classes == 0 {
        return Vec::new();
    }
    let per = r.len().div_ceil(classes);
    r.chunks(per)
        .map(|c| {
            let n = phx_rand::float::from_u64(phx_rand::float::len_u64(c.len()));
            let size = c.iter().map(|x| x.0).sum::<f64>() / n;
            let days = c.iter().map(|x| f64::from(x.1)).sum::<f64>() / n;
            (size, days, c.len())
        })
        .collect()
}

/// The most surprised change their decisions first: the more surprised half of the surprised firms changed its price no
/// later, on average, than the less surprised half.
fn most_surprised_first(w: Inspector<'_>) -> Outcome {
    let halves = responses_by_size(w, 2);
    match halves.as_slice() {
        [] => Outcome::NotYet("no firm a surprise bore on changed its price in the run"),
        [_] => Outcome::NotYet("too few firms a surprise bore on changed their price to rank them"),
        [less, more, ..] if more.1 <= less.1 => Outcome::Pass,
        [less, more, ..] => Outcome::Fail(format!(
            "the more surprised half ({:.2} of what was expected) took {:.2} days to change its price, the less ({:.2}) {:.2}",
            more.0, more.1, less.0, less.1
        )),
    }
}

pub const LC_1_44: Check = live_check! {
    id: "LC-1-44",
    title: "after each large surprise, the days until each stance's first changed decision, ranked by its surprise",
    from_step: "S1.01",
    check: most_surprised_first,
};
