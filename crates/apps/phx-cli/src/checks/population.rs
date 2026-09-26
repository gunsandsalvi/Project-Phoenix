use phx_core::SubStep;
use phx_world::Inspector;
use phx_world::rates::RateTally;

use super::{Check, Outcome};
use crate::live_check;

/// The world keeps no agent until the households and small firms are opened.
const NO_AGENTS: &str = "the world keeps no agent before the households and small firms are opened (S0.25)";

/// How many standard deviations of its sampling error a realised count may stand from its expectation: at this many,
/// a year's few hundred tallies all pass by chance but for about one run in a thousand.
const RATE_SIGMAS: f64 = 5.0;
/// Half a hit: how near its expectation a count with no sampling error must stand, being a whole number.
const HALF_A_HIT: f64 = 0.5;

/// Whether any population kind keeps an agent.
fn keeps_agents(w: Inspector<'_>) -> bool {
    (0..w.population().kinds.len()).any(|k| w.agent_table(k).slots().next().is_some())
}

/// The first finding of a family, if the audit made one.
fn finding(w: Inspector<'_>, family: &str) -> Option<String> {
    w.findings().iter().find(|f| f.family == family).map(|f| format!("day {}: {}", f.day.get(), f.detail))
}

/// Every agent's row on each line counts its multiplicity times its attachments there, holds a whole share for each
/// twin, and every person attachment names a present person, at the run's end; and the audit found none otherwise.
fn rows_count_twins(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    if let Some(f) = finding(w, phx_pop::audit::AGENTS.name) {
        return Outcome::Fail(format!("the agents family found, {f}"));
    }
    for k in 0..w.population().kinds.len() {
        let table = w.agent_table(k);
        for slot in table.slots() {
            if let Some(g) = phx_pop::audit::agent(table, slot).first() {
                return Outcome::Fail(g.detail.clone());
            }
        }
    }
    Outcome::Pass
}

/// Every agent's multiplicity is its kind's under the representation in force, but the seated twins, the player's
/// and its counterparts', of one each, and their donors, of one twin fewer, as many as the seats.
fn multiplicities_declared(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    let k = w.population().representation.multiplicity;
    let player = match w.player_queue().player() {
        phx_num::Missing::Present(p) => p.party,
        phx_num::Missing::Absent => return Outcome::Fail("no player was seated".to_owned()),
    };
    let (mut seats, mut donors, mut player_seated) = (0_u32, 0_u32, false);
    for kind in 0..w.population().kinds.len() {
        let table = w.agent_table(kind);
        for slot in table.slots() {
            let (party, m) = (table.party(slot), table.multiplicity(slot).get());
            if m == k {
                player_seated |= party == player;
            } else if m == 1 {
                seats += 1;
                player_seated |= party == player;
            } else if m + 1 == k {
                donors += 1;
            } else {
                return Outcome::Fail(format!("agent {} of {m} twins under a factor of {k}", party.get()));
            }
        }
    }
    if !player_seated {
        return Outcome::Fail("the player is no agent of one twin".to_owned());
    }
    if k > 1 && (seats == 0 || seats != donors) {
        return Outcome::Fail(format!("{seats} seated twins and {donors} donors, where each seat has its donor"));
    }
    Outcome::Pass
}

pub const LC_0_37: Check = live_check! {
    id: "LC-0-37",
    title: "Every agent's rows count its multiplicity times its attachments, which name its persons, one a line",
    from_step: "S0.28",
    check: rows_count_twins,
};

pub const LC_0_38: Check = live_check! {
    id: "LC-0-38",
    title: "Every agent's multiplicity is its kind's, but the seated twins of one and their donors of one fewer",
    from_step: "S0.28",
    check: multiplicities_declared,
};

/// The tallies summed.
fn total<'a>(tallies: impl Iterator<Item = &'a RateTally>) -> RateTally {
    tallies.fold(RateTally::default(), |a, t| RateTally {
        expected: a.expected + t.expected,
        variance: a.variance + t.variance,
        realised: a.realised + t.realised,
    })
}

/// The chance, one-sided, of a count at least as far from its expectation as five of its standard deviations: the
/// normal tail the exact tail below is held to.
const TAIL: f64 = 2.9e-7;
/// Expectations below this many hits are judged by the exact Poisson tail, where the normal one misleads.
const POISSON_BELOW: f64 = 30.0;

/// The chance of a Poisson count of mean `lambda` at least `n` when `n` is above the mean, at most `n` when below.
fn poisson_tail(lambda: f64, n: u64) -> f64 {
    let mut pmf = (-lambda).exp();
    let (mut below, mut at) = (0.0_f64, 0_u64);
    while at < n {
        below += pmf;
        at += 1;
        pmf *= lambda / phx_rand::float::from_u64(at);
    }
    if phx_rand::float::from_u64(n) >= lambda { 1.0 - below } else { below + pmf }
}

