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

/// Bytes to MiB, a shift.
const MIB_SHIFT: u32 = 20;

/// The classes of equal count the woken firms' answers to their surprises are reported in, by the surprise's size.
const SIZE_CLASSES: usize = 4;

/// Resident memory the world may take at its peak: the budget's.
const WORLD_BYTES: u64 = 4608 << 20;
const MONTHS_PER_YEAR: u16 = 12;
/// Days after settling at whose close the save the injections load is taken.
const INJECTION_SAVE_DAY: u16 = 30;
/// The live check that reads each family's injection into the day-30 save.
const INJECTION_CHECK: &str = "LC-0-10";
/// The key of a build's identity hash: any fixed value.
const BUILD_KEY: [u64; 2] = [0x5048_5820_4255_494c, 0x4420_4944_2031_3131];

#[derive(Debug, Deserialize)]
struct Ratchet {
    counter: String,
    /// A bound in the counter's unit; the design point's keys are fractional core-ms, the run's counts whole.
    value: f64,
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

/// Each counter against its ratchet: a counter with no entry is refused, one that moved the wrong way fails, and a
/// ratchet under `produced` that the run did not produce is refused, never read as met.
pub(crate) fn check_ratchets(
    path: &Path,
    counters: &[(&str, f64)],
    produced: Option<&str>,
) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    judge_counters(&text, counters, produced).map_err(|e| format!("{}: {e}", path.display()))
}

/// The judgement of `check_ratchets` over a ratchets file's text.
pub(crate) fn judge_counters(
    text: &str,
    counters: &[(&str, f64)],
    produced: Option<&str>,
) -> Result<Vec<String>, String> {
    let ratchets: Ratchets = toml::from_str(text).map_err(|e| e.to_string())?;
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
    if let Some(prefix) = produced {
        for r in ratchets.ratchet.iter().filter(|r| r.counter.starts_with(prefix)) {
            if !counters.iter().any(|(n, _)| *n == r.counter) {
                failures.push(format!("`{}` was not produced by the run", r.counter));
            }
        }
    }
    Ok(failures)
}

/// A count as a bound reads it: every count the run keeps is far inside a float's exact integers.
pub(crate) fn real(n: u64) -> f64 {
    n.to_string().parse().unwrap_or(f64::INFINITY)
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
        pool: pool_spec(args.workers),
    })
}

