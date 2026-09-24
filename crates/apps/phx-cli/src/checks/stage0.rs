use phx_core::{SUB_STEPS, SubStepInfo, SubStepKind};
use phx_id::Day;
use phx_world::{Dispatch, Inspector};

use super::{Check, Outcome};
use crate::live_check;

fn stage(info: &SubStepInfo) -> &str {
    info.label.trim_end_matches(|c: char| c.is_ascii_lowercase())
}

/// The sub-steps that should have run on a day, by the table alone: the audit's every day, a kernel apply of a stage
/// that runs, and a sub-step with handlers, unless it runs only on business days and no country has one.
fn expected(w: Inspector<'_>, day: Day) -> Vec<u8> {
    let any = w.any_business(day);
    SUB_STEPS
        .iter()
        .filter(|info| {
            let stage_runs = any || SUB_STEPS.iter().any(|i| stage(i) == stage(info) && !i.business_only);
            match w.dispatches(info.step) {
                Dispatch::Audit => true,
                Dispatch::KernelApply => stage_runs,
                Dispatch::KernelWork | Dispatch::Handlers => any || !info.business_only,
                Dispatch::Idle => false,
            }
        })
        .map(|info| info.step.ordinal())
        .collect()
}

fn days(w: Inspector<'_>) -> impl Iterator<Item = Day> {
    let (first, last) = (w.day_zero().succ(), w.today());
    std::iter::successors(Some(first), move |d| (*d < last).then(|| d.succ())).filter(move |d| *d <= last)
}

fn substeps_ran(w: Inspector<'_>) -> Outcome {
    let records = w.substep_records();
    for day in days(w) {
        let ran: Vec<u8> = records.iter().filter(|r| r.day == day).map(|r| r.substep).collect();
        let should = expected(w, day);
        if ran != should {
            return Outcome::Fail(format!("{:?}: ran {ran:?}, should have run {should:?}", w.date(day)));
        }
    }
    let kinds = SUB_STEPS.iter().filter(|i| i.kind == SubStepKind::KernelApply).count();
    if w.substep_count() == SUB_STEPS.len() && kinds > 0 {
        Outcome::Pass
    } else {
        Outcome::Fail("no sub-step table".to_owned())
    }
}

fn turns_cover_days(w: Inspector<'_>) -> Outcome {
    let mut next = w.day_zero().succ();
    for t in w.turn_records() {
        if t.first != next {
            return Outcome::Fail(format!("a turn begins on {:?}, not the day after the last", w.date(t.first)));
        }
        if !w.any_business(t.last) {
            return Outcome::Fail(format!("a turn ends on {:?}, a business day nowhere", w.date(t.last)));
        }
        let mut day = t.first;
        let mut count = 1;
        while day < t.last {
            if w.any_business(day) {
                return Outcome::Fail(format!("a turn runs past {:?}, a business day somewhere", w.date(day)));
            }
            day = day.succ();
            count += 1;
        }
        if count != t.days {
            return Outcome::Fail(format!("a turn of {count} days recorded as {}", t.days));
        }
        next = t.last.succ();
    }
    if next == w.today().succ() { Outcome::Pass } else { Outcome::Fail("the turns stop before today".to_owned()) }
}

fn trace_clean(w: Inspector<'_>) -> Outcome {
    let t = w.trace().total();
    match (w.read_traced(), w.trace().clean()) {
        (false, _) => Outcome::Fail("the run was not read-traced".to_owned()),
        (true, true) => Outcome::Pass,
        (true, false) => Outcome::Fail(format!(
            "{} undeclared reads, {} reads of a later write, {} duplicate opens",
            t.undeclared_reads, t.later_writes, t.duplicate_opens
        )),
    }
}

fn records_dated(w: Inspector<'_>) -> Outcome {
    let bad =
        w.record_dates().into_iter().find(|(day, step)| *day > w.today() || usize::from(*step) >= SUB_STEPS.len());
    bad.map_or(Outcome::Pass, |(day, step)| Outcome::Fail(format!("a record dated day {} sub-step {step}", day.get())))
}

fn audit_ran(w: Inspector<'_>) -> Outcome {
    let families = w.families().len();
    let mut closes = w.closes().iter();
    for day in days(w) {
        match closes.next() {
            Some(c) if c.day == day && c.families == families => {}
            Some(c) => {
                let (date, ran) = (w.date(day), c.families);
                return Outcome::Fail(format!("{date:?}: the close ran {ran} of {families} families"));
            }
            None => return Outcome::Fail(format!("{:?}: no close", w.date(day))),
        }
    }
    if closes.next().is_some() {
        return Outcome::Fail("a close beyond the last day".to_owned());
    }
    match w.findings().first() {
        None => Outcome::Pass,
        Some(f) => Outcome::Fail(format!(
            "{} findings, the first {} ({}): {}",
            w.findings().len(),
            f.family,
            f.clause,
            f.detail
        )),
    }
}

fn families_lit_alone(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("an injection needs a save loaded apart, and saves arrive with persistence (S0.20)")
}

pub const LC_0_01: Check = live_check! {
    id: "LC-0-01",
    title: "Every day has a record for every sub-step that should have run, and none for any that should not",
    from_step: "S0.11",
    check: substeps_ran,
};

pub const LC_0_02: Check = live_check! {
    id: "LC-0-02",
    title: "Every turn ends on a day that is a business day somewhere and covers every day since the last turn",
    from_step: "S0.11",
    check: turns_cover_days,
};

pub const LC_0_03: Check = live_check! {
    id: "LC-0-03",
    title: "read-trace found no undeclared read, no read of a later write and no duplicate stream open",
    from_step: "S0.11",
    check: trace_clean,
};

pub const LC_0_04: Check = live_check! {
    id: "LC-0-04",
    title: "Every record is dated with the day and sub-step that wrote it",
    from_step: "S0.11",
    check: records_dated,
};

pub const LC_0_05: Check = live_check! {
    id: "LC-0-05",
    title: "The world hash is the same whatever the worker count",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; the world runs once, and determinism is carried by construction",
};

pub const LC_0_06: Check = live_check! {
    id: "LC-0-06",
    title: "The world hash is the same whatever the registration order",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; canonical handler ids carry it by construction",
};

pub const LC_0_07: Check = live_check! {
    id: "LC-0-07",
    title: "Looking at the world changes nothing",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; observers reach the world only through &self",
};

pub const LC_0_08: Check = live_check! {
    id: "LC-0-08",
    title: "Adding a stream changes no other stream's draws",
    from_step: "S0.11",
    retired: "it compared the world with a second run of it; a stream's key comes from its own name alone",
};

pub const LC_0_09: Check = live_check! {
    id: "LC-0-09",
    title: "Every close ran every declared audit family, and they found nothing",
    from_step: "S0.12",
    check: audit_ran,
};

pub const LC_0_10: Check = live_check! {
    id: "LC-0-10",
    title: "Each family's injection into the day-30 save lights that family alone",
    from_step: "S0.12",
    check: families_lit_alone,
};
