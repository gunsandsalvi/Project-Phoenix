use std::path::{Path, PathBuf};

use phx_core::Period;
use phx_exec::Clock;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{Inspector, SaveMeasure, World, WorldConfig, assemble};
use serde::Deserialize;
use serde_json::json;

use crate::RunArgs;
use crate::checks::{CHECKS, Observed, Outcome, Run};
use crate::clock::WallClock;

/// Resident memory the empty world may take.
const EMPTY_WORLD_BYTES: u64 = 50 << 20;

/// Resident memory the map may take on top of it, its generation included.
const MAP_BYTES: u64 = 80 << 20;

/// Resident memory the individuals may take: their kind tables and facets, which hold the institutions' rows.
const INDIVIDUALS_BYTES: u64 = 225 << 20;

/// Resident memory the population may take: its agents, their persons and attachments at the run's factor.
const POPULATION_BYTES: u64 = 275 << 20;

/// Resident memory the lines the population holds may take: the relationship rows, the lines' holder lists with their
/// slack, the lines themselves, and the interned keys and terms.
const LINES_BYTES: u64 = (849 + 216 + 96 + 108) << 20;

/// Resident memory the population's indexes may take: the agenda.
const INDEXES_BYTES: u64 = 85 << 20;

/// Resident memory the worst day's buffers and the arenas' slack may take.
const DAY_BYTES: u64 = (600 + 214) << 20;

/// Resident memory the world may take at its peak: the budgets of the stores it holds.
const WORLD_BYTES: u64 =
    EMPTY_WORLD_BYTES + MAP_BYTES + INDIVIDUALS_BYTES + POPULATION_BYTES + LINES_BYTES + INDEXES_BYTES + DAY_BYTES;