/// The cores the world's pool runs on: the device's, or the first of them where a run asks for fewer workers.
pub(crate) fn pool_spec(workers: Option<usize>) -> phx_exec::PoolSpec {
    let mut spec = phx_exec::PoolSpec::detect();
    if let Some(n) = workers {
        spec.cores.truncate(n);
    }
    spec
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

/// The report's sections on the core's institutions, its saves and its plant.
fn sections(w: Inspector<'_>) -> Vec<(String, serde_json::Value)> {
    vec![
        ("decisions".to_owned(), json!(decisions_report(w))),
        ("fund_stages".to_owned(), json!(fund_report(w))),
        ("auctions".to_owned(), json!(auctions_report(w))),
        ("agencies".to_owned(), json!(agencies_report(w))),
        ("lenders".to_owned(), json!(lenders_report(w))),
        ("saves".to_owned(), saves_report(w)),
        ("injections".to_owned(), crate::inject::report(w.injections())),
        ("plant".to_owned(), plant_report(w)),
        ("freight".to_owned(), freight_report(w)),
        ("owners".to_owned(), owners_report(w)),
    ]
}

/// The plant's days: what was paid into projects, what entered service, wore and was retired, the makings plant
/// bound and those beyond it; and the capital units held at the close by condition, newest first.
fn plant_report(w: Inspector<'_>) -> serde_json::Value {
    let core = w.core();
    let mut by_condition: std::collections::BTreeMap<u8, i64> = std::collections::BTreeMap::new();
    for h in core.goods.stocks.all() {
        if let phx_core::goods::Held::Capital(c) = phx_core::goods::held(&core.goods.units, h.unit) {
            *by_condition.entry(c.condition).or_insert(0) += h.units;
        }
    }
    json!({
        "days": core.plant.days.iter().map(|d| json!({
            "day": d.day,
            "invested": d.invested,
            "completed": d.completed,
            "worn": d.worn,
            "retired": d.retired,
            "bound": d.bound,
            "beyond": d.beyond,
        })).collect::<Vec<_>>(),
        "projects": core.plant.projects.len(),
        "units_by_condition": by_condition.iter().map(|(c, u)| json!({ "condition": c, "units": u })).collect::<Vec<_>>(),
    })
}

/// Freight's days: the gaps weighed and the trips chosen, booked and refused, the units departed and arrived and the
/// shipments on their way; and each pair of places' basis against its freight at the close.
fn freight_report(w: Inspector<'_>) -> serde_json::Value {
    let core = w.core();
    json!({
        "days": core.freight.days.iter().map(|d| json!({
            "day": d.day,
            "weighed": d.weighed,
            "chose": d.chose,
            "booked": d.booked,
            "no_room": d.no_room,
            "over_capacity": d.over_capacity,
            "no_route": d.no_route,
            "unpaid": d.unpaid,
            "departed": d.departed,
            "arrived": d.arrived,
            "on_the_way": d.on_the_way,
        })).collect::<Vec<_>>(),
        "basis": core.basis(w.regions(), w.geo()).iter().map(|(g, f)| json!({ "gap": g, "freight": f })).collect::<Vec<_>>(),
    })
}

/// The firms' owners at the close: the self-employed the opening drew and dealt, the firms it left with no owner, the
/// firms with an owner working in them and those owners, the offices held, and the parties holding shares.
fn owners_report(w: Inspector<'_>) -> serde_json::Value {
    let o = &w.core().owners;
    json!({
        "drawn": o.drawn,
        "dealt": o.dealt,
        "unowned_at_opening": o.unowned,
        "firms_worked_by_owners": o.working.len(),
        "working_owners": o.works_at.len(),
        "offices_held": w.core().decisions.held(),
        "holders": o.holds.len(),
    })
}

/// The closures that balanced each country's opening sheet, each a share of its GDP.
fn closures_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .closures
        .iter()
        .map(|(c, name, share)| json!({ "country": c, "closure": name, "share_of_gdp": share }))
        .collect()
}

/// Every amount the opening shared by weight, with what its parties were given.
fn apportioned_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .apportioned
        .iter()
        .map(|a| {
            json!({
                "stratum": a.stratum,
                "country": a.country,
                "total": a.total,
                "given": a.given,
                "difference": a.total - a.given,
                "parties": a.parties,
                "unfounded": a.unfounded,
            })
        })
        .collect()
}

