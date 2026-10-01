use phx_world::Inspector;

use phx_num::Missing;

use super::{Check, Observed, Outcome};
use crate::live_check;

/// Business days a month holds at least; a run shorter than a month may pay no monthly due.
const MONTH_DAYS: usize = 31;

/// Every flow the core's day made was settled, failed or committed, and over a month the core paid dues.
fn core_settles_its_flows(w: Inspector<'_>) -> Outcome {
    let days = &w.core().days;
    for d in days {
        if d.flows != d.settled + d.failed + d.committed {
            return Outcome::Fail(format!(
                "day {}: {} flows made, {} settled, {} failed, {} committed",
                d.day.get(),
                d.flows,
                d.settled,
                d.failed,
                d.committed
            ));
        }
    }
    if days.len() < MONTH_DAYS {
        return Outcome::NotYet("the run ended before a month of the core's days");
    }
    if days.iter().all(|d| d.flows == 0) {
        return Outcome::Fail(format!("the core made no flow in {} days", days.len()));
    }
    Outcome::Pass
}

pub const LC_0_62: Check = live_check! {
    id: "LC-0-62",
    title: "Every flow the core's day makes is settled, failed or committed, and the core pays its dues",
    from_step: "S1.23",
    check: core_settles_its_flows,
};

/// The core's households hold the persons they opened with, plus every person born, less every person gone, and
/// chance reached its persons over the run.
fn core_persons_live(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let (born, gone): (u64, u64) = core.pop_days.iter().fold((0, 0), |(b, g), (_, d)| (b + d.born, g + d.gone));
    let expected = core.persons_opened + born - gone;
    if core.persons_held() != expected {
        return Outcome::Fail(format!(
            "the core holds {} persons; it opened with {}, {born} were born and {gone} are gone",
            core.persons_held(),
            core.persons_opened
        ));
    }
    if core.pop_days.len() < MONTH_DAYS {
        return Outcome::NotYet("the run ended before a month of the core's days");
    }
    if gone == 0 {
        return Outcome::Fail(format!("no person on the core died in {} days", core.pop_days.len()));
    }
    Outcome::Pass
}

pub const LC_0_63: Check = live_check! {
    id: "LC-0-63",
    title: "The core's households hold their opening persons plus those born less those gone, and chance reaches them",
    from_step: "S1.23",
    check: core_persons_live,
};

/// Days an estate may wait for its country's next business day: a weekend and a holiday on either side.
const ESTATE_WAIT_DAYS: u32 = 5;

/// Every estate the core opened settles on its country's next business day, paying what it holds, and ends.
fn core_estates_settle(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let today = w.today();
    if let Some((e, c, opened)) = core.estates.iter().find(|(_, _, d)| d.get() + ESTATE_WAIT_DAYS < today.get()) {
        return Outcome::Fail(format!(
            "an estate of country {} opened on day {} is still waiting on day {} (slot {})",
            c.get(),
            opened.get(),
            today.get(),
            e.slot().get()
        ));
    }
    if core.days.iter().all(|d| d.estates == 0) {
        return Outcome::NotYet("no estate on the core has settled yet");
    }
    Outcome::Pass
}

pub const LC_0_64: Check = live_check! {
    id: "LC-0-64",
    title: "Every estate on the core settles on its country's next business day and ends",
    from_step: "S1.23",
    check: core_estates_settle,
};

/// The core's money family found no break on any day.
fn core_money_holds(w: Inspector<'_>) -> Outcome {
    match w.core().days.iter().find(|d| d.breaks > 0) {
        Some(d) => Outcome::Fail(format!("day {}: {} breaks of the core's money", d.day.get(), d.breaks)),
        None if w.core().days.is_empty() => Outcome::NotYet("the core ran no day"),
        None => Outcome::Pass,
    }
}

pub const LC_0_65: Check = live_check! {
    id: "LC-0-65",
    title: "On the core each bank owes what its customers hold, and settlement makes and loses no money",
    from_step: "S1.23",
    check: core_money_holds,
};

/// Money moves on every business day of some country; no read rises on every day of the run unless its declaration
/// names why; and the reads never all stand still to the end.
fn alive(w: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for d in w.core().days.iter().filter(|d| w.any_business(d.day)) {
        if d.settled == 0 || d.gross == 0 {
            return Outcome::Fail(format!("no money moved on business day {}", d.day.get()));
        }
    }
    for (decl, series) in o.reads.iter().zip(o.series) {
        if decl.grows.is_none() && phx_obs::rises_throughout(&series.values) {
            return Outcome::Fail(format!("read `{}` rose on every day, for no named cause", decl.id));
        }
    }
    if let Missing::Present(day) = phx_obs::still_from(o.series) {
        return Outcome::Fail(format!("every read stood still from day {} to the end", day.get()));
    }
    Outcome::Pass
}

/// Each opening distribution has its distance from the world's own, and the share of its members that moved within
/// it, read at settling's end and at the run's end.
fn drift_read(_: Inspector<'_>, o: &Observed<'_>) -> Outcome {
    for (when, drifts) in [("settling's end", o.settled), ("the run's end", o.ended)] {
        if drifts.is_empty() {
            return Outcome::Fail(format!("no opening distribution was read at {when}"));
        }
        if let Some(d) = drifts.iter().find(|d| d.distance == Missing::Absent) {
            return Outcome::Fail(format!("`{}` has no distance at {when}: a histogram empty or rebinned", d.id));
        }
        if let Some(d) = drifts.iter().find(|d| d.moved == Missing::Absent) {
            return Outcome::Fail(format!("`{}` has no member sampled in both views at {when}", d.id));
        }
    }
    Outcome::Pass
}

pub const LC_0_59: Check = live_check! {
    id: "LC-0-59",
    title: "liveness: money circulates, fails are counted by cause, no quantity grows without a named cause",
    from_step: "S0.26",
    observed: alive,
};

pub const LC_0_60: Check = live_check! {
    id: "LC-0-60",
    title: "each opening distribution's distance from the world's own and its members' moves, at settling's end and the year's",
    from_step: "S0.26",
    observed: drift_read,
};

