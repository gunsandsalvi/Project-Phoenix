//! A run on the build machine: the world assembled, settled and run, the observer reading each day, the live checks
//! at the end, and the budget judged against its ratchets.

use std::path::{Path, PathBuf};

use phx_core::Period;
use phx_exec::Clock;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{Inspector, World, WorldConfig, assemble};
use serde::Deserialize;
use serde_json::json;

use crate::RunArgs;
use crate::checks::{CHECKS, Observed, Outcome, Run};
use crate::clock::WallClock;

/// The classes of equal count the woken firms' answers to their surprises are reported in, by the surprise's size.
const SIZE_CLASSES: usize = 4;

/// Resident memory the world may take at its peak: the budget's.
const WORLD_BYTES: u64 = 4608 << 20;
const MONTHS_PER_YEAR: u16 = 12;

#[derive(Debug, Deserialize)]
struct Ratchet {
    counter: String,
    value: u64,
    direction: String,
}

#[derive(Debug, Deserialize)]
struct Ratchets {
    ratchet: Vec<Ratchet>,
}

/// The process's peak resident memory, from the kernel's own account.
fn peak_resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kib.checked_mul(1 << 10)
}

/// Each counter against its ratchet: a counter with no entry is refused, and one that moved the wrong way fails.
pub(crate) fn check_ratchets(path: &Path, counters: &[(&str, u64)]) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let ratchets: Ratchets = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut failures = Vec::new();
    for (name, value) in counters {
        match ratchets.ratchet.iter().find(|r| r.counter == *name) {
            None => failures.push(format!("`{name}` has no ratchet")),
            Some(r) if r.direction == "down" && *value > r.value => {
                failures.push(format!("`{name}` is {value}; its ratchet allows {}", r.value));
            }
            Some(r) if r.direction == "up" && *value < r.value => {
                failures.push(format!("`{name}` is {value}; its ratchet needs {}", r.value));
            }
            Some(_) => {}
        }
    }
    Ok(failures)
}

/// The greatest of some counts, none being zero.
fn greatest(values: impl Iterator<Item = u64>) -> u64 {
    values.fold(0, |g, v| if v > g { v } else { g })
}

fn selected(checks: &str, id: &str) -> bool {
    checks == "all" || checks.split(',').any(|c| c.trim() == id)
}

/// The day settling ends, the owner's length after day zero, and the day the run ends, `days` after that; or, for a
/// run of `total` days from day zero, that day, with settling cut short at it and the injections' save counted from
/// day zero.
fn span(w: Inspector<'_>, days: u16, total: Option<u16>) -> Result<(phx_id::Day, phx_id::Day), String> {
    if let Some(total) = total {
        let end = Period::days(total).map_or(w.day_zero(), |p| w.calendar().plus(w.day_zero(), p));
        return Ok((w.day_zero(), end));
    }
    let years = u16::try_from(w.settling_years()).map_err(|_| "too long a settling")?;
    let months = years.checked_mul(MONTHS_PER_YEAR).ok_or("too long a settling")?;
    let settled = Period::months(months).map_or(w.day_zero(), |p| w.calendar().plus(w.day_zero(), p));
    let end = Period::days(days).map_or(settled, |p| w.calendar().plus(settled, p));
    Ok((settled, end))
}

/// The world's configuration from the run's arguments.
fn config(args: &RunArgs) -> Result<WorldConfig, String> {
    Ok(WorldConfig {
        seed: args.seed,
        data: args.data.clone(),
        setup: args.setup.clone(),
        run_dir: args.run_dir.clone(),
        representation: match args.persons {
            Some(n) => phx_num::Missing::Present(representation(n)?),
            None => phx_num::Missing::Absent,
        },
    })
}

/// The representation of so many persons.
///
/// # Errors
/// A world of no one.
pub(crate) fn representation(persons: u64) -> Result<phx_pop::prims::Representation, String> {
    if persons == 0 {
        return Err("a world of no persons holds no population".to_owned());
    }
    Ok(phx_pop::prims::Representation { persons })
}

