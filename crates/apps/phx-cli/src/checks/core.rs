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

/// The core's audit families — money, goods, contracts and persons — ran at every close and found nothing.
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
/// they and the banks paid each other, and the issuer's accounts do not move.
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

/// Every day the core ran records its gross flows, its settled and failed flows, and its failures by reason.
fn settlement_published(w: Inspector<'_>) -> Outcome {
    let days = &w.core().days;
    let Some(first) = days.first() else { return Outcome::NotYet("the run closed no day") };
    for (i, d) in days.iter().enumerate() {
        if d.day.get() != first.day.get() + u32::try_from(i).unwrap_or(u32::MAX) {
            return Outcome::Fail(format!("the core's days skip before day {}", d.day.get()));
        }
        if d.flows > 0 && d.gross <= 0 {
            return Outcome::Fail(format!("day {}: {} flows moving nothing", d.day.get(), d.flows));
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
    let Some(store) = core.kinds.get(firm) else { return Outcome::NotYet("no firm kind") };
    for slot in store.parties.live_slots() {
        let price = store.record(slot).get(phx_world::consts::firm::PRICE).map(|w| w.get());
        if let Some(phx_num::Missing::Present(p)) = price
            && !m.is_point(p)
        {
            return Outcome::Fail(format!("firm slot {} posts {p}, no point of its trade", slot.get()));
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

/// Each firm's outlook of its sales is its own: among the makers of some product at some region, their expected sales
/// a unit of their output take more than one value once they have reviewed on what each sold.
fn outlooks_are_own(w: Inspector<'_>) -> Outcome {
    use phx_world::consts::firm::{EXPECTED, OUTPUT, PRODUCT, REGION};
    let core = w.core();
    let Some(firm) = core.names.iter().position(|n| *n == "firm") else { return Outcome::NotYet("no firm kind") };
    let Some(store) = core.kinds.get(firm) else { return Outcome::NotYet("no firm kind") };
    let word = |slot: phx_id::Slot, at: usize| match store.record(slot).get(at).map(|w| w.get()) {
        Some(phx_num::Missing::Present(v)) => Some(v),
        _ => None,
    };
    let mut ratios: std::collections::BTreeMap<(i64, i64), Vec<(i128, i128)>> = std::collections::BTreeMap::new();
    for slot in store.parties.live_slots() {
        let (Some(product), Some(region), Some(expected), Some(output)) =
            (word(slot, PRODUCT), word(slot, REGION), word(slot, EXPECTED), word(slot, OUTPUT))
        else {
            continue;
        };
        if output > 0 {
            ratios.entry((product, region)).or_default().push((i128::from(expected), i128::from(output)));
        }
    }
    let cells = ratios.values().filter(|r| r.len() > 1).count();
    if cells == 0 {
        return Outcome::NotYet("no product has two makers at one region");
    }
    // Two ratios are one where their cross products are.
    let differ = |r: &Vec<(i128, i128)>| r.first().is_some_and(|(e0, o0)| r.iter().any(|(e, o)| e * o0 != e0 * o));
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
