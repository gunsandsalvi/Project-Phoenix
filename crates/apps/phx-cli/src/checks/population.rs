use phx_core::SubStep;
use phx_pop::explicit::role_counts;
use phx_world::Inspector;
use phx_world::rates::RateTally;

use super::{Check, Outcome};
use crate::live_check;

/// The world keeps no cell until the households and small firms are opened.
const NO_CELLS: &str = "the world keeps no cell before the households and small firms are opened (S0.25)";

/// How many standard deviations of its sampling error a realised count may stand from its expectation: at this many,
/// a year's few hundred tallies all pass by chance but for about one run in a thousand.
const RATE_SIGMAS: f64 = 5.0;

/// Whether any population kind keeps a cell.
fn keeps_cells(w: Inspector<'_>) -> bool {
    (0..w.population().kinds.len()).any(|k| w.cell_table(k).slots().next().is_some())
}

/// The first finding of a clause, if the audit made one.
fn finding(w: Inspector<'_>, clause: &str) -> Option<String> {
    w.findings().iter().find(|f| f.clause == clause).map(|f| format!("day {}: {}", f.day.get(), f.detail))
}

/// In every cell, each profile group counts every person of its role — the weight times the key's count — at the
/// run's end, and the audit found no cell otherwise at any close.
fn profiles_sum_to_weights(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    if let Some(f) = finding(w, "REP.14") {
        return Outcome::Fail(format!("the audit found a profile off its weight, {f}"));
    }
    for (k, kd) in w.population().kinds.iter().enumerate() {
        let table = w.cell_table(k);
        for slot in table.slots() {
            let counts = role_counts(&kd.decl, &kd.keys.record(table.hot(slot).key_id));
            let (weight, profile) = (u64::from(table.weight(slot).get()), table.profile(slot));
            for (g, group) in kd.decl.groups.iter().enumerate() {
                let want = weight * counts.get(group.role).map_or(0, |n| u64::from(*n));
                if profile.members(g) != want {
                    let party = table.party(slot).get();
                    return Outcome::Fail(format!(
                        "cell {party}'s {} counts {} of {want}",
                        group.name,
                        profile.members(g)
                    ));
                }
            }
        }
    }
    Outcome::Pass
}

/// Every cell's landing key is the one its key, positions and kinks give now.
fn landing_keys_current(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    for (k, kd) in w.population().kinds.iter().enumerate() {
        let table = w.cell_table(k);
        if let Some(slot) = table.slots().find(|s| table.hot(*s).landing_key != table.landing(*s, &kd.decl, &kd.levels))
        {
            return Outcome::Fail(format!(
                "cell {}'s landing key is not the one it holds now",
                table.party(slot).get()
            ));
        }
    }
    Outcome::Pass
}

pub const LC_0_37: Check = live_check! {
    id: "LC-0-37",
    title: "In every cell, the profile counts sum to the weight in each role, every close",
    from_step: "S0.21",
    check: profiles_sum_to_weights,
};

pub const LC_0_38: Check = live_check! {
    id: "LC-0-38",
    title: "Every cell's landing key equals the one recomputed from its key, positions and kinks",
    from_step: "S0.21",
    check: landing_keys_current,
};

/// A tally's distance from its expectation in standard deviations of its sampling error.
fn sigmas(t: &RateTally) -> f64 {
    (phx_rand::float::from_u64(t.realised) - t.expected) / t.variance.sqrt()
}

/// The tallies summed.
fn total<'a>(tallies: impl Iterator<Item = &'a RateTally>) -> RateTally {
    tallies.fold(RateTally::default(), |a, t| RateTally {
        expected: a.expected + t.expected,
        variance: a.variance + t.variance,
        realised: a.realised + t.realised,
    })
}

/// A tally within its sampling error, or why not.
fn within(what: &str, t: &RateTally) -> Result<(), String> {
    if t.variance > 0.0 && sigmas(t).abs() <= RATE_SIGMAS {
        return Ok(());
    }
    if t.variance <= 0.0 && t.realised == 0 {
        return Ok(());
    }
    Err(format!("{what}: {} hits against {:.1} expected ({:.1} sigmas)", t.realised, t.expected, sigmas(t)))
}

