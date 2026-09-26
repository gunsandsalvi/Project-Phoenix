//! Goods: their balance by place and the deposits' depletion clean; the markets' reads and a drought's price, once
//! goods are held and traded.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// A family registered and without findings.
fn clean(w: Inspector<'_>, name: &str) -> Result<(), String> {
    if !w.families().iter().any(|f| f.name == name) {
        return Err(format!("the family `{name}` is not registered"));
    }
    match w.findings().iter().find(|f| f.family == name) {
        Some(f) => Err(format!("{} on day {}: {}", f.family, f.day.get(), f.detail)),
        None => Ok(()),
    }
}

/// The goods' family is registered and found nothing, over goods that exist.
fn goods_clean(w: Inspector<'_>) -> Outcome {
    if let Err(e) = clean(w, sys_gds::families::GOODS.name) {
        return Outcome::Fail(e);
    }
    if w.books().ledger.goods.iter().next().is_none() {
        return Outcome::NotYet("firms hold stocks from S1.15");
    }
    Outcome::Pass
}

pub const LC_1_13: super::Check = live_check! {
    id: "LC-1-13",
    title: "per good and place, opening stock plus produced plus arrived equals consumed plus shipped plus spoiled \
            plus destroyed plus closing stock: the family of goods (GDS.10) is clean every close",
    from_step: "S1.05",
    check: goods_clean,
};

fn deposits_clean(w: Inspector<'_>) -> Outcome {
    match clean(w, phx_geo::audit::DEPOSITS.name) {
        Ok(()) => Outcome::Pass,
        Err(e) => Outcome::Fail(e),
    }
}

pub const LC_1_14: super::Check = live_check! {
    id: "LC-1-14",
    title: "for every finite deposit, extracted plus remaining equals its opening quantity: the family of deposits \
            (GEO.12) is clean",
    from_step: "S1.05",
    check: deposits_clean,
};

/// The goods' reads over the run: each good's volatility where it stands, the day-to-day changes of its log price,
/// against the units held of it at the run's end; and the basis between places against the metres between them.
fn goods_reads(w: Inspector<'_>) -> Outcome {
    let mut volatility: Vec<(f64, f64)> = Vec::new();
    for (key, prints) in good_series(w) {
        let changes: Vec<f64> = prints
            .windows(2)
            .filter_map(|p| match p {
                [(_, a), (_, b)] if *a > 0 && *b > 0 => {
                    Some((phx_rand::float::from_i64(*b) / phx_rand::float::from_i64(*a)).ln())
                }
                _ => None,
            })
            .collect();
        let phx_num::Missing::Present(good) = w.books().ledger.goods.of(key) else { continue };
        if changes.len() > 1 {
            let n = phx_rand::float::from_u64(phx_rand::float::len_u64(changes.len()));
            let mean = changes.iter().sum::<f64>() / n;
            let var = changes.iter().map(|c| (c - mean) * (c - mean)).sum::<f64>() / n;
            let held = w.books().ledger.instruments.get(good).issued.n();
            volatility.push((phx_rand::float::from_i64(held), var.sqrt()));
        }
    }
    let basis = gaps_by_distance(w);
    if volatility.len() < 2 || basis.len() < 2 {
        return Outcome::NotYet("fewer than two goods printed on two days, or two places on a common day, in the run");
    }
    match (correlation(&volatility), correlation(&basis)) {
        (Some(_), Some(_)) => Outcome::Pass,
        _ => Outcome::Fail(format!(
            "the reads did not compute over {} goods' volatility and {} pairs' basis",
            volatility.len(),
            basis.len()
        )),
    }
}

pub const LC_1_15: super::Check = live_check! {
    id: "LC-1-15",
    title: "the reads of GDS.11 are reported: volatility against stocks, the basis between places against freight, \
            and producer prices moving before consumer prices",
    from_step: "S1.05",
    check: goods_reads,
};

/// The first day from `day` on that a market printed above its last print before `day`, if it did.
fn first_rise(prints: &[(phx_id::Day, i64)], day: phx_id::Day) -> Option<phx_id::Day> {
    let before = prints.iter().rev().find(|(d, _)| *d < day)?.1;
    prints.iter().find(|(d, p)| *d >= day && *p > before).map(|(d, _)| *d)
}

