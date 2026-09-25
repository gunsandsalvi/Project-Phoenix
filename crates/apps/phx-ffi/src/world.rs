//! The world on the phone: assembled from the data the app unpacked, then turns run one after another, each turn's
//! wall time, its days and the process's resident peak sent to the app as the turn closes and written into the report
//! with the opening's time and the median and worst turn against the budget.

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
const WORLD_REPORT_VERSION: u64 = 1;
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
    let clock = Mono(Instant::now());
    let data = PathBuf::from(data);
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
    let opened = opening_ms.map_or_else(|| "unclocked".to_owned(), |ms| format!("{ms} ms"));
    show(host, "world", "opening", format!("{opened}, peak {}", mib(opened_peak)), String::new(), "");
    let mut rows = Vec::new();
    let mut walls = Vec::new();
    for turn in 0..turns {
        let record = world.run_turn(&[], &clock);
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
            ("wall_ms", Json::opt(wall_ms, Json::UInt)),
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
    let report = Json::obj([
        ("report_version", Json::UInt(WORLD_REPORT_VERSION)),
        ("commit", Json::str(option_env!("PHX_COMMIT").unwrap_or("unknown"))),
        ("opening_ms", Json::opt(opening_ms, Json::UInt)),
        ("opening_vm_hwm_bytes", Json::opt(opened_peak, Json::UInt)),
        ("turns", Json::Array(rows)),
        ("median_turn_ms", Json::opt(middle, Json::UInt)),
        ("worst_turn_ms", Json::opt(worst, Json::UInt)),
        ("vm_hwm_bytes", Json::opt(proc_kib("status", "VmHWM:"), Json::UInt)),
        ("world_hash", Json::str(format!("{:032x}", w.world_hash()))),
        ("findings", Json::UInt(u64::try_from(w.findings().len()).unwrap_or(u64::MAX))),
    ]);
    let text = report.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show(host, "report", "written", report_path.to_owned(), String::new(), "");
    Ok(text)
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
