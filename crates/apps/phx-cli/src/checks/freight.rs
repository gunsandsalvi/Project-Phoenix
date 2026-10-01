//! Freight's and the goods' reads: every shipment owned, carried and pledged, none left on its way past its day, the
//! basis between places tracking freight, and the goods' reads.

use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// The correlation of two series of equal length, none when either does not vary.
fn correlation(pairs: &[(f64, f64)]) -> Option<f64> {
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

/// Every shipment on its way has an owner and a carrier apart, and its owner's goods it left as are pledged at least
/// as much as its shipments of them carry; a carriage meeting's refusals are counted, never repriced.
fn shipments_owned(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(shipments) = core.freight.shipments.as_ref() else {
        return Outcome::Fail("the world keeps no shipments".to_owned());
    };
    let mut carried: std::collections::BTreeMap<(u32, u16), i64> = std::collections::BTreeMap::new();
    for s in shipments.live() {
        if s.owner == s.carrier {
            return Outcome::Fail(format!("a shipment of {} units its owner carries itself", s.units));
        }
        *carried.entry((s.owner.word(), s.from)).or_insert(0) += s.units;
    }
    for ((owner, unit), units) in carried {
        let key = phx_id::PartyKey::from_word(owner);
        let pledged = core.goods.stocks.holding(key, unit).map_or(0, |h| h.pledged);
        if pledged < units {
            return Outcome::Fail(format!("party {owner} ships {units} units of good {unit} with {pledged} pledged"));
        }
    }
    if core.freight.days.iter().all(|d| d.booked == 0) {
        return Outcome::NotYet("no trip was booked in the run");
    }
    Outcome::Pass
}

/// No shipment is on its way past its day of arrival, and goods arrived in the run.
fn arrived_on_time(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(shipments) = core.freight.shipments.as_ref() else {
        return Outcome::Fail("the world keeps no shipments".to_owned());
    };
    let overdue = shipments.overdue(w.today());
    if overdue > 0 {
        return Outcome::Fail(format!("{overdue} shipments on their way past their day"));
    }
    if core.freight.days.iter().all(|d| d.arrived == 0) {
        return Outcome::NotYet("no shipment arrived in the run");
    }
    Outcome::Pass
}

/// The gaps between places' marks a lot rise with the freight of a lot between them.
fn gaps_track_freight(w: Inspector<'_>) -> Outcome {
    let basis = w.core().basis(w.regions(), w.geo());
    if basis.len() < 2 {
        return Outcome::NotYet("fewer than two pairs of places mark a good carriage joins");
    }
    match correlation(&basis) {
        Some(r) if r > 0.0 => Outcome::Pass,
        Some(r) => Outcome::Fail(format!("the basis between places moves against freight: correlation {r:.3}")),
        None => Outcome::NotYet("the basis or the freight does not vary"),
    }
}

/// The goods' reads: a place's price is more volatile the fewer days of sales its stock covers; the basis between
/// places tracks freight; and each country's producer prices move a month before its consumer prices more than after.
fn goods_reads(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let cover = core.cover_by_place(w.regions());
    let pairs: Vec<(f64, f64)> =
        core.goods.outlooks.series.iter().filter_map(|(k, s)| Some((*cover.get(k)?, s.volatility()?))).collect();
    if pairs.len() < 2 {
        return Outcome::NotYet("fewer than two places' prices and stocks to read");
    }
    match correlation(&pairs) {
        Some(r) if r < 0.0 => {}
        Some(r) => return Outcome::Fail(format!("volatility does not rise as stocks fall: correlation {r:.3}")),
        None => return Outcome::NotYet("the stocks' cover or the prices' volatility does not vary"),
    }
    match gaps_track_freight(w) {
        Outcome::Pass => {}
        other => return other,
    }
    producers_lead(w)
}

/// Each country's monthly producer index changes against its consumer index's a month later, and the other way: the
/// producers' lead is the closer.
fn producers_lead(w: Inspector<'_>) -> Outcome {
    let published = &w.core().stats.published;
    let (mut lead, mut lag) = (Vec::new(), Vec::new());
    for country in 0..w.countries().len() {
        let series = |s: usize| -> Vec<f64> {
            let mut by: std::collections::BTreeMap<u32, f64> = std::collections::BTreeMap::new();
            for r in published.iter().filter(|r| usize::from(r.series) == s && usize::from(r.country) == country) {
                if let Some(v) = r.values.first() {
                    by.insert(r.period, phx_rand::float::from_i64(*v));
                }
            }
            let v: Vec<f64> = by.into_values().collect();
            v.windows(2).filter_map(|p| Some(p.get(1)? / p.first()? - 1.0)).collect()
        };
        let (cpi, ppi) = (series(if_state::stats::CPI), series(if_state::stats::PPI));
        for (i, p) in ppi.iter().enumerate() {
            if let Some(c) = cpi.get(i + 1) {
                lead.push((*p, *c));
            }
        }
        for (i, c) in cpi.iter().enumerate() {
            if let Some(p) = ppi.get(i + 1) {
                lag.push((*c, *p));
            }
        }
    }
    let (Some(ahead), Some(behind)) = (correlation(&lead), correlation(&lag)) else {
        return Outcome::NotYet("too few months of both indices published");
    };
    if ahead > behind {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("producer prices lead by {ahead:.3} and follow by {behind:.3}"))
    }
}

pub const LC_1_15: Check = live_check! {
    id: "LC-1-15",
    title: "the reads of GDS.11 are reported: volatility against stocks, the basis between places against freight, \
            and producer prices moving before consumer prices",
    from_step: "S1.05",
    check: goods_reads,
};

pub const LC_1_19: Check = live_check! {
    id: "LC-1-19",
    title: "FRT.9 and GEO.13: every shipment has one owner, one carrier and its goods pledged to it; no carrier books \
            beyond its vehicles' room and no segment beyond its capacity, which the carriage meeting refuses",
    from_step: "S1.07",
    check: shipments_owned,
};

pub const LC_1_20: Check = live_check! {
    id: "LC-1-20",
    title: "FRT.10: freight rates and price gaps between places are reported, and gaps track freight",
    from_step: "S1.07",
    check: gaps_track_freight,
};

pub const LC_1_47: Check = live_check! {
    id: "LC-1-47",
    title: "every lien of goods in transit is released on arrival (FRT.6, FRT.8): no shipment stays in transit past \
            its day",
    from_step: "S1.07",
    check: arrived_on_time,
};