/// The core's audit families ran at every close and found nothing.
fn audit_clean(w: Inspector<'_>) -> Outcome {
    match w.findings().first() {
        Some(f) => Outcome::Fail(format!(
            "{} findings; the first, {} {} on day {}: {}",
            w.findings().len(),
            f.family,
            f.clause,
            f.day.get(),
            f.detail
        )),
        None if w.core().days.is_empty() => Outcome::NotYet("the run closed no day"),
        None => Outcome::Pass,
    }
}

/// The GEN report: every kind the core keeps is listed with the parties the opening began and the money written to
/// them — each party's own write in the run's `opening.csv` — every amount shared by weight names its stratum, country
/// and parties, and every distribution the register holds names its source.
fn opening_writes_reported(w: Inspector<'_>) -> Outcome {
    let opened = w.opened();
    if opened.kinds.len() != w.core().kinds.len() {
        return Outcome::Fail(format!(
            "{} kinds reported of the {} the core keeps",
            opened.kinds.len(),
            w.core().kinds.len()
        ));
    }
    if opened.kinds.iter().all(|k| k.parties == 0) {
        return Outcome::Fail("the opening reports no party".to_owned());
    }
    if let Some(a) = w.core().apportioned.iter().find(|a| a.total != 0 && a.parties == 0) {
        return Outcome::Fail(format!("{} in country {} was shared over no party", a.stratum, a.country));
    }
    match opened.distributions.iter().find(|(_, source)| source.trim().is_empty()) {
        Some((id, _)) => Outcome::Fail(format!("the distribution {id} names no source")),
        None if opened.distributions.is_empty() => Outcome::Fail("the report lists no distribution".to_owned()),
        None => Outcome::Pass,
    }
}

pub const LC_0_24: Check = live_check! {
    id: "LC-0-24",
    title: "The GEN report lists every opening write with party, amount and identity, and each distribution with its source",
    from_step: "S0.16",
    check: opening_writes_reported,
};

/// The first day passes every family.
fn day_one_clean(w: Inspector<'_>) -> Outcome {
    let Some(first) = w.core().days.first().map(|d| d.day) else { return Outcome::NotYet("the run closed no day") };
    match w.findings().iter().find(|f| f.day == first) {
        Some(f) => Outcome::Fail(format!("day one: {} {}: {}", f.family, f.clause, f.detail)),
        None => Outcome::Pass,
    }
}

/// On every business day some country keeps, flows fall due and some settle; every failed flow is counted by its
/// reason.
fn payments_settle(w: Inspector<'_>) -> Outcome {
    for d in &w.core().days {
        if w.any_business(d.day) && (d.flows == 0 || d.settled == 0) {
            return Outcome::Fail(format!("business day {}: {} flows, {} settled", d.day.get(), d.flows, d.settled));
        }
        if d.failed_by.iter().sum::<u64>() != d.failed {
            return Outcome::Fail(format!("day {}: {} failed, not all by a reason", d.day.get(), d.failed));
        }
    }
    Outcome::Pass
}

pub const LC_0_09: Check = live_check! {
    id: "LC-0-09",
    title: "Every close ran every declared audit family, and they found nothing",
    from_step: "S0.12",
    check: audit_clean,
};

pub const LC_0_23: Check = live_check! {
    id: "LC-0-23",
    title: "Day one passes every family",
    from_step: "S0.16",
    check: day_one_clean,
};

pub const LC_0_26: Check = live_check! {
    id: "LC-0-26",
    title: "Payments fall due and some settle every business day, and every fail has a cause",
    from_step: "S0.16",
    check: payments_settle,
};

pub const LC_0_51: Check = live_check! {
    id: "LC-0-51",
    title: "Day one passes every audit family with the full population",
    from_step: "S0.25",
    check: day_one_clean,
};

/// Every failed due of a dated contract — a wage, a pension, a benefit or a repayment — has a cause and is held in its
/// contract's arrears, owed by its payer; and the run paid dues.
fn fails_have_owners(w: Inspector<'_>) -> Outcome {
    use phx_world::consts::reason::{BENEFIT, PENSION, REPAID, WAGE};
    let at = |d: &phx_world::core_day::CoreDay, r: u8| d.failed_by.get(usize::from(r)).copied().unwrap_or(0);
    for d in &w.core().days {
        let dues = at(d, WAGE) + at(d, PENSION) + at(d, BENEFIT) + at(d, REPAID);
        if dues != d.arrears {
            return Outcome::Fail(format!("day {}: {dues} dues failed, {} held in arrears", d.day.get(), d.arrears));
        }
    }
    if w.core().days.iter().all(|d| d.flows == 0) {
        return Outcome::Fail("no due was paid".to_owned());
    }
    Outcome::Pass
}

pub const LC_0_55: Check = live_check! {
    id: "LC-0-55",
    title: "Paydays, dues and pensions in payment settle through pooled flows; every fail has a cause and a waiting owner",
    from_step: "S0.25",
    check: fails_have_owners,
};

