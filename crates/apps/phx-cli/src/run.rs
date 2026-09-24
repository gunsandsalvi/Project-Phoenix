use std::path::{Path, PathBuf};

use phx_core::Period;
use phx_exec::Clock;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{Inspector, WorldConfig, assemble};
use serde::Deserialize;
use serde_json::json;

use crate::RunArgs;
use crate::checks::{CHECKS, Outcome};
use crate::clock::WallClock;

/// Resident memory the empty world may take.
const EMPTY_WORLD_BYTES: u64 = 50 << 20;
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

/// The day settling ends, the owner's length after day zero, and the day the run ends, `days` after that.
fn span(w: Inspector<'_>, days: u16) -> Result<(phx_id::Day, phx_id::Day), String> {
    let years = u16::try_from(w.settling_years()).map_err(|_| "too long a settling")?;
    let months = years.checked_mul(MONTHS_PER_YEAR).ok_or("too long a settling")?;
    let settled = Period::months(months).map_or(w.day_zero(), |p| w.calendar().plus(w.day_zero(), p));
    let end = Period::days(days).map_or(settled, |p| w.calendar().plus(settled, p));
    Ok((settled, end))
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
    let (settle_end, end) = span(Inspector::new(&world), args.days)?;
    let started = clock.now_ns();
    while world.today() < end {
        world.run_turn(&[], &clock);
    }
    let run_ns = clock.now_ns().checked_sub(started);
    let w = Inspector::new(&world);
    let mut results = Vec::new();
    let mut all_pass = true;
    for check in CHECKS.iter().filter(|c| selected(&args.checks, c.id)) {
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
    let empty_day_barriers = empty_day_barriers(w);
    let map_bytes = u64::try_from(w.geo().bytes()).unwrap_or(u64::MAX);
    let counters = [("phx_exec.barriers_per_empty_day", empty_day_barriers), ("phx_geo.map_bytes", map_bytes)];
    let ratchet_failures = check_ratchets(&args.ratchets, &counters)?;
    for f in &ratchet_failures {
        println!("ratchet: {f}");
    }
    let peak = peak_resident_bytes();
    let memory_ok = peak.is_some_and(|p| p <= EMPTY_WORLD_BYTES);
    let turns = w.turn_records();
    let report = json!({
        "seed": args.seed,
        "settle_years": w.settling_years(),
        "days": args.days,
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
        "geo": geo_report(w),
        "peak_resident_bytes": peak,
        "empty_world_budget_bytes": EMPTY_WORLD_BYTES,
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
