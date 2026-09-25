use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use phx_exec::probe::{CoreRate, GatherRate, barrier, core_rates, filled_region, gather, sweep};
use phx_exec::{Clock, IntentBuf, KeyedReduce, Pool, PoolSpec, mix64, radix_sort};
use phx_rand::{
    AliasTable, Draws, Seed, Subject, SubjectTag, accept, below_u64, beta, binomial, binomial_at_least_one, gamma,
    geometric, hypergeometric, multinomial, normal, open_unit, philox, philox_x4, pick_without_replacement, stream_key,
};

use crate::json::Json;

/// The report's layout; a change to it is a new version and a new schema.
const REPORT_VERSION: u64 = 2;
const GIB: u64 = 1 << 30;

/// The phone and the build, as the app knows them.
#[derive(Clone, Debug, uniffi::Record)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub soc: String,
    pub android_sdk: i32,
    pub total_ram_bytes: u64,
    pub app_version: String,
    /// When the run started, as the phone's calendar says; the bench reads no date itself.
    pub started_at: String,
}

/// One result as it completes, for the screen.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BenchLine {
    pub section: String,
    pub name: String,
    pub value: String,
    pub target: String,
    /// "met", "missed", or empty where the step sets no target.
    pub verdict: String,
    pub thermal_status: i32,
}

/// What the app offers the bench: the thermal status the system reports, and a place to show each result.
#[uniffi::export(with_foreign)]
pub trait BenchHost: Send + Sync {
    fn thermal_status(&self) -> i32;
    fn on_line(&self, line: BenchLine);
}

/// The monotonic clock, which the bench alone reads.
struct Mono(Instant);

impl Clock for Mono {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

struct Run<'a> {
    host: &'a dyn BenchHost,
    clock: Mono,
    targets: Vec<Json>,
}

enum Target {
    AtMost(f64),
    AtLeast(f64),
}

impl Run<'_> {
    fn line(&self, section: &str, name: &str, value: String, target: Option<(&Target, f64)>) -> Json {
        let (target_text, verdict) = match target {
            Some((Target::AtMost(t), v)) => (format!("≤ {t}"), if v <= *t { "met" } else { "missed" }),
            Some((Target::AtLeast(t), v)) => (format!("≥ {t}"), if v >= *t { "met" } else { "missed" }),
            None => (String::new(), ""),
        };
        let thermal = self.host.thermal_status();
        self.host.on_line(BenchLine {
            section: section.to_owned(),
            name: name.to_owned(),
            value: value.clone(),
            target: target_text.clone(),
            verdict: verdict.to_owned(),
            thermal_status: thermal,
        });
        Json::obj([
            ("section", Json::str(section)),
            ("name", Json::str(name)),
            ("value", Json::str(value)),
            ("target", Json::str(target_text)),
            ("verdict", Json::str(verdict)),
            ("thermal_status", Json::Int(i64::from(thermal))),
        ])
    }

    fn target(&mut self, name: &str, unit: &str, value: f64, target: &Target) {
        let line = self.line("targets", name, format!("{value:.3} {unit}"), Some((target, value)));
        self.targets.push(line);
    }

    fn now(&self) -> u64 {
        self.clock.now_ns()
    }
}

fn rates_json(at: &str, thermal: i32, rates: &[CoreRate], pool_cores: &[usize]) -> (Json, f64) {
    let fastest = largest(rates.iter().map(|r| r.alone_per_s));
    let loaded: u64 = rates.iter().filter(|r| pool_cores.contains(&r.core)).map(|r| r.loaded_per_s).sum();
    let fast_core_s = if fastest == 0 { 0.0 } else { to_f64(loaded) / to_f64(fastest) };
    let cores = rates
        .iter()
        .map(|r| {
            Json::obj([
                ("core", Json::UInt(count(r.core))),
                ("capacity", Json::opt(r.capacity, |c| Json::UInt(u64::from(c)))),
                ("in_pool", Json::Bool(pool_cores.contains(&r.core))),
                ("alone_units_per_s", Json::UInt(r.alone_per_s)),
                ("loaded_units_per_s", Json::UInt(r.loaded_per_s)),
            ])
        })
        .collect();
    let json = Json::obj([
        ("at", Json::str(at)),
        ("thermal_status", Json::Int(i64::from(thermal))),
        ("pool_fast_core_seconds_per_s", Json::Float(fast_core_s)),
        ("cores", Json::Array(cores)),
    ]);
    (json, fast_core_s)
}