/// A tally within its sampling error, or why not. Under twins every hit counts once for each twin, so the tally's
/// hits are its count over the factor; few expected hits are judged by the exact tail, many by the normal one.
fn within(what: &str, t: &RateTally, twins: f64) -> Result<(), String> {
    let hits = phx_rand::float::from_u64(t.realised) / twins;
    let (lambda, variance) = (t.expected / twins, t.variance / twins);
    // A certain hit, or none, has no sampling error: it is exact.
    if variance <= 0.0 && (hits - lambda).abs() < HALF_A_HIT {
        return Ok(());
    }
    let off = (hits - lambda) / variance.sqrt();
    let rounded = t.realised / twins_u64(twins);
    let fine = if lambda < POISSON_BELOW { poisson_tail(lambda, rounded) >= TAIL } else { off.abs() <= RATE_SIGMAS };
    if variance > 0.0 && fine {
        return Ok(());
    }
    Err(format!("{what}: {hits:.0} hits against {lambda:.2} expected ({off:.1} sigmas)"))
}

fn twins_u64(twins: f64) -> u64 {
    let Some(k) = phx_rand::float::floor_to_u64(twins) else {
        phx_num::violation!(clause = "REP.17", "a factor of twins beyond counting");
    };
    k
}

fn factor(w: Inspector<'_>) -> f64 {
    f64::from(w.population().representation.multiplicity)
}

/// Every process's realised hits over the sampled agents within their sampling error of its declared rates, in total
/// and at each age and year; a process nobody could be hit by over the run is dead.
fn realised_rates(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    if processes.is_empty() {
        return Outcome::NotYet(NO_AGENTS);
    }
    let rates = &w.rates().0;
    let twins = factor(w);
    for (p, (hazard, _, _)) in (0_u32..).zip(&processes) {
        let t = total(rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)).map(|(_, t)| t));
        if t.expected <= 0.0 {
            return Outcome::Fail(format!("{hazard} could hit nobody over the run"));
        }
        if let Err(e) = within(hazard, &t, twins) {
            return Outcome::Fail(e);
        }
        for ((_, year, age), t) in rates.range((p, 0, 0)..=(p, u32::MAX, u32::MAX)) {
            if let Err(e) = within(&format!("{hazard} in {year} at age {age}"), t, twins) {
                return Outcome::Fail(e);
            }
        }
    }
    Outcome::Pass
}

/// Each day every booking the agenda held for a live agent is read once, by the process it was booked for, and no
/// other is read.
fn only_the_active(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    for d in w.agent_days() {
        if d.read != d.due {
            return Outcome::Fail(format!("day {}: {} bookings read of {} due", d.day.get(), d.read, d.due));
        }
    }
    Outcome::Pass
}

