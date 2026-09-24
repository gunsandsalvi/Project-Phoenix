use std::path::{Path, PathBuf};

use phx_core::Period;
use phx_exec::Clock;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{Inspector, SaveMeasure, World, WorldConfig, assemble};
use serde::Deserialize;
use serde_json::json;

use crate::RunArgs;
use crate::checks::{CHECKS, Outcome};
use crate::clock::WallClock;

/// Resident memory the empty world may take.
const EMPTY_WORLD_BYTES: u64 = 50 << 20;

/// Resident memory the map may take on top of it, its generation included.
const MAP_BYTES: u64 = 80 << 20;

/// Resident memory the individuals may take: their kind tables and facets, which hold the institutions' rows.
const INDIVIDUALS_BYTES: u64 = 225 << 20;

/// Resident memory the world may take at its peak: the budgets of the steps it holds.
const WORLD_BYTES: u64 = EMPTY_WORLD_BYTES + MAP_BYTES + INDIVIDUALS_BYTES;
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

/// The ratcheted counters the run reads: the empty day's barriers, the map's bytes, and the most on any day of the
/// settlement's rows, heads, payments, iterations and buffers.
fn counters(w: Inspector<'_>) -> [(&'static str, u64); 9] {
    let most =
        |f: fn(&phx_ledger::apply_batch::DaySettlement) -> u64| greatest(w.settlements().iter().map(|s| f(&s.dues)));
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
fn live_checks(w: Inspector<'_>, checks: &str) -> (Vec<serde_json::Value>, bool) {
    let mut results = Vec::new();
    let mut all_pass = true;
    for check in CHECKS.iter().filter(|c| selected(checks, c.id)) {
        let (outcome, detail) = match (check.run, check.retired) {
            (Some(f), _) => match f(w) {
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

/// Runs the world to its last day, saving it at each save interval and taking the injections' save, whose directory
/// it returns.
fn play(
    world: &mut World,
    args: &RunArgs,
    settle_end: phx_id::Day,
    end: phx_id::Day,
    clock: &WallClock,
) -> Result<Option<PathBuf>, String> {
    let saves = args.saves.clone().unwrap_or_else(|| args.run_dir.join("saves"));
    let build = build_id()?;
    let mut period = save_period(Inspector::new(world), world.today())?;
    let w = Inspector::new(world);
    let injection_day = Period::days(INJECTION_SAVE_DAY).map_or(settle_end, |p| w.calendar().plus(settle_end, p));
    let mut injection_save = None;
    while world.today() < end {
        world.run_turn(&[], clock);
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

/// Assembles, settles and runs the world, then checks it and writes its report; true when every check passes, every
/// counter keeps its ratchet and the memory keeps its budget.
pub fn run(args: &RunArgs) -> Result<bool, String> {
    crate::panic_hook::install(format!("seed{}-pid{}", args.seed, std::process::id()), PathBuf::from("violations"));
    let config = WorldConfig {
        seed: args.seed,
        data: args.data.clone(),
        setup: args.setup.clone(),
        run_dir: args.run_dir.clone(),
        read_trace: args.read_trace,
    };
    let clock = WallClock::new();
    let assembling = clock.now_ns();
    let mut world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let assembly_ns = clock.now_ns().checked_sub(assembling);
    let (settle_end, end) = span(Inspector::new(&world), args.days, args.total_days)?;
    let started = clock.now_ns();
    let injection_save = play(&mut world, args, settle_end, end, &clock)?;
    let run_ns = clock.now_ns().checked_sub(started);
    let injecting = clock.now_ns();
    if let Some(dir) = &injection_save {
        for r in inject_apart(args, dir)? {
            world.record_injection(r);
        }
    }
    let inject_ns = clock.now_ns().checked_sub(injecting);
    let w = Inspector::new(&world);
    let (results, all_pass) = live_checks(w, &args.checks);
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
        "markets": markets_report(w),
        "accounts": accounts_report(w),
        "saves": saves_report(w),
        "injections": crate::inject::report(w.injections()),
        "peak_resident_bytes": peak,
        "memory_budget_bytes": WORLD_BYTES,
        "reserved_bytes": w.bytes_reserved(),
        "counters": counters.iter().map(|(n, v)| ((*n).to_owned(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "ratchet_failures": ratchet_failures,
        "findings": w.findings().len(),
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