#[expect(clippy::cast_precision_loss, clippy::as_conversions, reason = "report values, never the world's")]
fn to_f64(v: u64) -> f64 {
    v as f64
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The largest of some values, or zero for none.
fn largest(values: impl IntoIterator<Item = u64>) -> u64 {
    values.into_iter().fold(0, |m, v| if v > m { v } else { m })
}

fn gather_json(g: &GatherRate) -> Json {
    Json::obj([
        ("bytes", Json::UInt(g.bytes)),
        ("prefetch", Json::Bool(g.prefetch)),
        ("rows", Json::UInt(g.rows)),
        ("ns_per_row", Json::UInt(g.ns_per_row)),
        ("core_ns_per_row", Json::UInt(g.core_ns_per_row)),
        ("page_size", Json::opt(g.page_size, Json::UInt)),
    ])
}

/// Nanoseconds per call of `op`, run in batches on a fresh draw cursor each until `budget_ns` has passed.
fn per_op(clock: &Mono, budget_ns: u64, batch: u32, mut op: impl FnMut(&mut Draws)) -> (u64, u64) {
    let key = stream_key(Seed::new(7), "bench");
    let start = clock.now_ns();
    let (mut ops, mut day) = (0_u64, 0_u32);
    while clock.now_ns() - start < budget_ns {
        let mut d = Draws::new(key, Subject::new(SubjectTag::World, 0), day, 0);
        for _ in 0..batch {
            op(&mut d);
        }
        ops += u64::from(batch);
        day += 1;
    }
    ((clock.now_ns() - start) / ops, ops)
}

/// The samplers of the random-number crate, each on the fastest core alone.
fn samplers(clock: &Mono, budget_ns: u64) -> Vec<(&'static str, u64, u64)> {
    let table = AliasTable::new(&[3.0, 0.5, 1.0, 6.0, 0.25, 9.0, 2.0, 1.0]);
    let counts: Vec<u64> = (0..64).map(|i| 1 + i % 7).collect();
    let mut out5 = [0_u64; 5];
    let mut picks = vec![0_u64; 64];
    let mut results = Vec::new();
    let mut ctr = [1_u32, 2, 3, 4];
    let mut run = |name: &'static str, op: &mut dyn FnMut(&mut Draws)| {
        let (ns, ops) = per_op(clock, budget_ns, 1000, op);
        results.push((name, ns, ops));
    };
    run("philox", &mut |_| ctr = black_box(philox(ctr, [5, 6])));
    run("philox_x4", &mut |_| {
        black_box(philox_x4([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]], [5, 6]));
    });
    run("open_unit", &mut |d| {
        black_box(open_unit(d));
    });
    run("below_u64", &mut |d| {
        black_box(below_u64(d, 1_000_003));
    });
    run("binomial_inversion", &mut |d| {
        black_box(binomial(d, 40, 0.05));
    });
    run("binomial_btpe", &mut |d| {
        black_box(binomial(d, 1_000, 0.5));
    });
    run("binomial_at_least_one", &mut |d| {
        black_box(binomial_at_least_one(d, 12, 0.01));
    });
    run("multinomial", &mut |d| multinomial(d, 37, &[0.1, 0.05, 0.25, 0.4, 0.2], &mut out5));
    run("alias_draw", &mut |d| {
        black_box(table.draw(d));
    });
    run("hypergeometric_hin", &mut |d| {
        black_box(hypergeometric(d, 50, 20, 10));
    });
    run("hypergeometric_hrua", &mut |d| {
        black_box(hypergeometric(d, 1_000, 300, 100));
    });
    run("pick_without_replacement", &mut |d| pick_without_replacement(d, &counts, 8, &mut picks));
    run("geometric", &mut |d| {
        let _ = black_box(geometric(d, 0.01));
    });
    run("accept", &mut |d| {
        black_box(accept(d, 0.3));
    });
    run("normal", &mut |d| {
        black_box(normal(d));
    });
    run("gamma", &mut |d| {
        black_box(gamma(d, 3.0, 1.5));
    });
    run("beta", &mut |d| {
        black_box(beta(d, 2.0, 5.0));
    });
    results
}