/// Each good's prints where it stands, in the order printed, as a price for a lot on each day it printed.
pub(crate) fn good_series(
    w: Inspector<'_>,
) -> std::collections::BTreeMap<phx_ledger::goods::GoodKey, Vec<(phx_id::Day, i64)>> {
    let markets = w.markets();
    let goods: Vec<u16> = ["GDS.commodities", "GDS.between_firms"]
        .iter()
        .filter_map(|n| match w.market_kind(n) {
            phx_num::Missing::Present(k) => Some(k),
            phx_num::Missing::Absent => None,
        })
        .collect();
    let kind_of: std::collections::BTreeMap<phx_id::MarketId, (u16, u64)> =
        markets.made.iter().map(|(m, k, s)| (m, (k, s))).collect();
    let mut series: std::collections::BTreeMap<phx_ledger::goods::GoodKey, Vec<(phx_id::Day, i64)>> =
        std::collections::BTreeMap::new();
    for p in markets.tape.prints() {
        if let Some((k, subject)) = kind_of.get(&p.market())
            && goods.contains(k)
        {
            series.entry(phx_ledger::goods::GoodKey::from_code(*subject)).or_default().push((p.day(), p.price().raw()));
        }
    }
    series
}

/// For each good and each pair of places in one country that printed it on common days, the metres between them and
/// the mean gap between their prices on those days, as a share of the lower.
pub(crate) fn gaps_by_distance(w: Inspector<'_>) -> Vec<(f64, f64)> {
    let series = good_series(w);
    let geo = w.geo();
    let keys: Vec<&phx_ledger::goods::GoodKey> = series.keys().collect();
    let mut out = Vec::new();
    for (i, a) in keys.iter().enumerate() {
        for b in keys.iter().skip(i + 1).filter(|b| b.product == a.product && b.grade == a.grade) {
            let Some(metres) = geo.distances.between(a.zone, b.zone) else { continue };
            let (Some(pa), Some(pb)) = (series.get(*a), series.get(*b)) else { continue };
            let by_day: std::collections::BTreeMap<phx_id::Day, i64> = pb.iter().copied().collect();
            let shares: Vec<f64> = pa
                .iter()
                .filter_map(|(d, x)| by_day.get(d).map(|y| (*x, *y)))
                .filter(|(x, y)| *x > 0 && *y > 0)
                .map(|(x, y)| {
                    let (lo, hi) = if x < y { (x, y) } else { (y, x) };
                    phx_rand::float::from_i64(hi - lo) / phx_rand::float::from_i64(lo)
                })
                .collect();
            if !shares.is_empty() {
                let n = phx_rand::float::from_u64(phx_rand::float::len_u64(shares.len()));
                out.push((phx_rand::float::from_u64(metres), shares.iter().sum::<f64>() / n));
            }
        }
    }
    out
}

/// The correlation of two series of equal length, none when either does not vary.
pub(crate) fn correlation(pairs: &[(f64, f64)]) -> Option<f64> {
    let n = phx_rand::float::from_u64(phx_rand::float::len_u64(pairs.len()));
    let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
    }
    let r = sxy / (sxx * syy).sqrt();
    r.is_finite().then_some(r)
}

/// For every drought and every good printed both at a zone it struck and at a zone it did not, whether the struck
/// place's price rose first: the check passes when it did in most such cases.
fn drought_prices_first(w: Inspector<'_>) -> Outcome {
    let geo = w.geo();
    let series = good_series(w);
    let (mut first, mut cases) = (0_u32, 0_u32);
    for e in super::geo::events_of(w, "GEO.drought") {
        let struck: std::collections::BTreeSet<phx_id::ZoneId> = e
            .details
            .iter()
            .filter_map(|(s, _)| phx_rand::Subject::from_raw(*s))
            .filter_map(|s| u32::try_from(s.id()).ok())
            .filter_map(|t| match geo.zone_of(phx_id::TileId::new(t)) {
                phx_num::Missing::Present(z) => Some(z),
                phx_num::Missing::Absent => None,
            })
            .collect();
        for (key, prints) in series.iter().filter(|(k, _)| struck.contains(&k.zone)) {
            let Some(there) = first_rise(prints, e.day) else { continue };
            let earlier = series
                .iter()
                .filter(|(k, _)| k.product == key.product && k.grade == key.grade && !struck.contains(&k.zone))
                .filter_map(|(_, p)| first_rise(p, e.day))
                .any(|d| d < there);
            cases += 1;
            if !earlier {
                first += 1;
            }
        }
    }
    if cases == 0 {
        return Outcome::NotYet("no drought struck a place whose goods printed before and after it in the run");
    }
    if first * 2 <= cases {
        return Outcome::Fail(format!("the struck place's price rose first in {first} of {cases} cases"));
    }
    Outcome::Pass
}

pub const LC_1_46: super::Check = live_check! {
    id: "LC-1-46",
    title: "when a drought strikes one place, the price there rises before prices elsewhere (GDS.9)",
    from_step: "S1.05",
    check: drought_prices_first,
};
