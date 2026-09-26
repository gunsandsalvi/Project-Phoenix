//! Published statistics: each release on its calendar's day and never before its period ended; the period life table
//! published on its calendar, each rate the one its events and exposure give; output measured three ways.

use if_state::stats::{LIFE_ENTRY, LIFE_TABLE, PLACES};
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Every release was published on the day its agency's calendar names for its period and vintage, after its period
/// ended, and a revision after its first release; readers read through the latest published by their day.
fn published_on_calendar(w: Inspector<'_>) -> Outcome {
    let releases = w.releases();
    if releases.is_empty() {
        return Outcome::NotYet("no agency has published: the first releases follow the first month's end");
    }
    for r in releases {
        let series = usize::from(r.series);
        let due = w.release_day(r.country, series, r.period, r.vintage);
        if due != Some(r.published) {
            return Outcome::Fail(format!(
                "series {series} of period {} in country {} published on day {} against its calendar's {due:?}",
                r.period,
                r.country,
                r.published.get()
            ));
        }
        if w.month_of(r.published) <= r.period {
            return Outcome::Fail(format!("series {series} of period {} published within its period", r.period));
        }
        let first =
            releases.iter().find(|x| (x.series, x.country, x.period, x.vintage) == (r.series, r.country, r.period, 0));
        if r.vintage > 0 && first.is_none_or(|f| f.published >= r.published) {
            return Outcome::Fail(format!("series {series} of period {} revised before its first release", r.period));
        }
    }
    Outcome::Pass
}

/// Each life table's entries: its rates the ones its events and exposure give, within their rounding; and each
/// period whose release day has passed published.
fn life_table_traced(w: Inspector<'_>) -> Outcome {
    let Ok(series) = u8::try_from(LIFE_TABLE) else { return Outcome::Fail("no life table series".to_owned()) };
    let tables: Vec<_> = w.releases().iter().filter(|r| r.series == series).collect();
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
    let first = tables.iter().fold(u32::MAX, |m, t| if t.period < m { t.period } else { m });
    let today = w.today();
    for country in tables.iter().map(|t| t.country).collect::<std::collections::BTreeSet<_>>() {
        for period in first..w.month_of(today) {
            let due = w.release_day(country, LIFE_TABLE, period, 0);
            let found = tables.iter().any(|t| (t.country, t.period, t.vintage) == (country, period, 0));
            if due.is_some_and(|d| d <= today) && !found {
                return Outcome::Fail(format!(
                    "country {country}'s life table for period {period} was due and not published"
                ));
            }
        }
    }
    Outcome::Pass
}

/// Every release of the national accounts carries output by production, expenditure and income, the first two
/// positive, and the discrepancy it publishes is expenditure less income.
fn output_three_ways(w: Inspector<'_>) -> Outcome {
    let releases: Vec<&if_state::stats::Release> =
        w.releases().iter().filter(|r| usize::from(r.series) == if_state::stats::ACCOUNTS).collect();
    if releases.is_empty() {
        return Outcome::NotYet("no national accounts were published in the run");
    }
    for r in releases {
        let [production, expenditure, income, discrepancy] = r.values.as_slice() else {
            return Outcome::Fail(format!(
                "country {}'s accounts for period {} carry {} values",
                r.country,
                r.period,
                r.values.len()
            ));
        };
        if *production <= 0 || *expenditure <= 0 || expenditure - income != *discrepancy {
            return Outcome::Fail(format!(
                "country {}'s accounts for period {} (vintage {}): production {production}, expenditure {expenditure}, \
                 income {income}, discrepancy {discrepancy}",
                r.country, r.period, r.vintage
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_37: Check = live_check! {
    id: "LC-1-37",
    title: "STA.3: output by expenditure, income and production agree up to the published discrepancy",
    from_step: "S1.14",
    check: output_three_ways,
};

pub const LC_1_38: Check = live_check! {
    id: "LC-1-38",
    title: "STA.4: no party read a statistic before its publication day",
    from_step: "S1.14",
    check: published_on_calendar,
};

pub const LC_1_39: Check = live_check! {
    id: "LC-1-39",
    title: "IDX.5: an index's return equals the weighted return of its constituents",
    from_step: "S1.14",
    retired: "market indices, whose returns it read, are built with S3.09",
};

pub const LC_1_51: Check = live_check! {
    id: "LC-1-51",
    title: "STA.1: the period life table is published on its calendar, each rate traceable to the sampled events and \
            exposures it came from",
    from_step: "S1.14",
    check: life_table_traced,
};