/// Each macro read's days, first and last value, least, greatest and sum, and the view's histograms at the close.
fn reads_report(recorder: &phx_obs::Recorder, view: &phx_obs::View) -> serde_json::Value {
    let series: serde_json::Map<_, _> = recorder
        .series()
        .iter()
        .map(|s| {
            let values = s.values.iter().map(|(_, v)| *v);
            let at =
                |e: Option<&(phx_id::Day, i128)>| e.map(|(d, v)| json!({ "day": d.get(), "value": v.to_string() }));
            let summary = json!({
                "unit": s.unit,
                "days": s.values.len(),
                "first": at(s.values.first()),
                "last": at(s.values.last()),
                "least": values.clone().reduce(|a, b| if b < a { b } else { a }).map(|v| v.to_string()),
                "greatest": values.clone().reduce(|a, b| if b > a { b } else { a }).map(|v| v.to_string()),
                "sum": values.sum::<i128>().to_string(),
            });
            (s.id.clone(), summary)
        })
        .collect();
    let histograms: serde_json::Map<_, _> = view
        .histograms
        .iter()
        .map(|(id, h)| (id.clone(), json!({ "edges": h.edges(), "counts": h.counts(), "below": h.below() })))
        .collect();
    json!({ "series": series, "histograms": histograms })
}

/// Each opening distribution's distance from the world's own at settling's end and at the run's end.
fn drift_report(settled: &[phx_obs::Drift], ended: &[phx_obs::Drift]) -> serde_json::Value {
    let at = |drifts: &[phx_obs::Drift]| -> serde_json::Map<String, serde_json::Value> {
        drifts
            .iter()
            .map(|d| {
                let of = |m: phx_num::Missing<f64>| match m {
                    phx_num::Missing::Present(x) => json!(x),
                    phx_num::Missing::Absent => serde_json::Value::Null,
                };
                (d.id.clone(), json!({ "day": d.day.get(), "distance": of(d.distance), "moved": of(d.moved) }))
            })
            .collect()
    };
    json!({ "settled": at(settled), "ended": at(ended) })
}

fn live_checks(w: Inspector<'_>, observed: &Observed<'_>, checks: &str) -> (Vec<serde_json::Value>, bool) {
    let mut results = Vec::new();
    let mut all_pass = true;
    for check in CHECKS.iter().filter(|c| selected(checks, c.id)) {
        let run = check.run.map(|r| match r {
            Run::World(f) => f(w),
            Run::Observed(f) => f(w, observed),
        });
        let (outcome, detail) = match (run, check.retired) {
            (Some(outcome), _) => match outcome {
                Outcome::Pass => ("pass", String::new()),
                Outcome::Fail(why) => {
                    all_pass = false;
                    ("fail", why)
                }
                Outcome::NotYet(why) => ("not yet", why.to_owned()),
            },
            (None, Some(why)) => ("retired", why.to_owned()),
            (None, None) => ("empty", String::new()),
        };
        println!("{} {outcome} {detail}", check.id);
        results.push(json!({ "id": check.id, "title": check.title, "from_step": check.from_step, "outcome": outcome, "detail": detail }));
    }
    (results, all_pass)
}

/// What the observer keeps beside the run: the reads it takes each day, its views, and the view at
/// settling's end.
struct Observing {
    watch: phx_obs::Watch,
    views: phx_obs::Views,
    settled: Option<std::sync::Arc<phx_obs::View>>,
}

impl Observing {
    /// The observer of the declarations over the opened world, and the opening's view.
    fn open(
        w: Inspector<'_>,
        definitions: &phx_obs::Definitions,
    ) -> Result<(Observing, std::sync::Arc<phx_obs::View>), String> {
        let watch = phx_obs::Watch { recorder: phx_obs::Recorder::new(&definitions.reads, w)? };
        let mut views = phx_obs::Views::new(&definitions.histograms, w)?;
        let opening = views.close(w, &watch.recorder);
        Ok((Observing { watch, views, settled: None }, opening))
    }

    /// The view at settling's end, taken at the first close on or after it.
    fn settling(&mut self, w: Inspector<'_>, settle_end: phx_id::Day) {
        if self.settled.is_none() && w.today() >= settle_end {
            self.settled = Some(self.views.close(w, &self.watch.recorder));
        }
    }
}

