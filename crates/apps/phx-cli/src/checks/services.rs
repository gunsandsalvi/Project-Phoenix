//! What the firms make and sell, and how they end: every production's inputs used as its way states; the services'
//! reads — their share of the firms' sales and of the jobs, the retail margin of services and of goods, and how often
//! and by how much each's prices move at a review; and no firm in arrears past its grace.

use phx_num::Missing;
use phx_world::Inspector;
use phx_world::consts::firm::{EXPECTED, MARKUP, PART_ONE, PRICE, PRODUCT};

use super::{Check, Outcome};
use crate::live_check;

/// The services' reads, services first and goods second where a pair.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ServicesRead {
    pub sales_share: f64,
    pub jobs_share: f64,
    pub median_markup: [f64; 2],
    pub changes_a_review: [f64; 2],
    pub mean_change: [f64; 2],
}

/// A list's middle value.
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// The services' reads at the close, none while the world holds no firm.
#[must_use]
pub fn services(w: Inspector<'_>) -> Option<ServicesRead> {
    let core = w.core();
    let goods = &core.goods;
    let firm = core.names.iter().position(|n| *n == "firm")?;
    let store = core.kinds.get(firm)?;
    let is_service = |p: i64| usize::try_from(p).ok().and_then(|p| goods.stored.get(p)).is_some_and(|s| !*s);
    let mut sales = [0.0; 2];
    let mut markups: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let mut product_of = std::collections::BTreeMap::new();
    for slot in store.parties.live_slots() {
        let word = |at: usize| match store.record(slot).get(at).map(|x| x.get()) {
            Some(Missing::Present(v)) => Some(v),
            _ => None,
        };
        let (Some(p), Some(expected), Some(price), Some(markup)) =
            (word(PRODUCT), word(EXPECTED), word(PRICE), word(MARKUP))
        else {
            continue;
        };
        let at = usize::from(!is_service(p));
        let lot = usize::try_from(p).ok().and_then(|p| goods.lots.get(p)).copied().unwrap_or(1.0);
        if let (Some(s), Some(m)) = (sales.get_mut(at), markups.get_mut(at)) {
            *s += phx_rand::float::from_i64(expected) / PART_ONE * phx_rand::float::from_i64(price) / lot;
            m.push(phx_rand::float::from_i64(markup) / PART_ONE);
        }
        product_of.insert(slot.get(), p);
    }
    let mut jobs = [0_u64; 2];
    let firm_kind = u8::try_from(firm).ok()?;
    for f in core.families.iter().filter(|f| f.name == "LAB.employment") {
        for edge in f.store.edges.open_slots() {
            let Some(row) = f.store.edges.row(edge) else { continue };
            let [employer, _] = row.ends;
            if employer.kind() != firm_kind {
                continue;
            }
            let Some(p) = product_of.get(&employer.slot().get()) else { continue };
            if let Some(n) = jobs.get_mut(usize::from(!is_service(*p))) {
                *n += 1;
            }
        }
    }
    let mut tallies = [(0_u64, 0_u64, 0.0_f64); 2];
    for (p, t) in &goods.prices {
        if let Some(x) = tallies.get_mut(usize::from(!is_service(i64::from(*p)))) {
            *x = (x.0 + t.reviews, x.1 + t.changes, x.2 + t.size);
        }
    }
    let share = |a: f64, b: f64| if a + b > 0.0 { a / (a + b) } else { 0.0 };
    let per = |n: f64, d: f64| if d > 0.0 { n / d } else { 0.0 };
    let from = phx_rand::float::from_u64;
    let [s, g] = tallies;
    let [ms, mg] = markups;
    Some(ServicesRead {
        sales_share: share(sales[0], sales[1]),
        jobs_share: share(from(jobs[0]), from(jobs[1])),
        median_markup: [median(ms), median(mg)],
        changes_a_review: [per(from(s.1), from(s.0)), per(from(g.1), from(g.0))],
        mean_change: [per(s.2, from(s.1)), per(g.2, from(g.1))],
    })
}

