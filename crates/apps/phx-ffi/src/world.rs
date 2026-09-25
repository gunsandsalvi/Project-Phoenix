//! The world on the phone: assembled from the data the app unpacked and settled for the owner's length, then turns run
//! one after another, each turn's wall time, its days and the process's resident peak sent to the app as the turn
//! closes and written into the report with the opening's and the settling's times and the median and worst turn
//! against the budget; last the world is saved and read back, each timed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use phx_exec::Clock;
use phx_world::registry::WorldConfig;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{Inspector, assemble};

use crate::bench::{BenchHost, BenchLine};
use crate::json::Json;

/// The report's layout; a change to it is a new version and a new schema.
const WORLD_REPORT_VERSION: u64 = 2;
/// Settling's progress is shown once in so many turns.
const SETTLING_SHOWN_EVERY: u32 = 20;
/// The build a save names and a load expects: the commit the library was built from.
const BUILD: &str = match option_env!("PHX_COMMIT") {
    Some(c) => c,
    None => "unknown",
};
/// The budget's median and worst wall time of a turn, in milliseconds, on the phone.
const MEDIAN_MS: u64 = 1_000;
const WORST_MS: u64 = 2_000;
const NS_PER_MS: u64 = 1_000_000;
const KIB: u64 = 1 << 10;
const MIB: u64 = 1 << 20;

/// The monotonic clock, which the run alone reads.
struct Mono(Instant);

impl Clock for Mono {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// A line of `/proc/self/<file>` in kibibytes, as bytes; a system that does not report it reports nothing.
fn proc_kib(file: &str, field: &str) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/self/{file}")).ok()?;
    let line = text.lines().find(|l| l.starts_with(field))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kib.checked_mul(KIB)
}

fn show(host: &dyn BenchHost, section: &str, name: &str, value: String, target: String, verdict: &str) {
    host.on_line(BenchLine {
        section: section.to_owned(),
        name: name.to_owned(),
        value,
        target,
        verdict: verdict.to_owned(),
        thermal_status: host.thermal_status(),
    });
}

fn mib(bytes: Option<u64>) -> String {
    bytes.map_or_else(|| "unread".to_owned(), |b| format!("{} MiB", b / MIB))
}

/// Each macro read's values day by day, and the view's histograms at the last close.
fn reads(recorder: &phx_obs::Recorder, view: &phx_obs::View) -> Json {
    let series = recorder.series().iter().map(|s| {
        let values = s
            .values
            .iter()
            .map(|(d, v)| Json::Array(vec![Json::UInt(u64::from(d.get())), Json::str(v.to_string())]))
            .collect();
        Json::obj([
            ("id", Json::str(s.id.clone())),
            ("unit", Json::str(s.unit.clone())),
            ("values", Json::Array(values)),
        ])
    });
    let histograms = view.histograms.iter().map(|(id, h)| {
        let edges = h.edges().iter().map(|e| Json::Int(*e)).collect();
        let counts = h.counts().iter().map(|c| Json::UInt(*c)).collect();
        Json::obj([
            ("id", Json::str(id.clone())),
            ("edges", Json::Array(edges)),
            ("counts", Json::Array(counts)),
            ("below", Json::UInt(h.below())),
        ])
    });
    Json::obj([("series", Json::Array(series.collect())), ("histograms", Json::Array(histograms.collect()))])
}

/// The middle of a set of values, the upper of two middles.
fn median(values: &[u64]) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len() / 2).copied()
}

/// The world assembled from `data` and its `setup`, then `turns` turns run and reported; the report written to
/// `report_path` and returned.
///
/// # Errors
/// What stopped the assembly or the report's writing.
pub fn run(host: &dyn BenchHost, data: &str, run_dir: &str, turns: u32, report_path: &str) -> Result<String, String> {
    let text = measure(host, data, run_dir, turns)?.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show(host, "report", "written", report_path.to_owned(), String::new(), "");
    Ok(text)
}