/// Every process's realised hits over the sampled cells within their sampling error of its declared rates, in total
/// and at each value class and year; a process nobody could be hit by over the run is dead.
fn realised_rates(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    if processes.is_empty() {
        return Outcome::NotYet(NO_CELLS);
    }
    let rates = &w.rates().0;
    for (p, (hazard, _, _)) in (0_u32..).zip(&processes) {
        let t = total(rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)).map(|(_, t)| t));
        if t.expected <= 0.0 {
            return Outcome::Fail(format!("{hazard} could hit nobody over the run"));
        }
        if let Err(e) = within(hazard, &t) {
            return Outcome::Fail(e);
        }
        for ((_, year, class), t) in rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)) {
            if let Err(e) = within(&format!("{hazard} in {year} at value class {class}"), t) {
                return Outcome::Fail(e);
            }
        }
    }
    Outcome::Pass
}

/// No cell is screened but for its agenda's rows: each day's screens are of rows it gathered, and its hits of screens.
fn only_the_active(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    let processes = phx_rand::float::len_u64(w.processes().len());
    for d in w.cell_days() {
        if d.screened > d.gathered * processes || d.hits > d.screened {
            return Outcome::Fail(format!(
                "day {}: {} screens and {} hits of {} rows gathered",
                d.day.get(),
                d.screened,
                d.hits,
                d.gathered
            ));
        }
    }
    Outcome::Pass
}

/// Every hit's event is recorded at 3b, where the hit was drawn: the events of the processes' kinds recorded there
/// number the hits.
fn hazards_recorded(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    if processes.is_empty() {
        return Outcome::NotYet(NO_CELLS);
    }
    let kinds: Vec<u16> = processes.iter().map(|(_, _, e)| *e).collect();
    let events = w.events();
    let mut recorded = 0_u64;
    for id in 1..=phx_rand::float::len_u64(events.len()) {
        let e = events.get(id);
        if kinds.contains(&e.kind) {
            if e.substep != SubStep::S3b.ordinal() {
                return Outcome::Fail(format!("a hit's event {id} recorded at sub-step {}", e.substep));
            }
            recorded += 1;
        }
    }
    let hits: u64 = w.cell_days().iter().map(|d| d.hits).sum();
    if recorded == hits { Outcome::Pass } else { Outcome::Fail(format!("{recorded} events for {hits} hits")) }
}

/// Carried needs and notices decided on the first day their decision point ran.
fn carried_decided(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("no decision point carries a need or notice before the first decisions (Stage 1)")
}

pub const LC_0_39: Check = live_check! {
    id: "LC-0-39",
    title: "Every hazard's realised hit rate is within its sampling error of its declared rate",
    from_step: "S0.22",
    check: realised_rates,
};

pub const LC_0_40: Check = live_check! {
    id: "LC-0-40",
    title: "No cell is visited on a day it had no agenda entry",
    from_step: "S0.22",
    check: only_the_active,
};

pub const LC_0_41: Check = live_check! {
    id: "LC-0-41",
    title: "Every hazard occurrence has its event recorded at the sub-step that drew it",
    from_step: "S0.22",
    check: hazards_recorded,
};

pub const LC_0_42: Check = live_check! {
    id: "LC-0-42",
    title: "Carried needs and notices are decided on the first day their decision point runs",
    from_step: "S0.22",
    check: carried_decided,
};

/// The Representation family ran at every close and found nothing, and each kind's weights sum to the members the
/// events that began and ended them count.
fn representation_whole(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    if let Some(f) = w.findings().iter().find(|f| f.family == "REP.representation") {
        return Outcome::Fail(format!("the Representation family found, day {}: {}", f.day.get(), f.detail));
    }
    let pop = w.population();
    for (k, (kind, members)) in pop.members.iter().enumerate() {
        let table = w.cell_table(k);
        let weights: u64 = table.slots().map(|s| u64::from(table.weight(s).get())).sum();
        if weights != *members {
            return Outcome::Fail(format!("the {kind}s weigh {weights} against {members} members counted"));
        }
    }
    Outcome::Pass
}

/// Whether any cell holds a line's row.
fn cells_hold_lines(w: Inspector<'_>) -> bool {
    (0..w.population().kinds.len()).any(|k| {
        let t = w.cell_table(k);
        t.slots().any(|s| !t.words(s, phx_pop::table::CellList::Rows).is_empty())
    })
}

const NO_LINES: &str =
    "no cell holds a line, so no kink or straight rule reaches a landing, before the opening lines (S0.25d)";

/// Sampled landings re-checked against both sides' kinks from their records.
fn no_kink_crossed(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    if !cells_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let days = w.cell_days();
    let reread: u64 = days.iter().map(|d| d.reread).sum();
    if reread == 0 {
        return Outcome::Fail("no landing was re-read over the run".to_owned());
    }
    match days.iter().find(|d| d.kinks_crossed > 0) {
        Some(d) => Outcome::Fail(format!(
            "day {}: {} rows of re-read landings moved across a point of their line's kinks",
            d.day.get(),
            d.kinks_crossed
        )),
        None => Outcome::Pass,
    }
}