/// The memory probe: random gathers over 1, 2 and 3 GiB with and without prefetch, and a sweep of the largest.
fn memory(run: &mut Run<'_>, pool: &Pool) -> (Vec<Json>, Option<u64>) {
    let mut gathers = Vec::new();
    let mut sweep_rate = None;
    for gib in [1_u64, 2, 3] {
        let region = filled_region(pool, gib * GIB);
        for prefetch in [false, true] {
            if let Some(g) = gather(pool, &run.clock, &region, 1 << 22, prefetch) {
                let label = format!("random gather, {gib} GiB, {}", if prefetch { "prefetch" } else { "no prefetch" });
                run.line(
                    "gather",
                    &label,
                    format!("{} ns/row wall, {} core-ns/row", g.ns_per_row, g.core_ns_per_row),
                    None,
                );
                gathers.push(gather_json(&g));
            }
        }
        if gib == 3 {
            sweep_rate = sweep(pool, &run.clock, &region);
            let text = sweep_rate.map_or_else(|| "unmeasured".to_owned(), |r| format!("{:.2} GB/s", to_f64(r) / 1e9));
            run.line("sweep", "sequential read, 3 GiB, whole pool", text, None);
        }
    }
    (gathers, sweep_rate)
}

/// The pool's kernels at the sizes the budget names, each against its phone target.
fn kernels(run: &mut Run<'_>, pool: &Pool) {
    let pairs: Vec<(u64, u32)> = (0..10_000_000_u32).map(|i| (mix64(u64::from(i)), i % 1_000_000)).collect();
    let (mut items, mut scratch) = (pairs.clone(), Vec::with_capacity(pairs.len()));
    let t = run.now();
    radix_sort(Some(pool), &mut items, &mut scratch);
    let radix_ms = to_f64(run.now() - t) / 1e6;
    run.target("radix sort of 10^7 (u64, u32)", "ms", radix_ms, &Target::AtMost(250.0));
    drop((items, scratch));

    let chunks: Vec<Vec<(u64, u64)>> =
        pairs.chunks(4096).map(|c| c.iter().map(|(k, v)| (k % 1_000_000, u64::from(*v))).collect()).collect();
    drop(pairs);
    let t = run.now();
    let reduced = KeyedReduce::run(Some(pool), &chunks, |acc: &mut u64, v| *acc += v);
    let keyed_ms = to_f64(run.now() - t) / 1e6;
    black_box(reduced.shards.len());
    run.target("keyed reduce of 10^7 pairs", "ms", keyed_ms, &Target::AtMost(300.0));
    drop(chunks);

    let mut intents: IntentBuf<u64> = IntentBuf::new(4);
    intents.reset(64);
    for (c, handlers) in (0_u64..).zip(intents.chunks_mut()) {
        for (h, b) in (0_u64..).zip(handlers.iter_mut()) {
            b.extend((0..250_000).map(|i| c ^ h ^ i));
        }
    }
    let mut out = Vec::new();
    // The first gather sizes the output; the second, into memory already committed, is the one measured.
    phx_exec::gather(Some(pool), &intents, &mut out);
    let t = run.now();
    phx_exec::gather(Some(pool), &intents, &mut out);
    let secs = to_f64(run.now() - t) / 1e9;
    let bytes = to_f64(count(out.len() * size_of::<u64>()));
    run.target("gather of 512 MB of intents", "GB/s", bytes / secs / 1e9, &Target::AtLeast(4.0));
}

/// Agents the per-agent costs are measured over, of three persons and forty attachments each: the design point's.
const COST_AGENTS: u32 = 50_000;
const COST_PERSONS: u32 = 3;
const COST_ATTACHMENTS: u32 = 40;
const COST_TWINS: u32 = 170;
/// Next hits drawn and outcomes applied in the measure.
const COST_DRAWS: u32 = 200_000;
const COST_OUTCOMES: u32 = 50_000;