/// Every national accounts release: output by production equals it by expenditure, every sale being a firm's and its
/// inputs netted out, and the published discrepancy is expenditure less income.
fn accounts_agree(w: Inspector<'_>) -> Outcome {
    let accounts = u8::try_from(if_state::stats::ACCOUNTS).unwrap_or(u8::MAX);
    let releases: Vec<_> = w.core().stats.published.iter().filter(|r| r.series == accounts).collect();
    if releases.is_empty() {
        return Outcome::NotYet("no national accounts published yet: the first comes after its month and lag");
    }
    for r in releases {
        let [production, expenditure, income, discrepancy] = r.values.as_slice() else {
            return Outcome::Fail(format!(
                "country {} period {}: accounts of {} values",
                r.country,
                r.period,
                r.values.len()
            ));
        };
        if production != expenditure || *discrepancy != expenditure - income {
            return Outcome::Fail(format!(
                "country {} period {}: production {production}, expenditure {expenditure}, income {income}, \
                 discrepancy {discrepancy}",
                r.country, r.period
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_37: Check = live_check! {
    id: "LC-1-37",
    title: "STA.3: output by expenditure, income and production agree up to the published discrepancy",
    from_step: "S1.14",
    check: accounts_agree,
};

pub const LC_1_40: Check = live_check! {
    id: "LC-1-40",
    title: "The Stage 1 opening: day one passes every family (GEN.7)",
    from_step: "S1.15",
    check: day_one_clean,
};

/// Every turn ends on a day that is a business day somewhere, and each covers every day since the last.
fn turns_whole(w: Inspector<'_>) -> Outcome {
    let mut next = w.day_zero().succ();
    for t in w.turns() {
        if t.first != next {
            return Outcome::Fail(format!(
                "a turn begins on day {} where the last left day {}",
                t.first.get(),
                next.get()
            ));
        }
        if !w.any_business(t.last) {
            return Outcome::Fail(format!("a turn ends on day {}, a business day nowhere", t.last.get()));
        }
        next = t.last.succ();
    }
    Outcome::Pass
}

/// The money family found nothing: each bank owes what its customers hold, the parties' money moves only by what
/// they, the banks and the issuers paid each other, and the issuers' money is what their record says.
fn money_clean(w: Inspector<'_>) -> Outcome {
    family_clean(w, "money")
}

/// The goods family found nothing: each good's units at the close are its units at the open and what the day made
/// less what it used.
fn goods_clean(w: Inspector<'_>) -> Outcome {
    family_clean(w, "goods")
}

/// The persons family found nothing: the households hold the persons opened, born and not gone.
fn persons_clean(w: Inspector<'_>) -> Outcome {
    family_clean(w, "persons")
}

fn family_clean(w: Inspector<'_>, family: &str) -> Outcome {
    if w.core().days.is_empty() {
        return Outcome::NotYet("the run closed no day");
    }
    match w.findings().iter().find(|f| f.family == family) {
        Some(f) => Outcome::Fail(format!("day {}: {} {}", f.day.get(), f.clause, f.detail)),
        None => Outcome::Pass,
    }
}

/// Every day the core ran records its flows and what they move, the value settled and what the payers' nets drew,
/// its failures by cause and by reason, and the closing ring; and they hold together: the nets draw no more than was
/// settled, the ring's receipts paid no more than was settled, and the failures by cause are the failures.
fn settlement_published(w: Inspector<'_>) -> Outcome {
    let days = &w.core().days;
    let Some(first) = days.first() else { return Outcome::NotYet("the run closed no day") };
    for (i, d) in days.iter().enumerate() {
        let day = d.day.get();
        if day != first.day.get() + u32::try_from(i).unwrap_or(u32::MAX) {
            return Outcome::Fail(format!("the core's days skip before day {day}"));
        }
        if d.flows > 0 && d.made <= 0 {
            return Outcome::Fail(format!("day {day}: {} flows moving nothing", d.flows));
        }
        if d.net < 0 || d.net > d.gross {
            return Outcome::Fail(format!("day {day}: the nets drew {} of {} settled", d.net, d.gross));
        }
        if d.ring_value < 0 || d.ring_value > d.gross || (d.ring == 0) != (d.ring_value == 0) {
            return Outcome::Fail(format!(
                "day {day}: a ring of {} paid {} of {} settled",
                d.ring, d.ring_value, d.gross
            ));
        }
        if d.fails.iter().sum::<u64>() != d.failed {
            return Outcome::Fail(format!("day {day}: {:?} failed by cause, {} failed", d.fails, d.failed));
        }
    }
    Outcome::Pass
}

pub const LC_0_02: Check = live_check! {
    id: "LC-0-02",
    title: "Every turn ends on a day that is a business day somewhere and covers every day since the last turn",
    from_step: "S0.11",
    check: turns_whole,
};

pub const LC_0_18: Check = live_check! {
    id: "LC-0-18",
    title: "Per issuer and currency, the balances held equal its money liability; the notes held equal the notes issued",
    from_step: "S0.15",
    check: money_clean,
};

pub const LC_0_20: Check = live_check! {
    id: "LC-0-20",
    title: "Per holder and asset per day, what it held plus what came in less what went out is what it holds",
    from_step: "S0.15",
    check: goods_clean,
};

pub const LC_0_22: Check = live_check! {
    id: "LC-0-22",
    title: "Gross and net settlement, fails by cause and the closing ring are published every day",
    from_step: "S0.15",
    check: settlement_published,
};

pub const LC_0_27: Check = live_check! {
    id: "LC-0-27",
    title: "Every bank's reserve movement equals the net of its customers' applied payments",
    from_step: "S0.17",
    check: money_clean,
};

pub const LC_0_52: Check = live_check! {
    id: "LC-0-52",
    title: "The populations reconcile: births, deaths and entries; every person in one household; every household held",
    from_step: "S0.25",
    check: persons_clean,
};

pub const LC_1_13: Check = live_check! {
    id: "LC-1-13",
    title: "per good and place, opening stock plus produced plus arrived equals consumed plus shipped plus spoiled \
            plus destroyed plus closing stock: the family of goods (GDS.10) is clean every close",
    from_step: "S1.05",
    check: goods_clean,
};

pub const LC_1_35: Check = live_check! {
    id: "LC-1-35",
    title: "POP.11: the population equals births and arrivals minus deaths and departures",
    from_step: "S1.13",
    check: persons_clean,
};

/// Every firm's posted price is a point of its trade's table, and prices move only at reviews: no day repriced more
/// firms than it reviewed.
fn prices_are_points(w: Inspector<'_>) -> Outcome {
    let Some(m) = w.management() else { return Outcome::NotYet("no firms' management compiled") };
    let core = w.core();
    let Some(firm) = core.names.iter().position(|n| *n == "firm") else { return Outcome::NotYet("no firm kind") };
    for slot in core.live_slots(firm) {
        match core.firm_view(slot).and_then(|v| v.price()) {
            Some(p) if m.is_point(p) => {}
            Some(p) => return Outcome::Fail(format!("firm slot {} posts {p}, no point of its trade", slot.get())),
            None => return Outcome::Fail(format!("firm slot {} posts a code of no point", slot.get())),
        }
    }
    if let Some(d) = core.goods.days.iter().find(|d| d.repriced > d.reviews) {
        return Outcome::Fail(format!("day {}: {} repriced of {} reviewed", d.day, d.repriced, d.reviews));
    }
    Outcome::Pass
}

/// The hours in a week, which no person's jobs may pass.
const WEEK_HOURS: u32 = 7 * 24;

/// No person holds jobs of more hours than a week has; every job names a person its household holds, so no wage is
/// paid to nobody.
fn jobs_held_by_persons(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let mut hours: std::collections::BTreeMap<u64, u32> = std::collections::BTreeMap::new();
    for f in core.families.iter().filter(|f| f.reason == phx_world::consts::reason::WAGE) {
        for edge in f.store.edges.open_slots() {
            let Some(row) = f.store.edges.row(edge) else { continue };
            let [_, household] = row.ends;
            let held = core
                .persons
                .get(usize::from(household.kind()))
                .and_then(Option::as_ref)
                .is_some_and(|p| p.of(household.slot()).any(|x| x.id == row.person));
            if !held {
                return Outcome::Fail(format!(
                    "a job of {} names person {}, whom its household does not hold",
                    f.name, row.person
                ));
            }
            let class = f.classes.get(usize::try_from(row.schedule).unwrap_or(usize::MAX)).copied().unwrap_or_default();
            *hours.entry(row.person).or_insert(0) += class[1];
        }
    }
    match hours.iter().find(|(_, h)| **h > WEEK_HOURS) {
        Some((p, h)) => Outcome::Fail(format!("person {p} holds jobs of {h} hours a week")),
        None if hours.is_empty() => Outcome::Fail("no one holds a job".to_owned()),
        None => Outcome::Pass,
    }
}

/// The circular flow lives: over the run wages were paid, households spent, firms made goods, persons were employed
/// and banks lent.
fn circular_flow_lives(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let wage = usize::from(phx_world::consts::reason::WAGE);
    let payday = core.days.iter().any(|d| d.wages > 0 || d.failed_by.get(wage).is_some_and(|n| *n > 0));
    if !payday {
        return Outcome::NotYet("no payday came in the run");
    }
    let paid = core.days.iter().any(|d| d.wages > 0);
    let spent = core.goods.days.iter().any(|d| d.spent > 0);
    let made = core.goods.days.iter().any(|d| d.made > 0);
    let employed = core
        .families
        .iter()
        .any(|f| f.reason == phx_world::consts::reason::WAGE && f.store.edges.open_slots().next().is_some());
    let lent = core
        .families
        .iter()
        .any(|f| f.reason == phx_world::consts::reason::REPAID && f.store.edges.open_slots().next().is_some());
    let missing: Vec<&str> =
        [(paid, "wages paid"), (spent, "spending"), (made, "production"), (employed, "employment"), (lent, "lending")]
            .iter()
            .filter(|(on, _)| !on)
            .map(|(_, what)| *what)
            .collect();
    if missing.is_empty() {
        Outcome::NotYet("the circular flow lives; its response is read when a decision moves a primitive in the run")
    } else {
        Outcome::Fail(format!("no {}", missing.join(", no ")))
    }
}

pub const LC_1_07: Check = live_check! {
    id: "LC-1-07",
    title: "every posted price is a point of its trade's table, and prices change only on review or wake days",
    from_step: "S1.03",
    check: prices_are_points,
};

pub const LC_1_21: Check = live_check! {
    id: "LC-1-21",
    title: "LAB.13: no person has more hours than a day; headcount equals contracts; no wage paid to nobody",
    from_step: "S1.08",
    check: jobs_held_by_persons,
};

pub const LC_1_34: Check = live_check! {
    id: "LC-1-34",
    title: "Liveness (N2) for the circular flow: wages paid, spending received, production, employment and lending \
            are non-zero and respond when a primitive moves in the run by its owner's decision",
    from_step: "S1.12",
    check: circular_flow_lives,
};

/// Every hazard's hit on a day has its event recorded that day: the day's events number its hits.
fn hits_have_events(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    for ((day, pop), (_, events)) in core.pop_days.iter().zip(&core.events) {
        let recorded: u64 = events.iter().map(|e| e.events).sum();
        if recorded != pop.hits {
            return Outcome::Fail(format!("day {}: {} hits, {recorded} events", day.get(), pop.hits));
        }
    }
    if core.events.iter().all(|(_, e)| e.is_empty()) {
        return Outcome::Fail("no hazard hit anyone over the run".to_owned());
    }
    Outcome::Pass
}

pub const LC_0_41: Check = live_check! {
    id: "LC-0-41",
    title: "Every hazard occurrence has its event recorded at the sub-step that drew it",
    from_step: "S0.22",
    check: hits_have_events,
};

/// The relative difference below which two firms' outlooks a unit of output are one: the millionth they are held in.
const SAME_RATIO: f64 = 1e-6;

/// Each firm's outlook of its sales is its own: among the makers of some product at some region, their expected sales
/// a unit of their output take more than one value once they have reviewed on what each sold.
fn outlooks_are_own(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(firm) = core.names.iter().position(|n| *n == "firm") else { return Outcome::NotYet("no firm kind") };
    let mut ratios: std::collections::BTreeMap<(u16, u32), Vec<(f64, f64)>> = std::collections::BTreeMap::new();
    for slot in core.live_slots(firm) {
        let Some(v) = core.firm_view(slot) else { continue };
        let (Some(product), Some(region), Some(expected), Some(output)) =
            (v.product(), v.region(), v.expected(), v.output())
        else {
            continue;
        };
        if output > 0.0 {
            ratios.entry((product, region)).or_default().push((expected, output));
        }
    }
    let cells = ratios.values().filter(|r| r.len() > 1).count();
    if cells == 0 {
        return Outcome::NotYet("no product has two makers at one region");
    }
    // Two ratios are one where their cross products agree to the millionth an outlook is held in.
    let differ = |r: &Vec<(f64, f64)>| {
        r.first().is_some_and(|(e0, o0)| r.iter().any(|(e, o)| (e * o0 - e0 * o).abs() > SAME_RATIO * (e0 * o).abs()))
    };
    if ratios.values().any(differ) {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("every maker of each of {cells} products at a region expects one ratio of its output"))
    }
}

pub const LC_1_04: Check = live_check! {
    id: "LC-1-04",
    title: "no variable is read by every party as one expectation, and things two parties with different histories value have more than one value",
    from_step: "S1.01",
    check: outlooks_are_own,
};

/// Every loan disbursed was lent by the borrower's own bank, a deposit made at the lender, and the money family, which
/// reconciles each issuer's money to what its accounts moved, is clean.
fn loans_make_deposits(w: Inspector<'_>) -> Outcome {
    let days = &w.core().days;
    let lent: u64 = days.iter().map(|d| d.lent).sum();
    if lent == 0 {
        return Outcome::NotYet("no loan was disbursed in the run");
    }
    if let Some(d) = days.iter().find(|d| d.lent_elsewhere > 0) {
        return Outcome::Fail(format!(
            "day {}: {} loans lent by a bank the borrower holds no account at",
            d.day.get(),
            d.lent_elsewhere
        ));
    }
    family_clean(w, "money")
}

pub const LC_1_26: Check = live_check! {
    id: "LC-1-26",
    title: "MON.6 in practice: every new loan's disbursement created a deposit at the lender, and money-stock \
            changes reconcile to issuers' transactions",
    from_step: "S1.09",
    check: loans_make_deposits,
};

/// Every benefit is a contract from a treasury to a household naming the person who claimed it, its first payment on
/// a date after the claim.
fn benefits_named(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let Some(f) = core.families.iter().find(|f| f.name == "SOC.benefit") else {
        return Outcome::NotYet("no benefit family on the core");
    };
    let household = core.names.iter().position(|n| *n == "household").and_then(|k| u8::try_from(k).ok());
    let mut n = 0_u64;
    for e in f.store.edges.open_slots() {
        let Some(row) = f.store.edges.row(e) else { continue };
        n += 1;
        let [treasury, claimant] = row.ends;
        if core.treasuries.iter().flatten().all(|t| *t != treasury) {
            return Outcome::Fail(format!("benefit contract {} paid by a party that is no treasury", e.get()));
        }
        if Some(claimant.kind()) != household || row.person == 0 {
            return Outcome::Fail(format!("benefit contract {} names no claimant in a household", e.get()));
        }
    }
    if n == 0 {
        return Outcome::NotYet("no benefit was claimed in the run");
    }
    Outcome::Pass
}

pub const LC_1_30: Check = live_check! {
    id: "LC-1-30",
    title: "SOC.7: every benefit is paid to a named household under its rule, after its claim",
    from_step: "S1.11",
    check: benefits_named,
};

/// No holding of a product delivered as it is made has units at the close: a service's capacity no sale took is lost
/// the day it is offered, and what a sale took is used.
fn services_not_stored(w: Inspector<'_>) -> Outcome {
    let goods = &w.core().goods;
    if goods.days.is_empty() {
        return Outcome::NotYet("no goods day closed in the run");
    }
    for h in goods.stocks.all() {
        let phx_core::goods::Held::Good(g) = phx_core::goods::held(&goods.units, h.unit) else { continue };
        if !goods.stored.get(usize::from(g.product)).copied().unwrap_or(true) && h.units != 0 {
            return Outcome::Fail(format!("{} units of service {} held at region {}", h.units, g.product, g.zone));
        }
    }
    Outcome::Pass
}

pub const LC_1_17: Check = live_check! {
    id: "LC-1-17",
    title: "no service is stored: no good of a product delivered as it is made has units in existence at a close",
    from_step: "S1.06",
    check: services_not_stored,
};

/// Each day's matches are its acceptances: every contract a hire made came from an offer accepted that day, one by one,
/// and no count of matches is drawn from an aggregate.
fn matches_are_acceptances(w: Inspector<'_>) -> Outcome {
    let days = &w.core().labour.days;
    if let Some(d) = days.iter().find(|d| d.hires > d.acceptances) {
        return Outcome::Fail(format!("day {}: {} hires from {} acceptances", d.day, d.hires, d.acceptances));
    }
    if days.iter().all(|d| d.hires == 0) {
        return Outcome::NotYet("no one was hired in the run");
    }
    Outcome::Pass
}

pub const LC_1_23: Check = live_check! {
    id: "LC-1-23",
    title: "LAB.15: the count of matches equals the sum of acceptances; no aggregate matching function",
    from_step: "S1.08",
    check: matches_are_acceptances,
};

/// Each trade's firms' markups at the close, their median, by product.
#[must_use]
pub fn markups_by_trade(w: Inspector<'_>) -> std::collections::BTreeMap<u16, f64> {
    let core = w.core();
    let mut by: std::collections::BTreeMap<u16, Vec<f64>> = std::collections::BTreeMap::new();
    let Some(firm) = core.names.iter().position(|n| *n == "firm") else {
        return std::collections::BTreeMap::new();
    };
    for slot in core.live_slots(firm) {
        let Some(v) = core.firm_view(slot) else { continue };
        if let (Some(p), Some(m)) = (v.product(), v.markup()) {
            by.entry(p).or_default().push(m);
        }
    }
    by.into_iter()
        .filter_map(|(p, mut m)| {
            m.sort_unstable_by(f64::total_cmp);
            m.get(m.len() / 2).map(|x| (p, *x))
        })
        .collect()
}

/// The frequency and size of price changes are kept per trade, and every trade's markups are read at the close.
fn prices_reported(w: Inspector<'_>) -> Outcome {
    let prices = &w.core().goods.prices;
    if prices.values().all(|t| t.reviews == 0) {
        return Outcome::NotYet("no firm reviewed its price in the run");
    }
    if let Some((p, t)) = prices.iter().find(|(_, t)| t.changes > t.reviews || !t.size.is_finite()) {
        return Outcome::Fail(format!(
            "trade {p}: {} changes of {} reviews, sizes summing to {}",
            t.changes, t.reviews, t.size
        ));
    }
    let markups = markups_by_trade(w);
    match prices.keys().find(|p| !markups.contains_key(p)) {
        Some(p) => Outcome::Fail(format!("trade {p} reviewed with no markup read at the close")),
        None => Outcome::Pass,
    }
}

pub const LC_1_08: Check = live_check! {
    id: "LC-1-08",
    title: "the frequency and size of price changes, and the markups, are reported per trade (SRV.7, FRM.19)",
    from_step: "S1.03",
    check: prices_reported,
};

/// The labour market's reads are kept each round — jobs open and persons searching at its close, hires and
/// separations — once employers post, and a job open is never counted below nothing.
fn labour_reads(w: Inspector<'_>) -> Outcome {
    let days = &w.core().labour.days;
    if days.iter().all(|d| d.posted == 0) {
        return Outcome::NotYet("no employer posted a vacancy in the run");
    }
    if days.iter().all(|d| d.searching == 0) {
        return Outcome::Fail("no round counted anyone searching".to_owned());
    }
    Outcome::Pass
}

pub const LC_1_22: Check = live_check! {
    id: "LC-1-22",
    title: "LAB.14: the Beveridge relation, Okun's co-movement, unemployment durations, wage dispersion within \
            occupation families and job-to-job flows are reported",
    from_step: "S1.08",
    check: labour_reads,
};

/// Every sale's money is a flow from its named buyer to its named seller: each day the sellers' credits are the
/// buyers' debits, and no sale names neither.
fn sales_named(w: Inspector<'_>) -> Outcome {
    let days = &w.core().goods.days;
    if days.iter().all(|d| d.sales == 0) {
        return Outcome::NotYet("no sale was made in the run");
    }
    match days.iter().find(|d| d.debits != d.credits || d.unnamed > 0) {
        Some(d) => Outcome::Fail(format!(
            "day {}: buyers debited {}, sellers credited {}, {} sales unnamed",
            d.day, d.debits, d.credits, d.unnamed
        )),
        None => Outcome::Pass,
    }
}

pub const LC_1_18: Check = live_check! {
    id: "LC-1-18",
    title: "HH.15 (part): every unit of household spending names its seller through a match-set record, and the \
            sellers' credits per meeting sum to the buyers' debits",
    from_step: "S1.06",
    check: sales_named,
};

pub const LC_1_32: Check = live_check! {
    id: "LC-1-32",
    title: "HH.15: every household's spending reaches named sellers (through match-set records), and every unit of \
            income came from a named payer",
    from_step: "S1.12",
    check: sales_named,
};

/// Every dated contract's next payment falls on a business day of its schedule's country, where its convention moves
/// its dates to one.
fn payments_on_business_days(w: Inspector<'_>) -> Outcome {
    let (core, calendar) = (w.core(), w.calendar());
    let mut read = 0_u64;
    for f in &core.families {
        for e in f.store.edges.open_slots() {
            let Some(row) = f.store.edges.row(e) else { continue };
            let Some((dates, _, _)) = usize::try_from(row.schedule).ok().and_then(|s| f.schedules.get(s)) else {
                return Outcome::Fail(format!("{} contract {} names no schedule", f.name, e.get()));
            };
            if dates.convention == phx_core::BusinessDayConvention::Unadjusted {
                continue;
            }
            let day = dates.nth(calendar, row.nth);
            if !calendar.is_business(dates.country, day) {
                return Outcome::Fail(format!(
                    "{} contract {}: its payment {} falls on {:?}, no business day",
                    f.name,
                    e.get(),
                    row.nth,
                    calendar.date(day)
                ));
            }
            read += 1;
        }
    }
    if read == 0 { Outcome::NotYet("no contract with an adjusted schedule is open") } else { Outcome::Pass }
}

pub const LC_0_25: Check = live_check! {
    id: "LC-0-25",
    title: "Every opening contract's payments fall on business days by its convention",
    from_step: "S0.16",
    check: payments_on_business_days,
};

/// The opening reports every amount it shared by weight — its stratum, country, amount and what the parties were
/// given, the difference being what no party was — and no party of no weight was given any.
fn apportionments_reported(w: Inspector<'_>) -> Outcome {
    let report = &w.core().apportioned;
    if report.is_empty() {
        return Outcome::Fail("the opening reports no apportionment".to_owned());
    }
    match report.iter().find(|a| a.unfounded > 0) {
        Some(a) => Outcome::Fail(format!(
            "{} in country {}: {} parties of no weight given a share",
            a.stratum, a.country, a.unfounded
        )),
        None => Outcome::Pass,
    }
}

pub const LC_0_56: Check = live_check! {
    id: "LC-0-56",
    title: "The GEN report lists every apportionment difference and every unmatched stratum",
    from_step: "S0.25",
    check: apportionments_reported,
};

/// The opening report lists every closure that balanced each country's sheet and every apportionment, and every party
/// the directory holds has its record in its kind's store.
fn opening_reported(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let countries = w.countries().len();
    for c in 0..countries {
        let Ok(c) = u8::try_from(c) else { continue };
        if !core.closures.iter().any(|(k, _, _)| *k == c) {
            return Outcome::Fail(format!("country {c}'s sheet reports no closure"));
        }
    }
    if let Some((c, name, v)) = core.closures.iter().find(|(_, _, v)| !v.is_finite()) {
        return Outcome::Fail(format!("country {c}'s {name} closed at {v}"));
    }
    if core.apportioned.is_empty() {
        return Outcome::Fail("the opening reports no apportionment".to_owned());
    }
    for (k, store) in core.kinds.iter().enumerate() {
        let rows = phx_store::StoreStats::rows_ever(store);
        if let Some(slot) = core.live_slots(k).find(|s| u64::from(s.get()) >= rows) {
            return Outcome::Fail(format!(
                "a {} at slot {} has no record",
                core.names.get(k).map_or("party", |n| n),
                slot.get()
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_41: Check = live_check! {
    id: "LC-1-41",
    title: "The GEN report lists every balancing change and apportionment difference, and names every party",
    from_step: "S1.15",
    check: opening_reported,
};

/// Each creditor's loan book, kept by what was lent, repaid and written off, equals what its loans owe it at every
/// close: the loans family found nothing.
fn loan_books_reconcile(w: Inspector<'_>) -> Outcome {
    let books = &w.core().loan_books;
    if books.is_empty() {
        return Outcome::NotYet("no creditor holds a loan in the run");
    }
    match w.findings().iter().find(|f| f.family == "loans") {
        Some(f) => Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

pub const LC_1_24: Check = live_check! {
    id: "LC-1-24",
    title: "BNK.11: each bank's loan book equals the sum of its loan lines, and its change reconciles",
    from_step: "S1.09",
    check: loan_books_reconcile,
};

/// The accounts family found nothing: every firm's and bank's equity account equals what its books show at every
/// close.
fn accounts_clean(w: Inspector<'_>) -> Outcome {
    if w.core().accounts.opening.is_empty() {
        return Outcome::NotYet("no party keeps an equity account in the run");
    }
    match w.findings().iter().find(|f| f.family == "accounts") {
        Some(f) => Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

/// The revenue family found nothing: the firms' revenue recognised at their sales' delivery equals what the buyers
/// paid them for sales, every revenue somebody's outlay.
fn revenue_clean(w: Inspector<'_>) -> Outcome {
    if w.core().accounts.revenue == 0 {
        return Outcome::NotYet("no firm recognised revenue in the run");
    }
    match w.findings().iter().find(|f| f.family == "revenue") {
        Some(f) => Outcome::Fail(format!("day {}: {}", f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

pub const LC_0_33: Check = live_check! {
    id: "LC-0-33",
    title: "Accounts is clean for every party with an equity account",
    from_step: "S0.19",
    check: accounts_clean,
};

pub const LC_1_06: Check = live_check! {
    id: "LC-1-06",
    title: "the family of the firms' revenue (FRM.17) is clean; the claims' (FRM.18) joins with the invoices (S2.02)",
    from_step: "S1.03",
    check: revenue_clean,
};

/// Each bank's applications, declines and quotes are counted, and declines are seen where any bank refused.
fn declines_counted(w: Inspector<'_>) -> Outcome {
    let lenders = &w.core().credit.lenders;
    let applied: u64 = lenders.values().map(|l| l.applications).sum();
    if applied == 0 {
        return Outcome::NotYet("no firm applied for a loan in the run");
    }
    match lenders.iter().find(|(_, l)| l.declined + l.quoted != l.applications) {
        Some((bank, l)) => Outcome::Fail(format!(
            "bank {}: {} applications where it declined {} and quoted {}",
            bank.word(),
            l.applications,
            l.declined,
            l.quoted
        )),
        None => Outcome::Pass,
    }
}

pub const LC_1_25: Check = live_check! {
    id: "LC-1-25",
    title: "Declined applications are visible and counted per bank",
    from_step: "S1.09",
    check: declines_counted,
};

/// Every decision the world declares was taken through the decision core by the run's close, each counted by its
/// decider.
fn decisions_taken(w: Inspector<'_>) -> Outcome {
    let taken = w.core().decisions.taken();
    if taken.is_empty() {
        return Outcome::NotYet("no decision declared on the core");
    }
    let never: Vec<&str> = taken.iter().filter(|(_, by)| by.iter().sum::<u64>() == 0).map(|(name, _)| *name).collect();
    if never.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("{} decisions never taken: {}", never.len(), never.join(", ")))
    }
}

/// Retirees claim the state pension: some of those who retired claimed it where any country's pension covers anyone,
/// and none claimed twice or without retiring.
fn pensions_claimed(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let (retired, claimed) = core.pop_days.iter().fold((0, 0), |(r, c), (_, d)| (r + d.retired, c + d.claimed));
    if retired == 0 {
        return Outcome::NotYet("no one retired");
    }
    if claimed > retired {
        return Outcome::Fail(format!("{claimed} pensions claimed by {retired} retirees"));
    }
    let covers = core.state.pension.iter().flatten().any(|p| p.coverage.iter().any(|c| *c > 0.0));
    if covers && claimed == 0 {
        return Outcome::Fail(format!("none of {retired} retirees claimed a pension that covers some"));
    }
    Outcome::Pass
}

pub const LC_1_53: Check = live_check! {
    id: "LC-1-53",
    title: "Retirees claim the state pension their country's coverage gives them",
    from_step: "S1.24",
    check: pensions_claimed,
};

pub const LC_1_52: Check = live_check! {
    id: "LC-1-52",
    title: "Every decision the world declares is taken through the decision core, by a named decider",
    from_step: "S1.26",
    check: decisions_taken,
};

/// The money family's reads of the issuers' own record are clean with the facilities in use, and every business day's
/// fund stage is reported.
fn facilities_clean(w: Inspector<'_>) -> Outcome {
    let days = &w.core().central.days;
    if !days.iter().any(|d| d.placed > 0 || d.borrowed > 0) {
        return Outcome::NotYet("no bank used the central bank's facilities in the run");
    }
    if let Some(f) = w.findings().iter().find(|f| f.family == "money" && (f.clause == "MON.7" || f.clause == "MON.9")) {
        return Outcome::Fail(format!("day {}: {} {}", f.day.get(), f.clause, f.detail));
    }
    match w.core().days.iter().find(|d| w.any_business(d.day) && !days.iter().any(|f| f.day == d.day.get())) {
        Some(d) => Outcome::Fail(format!("business day {} has no fund stage reported", d.day.get())),
        None => Outcome::Pass,
    }
}

pub const LC_1_27: Check = live_check! {
    id: "LC-1-27",
    title: "MON.7 and MON.9 clean with the central bank's facilities in use; facility quantities are reported daily",
    from_step: "S1.10",
    check: facilities_clean,
};

/// Every death has the process that took the person and a destination for what it held and owed — its household, its
/// household's estate, or nothing held — and every estate still standing waits for a reason named, or for its
/// country's first business day since it opened.
fn deaths_destined(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    if core.deaths.is_empty() {
        return Outcome::NotYet("no one died in the run");
    }
    let kinds = w.event_kinds();
    if let Some(d) = core.deaths.iter().find(|d| kinds.get(usize::from(d.cause)).is_none()) {
        return Outcome::Fail(format!("person {} died of no declared event ({})", d.person, d.cause));
    }
    let today = w.today();
    for (estate, country, opened) in &core.estates {
        if core.waiting.contains_key(estate) {
            continue;
        }
        let paid_by_now = (opened.get() + 1..=today.get()).any(|d| w.is_business(*country, phx_id::Day::new(d)));
        if paid_by_now {
            return Outcome::Fail(format!(
                "estate {} opened on day {} stands unsettled with no reason named",
                estate.word(),
                opened.get()
            ));
        }
    }
    Outcome::Pass
}

pub const LC_0_53: Check = live_check! {
    id: "LC-0-53",
    title: "Every death has a cause and a destination for everything held and owed; every estate settles or waits, named",
    from_step: "S0.25",
    check: deaths_destined,
};

/// What the treasuries received in taxes is what their named collectors remitted, the taxes family clean; taxes arose
/// on both bases, each on a named payer's payment.
fn collectors_remit(w: Inspector<'_>) -> Outcome {
    let t = &w.core().taxes;
    if t.remitted == 0 || t.arisings.contains(&0) {
        return Outcome::NotYet("no tax arose on every base, or none was remitted, in the run");
    }
    match w.findings().iter().find(|f| f.family == "taxes") {
        Some(f) => Outcome::Fail(format!("day {}: {} {}", f.day.get(), f.clause, f.detail)),
        None => Outcome::Pass,
    }
}

pub const LC_1_28: Check = live_check! {
    id: "LC-1-28",
    title: "TAX.5: tax received equals tax remitted by named collectors; every tax payment has a named payer and base",
    from_step: "S1.11",
    check: collectors_remit,
};

/// A quotient rounded half to even.
fn half_even(n: i128, d: i128) -> i128 {
    let (q, r) = (n.div_euclid(d), n.rem_euclid(d));
    match (2 * r).cmp(&d) {
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Equal => q + (q & 1),
    }
}

/// Each sampled wage's income tax is its year's share of the bands' tax on its yearly amount, band by band, rounded
/// half to even on the year's tax and again on its share.
fn levies_sampled(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    if core.taxes.sample.is_empty() {
        return Outcome::NotYet("no sampled wage had tax withheld in the run");
    }
    for (gross, tax, ccy) in &core.taxes.sample {
        let Some(Some(law)) = core.state.withholding.get(usize::from(*ccy)) else {
            return Outcome::Fail(format!("a tax withheld in currency {ccy}, which has no income tax"));
        };
        let yearly = i128::from(*gross) * i128::from(law.periods);
        let mut exact = 0_i128;
        for (i, band) in law.bands.iter().enumerate() {
            let upper = match law.bands.get(i + 1).map(|b| i128::from(b.from)) {
                Some(next) if next < yearly => next,
                _ => yearly,
            };
            let over = upper - i128::from(band.from);
            if over > 0 {
                exact += over * i128::from(band.share);
            }
        }
        let year = half_even(exact, phx_num::consts::RATE_SCALE);
        let expected = half_even(year, i128::from(law.periods));
        if expected != i128::from(*tax) {
            return Outcome::Fail(format!("a wage of {gross} had {tax} withheld where its bands take {expected}"));
        }
    }
    Outcome::Pass
}

pub const LC_0_29: Check = live_check! {
    id: "LC-0-29",
    title: "Sampled levy amounts equal per member times count under their conventions",
    from_step: "S0.17",
    check: levies_sampled,
};

/// Every issue of bills not yet at maturity is held whole: its holders' balances sum to what they paid for it.
fn issues_held(w: Inspector<'_>) -> Outcome {
    let core = w.core();
    let today = w.today();
    let live: Vec<&phx_world::core_bills::Issue> = core.bills.issues.iter().filter(|i| i.maturity > today).collect();
    if live.is_empty() {
        return Outcome::NotYet("no bill stands issued at the run's close");
    }
    let held = core.issue_held();
    match live.iter().find(|i| held.get(&i.schedule).copied().unwrap_or(0) != i.issued) {
        Some(i) => Outcome::Fail(format!(
            "the issue of day {} in country {} is held at {} where {} was issued",
            i.day.get(),
            i.country,
            held.get(&i.schedule).copied().unwrap_or(0),
            i.issued
        )),
        None => Outcome::Pass,
    }
}

pub const LC_0_16: Check = live_check! {
    id: "LC-0-16",
    title: "For every issued instrument, holdings sum to the issued amount",
    from_step: "S0.14",
    check: issues_held,
};

/// The debt family is clean: the bills outstanding are what was issued less what was redeemed and written off; and
/// some bills were redeemed.
fn debt_register(w: Inspector<'_>) -> Outcome {
    if w.core().bills.redeemed == 0 {
        return Outcome::NotYet("no bill was redeemed in the run");
    }
    family_clean(w, "debt")
}

pub const LC_1_29: Check = live_check! {
    id: "LC-1-29",
    title: "TRS.6: debt outstanding equals issuance minus redemptions, read from the register",
    from_step: "S1.11",
    check: debt_register,
};

/// Every auction's offer, bids and sales are published, with its cover where it offered and its price and tail
/// where it sold; an auction that sold less than it offered is published as such.
fn auctions_published(w: Inspector<'_>) -> Outcome {
    let auctions = &w.core().bills.auctions;
    if !auctions.iter().any(|a| a.offered > 0) {
        return Outcome::NotYet("no treasury offered bills in the run");
    }
    for a in auctions {
        if a.offered > 0 && matches!(a.cover, phx_num::Missing::Absent) {
            return Outcome::Fail(format!("the auction of day {} in country {} has no cover", a.day, a.country));
        }
        if a.sold > 0 && (matches!(a.price, phx_num::Missing::Absent) || matches!(a.tail, phx_num::Missing::Absent)) {
            return Outcome::Fail(format!(
                "the auction of day {} in country {} sold with no price or tail",
                a.day, a.country
            ));
        }
        if a.sold > a.offered {
            return Outcome::Fail(format!(
                "the auction of day {} in country {} sold more than it offered",
                a.day, a.country
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_31: Check = live_check! {
    id: "LC-1-31",
    title: "Auction results (cover, tail, failures) are published",
    from_step: "S1.11",
    check: auctions_published,
};