/// Sampled landings' straight rules' totals unchanged at the moment of landing: every line side's members and balance
/// the cell and its parts held are the cell's after the join, a straight rule being linear in them.
fn straight_rules_exact(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    if !cells_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let days = w.cell_days();
    if days.iter().map(|d| d.reread_sides).sum::<u64>() == 0 {
        return Outcome::Fail("no re-read landing held a line".to_owned());
    }
    match days.iter().find(|d| d.sides_unkept > 0) {
        Some(d) => Outcome::Fail(format!(
            "day {}: {} line sides of re-read landings lost or gained members or balance in the join",
            d.day.get(),
            d.sides_unkept
        )),
        None => Outcome::Pass,
    }
}

/// The representation's costs reported each day: the dispersion joins erased, and splits, landings and new cells,
/// one record a day from the first.
fn costs_reported(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    let days = w.cell_days();
    let first = w.day_zero().succ();
    for (i, d) in (0_u32..).zip(days) {
        if d.day.get() != first.get() + i {
            return Outcome::Fail(format!("no record of the cells' costs on day {}", first.get() + i));
        }
        if d.erased.is_nan() {
            return Outcome::Fail(format!("day {}'s dispersion erased is no number", d.day.get()));
        }
    }
    if days.iter().map(|d| d.parts).sum::<u64>() == 0 {
        return Outcome::Fail("no cell split over the run".to_owned());
    }
    Outcome::Pass
}

pub const LC_0_43: Check = live_check! {
    id: "LC-0-43",
    title: "Weights sum to each population, lines' sides are equal and attachments reconcile with profiles, every close",
    from_step: "S0.23",
    check: representation_whole,
};

pub const LC_0_44: Check = live_check! {
    id: "LC-0-44",
    title: "No landing crossed a kink: sampled landings re-checked against both sides' kinks",
    from_step: "S0.23",
    check: no_kink_crossed,
};

pub const LC_0_45: Check = live_check! {
    id: "LC-0-45",
    title: "Sampled landings leave every straight rule's total unchanged to a smallest unit per member",
    from_step: "S0.23",
    check: straight_rules_exact,
};

pub const LC_0_46: Check = live_check! {
    id: "LC-0-46",
    title: "The representation's costs are reported per day: dispersion erased, splits, landings and new cells",
    from_step: "S0.23",
    check: costs_reported,
};

/// The cells carried never exceed the cell budget at any close.
fn within_cell_budget(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    match w.cell_days().iter().find(|d| d.cells > d.budget) {
        Some(d) => Outcome::Fail(format!("day {}: {} cells against a budget of {}", d.day.get(), d.cells, d.budget)),
        None => Outcome::Pass,
    }
}

/// Every party within the promotion rank an individual at each monthly read; none with a public instrument in a cell.
fn ranks_carried(w: Inspector<'_>) -> Outcome {
    if w.population().kinds.iter().any(|k| k.ranks.is_some()) {
        return Outcome::Fail("a kind reads ranks but its promotions are not recorded for this check".to_owned());
    }
    Outcome::NotYet("no population kind reads ranks before the firms' positions (S1.03)")
}

/// The representation's measures reported every day, with the share of each population at weight one.
fn measures_reported(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    for d in w.cell_days() {
        if d.members == 0 || d.at_weight_one > d.cells + d.individuals {
            return Outcome::Fail(format!(
                "day {}: {} members, {} at weight one of {} rows",
                d.day.get(),
                d.members,
                d.at_weight_one,
                d.cells + d.individuals
            ));
        }
    }
    Outcome::Pass
}

/// The identity hash equal immediately before and after each renumbering slice.
fn renumbering_changes_no_identity(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    let slices: Vec<_> = w.cell_days().iter().filter(|d| d.renumbered > 0).collect();
    if slices.is_empty() {
        return Outcome::NotYet("no light day has come to renumber on");
    }
    match slices.iter().find(|d| d.renumber_changed > 0) {
        Some(d) => Outcome::Fail(format!("day {}'s renumbering changed what the cells hold", d.day.get())),
        None => Outcome::Pass,
    }
}

pub const LC_0_47: Check = live_check! {
    id: "LC-0-47",
    title: "The cells carried never exceed the cell budget at any close",
    from_step: "S0.24",
    check: within_cell_budget,
};

pub const LC_0_48: Check = live_check! {
    id: "LC-0-48",
    title: "Every party within the promotion rank is an individual, and no individual with a public instrument is in a cell",
    from_step: "S0.24",
    check: ranks_carried,
};