/// Nanoseconds an agent's next hit takes to draw, and a hit's outcome to apply, measured on one core.
fn agent_costs(clock: &Mono) -> (u64, u64) {
    let mut d = Draws::new(stream_key(Seed::new(1), "REP.bench"), Subject::new(SubjectTag::World, 0), 0, 0);
    let stream = stream_key(Seed::new(1), "REP.bench_agents");
    let mut population = crate::agents::agents(COST_AGENTS, COST_PERSONS, COST_ATTACHMENTS, COST_TWINS, stream);
    let Some(date) = phx_id::Date::new(2026, 3, 2) else { return (0, 0) };
    let slots = population.slots.clone();
    let n = u64::try_from(slots.len()).unwrap_or(0);
    let t0 = clock.now_ns();
    for _ in 0..COST_DRAWS {
        let at = usize::try_from(phx_rand::below_u64(&mut d, n)).unwrap_or(0);
        if let Some(slot) = slots.get(at) {
            black_box(crate::agents::hazard(&population, *slot, (phx_id::Day::new(0), date), &mut d));
        }
    }
    let t1 = clock.now_ns();
    for _ in 0..COST_OUTCOMES {
        let at = usize::try_from(phx_rand::below_u64(&mut d, n)).unwrap_or(0);
        if let Some(slot) = slots.get(at) {
            crate::agents::outcome(&mut population, *slot, date, &mut d);
        }
    }
    let t2 = clock.now_ns();
    ((t1 - t0) / u64::from(COST_DRAWS), (t2 - t1) / u64::from(COST_OUTCOMES))
}

/// An agent's costs against the budget's units: a next hit drawn ahead (180 ns) and a hit's outcome applied in place,
/// measured on the fastest core like the samplers.
fn agent_units(run: &mut Run<'_>, pool: &Pool) {
    let clock = &run.clock;
    let Some((draw, apply)) = pool.on_every_worker(|| agent_costs(clock)).into_iter().next() else { return };
    run.target("an agent's next hit drawn ahead", "ns", to_f64(draw), &Target::AtMost(180.0));
    run.target(
        "a hit's outcome on its agent, made explicit and written back",
        "ns",
        to_f64(apply),
        &Target::AtMost(2500.0),
    );
}

/// Core rates sampled at one moment of the run, shown and kept for the report.
fn sample_rates(run: &mut Run<'_>, rates: &mut Vec<Json>, at: &str, pool_cores: &[usize]) -> (Vec<CoreRate>, f64) {
    let thermal = run.host.thermal_status();
    let measured = core_rates(&run.clock, 1_000_000_000).unwrap_or_default();
    let (json, fast) = rates_json(at, thermal, &measured, pool_cores);
    run.line("cores", &format!("pool fast-core-seconds per second ({at})"), format!("{fast:.2}"), None);
    rates.push(json);
    (measured, fast)
}

/// Runs the probe and the micro-benchmarks, showing each result as it completes, and returns the report.
///
/// # Errors
/// When the pool cannot start or the report cannot be written.
pub fn run(device: &DeviceInfo, host: &dyn BenchHost, report_path: &str) -> Result<String, String> {
    let text = measure(device, host)?.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show_written(host, report_path);
    Ok(text)
}

/// Shows the report's path once it is written.
pub(crate) fn show_written(host: &dyn BenchHost, report_path: &str) {
    host.on_line(BenchLine {
        section: "report".to_owned(),
        name: "written".to_owned(),
        value: report_path.to_owned(),
        target: String::new(),
        verdict: String::new(),
        thermal_status: host.thermal_status(),
    });
}