/// Runs the world to its last day, the observer reading each day.
fn play(world: &mut World, settle_end: phx_id::Day, end: phx_id::Day, clock: &WallClock, obs: &mut Observing) {
    obs.settling(Inspector::new(world), settle_end);
    while world.today() < end {
        let seen = Inspector::new(world).findings().len();
        world.run_turn_observed(&[], clock, Some(&mut obs.watch));
        println!("{}", progress(Inspector::new(world), settle_end));
        // Each finding as the turn found it, so a run that stops later still shows what the audit saw.
        for f in Inspector::new(world).findings().iter().skip(seen) {
            println!("finding {} {} day {}: {:?} {}: {}", f.family, f.clause, f.day.get(), f.owner, f.size, f.detail);
        }
        obs.settling(Inspector::new(world), settle_end);
    }
}

/// The turn just run, so a run can be followed as it goes: its dates and days, its wall time, and what the core's
/// days did in it.
fn progress(w: Inspector<'_>, settle_end: phx_id::Day) -> String {
    let Some(turn) = w.turns().last() else { return "no turn run".to_owned() };
    let in_turn = |d: phx_id::Day| d >= turn.first && d <= turn.last;
    let core = w.core();
    let (mut flows, mut settled, mut failed, mut committed, mut breaks, mut arrears) = (0, 0, 0, 0, 0, 0);
    for d in core.days.iter().filter(|d| in_turn(d.day)) {
        (flows, settled, failed, committed, breaks) =
            (flows + d.flows, settled + d.settled, failed + d.failed, committed + d.committed, breaks + d.breaks);
        arrears += d.arrears;
    }
    let (mut born, mut gone, mut ended) = (0, 0, 0);
    for (_, p) in core.pop_days.iter().filter(|(d, _)| in_turn(*d)) {
        (born, gone, ended) = (born + p.born, gone + p.gone, ended + p.ended);
    }
    let date = |d| crate::measure::calendar::date_text(w.date(d));
    let phase = if turn.last < settle_end { "settling" } else { "running" };
    let wall = turn.wall_ns.map_or_else(|| "untimed".to_owned(), |ns| format!("{} ms", ns / 1_000_000));
    format!(
        "{phase} {} to {}: {} days in {wall}; flows {flows}, settled {settled}, failed {failed} ({arrears} into \
         arrears), committed {committed}, money breaks {breaks}; born {born}, gone {gone}, households ended {ended}; persons {}",
        date(turn.first),
        date(turn.last),
        turn.days,
        core.persons_held(),
    )
}

/// The counters the ratchets hold: each the greatest a day of the run came to.
fn counters(w: Inspector<'_>) -> [(&'static str, u64); 3] {
    let core = w.core();
    [
        ("phx_geo.map_bytes", u64::try_from(w.geo().bytes()).unwrap_or(u64::MAX)),
        ("phx_core.flows_a_day", greatest(core.days.iter().map(|d| d.flows))),
        ("phx_pop.hits", greatest(core.pop_days.iter().map(|(_, d)| d.hits))),
    ]
}

/// Each trade's price changes over the run — their frequency a review and mean size — and its firms' markups at the
/// close, their median, as the run report publishes them.
fn prices_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    let core = w.core();
    let markups = crate::checks::core::markups_by_trade(w);
    core.goods
        .prices
        .iter()
        .map(|(p, t)| {
            let per =
                |n: u64, d: u64| if d > 0 { phx_rand::float::from_u64(n) / phx_rand::float::from_u64(d) } else { 0.0 };
            let size = if t.changes > 0 { t.size / phx_rand::float::from_u64(t.changes) } else { 0.0 };
            json!({
                "product": p,
                "reviews": t.reviews,
                "changes_a_review": per(t.changes, t.reviews),
                "mean_change": size,
                "median_markup": markups.get(p).copied(),
            })
        })
        .collect()
}

/// Each labour round's jobs open and persons searching at its close, its hires and separations, its pay rounds'
/// contracts reviewed, raised and cut, the employees applying on from their jobs and those who moved job to job:
/// the Beveridge relation's points and the flows between jobs and search, as the run report publishes them.
fn labour_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .labour
        .days
        .iter()
        .map(|d| {
            json!({
                "day": d.day,
                "open": d.open,
                "searching": d.searching,
                "hires": d.hires,
                "separated": d.separated,
                "posted": d.posted,
                "reviewed": d.reviewed,
                "raised": d.raised,
                "cut": d.cut,
                "searching_on": d.searching_on,
                "job_to_job": d.job_to_job,
                "quits": d.quits,
                "defaults": d.defaults,
                "closures": d.closures,
            })
        })
        .collect()
}