/// The opening's writes, every party's: its kind, its identity, the bank its account is at and the money written to
/// it, as the GEN report lists them, in the run's own directory.
fn write_opening(w: Inspector<'_>, path: &Path) -> Result<(), String> {
    use std::fmt::Write as _;
    let mut text = String::from("kind,identity,bank,amount\n");
    let core = w.core();
    for (name, store) in core.names.iter().zip(&core.kinds) {
        let Some(a) = store.accounts.as_ref() else { continue };
        for slot in store.parties.live_slots() {
            let (Some(id), Some(bank), Some(m), Some(p)) =
                (store.parties.id(slot), a.bank.get(slot), a.balance.get(slot), a.pending.get(slot))
            else {
                continue;
            };
            writeln!(text, "{name},{},{bank},{}", id.get(), i128::from(m) + i128::from(p))
                .map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The saves the run took: each one's day, its stores' sizes, its write and check times, and whether it read back to
/// its close's hash.
fn saves_report(w: Inspector<'_>) -> serde_json::Value {
    let saves: Vec<serde_json::Value> = w
        .saves()
        .iter()
        .map(|s| {
            json!({
                "day": date_text(w.date(s.day)),
                "bytes": s.stores.iter().map(|(_, b, _)| *b).sum::<u64>() + s.run_bytes,
                "raw_bytes": s.stores.iter().map(|(_, _, r)| *r).sum::<u64>(),
                "stores": s.stores.iter().map(|(n, b, r)| json!({ "name": n, "bytes": b, "raw_bytes": r })).collect::<Vec<_>>(),
                "write_ms": s.write_ns.map(|n| n / 1_000_000),
                "check_ms": s.check_ns.map(|n| n / 1_000_000),
                "hash_matched": s.mismatch.is_none(),
            })
        })
        .collect();
    json!(saves)
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

/// The running binary's identity, which a save names so that another build refuses it: the hash of its bytes.
///
/// # Errors
/// When the running binary cannot be read.
pub fn build_id() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("the running binary: {e}"))?;
    let bytes = std::fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let mut h = phx_store::Sip128::new(BUILD_KEY);
    h.write(&bytes);
    Ok(format!("{:032x}", h.finish()))
}

/// The save interval a day falls in: its months since the calendar's year nought, over the months between saves.
fn save_period(w: Inspector<'_>, day: phx_id::Day) -> Result<u64, String> {
    let date = w.date(day);
    let year = u64::try_from(date.year()).map_err(|_| "a day before the calendar's year nought")?;
    let months = year * u64::from(MONTHS_PER_YEAR) + u64::from(date.month());
    months.checked_div(w.save_every_months()).ok_or_else(|| "a save interval of no months".to_owned())
}

/// A save of the world at this close, checked by reading its files back; its sizes and times join the run's
/// measures.
fn save_and_check(world: &mut World, root: &Path, build: &str, clock: &WallClock) -> Result<(), String> {
    let t0 = clock.now_ns();
    let rec = world.save(root, build)?;
    let t1 = clock.now_ns();
    let checked = world.check_save(&rec.dir);
    let t2 = clock.now_ns();
    world.record_save(phx_world::SaveMeasure {
        day: rec.day,
        stores: rec.stores.iter().map(|s| (s.name.to_owned(), s.bytes, s.raw_bytes)).collect(),
        run_bytes: rec.run_bytes,
        write_ns: t1.checked_sub(t0),
        check_ns: t2.checked_sub(t1),
        mismatch: checked.err(),
    });
    Ok(())
}

/// Every family's injection into the injections' save, each loaded apart in a process of its own so the run's
/// memory is the world's alone.
fn inject_apart(args: &RunArgs, save: &Path) -> Result<Vec<phx_world::InjectionRecord>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("the running binary: {e}"))?;
    let report = args.run_dir.join("inject.json");
    if report.exists() {
        std::fs::remove_file(&report).map_err(|e| format!("{}: {e}", report.display()))?;
    }
    let status = std::process::Command::new(exe)
        .arg("inject")
        .arg("--from")
        .arg(save)
        .arg("--data")
        .arg(&args.data)
        .arg("--setup")
        .arg(&args.setup)
        .arg("--run-dir")
        .arg(args.run_dir.join("inject"))
        .arg("--report")
        .arg(&report)
        .status()
        .map_err(|e| format!("phx inject: {e}"))?;
    let text = std::fs::read_to_string(&report).map_err(|e| format!("phx inject ({status}) left no report: {e}"))?;
    crate::inject::parse(&text)
}

/// Runs the world to its last day, the observer reading each day, saving it at each save interval into the run's own
/// directory and taking the injections' save, whose directory it returns.
fn play(
    world: &mut World,
    args: &RunArgs,
    (settle_end, end): (phx_id::Day, phx_id::Day),
    clock: &WallClock,
    (obs, stores, allocs): (&mut Observing, &mut StoreSamples, &mut AllocMark),
) -> Result<Option<PathBuf>, String> {
    let (saves, build) = (args.run_dir.join("saves"), build_id()?);
    let mut period = save_period(Inspector::new(world), world.today())?;
    let w = Inspector::new(world);
    let injection_day = Period::days(INJECTION_SAVE_DAY).map_or(settle_end, |p| w.calendar().plus(settle_end, p));
    let mut injection_save = None;
    obs.settling(Inspector::new(world), settle_end);
    while world.today() < end {
        let seen = Inspector::new(world).findings().len();
        world.run_turn_observed(&[], clock, Some(&mut obs.watch));
        println!("{}", progress(Inspector::new(world), settle_end));
        stores.take_at_month_end(Inspector::new(world));
        allocs.after_turn(Inspector::new(world).turns().iter().map(|t| u64::from(t.days)).sum());
        // Each finding as the turn found it, so a run that stops later still shows what the audit saw.
        for f in Inspector::new(world).findings().iter().skip(seen) {
            println!("finding {} {} day {}: {:?} {}: {}", f.family, f.clause, f.day.get(), f.owner, f.size, f.detail);
        }
        obs.settling(Inspector::new(world), settle_end);
        let now = save_period(Inspector::new(world), world.today())?;
        if now != period {
            save_and_check(world, &saves, &build, clock)?;
            period = now;
        }
        // The injections' save serves the check that reads them alone, so a run that does not ask for it takes none.
        if injection_save.is_none() && world.today() >= injection_day && selected(&args.checks, INJECTION_CHECK) {
            injection_save = Some(world.save(&args.run_dir.join("inject-save"), &build)?.dir);
        }
    }
    Ok(injection_save)
}