pub const LC_0_49: Check = live_check! {
    id: "LC-0-49",
    title: "The representation's measures are reported every day, with the share of each population at weight one",
    from_step: "S0.24",
    check: measures_reported,
};

pub const LC_0_50: Check = live_check! {
    id: "LC-0-50",
    title: "The identity hash is equal immediately before and after each renumbering slice",
    from_step: "S0.24",
    check: renumbering_changes_no_identity,
};

/// Day one passes every audit family with the full population.
fn day_one_whole(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    let one = w.day_zero().succ();
    let Some(close) = w.closes().iter().find(|c| c.day == one) else {
        return Outcome::Fail("the audit did not close day one".to_owned());
    };
    if close.families != w.families().len() {
        return Outcome::Fail(format!("day one closed {} of {} families", close.families, w.families().len()));
    }
    match w.findings().iter().find(|f| f.day == one) {
        Some(f) => Outcome::Fail(format!("{} on day one: {}", f.family, f.detail)),
        None => Outcome::Pass,
    }
}

/// The persons reconcile day by day — each day's persons are the last day's less those gone, none being born or
/// entering yet — and every household holds a head.
fn populations_reconcile(w: Inspector<'_>) -> Outcome {
    if !keeps_cells(w) {
        return Outcome::NotYet(NO_CELLS);
    }
    for pair in w.cell_days().windows(2) {
        let [before, after] = pair else { continue };
        if before.persons.checked_sub(after.gone) != Some(after.persons) {
            return Outcome::Fail(format!(
                "day {}: {} persons, {} gone, {} left",
                after.day.get(),
                before.persons,
                after.gone,
                after.persons
            ));
        }
    }
    for (k, kd) in w.population().kinds.iter().enumerate() {
        let table = w.cell_table(k);
        for slot in table.slots() {
            if role_counts(&kd.decl, &kd.keys.record(table.hot(slot).key_id)).iter().all(|n| *n == 0) {
                return Outcome::Fail(format!("cell {} holds households of nobody", table.party(slot).get()));
            }
        }
    }
    Outcome::Pass
}

pub const LC_0_51: Check = live_check! {
    id: "LC-0-51",
    title: "Day one passes every audit family with the full population",
    from_step: "S0.25",
    check: day_one_whole,
};

pub const LC_0_52: Check = live_check! {
    id: "LC-0-52",
    title: "The populations reconcile: births, deaths and entries; every person in one household; every household held",
    from_step: "S0.25",
    check: populations_reconcile,
};

/// Every death has a cause and a destination for everything held and owed; every estate distributes and ends.
fn estates_settle(w: Inspector<'_>) -> Outcome {
    if !cells_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let days = w.cell_days();
    let (Some(last), opened) = (days.last(), days.iter().map(|d| d.estates).sum::<u64>()) else {
        return Outcome::NotYet(NO_CELLS);
    };
    let settled: u64 = days.iter().map(|d| d.estates_settled).sum();
    let books = w.books();
    let open: Vec<phx_id::PartyId> = books.parties.of_kind(phx_core::ESTATE_KIND.name).collect();
    if settled + phx_rand::float::len_u64(open.len()) != opened {
        return Outcome::Fail(format!("{opened} estates opened, {settled} settled and {} open", open.len()));
    }
    // An estate still open at the close opened on the last day, or waited on a payment that failed that day.
    let stale = open
        .iter()
        .filter(|p| {
            let (place, slot) = books.parties.row(**p);
            books.parties.table(place).created(slot) < last.day
        })
        .count();
    if phx_rand::float::len_u64(stale) != last.estates_waiting {
        return Outcome::Fail(format!(
            "{stale} estates open from before the last day, {} counted waiting",
            last.estates_waiting
        ));
    }
    Outcome::Pass
}

pub const LC_0_53: Check = live_check! {
    id: "LC-0-53",
    title: "Every death has a cause and a destination for everything held and owed; every estate settles or waits, named",
    from_step: "S0.25",
    check: estates_settle,
};

/// The processes on lives whose rates this check holds: death and the onset of disability.
const LIFE_HAZARDS: [&str; 2] = ["DEM.death", "DEM.disability_onset"];