/// The age structure and fertility at the close, as the run report publishes them.
fn population_report(w: Inspector<'_>) -> serde_json::Value {
    let Some(s) = crate::checks::lives::structure(w) else { return serde_json::Value::Null };
    json!({
        "age_structure": s.classes.iter().map(|(a, [f, m])| json!({"from": a, "women": f, "men": m})).collect::<Vec<_>>(),
        "persons": s.persons,
        "births": s.births,
        "general_fertility_per_thousand": s.general_fertility(),
    })
}

/// The services' reads at the close: their share of the sales and of the jobs, and services' and goods' median markups,
/// changes a review and mean change, as the run report publishes them.
fn services_report(w: Inspector<'_>) -> Option<serde_json::Value> {
    crate::checks::services::services(w).map(|r| {
        json!({
            "sales_share": r.sales_share,
            "jobs_share": r.jobs_share,
            "median_markup": r.median_markup,
            "changes_a_review": r.changes_a_review,
            "mean_change": r.mean_change,
        })
    })
}

/// The outlooks' reads: each method's lag behind the turns, the surprised firms' first price changes by the
/// surprise's size, and each day's stances.
fn outlooks_report(w: Inspector<'_>) -> serde_json::Value {
    json!({
        "lags": crate::checks::outlooks::lags(w).iter().map(|((memory, heuristic), (sum, n))| json!({
            "memory": memory,
            "heuristic": heuristic,
            "turns": n,
            "prints_behind": sum,
        })).collect::<Vec<_>>(),
        "surprise_responses": crate::checks::outlooks::responses_by_size(w, SIZE_CLASSES).iter().map(|(size, days, n)| json!({
            "surprise": size,
            "days_to_change": days,
            "firms": n,
        })).collect::<Vec<_>>(),
        "stances": w.core().goods.outlooks.days.iter().map(|d| json!({
            "day": d.day,
            "by_heuristic": d.by_heuristic,
            "reconsidered": d.reconsidered,
            "changed": d.changed,
            "surprised": d.surprised,
        })).collect::<Vec<_>>(),
    })
}

/// Each release the agencies published: its series, country, period, day and values.
fn statistics_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .stats
        .published
        .iter()
        .map(|r| {
            json!({
                "series": if_state::stats::NAMES.get(usize::from(r.series)),
                "country": r.country,
                "period": r.period,
                "published": r.published.get(),
                "values": r.values,
            })
        })
        .collect()
}

/// The core's days: each one's time, flows and what came of them.
fn days_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .days
        .iter()
        .map(|d| {
            json!({
                "day": d.day.get(),
                "ms": d.ns / 1_000_000,
                "flows": d.flows,
                "settled": d.settled,
                "failed": d.failed,
                "failed_by": d.failed_by,
                "arrears": d.arrears,
                "committed": d.committed,
                "gross": d.gross.to_string(),
                "breaks": d.breaks,
            })
        })
        .collect()
}