/// The services' reads are made at the close: services hold a share of the sales and of the jobs, and each kind of trade
/// has reviewed its prices.
fn services_reported(w: Inspector<'_>) -> Outcome {
    let Some(r) = services(w) else { return Outcome::NotYet("no firm in the world") };
    if r.changes_a_review.iter().all(|c| *c == 0.0) {
        return Outcome::NotYet("no firm changed its price in the run");
    }
    if r.sales_share <= 0.0 || r.jobs_share <= 0.0 {
        return Outcome::Fail(format!("services hold {} of the sales and {} of the jobs", r.sales_share, r.jobs_share));
    }
    Outcome::Pass
}

pub const LC_1_16: Check = live_check! {
    id: "LC-1-16",
    title: "SRV.7: the services' share of output and employment, the retail margin and its compression when \
            wholesale costs rise, and the frequency and size of retail price changes are reported",
    from_step: "S1.06",
    check: services_reported,
};

/// Every production by a way that uses stored inputs used them as the way states: no day made a unit whose inputs its
/// maker did not hold to use.
fn made_by_way(w: Inspector<'_>) -> Outcome {
    let days = &w.core().goods.days;
    let made: u64 = days.iter().map(|d| d.productions).sum();
    if made == 0 {
        return Outcome::NotYet("nothing was made from stored inputs in the run");
    }
    match days.iter().find(|d| d.unfed > 0) {
        Some(d) => Outcome::Fail(format!(
            "day {}: {} of {} productions made without the inputs their way uses",
            d.day, d.unfed, d.productions
        )),
        None => Outcome::Pass,
    }
}

pub const LC_1_05: Check = live_check! {
    id: "LC-1-05",
    title: "every production names a way its producer knew, with the inputs it consumed as the way states",
    from_step: "S1.02",
    check: made_by_way,
};

/// No firm keeps failing its payments past its grace without ending: each contract a firm has in arrears began to be
/// so no more than its country's grace and the day its default takes before; and firms end in every industry over a
/// year, by closure or by default, each into an estate.
fn firms_end(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(firm) = core.names.iter().position(|n| *n == "firm") else { return Outcome::NotYet("no firm kind") };
    let Some(store) = core.kinds.get(firm) else { return Outcome::NotYet("no firm kind") };
    for ((i, edge), since) in &core.insolvency.since {
        let Some(row) = core.families.get(*i).and_then(|f| f.store.edges.row(phx_id::Slot::new(*edge))) else {
            continue;
        };
        let payer = row.ends[0];
        if usize::from(payer.kind()) != firm {
            continue;
        }
        let Some(Missing::Present(region)) =
            store.record(payer.slot()).get(phx_world::consts::firm::REGION).map(|x| x.get())
        else {
            continue;
        };
        let country = usize::try_from(region).ok().and_then(|r| w.regions().get(r)).map(|c| usize::from(c.get()));
        let Some(Some(grace)) = country.and_then(|c| core.insolvency.grace.get(c)).copied() else { continue };
        let days = w.calendar().days_between(*since, w.today()).unwrap_or(0);
        if days > grace + 1 {
            return Outcome::Fail(format!(
                "firm {} has been in arrears for {days} days, past its grace of {grace}",
                payer.slot().get()
            ));
        }
    }
    let first = w.date(w.day_zero()).year();
    if w.date(w.today()).year() <= first {
        return Outcome::NotYet("no whole year ran, over which firms end in every industry");
    }
    let ended: std::collections::BTreeSet<u16> = core.insolvency.endings.iter().map(|e| e.product).collect();
    let mut held: std::collections::BTreeSet<u16> = ended.clone();
    for slot in store.parties.live_slots() {
        if let Some(Missing::Present(p)) = store.record(slot).get(PRODUCT).map(|x| x.get())
            && let Ok(p) = u16::try_from(p)
        {
            held.insert(p);
        }
    }
    if let Some(p) = held.difference(&ended).next() {
        return Outcome::Fail(format!(
            "no firm of industry {p} ended over the run; firms ended in {} of {}",
            ended.len(),
            held.len()
        ));
    }
    Outcome::Pass
}

pub const LC_1_45: Check = live_check! {
    id: "LC-1-45",
    title: "firms end every year in every industry, by closure and by default, each with an estate or a successor; \
            no firm keeps failing payments past its grace without ending",
    from_step: "S1.03",
    check: firms_end,
};