/// Realised mortality and illness per age class within their sampling error of the declared tables, each year.
fn life_rates_by_class(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    let lives: Vec<(u32, &str)> = (0_u32..)
        .zip(&processes)
        .filter(|(_, (h, _, _))| LIFE_HAZARDS.contains(h))
        .map(|(p, (h, _, _))| (p, *h))
        .collect();
    if lives.len() != LIFE_HAZARDS.len() {
        return Outcome::NotYet(NO_CELLS);
    }
    let bounds = match w.register().partition("DEM.age_classes") {
        Ok(p) => p.bounds.clone(),
        Err(e) => return Outcome::Fail(e),
    };
    let class = |age: i64| bounds.partition_point(|b| *b <= age);
    let mut by_class: std::collections::BTreeMap<(u32, u32, usize), RateTally> = std::collections::BTreeMap::new();
    for ((p, year, born), t) in &w.rates().0 {
        if !lives.iter().any(|(q, _)| q == p) {
            continue;
        }
        let age = i64::from(*year) - i64::from(if_pop::consts::FIRST_BIRTH_YEAR) - i64::from(*born);
        let slot = by_class.entry((*p, *year, class(age))).or_default();
        *slot = total([*slot, *t].iter());
    }
    for ((p, year, c), t) in &by_class {
        let hazard = lives.iter().find(|(q, _)| q == p).map_or("", |(_, h)| h);
        if let Err(e) = within(&format!("{hazard} in {year} at age class {c}"), t) {
            return Outcome::Fail(e);
        }
    }
    Outcome::Pass
}

pub const LC_0_54: Check = live_check! {
    id: "LC-0-54",
    title: "Realised mortality and illness per age class match their declared tables within sampling error",
    from_step: "S0.25",
    check: life_rates_by_class,
};

/// Paydays, dues and pensions in payment settle through pooled flows, every fail of a contract's due with the system
/// its line kind names to decide on it; and no row held by persons counts more members than its cell holds persons of
/// the roles that hold it, so the dead are paid nothing.
fn lines_pay(w: Inspector<'_>) -> Outcome {
    if !cells_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let settled = w.settlements();
    if settled.iter().map(|s| s.dues.payments).sum::<u64>() == 0 {
        return Outcome::Fail("no due was paid over the run".to_owned());
    }
    let lines = &w.books().ledger.lines;
    for s in settled {
        for f in &s.fails {
            if let phx_num::Missing::Present(row) = f.row
                && lines.decl(row.line).transfer_requesters.is_empty()
            {
                return Outcome::Fail(format!(
                    "day {}: a fail on a {} line, whose kind names no system to decide on it",
                    s.day.get(),
                    lines.kind_name(row.line)
                ));
            }
        }
    }
    for (k, kd) in w.population().kinds.iter().enumerate() {
        let table = w.cell_table(k);
        for slot in table.slots() {
            let counts = role_counts(&kd.decl, &kd.keys.record(table.hot(slot).key_id));
            let weight = u64::from(table.weight(slot).get());
            for r in phx_ledger::rows::iter(table, slot) {
                let roles = lines.side_decl(r.row.line, r.side()).holder_roles;
                if roles.is_empty() {
                    continue;
                }
                let persons: u64 = kd
                    .decl
                    .roles
                    .iter()
                    .zip(&counts)
                    .filter(|(role, _)| roles.contains(&role.item.name))
                    .map(|(_, n)| weight * u64::from(*n))
                    .sum();
                if u64::from(r.row.count) > persons {
                    return Outcome::Fail(format!(
                        "cell {} holds {} members of a {} line with {persons} persons to hold them",
                        table.party(slot).get(),
                        r.row.count,
                        lines.kind_name(r.row.line)
                    ));
                }
            }
        }
    }
    Outcome::Pass
}

/// The GEN report lists the counterparties each derived side was apportioned over, what each drew and what it was
/// given; no party of no drawn size was given any. A stratum with no eligible counterparty stops the opening, so none
/// is left unmatched.
fn apportionments_reported(w: Inspector<'_>) -> Outcome {
    if !cells_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let report = &w.opening().apportioned;
    if report.is_empty() {
        return Outcome::Fail("the opening reports no apportionment".to_owned());
    }
    match report.iter().find(|a| a.drawn == 0 && a.realised > 0) {
        Some(a) => {
            Outcome::Fail(format!("{}: party {} of no drawn size was given {}", a.stratum, a.party.get(), a.realised))
        }
        None => Outcome::Pass,
    }
}

pub const LC_0_55: Check = live_check! {
    id: "LC-0-55",
    title: "Paydays, dues and pensions in payment settle through pooled flows; every fail has a cause and a waiting owner",
    from_step: "S0.25",
    check: lines_pay,
};

pub const LC_0_56: Check = live_check! {
    id: "LC-0-56",
    title: "The GEN report lists every apportionment difference and every unmatched stratum",
    from_step: "S0.25",
    check: apportionments_reported,
};