/// The process's resident MiB before anything of the world is built, and the rate of random row reads on a pool of the
/// world's cores, plain and prefetched: the miss a gather's budget is counted in. Missing where the machine gives none.
fn baseline_report(clock: &WallClock) -> (Option<u64>, serde_json::Value) {
    let Ok(pool) = phx_exec::Pool::new(&phx_exec::PoolSpec::detect()) else { return (None, serde_json::Value::Null) };
    let b = phx_exec::probe::baseline(&pool, clock);
    let rate = |g: Option<phx_exec::probe::GatherRate>| {
        g.map(|g| json!({ "ns_per_row": g.ns_per_row, "core_ns_per_row": g.core_ns_per_row, "rows": g.rows }))
    };
    let gather = json!({ "region_bytes": b.gather.map(|g| g.bytes), "plain": rate(b.gather),
                         "prefetched": rate(b.gather_prefetched) });
    (b.resident_bytes.map(|r| r >> MIB_SHIFT), gather)
}

/// The days of a run's first buffers growing to the heaviest day's, after which a day allocates nothing.
const FIRST_DAYS: u64 = 10;

/// The allocations counted from the run's tenth day on, and at its last turn, with the days run at each.
#[derive(Debug, Default)]
struct AllocMark {
    from: Option<(u64, u64)>,
    last: Option<(u64, u64)>,
}

/// The allocations counted so far, where the bench's allocator counts them.
#[cfg_attr(feature = "bench", expect(clippy::unnecessary_wraps, reason = "none where the allocator does not count"))]
fn allocated() -> Option<u64> {
    #[cfg(feature = "bench")]
    return Some(phx_exec::alloc::allocated().0);
    #[cfg(not(feature = "bench"))]
    None
}

impl AllocMark {
    fn after_turn(&mut self, days_run: u64) {
        let Some(calls) = allocated() else { return };
        if self.from.is_none() && days_run >= FIRST_DAYS {
            self.from = Some((days_run, calls));
        }
        self.last = Some((days_run, calls));
    }

    /// Allocations a day from the tenth day on; none where they were not counted or no day followed.
    fn per_day(&self) -> Option<f64> {
        let ((d0, c0), (d1, c1)) = (self.from?, self.last?);
        crate::budget::per(c1.checked_sub(c0), d1.checked_sub(d0))
    }
}