/// The world assembled from the data and run for `turns` turns, each shown as it closes: the world's section of the
/// device report.
///
/// # Errors
/// Data that cannot be read, or a world the assembly refuses.
pub fn measure(host: &dyn BenchHost, data: &str, run_dir: &str, turns: u32) -> Result<Json, String> {
    let clock = Mono(Instant::now());
    let data = PathBuf::from(data);
    let definitions = phx_obs::Definitions::read(&data)?;
    let config = WorldConfig {
        seed: 1,
        setup: data.join("setup").join("default.toml"),
        data,
        run_dir: PathBuf::from(run_dir),
        read_trace: false,
    };
    show(host, "world", "opening", "assembling".to_owned(), String::new(), "");
    let started = clock.now_ns();
    let mut world = assemble(SYSTEMS, INTERFACES, &config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let opening_ms = clock.now_ns().checked_sub(started).map(|n| n / NS_PER_MS);
    let opened_peak = proc_kib("status", "VmHWM:");
    let w = Inspector::new(&world);
    let tracers = phx_obs::Tracers::declared(w)?;
    let mut watch = phx_obs::Watch { tracers, recorder: phx_obs::Recorder::new(&definitions.reads, w)? };
    let mut views = phx_obs::Views::new(&definitions.histograms, w)?;
    let opened = opening_ms.map_or_else(|| "unclocked".to_owned(), |ms| format!("{ms} ms"));
    show(host, "world", "opening", format!("{opened}, peak {}", mib(opened_peak)), String::new(), "");
    let settling = settle(host, &mut world, &clock)?;
    let w = Inspector::new(&world);
    let opening = views.close(w, &watch.recorder);
    let mut rows = Vec::new();
    let mut walls = Vec::new();
    for turn in 0..turns {
        // The observer follows its tracers and takes its reads at each day's end, inside the turn's time.
        let record = world.run_turn_observed(&[], &clock, Some(&mut watch));
        let wall_ms = record.wall_ns.map(|n| n / NS_PER_MS);
        let peak = proc_kib("status", "VmHWM:");
        let pss = proc_kib("smaps_rollup", "Pss:");
        let (value, verdict) = match wall_ms {
            Some(ms) => {
                walls.push(ms);
                (format!("{ms} ms, {} days, peak {}, PSS {}", record.days, mib(peak), mib(pss)), ms <= WORST_MS)
            }
            None => (format!("unclocked, {} days", record.days), false),
        };
        show(
            host,
            "world",
            &format!("turn {}", turn + 1),
            value,
            format!("≤ {WORST_MS} ms"),
            if verdict { "met" } else { "missed" },
        );
        rows.push(Json::obj([
            ("turn", Json::UInt(u64::from(turn + 1))),
            ("first_day", Json::UInt(u64::from(record.first.get()))),
            ("days", Json::UInt(u64::from(record.days))),
            ("business", Json::Bool(Inspector::new(&world).any_business(record.last))),
            ("wall_ms", Json::opt(wall_ms, Json::UInt)),
            ("payments", Json::UInt(settled(Inspector::new(&world), (record.first, record.last), |s| s.payments))),
            (
                "rows_scanned",
                Json::UInt(settled(Inspector::new(&world), (record.first, record.last), |s| s.rows_scanned)),
            ),
            ("substeps", substeps(Inspector::new(&world), record.first, record.last)),
            ("vm_hwm_bytes", Json::opt(peak, Json::UInt)),
            ("pss_bytes", Json::opt(pss, Json::UInt)),
            ("thermal_status", Json::Int(i64::from(host.thermal_status()))),
        ]));
    }
    let (middle, worst) = (median(&walls), walls.iter().copied().reduce(|a, b| if b > a { b } else { a }));
    for (name, value, budget) in [("median turn", middle, MEDIAN_MS), ("worst turn", worst, WORST_MS)] {
        let (text, verdict) = match value {
            Some(ms) => (format!("{ms} ms"), if ms <= budget { "met" } else { "missed" }),
            None => ("no turn clocked".to_owned(), "missed"),
        };
        show(host, "world", name, text, format!("≤ {budget} ms"), verdict);
    }
    let w = Inspector::new(&world);
    let view = views.close(w, &watch.recorder);
    let (hash, findings) = (w.world_hash(), w.findings().len());
    let reads = reads(&watch.recorder, &view);
    let drift = drift(&phx_obs::drift(&opening, &view));
    let saved = save_and_load(host, world, &config, &clock)?;
    let report = Json::obj([
        ("report_version", Json::UInt(WORLD_REPORT_VERSION)),
        ("commit", Json::str(option_env!("PHX_COMMIT").unwrap_or("unknown"))),
        ("opening_ms", Json::opt(opening_ms, Json::UInt)),
        ("opening_vm_hwm_bytes", Json::opt(opened_peak, Json::UInt)),
        ("settling", settling),
        ("turns", Json::Array(rows)),
        ("median_turn_ms", Json::opt(middle, Json::UInt)),
        ("worst_turn_ms", Json::opt(worst, Json::UInt)),
        ("vm_hwm_bytes", Json::opt(proc_kib("status", "VmHWM:"), Json::UInt)),
        ("world_hash", Json::str(format!("{hash:032x}"))),
        ("findings", Json::UInt(u64::try_from(findings).unwrap_or(u64::MAX))),
        ("reads", reads),
        ("drift", drift),
        ("save", saved),
    ]);
    Ok(report)
}

/// The world run from day zero to the end of the owner's settling length, turn after turn, its progress shown now
/// and then: the turns, the days and the wall time it took.
///
/// # Errors
/// A settling length the calendar cannot count.
fn settle(host: &dyn BenchHost, world: &mut phx_world::world::World, clock: &Mono) -> Result<Json, String> {
    let w = Inspector::new(world);
    let years = u16::try_from(w.settling_years()).map_err(|_| "too long a settling".to_owned())?;
    let months = years.checked_mul(phx_core::consts::MONTHS_PER_YEAR).ok_or("too long a settling")?;
    let end =
        phx_core::calendar::period::Period::months(months).map_or(w.day_zero(), |p| w.calendar().plus(w.day_zero(), p));
    let started = clock.now_ns();
    let (mut turns, mut days) = (0_u32, 0_u64);
    while Inspector::new(world).today() < end {
        let record = world.run_turn_observed(&[], clock, None);
        turns += 1;
        days += u64::from(record.days);
        if turns % SETTLING_SHOWN_EVERY == 0 {
            let date = Inspector::new(world).date(record.last);
            show(
                host,
                "world",
                "settling",
                format!("{turns} turns, to {}-{:02}-{:02}", date.year(), date.month(), date.day()),
                String::new(),
                "",
            );
        }
    }
    let wall_ms = clock.now_ns().checked_sub(started).map(|n| n / NS_PER_MS);
    let shown = wall_ms.map_or_else(|| "unclocked".to_owned(), |ms| format!("{ms} ms"));
    show(host, "world", "settled", format!("{turns} turns, {days} days, {shown}"), String::new(), "");
    Ok(Json::obj([
        ("years", Json::UInt(u64::from(years))),
        ("turns", Json::UInt(u64::from(turns))),
        ("days", Json::UInt(days)),
        ("wall_ms", Json::opt(wall_ms, Json::UInt)),
        ("vm_hwm_bytes", Json::opt(proc_kib("status", "VmHWM:"), Json::UInt)),
    ]))
}

/// The world saved whole into the run's directory and dropped, then read back: each timed, the save's size, and
/// whether the world read back hashes as the one saved.
///
/// # Errors
/// A save that cannot be written, or one the load refuses.
fn save_and_load(
    host: &dyn BenchHost,
    world: phx_world::world::World,
    config: &WorldConfig,
    clock: &Mono,
) -> Result<Json, String> {
    let root = config.run_dir.join("saves");
    let started = clock.now_ns();
    let rec = world.save(&root, BUILD)?;
    let save_ms = clock.now_ns().checked_sub(started).map(|n| n / NS_PER_MS);
    let bytes: u64 = rec.stores.iter().map(|s| s.bytes).sum();
    drop(world);
    let started = clock.now_ns();
    let loaded = phx_world::registry::load(SYSTEMS, INTERFACES, config, &rec.dir, BUILD)
        .map_err(|e| format!("the save read back was refused:\n{e}"))?;
    let load_ms = clock.now_ns().checked_sub(started).map(|n| n / NS_PER_MS);
    let same = Inspector::new(&loaded).world_hash() == rec.world_hash;
    let ms = |v: Option<u64>| v.map_or_else(|| "unclocked".to_owned(), |ms| format!("{ms} ms"));
    show(host, "world", "save", format!("{}, {}", ms(save_ms), mib(Some(bytes))), String::new(), "");
    show(host, "world", "load", ms(load_ms), String::new(), if same { "met" } else { "missed" });
    Ok(Json::obj([
        ("save_ms", Json::opt(save_ms, Json::UInt)),
        ("save_bytes", Json::UInt(bytes)),
        ("load_ms", Json::opt(load_ms, Json::UInt)),
        ("hash_holds", Json::Bool(same)),
    ]))
}

/// A count of the settlements of the turn's days, summed.
fn settled(
    w: Inspector<'_>,
    (first, last): (phx_id::Day, phx_id::Day),
    count: fn(&phx_ledger::apply_batch::DaySettlement) -> u64,
) -> u64 {
    w.settlements().iter().filter(|s| first <= s.day && s.day <= last).map(|s| count(&s.dues)).sum()
}

/// Each opening distribution's distance from the world's own at the run's end.
fn drift(drifts: &[phx_obs::Drift]) -> Json {
    Json::Array(
        drifts
            .iter()
            .map(|d| {
                let distance = match d.distance {
                    phx_num::Missing::Present(x) => Json::Float(x),
                    phx_num::Missing::Absent => Json::Null,
                };
                Json::obj([
                    ("id", Json::str(d.id.clone())),
                    ("day", Json::UInt(u64::from(d.day.get()))),
                    ("distance", distance),
                ])
            })
            .collect(),
    )
}

/// Each sub-step that ran in a turn's days, by its label, with its wall time summed over them: absent where the clock
/// ran backwards on any of them.
fn substeps(w: Inspector<'_>, first: phx_id::Day, last: phx_id::Day) -> Json {
    let mut by: Vec<(u8, Option<u64>)> = Vec::new();
    for r in w.substep_records().iter().rev().take_while(|r| r.day >= first).filter(|r| r.day <= last) {
        match by.iter_mut().find(|(s, _)| *s == r.substep) {
            Some((_, ns)) => *ns = ns.zip(r.wall_ns).map(|(a, b)| a + b),
            None => by.push((r.substep, r.wall_ns)),
        }
    }
    by.sort_unstable_by_key(|(s, _)| *s);
    Json::Array(
        by.into_iter()
            .filter_map(|(s, ns)| {
                let info = phx_core::SUB_STEPS.get(usize::from(s))?;
                Some(Json::obj([("substep", Json::str(info.label)), ("wall_ns", Json::opt(ns, Json::UInt))]))
            })
            .collect(),
    )
}

/// The app's entry: assembles and runs the world on the calling thread, which must not be the interface's.
#[uniffi::export]
#[expect(clippy::needless_pass_by_value, reason = "the foreign interface hands over owned values")]
pub fn run_world(
    host: Arc<dyn BenchHost>,
    data_dir: String,
    run_dir: String,
    turns: u32,
    report_path: String,
) -> String {
    match run(host.as_ref(), &data_dir, &run_dir, turns, &report_path) {
        Ok(report) => report,
        Err(error) => {
            show(host.as_ref(), "error", "the world stopped", error.clone(), String::new(), "");
            error
        }
    }
}