const MONTHS_PER_YEAR: u16 = 12;
/// Days after settling at whose close the save the injections load is taken.
const INJECTION_SAVE_DAY: u16 = 30;
/// The key of a build's identity hash: any fixed value.
const BUILD_KEY: [u64; 2] = [0x5048_5820_4255_494c, 0x4420_4944_2031_3131];

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
fn check_ratchets(path: &Path, counters: &[(&str, u64)]) -> Result<Vec<String>, String> {
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

/// The running binary's identity, which a save names so that another build refuses it: the hash of its bytes.
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
    world.record_save(SaveMeasure {
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

/// The most barriers any empty day crossed, a day being empty when its sub-steps visited no row.
fn empty_day_barriers(w: Inspector<'_>) -> u64 {
    let mut per_day: Vec<(phx_id::Day, u64, u64)> = Vec::new();
    for r in w.substep_records() {
        match per_day.last_mut() {
            Some((day, barriers, rows)) if *day == r.day => {
                *barriers += r.barriers;
                *rows += r.rows;
            }
            _ => per_day.push((r.day, r.barriers, r.rows)),
        }
    }
    greatest(per_day.iter().filter(|(_, _, rows)| *rows == 0).map(|(_, b, _)| *b))
}

/// What the map's generation and GEO's day left: its attempts and rejections, its places and deposits, and the events
/// its handlers recorded.
fn geo_report(w: Inspector<'_>) -> serde_json::Value {
    let geo = w.geo();
    let land: Vec<bool> = geo.map.tiles.iter().map(phx_geo::tile::Tile::is_land).collect();
    let sea: Vec<bool> = land.iter().map(|l| !l).collect();
    let (sea_across, sea_down) = phx_geo::partition::winds(&geo.map.grid, &sea);
    let (land_across, land_down) = phx_geo::partition::winds(&geo.map.grid, &land);
    json!({
        "sea_goes_round": { "east_west": sea_across, "north_south": sea_down },
        "land_goes_round": { "east_west": land_across, "north_south": land_down },
        "phx_geo.generation_attempts": geo.map.attempt + 1,
        "rejections": geo.map.rejections.iter().map(|r| r.condition.clone()).collect::<Vec<_>>(),
        "land_tiles": geo.map.tiles.iter().filter(|t| t.is_land()).count(),
        "regions": geo.map.regions.len(),
        "zones": geo.map.zones.len(),
        "deposits": geo.deposits.len(),
        "events": w.events().len(),
    })
}

/// What the opening left: for each kind, its parties and their equity summed; for each line kind, the balances on
/// each side; the opening's writes, its adjustments and the distributions it read.
fn opening_report(w: Inspector<'_>) -> serde_json::Value {
    let books = w.books();
    let opening = w.opening();
    let mut kinds = serde_json::Map::new();
    let mut lines: std::collections::BTreeMap<&str, [i128; 2]> = std::collections::BTreeMap::new();
    for kind in books.parties.kinds() {
        let parties: Vec<_> = books.parties.of_kind(kind).collect();
        let equity: i128 = parties.iter().map(|p| books.equity(*p)).sum();
        kinds.insert(kind.to_owned(), json!({ "parties": parties.len(), "equity": equity.to_string() }));
        for p in &parties {
            let (place, slot) = books.parties.row(*p);
            for r in phx_ledger::rows::rows(books.parties.table(place), slot) {
                if let phx_num::Missing::Present(b) = r.optional.balance {
                    let side = usize::from(r.side() == phx_ledger::algebra::Side::Liability);
                    let entry = lines.entry(books.ledger.lines.kind_name(r.row.line)).or_default();
                    if let Some(t) = entry.get_mut(side) {
                        *t += i128::from(b);
                    }
                }
            }
        }
    }
    json!({
        "kinds": kinds,
        "lines": lines.iter().map(|(k, [a, l])| ((*k).to_owned(), json!({ "asset": a.to_string(), "liability": l.to_string() }))).collect::<serde_json::Map<_, _>>(),
        "writes": opening.writes.len(),
        "apportioned": opening.apportioned.len(),
        "adjustments": opening.adjustments.iter().map(|a| json!({ "what": a.what, "drawn": a.drawn.to_string(), "set": a.set.to_string() })).collect::<Vec<_>>(),
        "distributions": opening.distributions.iter().map(|(n, s)| json!({ "name": n, "source": s })).collect::<Vec<_>>(),
    })
}

/// What the days settled: the dated flows' lines, instructions and those settled, the value paid gross per
/// currency, and the fails by cause, summed over the run.
fn settlement_report(w: Inspector<'_>) -> serde_json::Value {
    let days = w.settlements();
    let sum = |f: fn(&phx_world::Settled) -> u64| days.iter().map(f).sum::<u64>();
    let mut gross: std::collections::BTreeMap<u8, i128> = std::collections::BTreeMap::new();
    let mut fails: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for d in days {
        for (c, v) in &d.measure.gross {
            *gross.entry(*c).or_default() += v;
        }
        for (cause, n) in &d.measure.fails {
            *fails.entry(format!("{cause:?}")).or_default() += n;
        }
    }
    json!({
        "days": days.len(),
        "lines_due": sum(|d| d.dues.lines),
        "payments": sum(|d| d.dues.payments),
        "failed": sum(|d| d.dues.failed),
        "lost": sum(|d| d.dues.lost),
        "lost_past_failed": sum(|d| d.dues.lost_past_failed),
        "heads_read": sum(|d| d.dues.heads_read),
        "rows_scanned": sum(|d| d.dues.rows_scanned),
        "rows_due": sum(|d| d.dues.rows_due),
        "fixed_point_iterations": sum(|d| d.dues.iterations),
        "ring_parties": sum(|d| d.dues.ring_parties),
        "largest_ring": greatest(days.iter().map(|d| d.dues.ring_parties)),
        "ring_value": days.iter().map(|d| d.dues.ring_value).sum::<i128>().to_string(),
        "runs_read": sum(|d| d.dues.runs_read),
        "gross_paid": days.iter().map(|d| d.dues.gross).sum::<i128>().to_string(),
        "settled": sum(|d| d.dues.settled),
        "days_with_payments": days.iter().filter(|d| d.dues.settled > 0).count(),
        "gross": gross.iter().map(|(c, v)| (c.to_string(), json!(v.to_string()))).collect::<serde_json::Map<_, _>>(),
        "fails": fails,
    })
}

/// The ratcheted counters the run reads: the empty day's barriers, the map's bytes, the most on any day of the
/// settlement's rows, heads, payments, iterations, buffers and losers drawn, and of the agents' gathered, read, drawn
/// again, hit and drawn afresh.
fn counters(w: Inspector<'_>) -> [(&'static str, u64); 15] {
    let most =
        |f: fn(&phx_ledger::apply_batch::DaySettlement) -> u64| greatest(w.settlements().iter().map(|s| f(&s.dues)));
    let agents = |f: fn(&phx_world::agents::AgentDay) -> u64| greatest(w.agent_days().iter().map(f));
    [
        ("phx_exec.barriers_per_empty_day", empty_day_barriers(w)),
        ("phx_geo.map_bytes", u64::try_from(w.geo().bytes()).unwrap_or(u64::MAX)),
        ("phx_ledger.rows_streamed", most(|d| d.rows_due)),
        ("phx_ledger.run_heads_read", most(|d| d.heads_read)),
        ("phx_ledger.run_rows_scanned", most(|d| d.rows_scanned)),
        ("phx_ledger.run_rows_not_due", most(|d| d.rows_scanned - d.rows_due)),
        ("phx_ledger.payments", most(|d| d.payments)),
        ("phx_ledger.fixed_point_iterations", most(|d| d.iterations)),
        ("phx_ledger.day_buffer_peak_bytes", most(|d| d.buffer_bytes)),
        ("phx_ledger.losers_drawn", most(|d| d.lost)),
        ("phx_pop.agents_gathered", agents(|d| d.gathered)),
        ("phx_pop.bookings_read", agents(|d| d.read)),
        ("phx_pop.redraws", agents(|d| d.redraws)),
        ("phx_pop.hits", agents(|d| d.hits)),
        ("phx_pop.agents_booked", agents(|d| d.booked)),
    ]
}

/// What the markets did over the run: matches, failures, re-choice rounds and commitments drawn, and the prints.
fn markets_report(w: Inspector<'_>) -> serde_json::Value {
    let m = w.markets();
    let matches: usize = m.tape.sets().iter().map(|s| s.matches.len()).sum();
    let drawn = m
        .tape
        .sets()
        .iter()
        .flat_map(|s| &s.matches)
        .filter(|x| matches!(x.draws, phx_num::Missing::Present(_)))
        .count();
    json!({
        "phx_market.matches": matches,
        "phx_market.failures": m.tape.failures().len(),
        "phx_market.rechoice_rounds": m.days.iter().map(|d| d.rechoice_rounds).sum::<u64>(),
        "phx_market.commitments_drawn": drawn,
        "prints": m.tape.prints().len(),
        "market_days": m.days.len(),
    })
}

/// What the accounts hold at the run's end: the equity accounts, the unpaid claims, and the periods closed on its
/// last posting.
fn accounts_report(w: Inspector<'_>) -> serde_json::Value {
    let a = w.accounts();
    let (receivable, payable) = a.claims.totals();
    json!({
        "equity_accounts": a.equity.len(),
        "equity_total": a.equity.parties().filter_map(|p| match a.equity.of(p) {
            phx_num::Missing::Present(x) => Some(i128::from(x.balance())),
            phx_num::Missing::Absent => None,
        }).sum::<i128>().to_string(),
        "receivable": receivable.to_string(),
        "payable": payable.to_string(),
        "claim_entries": a.claims.lines(),
    })
}

/// The saves the run took: each one's day, its stores' sizes, its write and check times, and whether it read back to
/// its close's hash.
/// The world's configuration from the run's arguments.
fn config(args: &RunArgs) -> Result<WorldConfig, String> {
    Ok(WorldConfig {
        seed: args.seed,
        data: args.data.clone(),
        setup: args.setup.clone(),
        run_dir: args.run_dir.clone(),
        read_trace: args.read_trace,
        representation: match &args.representation {
            Some(named) => phx_num::Missing::Present(representation(named)?),
            None => phx_num::Missing::Absent,
        },
    })
}

/// The representation `twins:K` or `small:K` names.
///
/// # Errors
/// Anything else, or a factor of nought.
pub(crate) fn representation(named: &str) -> Result<phx_pop::prims::Representation, String> {
    let bad = || format!("`{named}` is neither `twins:K` nor `small:K` for a factor K of one or more");
    let (mode, factor) = named.split_once(':').ok_or_else(bad)?;
    let k: u32 = factor.parse().map_err(|_| bad())?;
    if k == 0 {
        return Err(bad());
    }
    match mode {
        "twins" => Ok(phx_pop::prims::Representation { multiplicity: k, population_divisor: 1 }),
        "small" => Ok(phx_pop::prims::Representation { multiplicity: 1, population_divisor: k }),
        _ => Err(bad()),
    }
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
                let distance = match d.distance {
                    phx_num::Missing::Present(x) => json!(x),
                    phx_num::Missing::Absent => serde_json::Value::Null,
                };
                (d.id.clone(), json!({ "day": d.day.get(), "distance": distance }))
            })
            .collect()
    };
    json!({ "settled": at(settled), "ended": at(ended) })
}