/// The probe and the micro-benchmarks, each result shown as it completes: the report of the device's fundamentals.
///
/// # Errors
/// When the pool cannot start.
pub fn measure(device: &DeviceInfo, host: &dyn BenchHost) -> Result<Json, String> {
    let mut run = Run { host, clock: Mono(Instant::now()), targets: Vec::new() };
    let spec = PoolSpec::detect();
    let pool = Pool::new(&spec).map_err(|e| e.0)?;
    run.line(
        "pool",
        "workers",
        format!("{} on cores {:?}, {} unpinned", pool.workers(), spec.cores, pool.unpinned()),
        None,
    );
    let pool_json = Json::obj([
        ("cores", Json::Array(spec.cores.iter().map(|c| Json::UInt(count(*c))).collect())),
        ("pinned", Json::Bool(spec.pin)),
        ("unpinned", Json::UInt(count(pool.unpinned()))),
    ]);

    let mut rates = Vec::new();
    let (first, _) = sample_rates(&mut run, &mut rates, "start", &spec.cores);
    let fastest = first.iter().fold(None, |best: Option<&CoreRate>, r| match best {
        Some(b) if b.alone_per_s >= r.alone_per_s => Some(b),
        _ => Some(r),
    });

    let barrier_ns = barrier(&pool, &run.clock, 10_000);
    if let Some(ns) = barrier_ns {
        run.target("barrier, hot, whole pool", "µs", to_f64(ns) / 1e3, &Target::AtMost(60.0));
    }
    let (gathers, sweep_rate) = memory(&mut run, &pool);
    sample_rates(&mut run, &mut rates, "after the memory probe", &spec.cores);

    // The random-number samplers, on the fastest core alone.
    let micro_pool =
        Pool::new(&PoolSpec { cores: fastest.map(|r| r.core).into_iter().collect(), pin: true }).map_err(|e| e.0)?;
    let clock = &run.clock;
    let micro = micro_pool.on_every_worker(|| samplers(clock, 300_000_000)).into_iter().next().unwrap_or_default();
    let mut micro_json = Vec::new();
    for (name, ns, ops) in &micro {
        run.line("phx_rand", name, format!("{ns} ns/op"), None);
        micro_json.push(Json::obj([
            ("name", Json::str(format!("phx_rand.{name}"))),
            ("ns_per_op", Json::UInt(*ns)),
            ("ops", Json::UInt(*ops)),
        ]));
    }

    kernels(&mut run, &pool);
    agent_units(&mut run, &micro_pool);
    let (_, fast_end) = sample_rates(&mut run, &mut rates, "end", &spec.cores);
    run.target("pool fast-core-seconds per second at the end", "", fast_end, &Target::AtLeast(3.0));

    let report = Json::obj([
        ("report_version", Json::UInt(REPORT_VERSION)),
        ("step", Json::str("S0.28")),
        ("commit", Json::str(option_env!("PHX_COMMIT").unwrap_or("unknown"))),
        (
            "device",
            Json::obj([
                ("manufacturer", Json::str(device.manufacturer.clone())),
                ("model", Json::str(device.model.clone())),
                ("soc", Json::str(device.soc.clone())),
                ("android_sdk", Json::Int(i64::from(device.android_sdk))),
                ("total_ram_bytes", Json::UInt(device.total_ram_bytes)),
                ("app_version", Json::str(device.app_version.clone())),
                ("started_at", Json::str(device.started_at.clone())),
            ]),
        ),
        ("pool", pool_json),
        ("core_rates", Json::Array(rates)),
        ("barrier_ns", Json::opt(barrier_ns, Json::UInt)),
        ("gathers", Json::Array(gathers)),
        ("sweep_bytes_per_s", Json::opt(sweep_rate, Json::UInt)),
        ("micro", Json::Array(micro_json)),
        ("targets", Json::Array(run.targets.clone())),
    ]);
    Ok(report)
}

/// The app's entry: runs the bench on the calling thread, which must not be the interface's.
#[uniffi::export]
#[expect(clippy::needless_pass_by_value, reason = "the foreign interface hands over owned values")]
pub fn run_bench(device: DeviceInfo, host: Arc<dyn BenchHost>, report_path: String) -> String {
    crate::stopped::watch(&report_path);
    match run(&device, host.as_ref(), &report_path) {
        Ok(report) => report,
        Err(error) => {
            host.on_line(BenchLine {
                section: "error".to_owned(),
                name: "the bench stopped".to_owned(),
                value: error.clone(),
                target: String::new(),
                verdict: String::new(),
                thermal_status: host.thermal_status(),
            });
            error
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Sender, channel};

    use super::{BenchHost, BenchLine, DeviceInfo, run};

    struct Host(Sender<BenchLine>);

    impl BenchHost for Host {
        fn thermal_status(&self) -> i32 {
            0
        }

        fn on_line(&self, line: BenchLine) {
            self.0.send(line).unwrap();
        }
    }

    #[test]
    #[ignore = "the whole bench: minutes of work and 3 GiB of memory, run by hand on a machine that has them"]
    fn bench_runs_end_to_end() {
        let (tx, rx) = channel();
        let device = DeviceInfo {
            manufacturer: "build machine".to_owned(),
            model: "x86-64".to_owned(),
            soc: String::new(),
            android_sdk: 0,
            total_ram_bytes: 0,
            app_version: "test".to_owned(),
            started_at: String::new(),
        };
        let path = std::env::temp_dir().join("phx-bench-report.json");
        let report = run(&device, &Host(tx), path.to_str().unwrap()).unwrap();
        let lines: Vec<BenchLine> = rx.try_iter().collect();
        let summary: Vec<String> = lines
            .iter()
            .map(|l| format!("{} | {} | {} | {} {}", l.section, l.name, l.value, l.target, l.verdict))
            .collect();
        std::fs::write(std::env::temp_dir().join("phx-bench-lines.txt"), summary.join("\n")).unwrap();
        assert!(report.contains("\"targets\"") && lines.iter().any(|l| l.section == "report"));
    }
}