/// Every kind's store sampled at the run's start and at each simulated month's end.
#[derive(Debug, Default)]
struct StoreSamples {
    taken: Vec<(phx_id::Date, Vec<(&'static str, phx_exec::stats::Sample)>)>,
}

impl StoreSamples {
    /// The stores as the opening left them, the first sample.
    fn opened(w: Inspector<'_>) -> StoreSamples {
        let mut samples = StoreSamples::default();
        samples.take(w);
        samples
    }

    fn take(&mut self, w: Inspector<'_>) {
        let core = w.core();
        let stores = core.names.iter().zip(&core.kinds).map(|(n, k)| (*n, phx_exec::stats::Sample::of(k))).collect();
        self.taken.push((w.date(w.today()), stores));
    }

    /// A sample when the turn just run crossed into a new month: the stores as the month it closed left them.
    fn take_at_month_end(&mut self, w: Inspector<'_>) {
        let month = |d: phx_id::Date| (d.year(), d.month());
        if self.taken.last().is_none_or(|(d, _)| month(*d) != month(w.date(w.today()))) {
            self.take(w);
        }
    }

    /// Each sample's date and stores, and each store's rows live gained a simulated year over the whole years sampled.
    fn report(&self) -> serde_json::Value {
        let samples: Vec<serde_json::Value> = self
            .taken
            .iter()
            .map(|(date, stores)| {
                let by_name: serde_json::Map<_, _> = stores
                    .iter()
                    .map(|(n, s)| {
                        let row = json!({ "rows_live": s.rows_live, "rows_ever": s.rows_ever, "bytes": s.bytes });
                        ((*n).to_owned(), row)
                    })
                    .collect();
                json!({ "date": date_text(*date), "stores": by_name })
            })
            .collect();
        let names: Vec<&str> = self.taken.first().map(|(_, s)| s.iter().map(|(n, _)| *n).collect()).unwrap_or_default();
        let growth: serde_json::Map<_, _> = names
            .iter()
            .map(|name| {
                let rows: Vec<u64> = self
                    .taken
                    .iter()
                    .filter_map(|(_, s)| s.iter().find(|(n, _)| n == name).map(|(_, x)| x.rows_live))
                    .collect();
                ((*name).to_owned(), json!(phx_exec::stats::growth_per_year(&rows).map(|g| g.to_string())))
            })
            .collect();
        json!({ "samples": samples, "rows_live_growth_per_year": growth })
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
    let (mut born, mut gone, mut ended, mut retired, mut claimed) = (0, 0, 0, 0, 0);
    for (_, p) in core.pop_days.iter().filter(|(d, _)| in_turn(*d)) {
        (born, gone, ended) = (born + p.born, gone + p.gone, ended + p.ended);
        (retired, claimed) = (retired + p.retired, claimed + p.claimed);
    }
    let endings = core.insolvency.endings.iter().filter(|e| in_turn(phx_id::Day::new(e.day)));
    let (wound, defaulted) = endings.fold((0, 0), |(w, d), e| if e.defaulted { (w, d + 1) } else { (w + 1, d) });
    let date = |d| date_text(w.date(d));
    let phase = if turn.last < settle_end { "settling" } else { "running" };
    let wall = turn.wall_ns.map_or_else(|| "untimed".to_owned(), |ns| format!("{} ms", ns / 1_000_000));
    format!(
        "{phase} {} to {}: {} days in {wall}; flows {flows}, settled {settled}, failed {failed} ({arrears} into \
         arrears), committed {committed}, money breaks {breaks}; born {born}, gone {gone}, households ended {ended}, \
         retired {retired} ({claimed} claiming a pension); firms ended {wound} wound down, {defaulted} defaulted; \
         persons {}",
        date(turn.first),
        date(turn.last),
        turn.days,
        core.persons_held(),
    )
}

/// The counters the ratchets hold: each the greatest a day of the run came to.
/// The run's counters that `perf/ratchets.toml` holds.
type RunCounters = [(&'static str, u64); 3];

fn counters(w: Inspector<'_>) -> RunCounters {
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
/// A date as the report writes it.
fn date_text(d: phx_id::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

fn days_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    let timings: std::collections::BTreeMap<u32, &Vec<(&'static str, u64)>> =
        w.core().timings.iter().map(|(d, t)| (d.get(), t)).collect();
    w.core()
        .days
        .iter()
        .map(|d| {
            // Each stage's time in microseconds, by the run's clock.
            let stages: serde_json::Map<String, serde_json::Value> = timings
                .get(&d.day.get())
                .into_iter()
                .flat_map(|t| t.iter())
                .map(|(name, ns)| ((*name).to_owned(), json!(ns / 1_000)))
                .collect();
            json!({
                "day": d.day.get(),
                "ms": d.ns / 1_000_000,
                "stages_us": stages,
                "flows": d.flows,
                "settled": d.settled,
                "failed": d.failed,
                "failed_by": d.failed_by,
                "arrears": d.arrears,
                "committed": d.committed,
                "made": d.made.to_string(),
                "gross": d.gross.to_string(),
                "net": d.net.to_string(),
                "fails": { "payer": d.fails[0], "bank": d.fails[1] },
                "ring": d.ring,
                "ring_value": d.ring_value.to_string(),
                "breaks": d.breaks,
            })
        })
        .collect()
}

/// Assembles, settles and runs the world, then checks it and writes its report; true when every check passes, every
/// counter keeps its ratchet and the memory keeps its budget.
pub fn run(args: &RunArgs) -> Result<bool, String> {
    crate::panic_hook::install(format!("seed{}-pid{}", args.seed, std::process::id()), PathBuf::from("violations"));
    let printer = crate::trace::start();
    let baseline = baseline_report(&WallClock::new());
    let config = config(args)?;
    let clock = WallClock::new();
    let assembling = clock.now_ns();
    let mut world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let assembly_ns = clock.now_ns().checked_sub(assembling);
    std::fs::create_dir_all(&args.run_dir).map_err(|e| format!("{}: {e}", args.run_dir.display()))?;
    write_opening(Inspector::new(&world), &args.run_dir.join("opening.csv"))?;
    let (settle_end, end) = span(Inspector::new(&world), args.days, args.total_days)?;
    let definitions = phx_obs::Definitions::read(&args.data)?;
    let (mut obs, opening) = Observing::open(Inspector::new(&world), &definitions)?;
    println!("opened in {} ms", assembly_ns.map_or(0, |n| n / 1_000_000));
    let meter = crate::budget::Meter::start(clock.now_ns(), Inspector::new(&world));
    let mut allocs = AllocMark::default();
    let mut stores = StoreSamples::opened(Inspector::new(&world));
    let injection_save = play(&mut world, args, (settle_end, end), &clock, (&mut obs, &mut stores, &mut allocs))?;
    let span = meter.stop(clock.now_ns(), Inspector::new(&world));
    let inject_ns = inject(args, injection_save.as_deref(), &mut world, &clock)?;
    let w = Inspector::new(&world);
    let view = obs.views.close(w, &obs.watch.recorder);
    let settled = obs.settled.as_ref().map(|v| phx_obs::drift(&opening, v)).unwrap_or_default();
    let ended = phx_obs::drift(&opening, &view);
    let observed =
        Observed { reads: &definitions.reads, series: obs.watch.recorder.series(), settled: &settled, ended: &ended };
    let (results, all_pass) = live_checks(w, &observed, &args.checks);
    let (counters, ratchet_failures) = ratchet_checks(w, &args.ratchets)?;
    let peak = peak_resident_bytes();
    let memory_ok = peak.is_some_and(|p| p <= WORLD_BYTES);
    let beside = crate::budget::Beside {
        allocs_per_day: allocs.per_day(),
        spans_below_busy: printer.below_busy(),
        baseline_mb: baseline.0,
    };
    let (budget_block, budget_kept) = crate::budget::judge(w, peak, (&span, &beside), args.budget.as_deref())?;
    let turns = w.turns();
    let mut report = json!({
        "seed": args.seed,
        "settle_years": w.settling_years(),
        "days": args.days,
        "total_days": args.total_days,
        "workers": args.workers,
        "first_day": date_text(w.date(w.day_zero().succ())),
        "last_day": date_text(w.date(w.today())),
        "settled_on": date_text(w.date(settle_end)),
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
        "closures": closures_report(w),
        "apportioned": apportioned_report(w),
        "reads": reads_report(&obs.watch.recorder, &view),
        "drift": drift_report(&settled, &ended),
        "peak_resident_bytes": peak,
        "spans": printer.spans(),
        "stores": stores.report(),
        "baseline_mb": baseline.0,
        "gather_rate": baseline.1,
        "budget": budget_block,
        "memory_budget_bytes": WORLD_BYTES,
        "counters": counters.iter().map(|(n, v)| ((*n).to_owned(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "ratchet_failures": ratchet_failures,
        "findings": w.findings().len(),
        "checks": results,
    });
    if let Some(o) = report.as_object_mut() {
        o.insert("inject_ms".to_owned(), json!(inject_ns.map(|n| n / 1_000_000)));
        o.extend(sections(w));
    }
    if let Some(path) = &args.report {
        write_report(path, &report)?;
    }
    // A run is clean only when its audit found nothing: a finding is a missing or wrong mechanism.
    let findings = w.findings().len();
    let clean = all_pass && ratchet_failures.is_empty() && memory_ok && findings == 0;
    println!(
        "{} turns, {} days, peak {} MiB; {} findings; {}; budget {}",
        turns.len(),
        turns.iter().map(|t| u64::from(t.days)).sum::<u64>(),
        peak.map_or(0, |p| p >> 20),
        findings,
        if clean { "clean" } else { "not clean" },
        if budget_kept { "kept" } else { "missed" }
    );
    Ok(clean && budget_kept)
}

/// The injections' checks run on the save taken for them, each recorded on the world; their wall time.
fn inject(args: &RunArgs, save: Option<&Path>, world: &mut World, clock: &WallClock) -> Result<Option<u64>, String> {
    let injecting = clock.now_ns();
    if let Some(dir) = save {
        for r in inject_apart(args, dir)? {
            world.record_injection(r);
        }
    }
    Ok(clock.now_ns().checked_sub(injecting))
}

/// The run's counters of `perf/ratchets.toml` and each ratchet they broke, printed as found.
fn ratchet_checks(w: Inspector<'_>, path: &Path) -> Result<(RunCounters, Vec<String>), String> {
    let counters = counters(w);
    let as_real: Vec<(&str, f64)> = counters.iter().map(|(n, v)| (*n, real(*v))).collect();
    let failures = check_ratchets(path, &as_real, None)?;
    for f in &failures {
        println!("ratchet: {f}");
    }
    Ok((counters, failures))
}

/// The report written where the run was told, its directory made first.
fn write_report(path: &Path, report: &serde_json::Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    std::fs::write(path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

/// Each country's fund stage a day: what its banks placed and borrowed, what stood overdue, what was remitted.
fn fund_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .central
        .days
        .iter()
        .map(|d| {
            json!({
                "day": d.day,
                "country": d.country,
                "placed": d.placed,
                "borrowed": d.borrowed,
                "banks_placing": d.placing,
                "banks_borrowing": d.borrowing,
                "overdue": d.overdue,
                "banks_overdue": d.overdue_banks,
                "unreturned": d.unreturned,
                "remitted": d.remitted,
            })
        })
        .collect::<Vec<_>>()
}

/// Each bill auction: its offer, bids and sales, price, cover and tail, where it has them.
fn auctions_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    let read = |m: phx_num::Missing<f64>| match m {
        phx_num::Missing::Present(v) => json!(v),
        phx_num::Missing::Absent => serde_json::Value::Null,
    };
    w.core()
        .bills
        .auctions
        .iter()
        .map(|a| {
            json!({
                "day": a.day,
                "country": a.country,
                "offered": a.offered,
                "bid": a.bid,
                "sold": a.sold,
                "unsold": a.offered - a.sold,
                "price": read(a.price),
                "cover": read(a.cover),
                "tail": read(a.tail),
            })
        })
        .collect()
}

/// Each agency's day: its staff, wage bill and appropriation, and what it posted.
fn agencies_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .agencies_kept
        .days
        .iter()
        .map(|d| {
            json!({
                "day": d.day,
                "country": d.country,
                "staff": d.staff,
                "wage_bill": d.bill,
                "appropriation": d.budget,
                "posted": d.posted,
            })
        })
        .collect::<Vec<_>>()
}

/// Each bank's applications, declines, quotes and loans, and its standard.
fn lenders_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .credit
        .lenders
        .iter()
        .map(|(b, l)| {
            json!({
                "bank": b.word(),
                "standard": l.standard,
                "applications": l.applications,
                "declined": l.declined,
                "quoted": l.quoted,
                "lent": l.lent,
            })
        })
        .collect::<Vec<_>>()
}

/// Each decision the world declares, with how many times each standing of decider took it.
fn decisions_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    w.core()
        .decisions
        .taken()
        .iter()
        .map(|(name, by)| {
            let by: std::collections::BTreeMap<&str, u64> =
                phx_core::Standing::ALL.iter().zip(by).map(|(s, n)| (s.name(), *n)).collect();
            json!({ "decision": name, "by": by })
        })
        .collect()
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