fn saves_report(w: Inspector<'_>) -> serde_json::Value {
    let saves: Vec<serde_json::Value> = w
        .saves()
        .iter()
        .map(|s| {
            json!({
                "day": crate::measure::calendar::date_text(w.date(s.day)),
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

/// The live checks selected, each printed as it runs: their outcomes for the report, and whether none failed.
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

/// Runs the world to its last day, saving it at each save interval and taking the injections' save, whose directory
/// it returns.
fn play(
    world: &mut World,
    args: &RunArgs,
    settle_end: phx_id::Day,
    end: phx_id::Day,
    clock: &WallClock,
    obs: &mut Observing,
) -> Result<Option<PathBuf>, String> {
    let saves = args.saves.clone().unwrap_or_else(|| args.run_dir.join("saves"));
    let build = build_id()?;
    let mut period = save_period(Inspector::new(world), world.today())?;
    let w = Inspector::new(world);
    let injection_day = Period::days(INJECTION_SAVE_DAY).map_or(settle_end, |p| w.calendar().plus(settle_end, p));
    let mut injection_save = None;
    obs.settling(w, settle_end);
    while world.today() < end {
        world.run_turn_observed(&[], clock, Some(&mut obs.watch));
        println!("{}", progress(Inspector::new(world), settle_end));
        obs.settling(Inspector::new(world), settle_end);
        let now = save_period(Inspector::new(world), world.today())?;
        if now != period {
            save_and_check(world, &saves, &build, clock)?;
            period = now;
        }
        if injection_save.is_none() && world.today() >= injection_day {
            injection_save = Some(world.save(&args.run_dir.join("inject-save"), &build)?.dir);
        }
    }
    Ok(injection_save)
}

/// The turn just run, so a run can be followed as it goes: its dates and days, its wall time, the payments due and
/// failed in it, and the audit's findings so far.
fn progress(w: Inspector<'_>, settle_end: phx_id::Day) -> String {
    let Some(turn) = w.turn_records().last() else { return "no turn run".to_owned() };
    let (mut due, mut failed) = (0_u64, 0_u64);
    for s in w.settlements().iter().filter(|s| s.day >= turn.first && s.day <= turn.last) {
        due += s.dues.payments;
        failed += s.dues.failed;
    }
    let date = |d| crate::measure::calendar::date_text(w.date(d));
    let phase = if turn.last < settle_end { "settling" } else { "running" };
    let wall = turn.wall_ns.map_or_else(|| "untimed".to_owned(), |ns| format!("{} ms", ns / 1_000_000));
    format!(
        "{phase} {} to {}: {} days in {wall}; payments {due} due, {failed} failed; {} findings",
        date(turn.first),
        date(turn.last),
        turn.days,
        w.findings().len()
    )
}

/// Each sub-step's wall time over the days it ran, in the day's order: its total, median and greatest, so a run shows
/// where its days go.
fn substeps_report(w: Inspector<'_>) -> Vec<serde_json::Value> {
    phx_core::SUB_STEPS
        .iter()
        .filter_map(|info| {
            let mut times: Vec<u64> = w
                .substep_records()
                .iter()
                .filter(|r| r.substep == info.step.ordinal())
                .filter_map(|r| r.wall_ns)
                .collect();
            if times.is_empty() {
                return None;
            }
            times.sort_unstable();
            let total: u64 = times.iter().sum();
            let median = times.get((times.len() - 1) / 2).copied();
            Some(json!({
                "substep": info.label,
                "days": times.len(),
                "total_ms": total / 1_000_000,
                "median_ns": median,
                "greatest_ns": times.last().copied(),
            }))
        })
        .collect()
}

/// The pool the world's sharded work runs on: the cores the system allows, or the first `workers` of them.
pub(crate) fn pool(workers: Option<usize>) -> Result<std::sync::Arc<phx_exec::Pool>, String> {
    let mut spec = phx_exec::spec::PoolSpec::detect();
    if let Some(n) = workers {
        spec.cores.truncate(n);
    }
    phx_exec::Pool::new(&spec).map(std::sync::Arc::new).map_err(|e| e.0)
}

/// Assembles, settles and runs the world, then checks it and writes its report; true when every check passes, every
/// counter keeps its ratchet and the memory keeps its budget.
pub fn run(args: &RunArgs) -> Result<bool, String> {
    crate::panic_hook::install(format!("seed{}-pid{}", args.seed, std::process::id()), PathBuf::from("violations"));
    let config = config(args)?;
    let clock = WallClock::new();
    let assembling = clock.now_ns();
    let mut world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    world.use_pool(pool(args.workers)?);
    let assembly_ns = clock.now_ns().checked_sub(assembling);
    let (settle_end, end) = span(Inspector::new(&world), args.days, args.total_days)?;
    let definitions = phx_obs::Definitions::read(&args.data)?;
    let (mut obs, opening) = Observing::open(Inspector::new(&world), &definitions)?;
    let started = clock.now_ns();
    let injection_save = play(&mut world, args, settle_end, end, &clock, &mut obs)?;
    let run_ns = clock.now_ns().checked_sub(started);
    let injecting = clock.now_ns();
    if let Some(dir) = &injection_save {
        for r in inject_apart(args, dir)? {
            world.record_injection(r);
        }
    }
    let inject_ns = clock.now_ns().checked_sub(injecting);
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
    let turns = w.turn_records();
    let report = json!({
        "seed": args.seed,
        "settle_years": w.settling_years(),
        "days": args.days,
        "total_days": args.total_days,
        "read_trace": args.read_trace,
        "workers": args.workers,
        "first_day": crate::measure::calendar::date_text(w.date(w.day_zero().succ())),
        "last_day": crate::measure::calendar::date_text(w.date(w.today())),
        "settled_on": crate::measure::calendar::date_text(w.date(settle_end)),
        "turns": turns.len(),
        "days_run": turns.iter().map(|t| u64::from(t.days)).sum::<u64>(),
        "longest_turn_days": greatest(turns.iter().map(|t| u64::from(t.days))),
        "build_seconds": args.build_seconds,
        "run_ms": run_ns.map(|n| n / 1_000_000),
        "assembly_ms": assembly_ns.map(|n| n / 1_000_000),
        "inject_ms": inject_ns.map(|n| n / 1_000_000),
        "geo": geo_report(w),
        "opening": opening_report(w),
        "settlement": settlement_report(w),
        "substeps": substeps_report(w),
        "markets": markets_report(w),
        "accounts": accounts_report(w),
        "saves": saves_report(w),
        "reads": reads_report(&obs.watch.recorder, &view),
        "drift": drift_report(&settled, &ended),
        "representation": w.population().representation.name(),
        "injections": crate::inject::report(w.injections()),
        "peak_resident_bytes": peak,
        "memory_budget_bytes": WORLD_BYTES,
        "reserved_bytes": w.bytes_reserved(),
        "counters": counters.iter().map(|(n, v)| ((*n).to_owned(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "ratchet_failures": ratchet_failures,
        "findings": w.findings().len(),
        "findings_by": findings_by(w.findings()),
        "audit": {
            "families": w.families().iter().map(|f| f.name).collect::<Vec<_>>(),
            "closes": w.closes().len(),
            "phx_audit.rows_checked": w.closes().iter().map(|c| c.rows_checked).sum::<u64>(),
            "most_rows_checked_in_a_close": greatest(w.closes().iter().map(|c| c.rows_checked)),
        },
        "placeholders": w.placeholders().len(),
        "standing_shapes": w.standing_shapes().len(),
        "world_hash": format!("{:032x}", w.world_hash()),
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
        "{} turns, {} days, peak {} MiB; {}",
        turns.len(),
        turns.iter().map(|t| u64::from(t.days)).sum::<u64>(),
        peak.map_or(0, |p| p >> 20),
        if all_pass && ratchet_failures.is_empty() && memory_ok { "clean" } else { "not clean" }
    );
    Ok(all_pass && ratchet_failures.is_empty() && memory_ok)
}

/// Assembles the world and writes its calendar's measurement.
pub fn measure_calendar(data: &Path, setup: &Path, run_dir: &Path, out: &Path) -> Result<bool, String> {
    let config = WorldConfig {
        seed: 0,
        data: data.to_path_buf(),
        setup: setup.to_path_buf(),
        run_dir: run_dir.to_path_buf(),
        read_trace: false,
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

/// The run's findings counted by family and clause, each with its first day and detail, so a report says what fired.
fn findings_by(findings: &[phx_core::Finding]) -> Vec<serde_json::Value> {
    let mut by: std::collections::BTreeMap<(&str, &str), (usize, &phx_core::Finding)> =
        std::collections::BTreeMap::new();
    for f in findings {
        by.entry((f.family, f.clause)).or_insert((0, f)).0 += 1;
    }
    by.into_iter()
        .map(|((family, clause), (n, first))| {
            json!({ "family": family, "clause": clause, "count": n, "first_day": first.day.get(), "first": first.detail })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::representation;

    #[test]
    fn representation_names_one_factor() {
        let r = representation("twins:170").expect("twins");
        assert_eq!((r.multiplicity, r.population_divisor), (170, 1));
        let r = representation("small:170").expect("a small world");
        assert_eq!((r.multiplicity, r.population_divisor), (1, 170));
        for bad in ["twins:0", "small:0", "twins", "cells:4", "twins:-1"] {
            assert!(representation(bad).is_err(), "`{bad}` refused");
        }
    }
}