/// Assembles, settles and runs the world, then checks it and writes its report; true when every check passes, every
/// counter keeps its ratchet and the memory keeps its budget.
pub fn run(args: &RunArgs) -> Result<bool, String> {
    crate::panic_hook::install(format!("seed{}-pid{}", args.seed, std::process::id()), PathBuf::from("violations"));
    let config = config(args)?;
    let clock = WallClock::new();
    let assembling = clock.now_ns();
    let mut world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let assembly_ns = clock.now_ns().checked_sub(assembling);
    let (settle_end, end) = span(Inspector::new(&world), args.days, args.total_days)?;
    let definitions = phx_obs::Definitions::read(&args.data)?;
    let (mut obs, opening) = Observing::open(Inspector::new(&world), &definitions)?;
    println!("opened in {} ms", assembly_ns.map_or(0, |n| n / 1_000_000));
    let meter = crate::budget::Meter::start(clock.now_ns());
    play(&mut world, settle_end, end, &clock, &mut obs);
    let span = meter.stop(clock.now_ns());
    let w = Inspector::new(&world);
    let view = obs.views.close(w, &obs.watch.recorder);
    let settled = obs.settled.as_ref().map(|v| phx_obs::drift(&opening, v)).unwrap_or_default();
    let ended = phx_obs::drift(&opening, &view);
    let observed =
        Observed { reads: &definitions.reads, series: obs.watch.recorder.series(), settled: &settled, ended: &ended };
    let (results, all_pass) = live_checks(w, &observed, &args.checks);
    let counters = counters(w);
    let ratchet_failures = check_ratchets(&args.ratchets, &counters)?;
    for f in &ratchet_failures {
        println!("ratchet: {f}");
    }
    let peak = peak_resident_bytes();
    let memory_ok = peak.is_some_and(|p| p <= WORLD_BYTES);
    let (budget_block, budget_kept) = crate::budget::judge(w, peak, &span, args.budget.as_deref())?;
    let turns = w.turns();
    let report = json!({
        "seed": args.seed,
        "settle_years": w.settling_years(),
        "days": args.days,
        "total_days": args.total_days,
        "workers": args.workers,
        "first_day": crate::measure::calendar::date_text(w.date(w.day_zero().succ())),
        "last_day": crate::measure::calendar::date_text(w.date(w.today())),
        "settled_on": crate::measure::calendar::date_text(w.date(settle_end)),
        "turns": turns.len(),
        "days_run": turns.iter().map(|t| u64::from(t.days)).sum::<u64>(),
        "longest_turn_days": greatest(turns.iter().map(|t| u64::from(t.days))),
        "build_seconds": args.build_seconds,
        "run_ms": span.run_ns.map(|n| n / 1_000_000),
        "assembly_ms": assembly_ns.map(|n| n / 1_000_000),
        "persons_opened": w.core().persons_opened,
        "persons": w.core().persons_held(),
        "core_days": days_report(w),
        "statistics": statistics_report(w),
        "population": population_report(w),
        "prices": prices_report(w),
        "labour": labour_report(w),
        "services": services_report(w),
        "outlooks": outlooks_report(w),
        "closures": w.core().closures.iter().map(|(c, name, share)| json!({ "country": c, "closure": name, "share_of_gdp": share })).collect::<Vec<_>>(),
        "apportioned": w.core().apportioned.iter().map(|a| json!({
            "stratum": a.stratum,
            "country": a.country,
            "total": a.total,
            "given": a.given,
            "difference": a.total - a.given,
            "parties": a.parties,
            "unfounded": a.unfounded,
        })).collect::<Vec<_>>(),
        "reads": reads_report(&obs.watch.recorder, &view),
        "drift": drift_report(&settled, &ended),
        "peak_resident_bytes": peak,
        "budget": budget_block,
        "memory_budget_bytes": WORLD_BYTES,
        "counters": counters.iter().map(|(n, v)| ((*n).to_owned(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "ratchet_failures": ratchet_failures,
        "findings": w.findings().len(),
        "checks": results,
    });
    if let Some(path) = &args.report {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        std::fs::write(path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    }
    println!(
        "{} turns, {} days, peak {} MiB; {}; budget {}",
        turns.len(),
        turns.iter().map(|t| u64::from(t.days)).sum::<u64>(),
        peak.map_or(0, |p| p >> 20),
        if all_pass && ratchet_failures.is_empty() && memory_ok { "clean" } else { "not clean" },
        if budget_kept { "kept" } else { "missed" }
    );
    Ok(all_pass && ratchet_failures.is_empty() && memory_ok && budget_kept)
}

/// Assembles the world and writes its calendar's measurement.
pub fn measure_calendar(data: &Path, setup: &Path, run_dir: &Path, out: &Path) -> Result<bool, String> {
    let config = WorldConfig {
        seed: 0,
        data: data.to_path_buf(),
        setup: setup.to_path_buf(),
        run_dir: run_dir.to_path_buf(),
        representation: phx_num::Missing::Absent,
    };
    let world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let report = crate::measure::calendar::measure(Inspector::new(&world));
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(out, text + "\n").map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(true)
}

/// The budget as measured, from a build run's report and a device report of the same commit's world, written to
/// `out`.
pub fn measure_budget(build_run: &Path, device: &Path, out: &Path) -> Result<bool, String> {
    let read = |p: &Path| -> Result<serde_json::Value, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
    };
    let report = crate::measure::budget::measure(&read(build_run)?, &read(device)?);
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(out, text + "\n").map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::representation;

    #[test]
    fn representation_holds_its_persons() {
        assert_eq!(representation(750_000).map(|r| r.persons), Ok(750_000));
        assert!(representation(0).is_err(), "a world of no one refused");
    }
}