/// Every hit's event is recorded at 3b, where the hit was drawn: the events of the processes' kinds recorded there
/// number the hits.
fn hazards_recorded(w: Inspector<'_>) -> Outcome {
    let processes = w.processes();
    if processes.is_empty() {
        return Outcome::NotYet(NO_AGENTS);
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
    let hits: u64 = w.agent_days().iter().map(|d| d.hits).sum();
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
    title: "Every booking the agenda holds for a day is read that day, and no other",
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

/// The agents family ran at every close and found nothing, nor did the contracts family, which holds every line's
/// sides equal; and each kind's multiplicities sum to the parties the events that began and ended them count, its
/// persons to the persons counted.
fn populations_whole(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    if let Some(f) = finding(w, phx_pop::audit::AGENTS.name) {
        return Outcome::Fail(format!("the agents family found, {f}"));
    }
    if let Some(f) = finding(w, phx_ledger::audit::CONTRACTS.name) {
        return Outcome::Fail(format!("the contracts family found, {f}"));
    }
    let pop = w.population();
    for (k, ((kind, parties), persons)) in pop.members.iter().zip(&pop.persons).enumerate() {
        let table = w.agent_table(k);
        let twins: u64 = table.slots().map(|s| u64::from(table.multiplicity(s).get())).sum();
        let held: u64 = table
            .slots()
            .map(|s| u64::from(table.multiplicity(s).get()) * phx_rand::float::len_u64(table.persons(s).len()))
            .sum();
        if twins != *parties || held != *persons {
            return Outcome::Fail(format!(
                "the {kind}s stand for {twins} parties and {held} persons against {parties} and {persons} counted"
            ));
        }
    }
    Outcome::Pass
}

/// Whether any agent holds a line's row.
fn agents_hold_lines(w: Inspector<'_>) -> bool {
    (0..w.population().kinds.len()).any(|k| {
        let t = w.agent_table(k);
        t.slots().any(|s| !t.words(s, phx_pop::table::AgentList::Rows).is_empty())
    })
}

const NO_LINES: &str = "no agent holds a line before the opening lines (S0.25d)";

/// The representation's measures reported each day, one record a day from the first: the agents, the real parties
/// they stand for, their persons, and the hits and bookings of the day.
fn measures_reported(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    let days = w.agent_days();
    let first = w.day_zero().succ();
    let k = u64::from(w.population().representation.multiplicity);
    for (i, d) in (0_u32..).zip(days) {
        if d.day.get() != first.get() + i {
            return Outcome::Fail(format!("no record of the agents' day on day {}", first.get() + i));
        }
        if d.agents == 0 || d.parties < d.agents || d.parties > d.agents * k {
            return Outcome::Fail(format!(
                "day {}: {} agents standing for {} parties at a factor of {k}",
                d.day.get(),
                d.agents,
                d.parties
            ));
        }
    }
    Outcome::Pass
}

pub const LC_0_43: Check = live_check! {
    id: "LC-0-43",
    title: "Multiplicities sum to each population, persons to the persons counted, and lines' sides are equal",
    from_step: "S0.28",
    check: populations_whole,
};

pub const LC_0_44: Check = live_check! {
    id: "LC-0-44",
    title: "No landing crossed a kink: sampled landings re-checked against both sides' kinks",
    from_step: "S0.23",
    retired: "agents are never joined, so nothing lands (S0.28)",
};

pub const LC_0_45: Check = live_check! {
    id: "LC-0-45",
    title: "Sampled landings leave every straight rule's total unchanged to a smallest unit per member",
    from_step: "S0.23",
    retired: "agents are never joined, so nothing lands (S0.28)",
};

pub const LC_0_46: Check = live_check! {
    id: "LC-0-46",
    title: "The representation's measures are reported per day: agents, parties, persons, hits and bookings",
    from_step: "S0.28",
    check: measures_reported,
};

pub const LC_0_47: Check = live_check! {
    id: "LC-0-47",
    title: "The cells carried never exceed the cell budget at any close",
    from_step: "S0.24",
    retired: "there are no cells and no cell budget; the factor is the representation's one valve (S0.28)",
};

/// Every party within the individuals' rank an individual at the opening.
fn ranks_carried(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("the individuals' rank reads the firms' positions (S1.03, F-048)")
}

pub const LC_0_48: Check = live_check! {
    id: "LC-0-48",
    title: "Every party within the individuals' rank is an individual, and no party with a public instrument is an agent",
    from_step: "S0.24",
    check: ranks_carried,
};

/// The representation named every day beside its measures, and the last day's agents, the parties they stand for
/// and those parties' persons the ones the agent tables hold at the run's end.
fn representation_named(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    let r = w.population().representation;
    if r.multiplicity == 0 || r.population_divisor == 0 {
        return Outcome::Fail(format!("the representation {} has a factor of nought", r.name()));
    }
    if w.agent_days().iter().any(|d| d.persons == 0) {
        return Outcome::Fail("a day on which the agents held no person".to_owned());
    }
    let Some(last) = w.agent_days().last() else { return Outcome::NotYet("no day has run") };
    let (mut agents, mut parties, mut persons) = (0_u64, 0_u64, 0_u64);
    for k in 0..w.population().kinds.len() {
        let table = w.agent_table(k);
        for s in table.slots() {
            let m = u64::from(table.multiplicity(s).get());
            agents += 1;
            parties += m;
            persons += m * phx_rand::float::len_u64(table.persons(s).len());
        }
    }
    if (last.agents, last.parties, last.persons) != (agents, parties, persons) {
        return Outcome::Fail(format!(
            "day {} reported {} agents, {} parties and {} persons where the tables hold {agents}, {parties} and \
             {persons}",
            last.day.get(),
            last.agents,
            last.parties,
            last.persons
        ));
    }
    Outcome::Pass
}

pub const LC_0_49: Check = live_check! {
    id: "LC-0-49",
    title: "The representation and its factor are reported every day, with its agents, parties and persons",
    from_step: "S0.28",
    check: representation_named,
};

pub const LC_0_50: Check = live_check! {
    id: "LC-0-50",
    title: "The identity hash is equal immediately before and after each renumbering slice",
    from_step: "S0.24",
    retired: "agents are never renumbered (S0.28)",
};

/// Day one passes every audit family with the full population.
fn day_one_whole(w: Inspector<'_>) -> Outcome {
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
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
    if !keeps_agents(w) {
        return Outcome::NotYet(NO_AGENTS);
    }
    for pair in w.agent_days().windows(2) {
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
        let Some(head) = kd.decl.roles.first().map(|r| r.item.name) else { continue };
        let table = w.agent_table(k);
        for slot in table.slots() {
            let h = phx_pop::explicit::household(&kd.decl, table, slot);
            if h.persons.iter().filter(|p| p.role == head).count() != 1 {
                return Outcome::Fail(format!("agent {} holds other than one {head}", table.party(slot).get()));
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
    if !agents_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let days = w.agent_days();
    let (Some(last), opened) = (days.last(), days.iter().map(|d| d.estates).sum::<u64>()) else {
        return Outcome::NotYet(NO_AGENTS);
    };
    let settled: u64 = days.iter().map(|d| d.estates_settled).sum();
    let books = w.books();
    let open: Vec<phx_id::PartyId> = books.parties.of_kind(phx_core::ESTATE_KIND.name).collect();
    if settled + phx_rand::float::len_u64(open.len()) != opened {
        return Outcome::Fail(format!("{opened} estates opened, {settled} settled and {} open", open.len()));
    }
    // An estate still open at the close opened on the last day, or waited that day: on a payment that failed, or to
    // sell the units it holds.
    let stale = open
        .iter()
        .filter(|p| {
            let (place, slot) = books.parties.row(**p);
            books.parties.table(place).created(slot) < last.day
        })
        .count();
    let waiting = last.estates_waiting + last.estates_unsold;
    if phx_rand::float::len_u64(stale) != waiting {
        return Outcome::Fail(format!("{stale} estates open from before the last day, {waiting} counted waiting"));
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
        return Outcome::NotYet(NO_AGENTS);
    }
    let bounds = match w.register().partition("DEM.age_classes") {
        Ok(p) => p.bounds.clone(),
        Err(e) => return Outcome::Fail(e),
    };
    let class = |age: i64| bounds.partition_point(|b| *b <= age);
    let mut by_class: std::collections::BTreeMap<(u32, u32, usize), RateTally> = std::collections::BTreeMap::new();
    for ((p, year, age), t) in &w.rates().0 {
        if !lives.iter().any(|(q, _)| q == p) {
            continue;
        }
        let slot = by_class.entry((*p, *year, class(i64::from(*age)))).or_default();
        *slot = total([*slot, *t].iter());
    }
    let twins = factor(w);
    for ((p, year, c), t) in &by_class {
        let hazard = lives.iter().find(|(q, _)| q == p).map_or("", |(_, h)| h);
        if let Err(e) = within(&format!("{hazard} in {year} at age class {c}"), t, twins) {
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
/// its line kind names to decide on it; and no agent holds a contract for a person it no longer holds, so the dead are
/// paid nothing.
fn lines_pay(w: Inspector<'_>) -> Outcome {
    if !agents_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let settled = w.settlements();
    if settled.iter().map(|s| s.dues.payments).sum::<u64>() == 0 {
        return Outcome::Fail("no due was paid over the run".to_owned());
    }
    let lines = &w.books().ledger.lines;
    for s in settled {
        for (kind, _) in &s.by_kind {
            let decl = lines.kind_decl(*kind);
            if decl.transfer_requesters.is_empty() {
                return Outcome::Fail(format!(
                    "day {}: a fail on a {} line, whose kind names no system to decide on it",
                    s.day.get(),
                    decl.name
                ));
            }
        }
    }
    rows_count_twins(w)
}

/// The GEN report lists the counterparties each derived side was apportioned over, what each drew and what it was
/// given; no party of no drawn size was given any. A stratum with no eligible counterparty stops the opening, so none
/// is left unmatched.
fn apportionments_reported(w: Inspector<'_>) -> Outcome {
    if !agents_hold_lines(w) {
        return Outcome::NotYet(NO_LINES);
    }
    let report = w.opening();
    if report.apportioned == 0 {
        return Outcome::Fail("the opening reports no apportionment".to_owned());
    }
    match report.unfounded.first() {
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

#[cfg(test)]
mod tests {
    use super::poisson_tail;

    #[test]
    fn the_poisson_tail_is_exact() {
        assert!((poisson_tail(0.025, 1) - (1.0 - (-0.025_f64).exp())).abs() < 1e-12);
        assert!((poisson_tail(2.0, 0) - (-2.0_f64).exp()).abs() < 1e-12);
        let at_most_one = (-3.0_f64).exp() * (1.0 + 3.0);
        assert!((poisson_tail(3.0, 1) - at_most_one).abs() < 1e-12);
    }
}
