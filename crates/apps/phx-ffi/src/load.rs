//! The full-load bench: the finished world's cost on the phone before the finished world exists. It builds the
//! stores whose kernels exist at the finished world's sizes — the household agents with their persons and
//! attachments, and books whose holders' dated rows fall due every business day or once a month — holds the rest of
//! the declared bytes as random words from its own stream, and runs a simulated month, each day's kinds of work at
//! that day type's declared counts through their kernels: settlement, agents' next hits drawn, their outcomes applied,
//! the agenda, gathers of the rows a visit reads with arithmetic on them for the mechanisms not yet built, the audit's
//! slice, and full saves. Its numbers are costs, never the world's.

use std::hint::black_box;
use std::time::Instant;

use phx_exec::{Clock, Pool, PoolSpec, mix64};
use phx_id::{Date, Day, Slot};
use phx_ledger::synthetic::{Settlement, SettlementSize, Unaudited, calendar, settlement};
use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};
use phx_store::SystemBacking;
use serde::Deserialize;

use crate::agents::{Agents, agents, hazard, outcome};
use crate::bench::{BenchHost, BenchLine};
use crate::json::Json;

/// The report section's layout.
const LOAD_VERSION: u64 = 2;
/// The budget: a turn's median and worst wall time, peak resident memory, a full save's time and two saves' bytes.
const MEDIAN_MS: u64 = 1_000;
const WORST_MS: u64 = 2_000;
const MEMORY_BYTES: u64 = 4_500_000_000;
const SAVE_MS: u64 = 5_000;
const TWO_SAVES_BYTES: u64 = 4 * GIB;
const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
const KIB: u64 = 1 << 10;
const NS_PER_MS: u64 = 1_000_000;
/// The share of the held stores and of the agents the audit reads each day: its rolling slice.
const AUDIT_SLICES: u64 = 30;
/// Pieces a parallel kind of work is cut into, per worker.
const PIECES_PER_WORKER: usize = 4;
/// The first date the month runs from, a Monday.
const FIRST: (i32, u8, u8) = (2026, 3, 2);
/// The Monday before it, which the weekly rows are anchored on so that each first falls due in the month's first week.
const WEEK_BEFORE: (i32, u8, u8) = (2026, 2, 23);
/// Words a save writes at a time.
const SAVE_WORDS: usize = 1 << 16;
/// The fold a gather's reads go through, so the reads cannot be skipped.
const FOLD_ROTATE: u32 = 5;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DayType {
    Business,
    Closed,
    Heavy,
}

impl DayType {
    fn index(self) -> usize {
        match self {
            DayType::Business => 0,
            DayType::Closed => 1,
            DayType::Heavy => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            DayType::Business => "business",
            DayType::Closed => "closed",
            DayType::Heavy => "heavy",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Kernel {
    Hazard,
    Outcome,
    Redraw,
    Agenda,
    Gather,
    Settle,
    Audit,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PopulationVolumes {
    agents: u32,
    multiplicity: u32,
    persons_per_agent: u32,
    attachments_per_agent: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettlementVolumes {
    holders: u32,
    daily_rows: u32,
    monthly_rows: u32,
    banks: u32,
    holders_per_line: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    name: String,
    line: String,
    bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Month {
    days: Vec<DayType>,
    saves: Vec<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Work {
    name: String,
    kernel: Kernel,
    line: String,
    /// The words each unit reads, where the kernel gathers.
    reads: Option<u32>,
    /// The held store the kernel updates or sweeps, where it works on one.
    store: Option<String>,
    counts: [u64; 3],
}

/// The finished world's declared volumes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Volumes {
    population: PopulationVolumes,
    settlement: SettlementVolumes,
    #[serde(rename = "store")]
    stores: Vec<Store>,
    month: Month,
    #[serde(rename = "work")]
    works: Vec<Work>,
}

/// The monotonic clock, which the bench alone reads.
struct Mono(Instant);

impl Clock for Mono {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

fn proc_kib(file: &str, field: &str) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/self/{file}")).ok()?;
    let line = text.lines().find(|l| l.starts_with(field))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kib.checked_mul(KIB)
}

fn show(host: &dyn BenchHost, name: &str, value: String, target: String, verdict: &str) {
    host.on_line(BenchLine {
        section: "full load".to_owned(),
        name: name.to_owned(),
        value,
        target,
        verdict: verdict.to_owned(),
        thermal_status: host.thermal_status(),
    });
}

fn draws(seed: u64, subject: u64, day: u32) -> Draws {
    Draws::new(stream_key(Seed::new(seed), "LOAD.bench"), Subject::new(SubjectTag::World, subject), day, 0)
}

fn words_of(bytes: u64) -> usize {
    usize::try_from(bytes / u64::try_from(size_of::<u64>()).unwrap_or(u64::MAX)).unwrap_or(usize::MAX)
}

/// The pieces a parallel kind of work is cut into: a few a worker, so the pool balances them.
fn piece_count(pool: &Pool) -> usize {
    pool.workers() * PIECES_PER_WORKER
}

/// The `p`th of `n` equal pieces of `total`: its first unit and its number of units, the last taking what is left.
fn piece(total: u64, n: usize, p: usize) -> (u64, u64) {
    let (n, p) = (u64::try_from(n).unwrap_or(1), u64::try_from(p).unwrap_or(0));
    let each = total.div_ceil(n);
    let first = p * each;
    if first >= total {
        return (total, 0);
    }
    let rest = total - first;
    (first, if rest < each { rest } else { each })
}

/// A held store: its declared bytes as random words, written by the pool's workers.
fn held(pool: &Pool, bytes: u64, seed: u64) -> Vec<u64> {
    let words = u64::try_from(words_of(bytes)).unwrap_or(0);
    let n = piece_count(pool);
    pool.map(n, |p| {
        let (first, len) = piece(words, n, p);
        (first..first + len).map(|i| mix64(seed ^ i)).collect::<Vec<u64>>()
    })
    .concat()
}

/// A word folded into an accumulator so no read can be skipped.
fn fold(acc: u64, word: u64) -> u64 {
    acc.rotate_left(FOLD_ROTATE) ^ word
}

/// Gathers: each unit reads `reads` random words from every held store in turn and folds them, as a visit reads the
/// rows its decision needs and computes on them.
fn gather(pool: &Pool, stores: &[Vec<u64>], count: u64, reads: u32, day: u32) -> u64 {
    let n = piece_count(pool);
    let folded = pool.map(n, |p| {
        let (first, units) = piece(count, n, p);
        let mut acc = 0_u64;
        let mut d = draws(1, first, day);
        for unit in first..first + units {
            let Some(store) = usize::try_from(unit).ok().and_then(|u| stores.get(u % stores.len())) else { continue };
            let len = u64::try_from(store.len()).unwrap_or(0);
            if len == 0 {
                continue;
            }
            for _ in 0..reads {
                let at = usize::try_from(below_u64(&mut d, len)).unwrap_or(0);
                acc = fold(acc, store.get(at).copied().unwrap_or(0));
            }
        }
        acc
    });
    folded.into_iter().fold(0, fold)
}

/// Read-modify-writes into one held store, each worker in its own shard, as group aggregates and agenda redraws are.
fn update(pool: &Pool, store: &mut [u64], count: u64, day: u32) {
    let n = piece_count(pool);
    let each = store.len().div_ceil(n);
    if each == 0 {
        return;
    }
    let chunks: Vec<(usize, &mut [u64])> = store.chunks_mut(each).enumerate().collect();
    pool.for_each(chunks, |(c, shard)| {
        let (first, units) = piece(count, n, c);
        let mut d = draws(2, first, day);
        let len = u64::try_from(shard.len()).unwrap_or(0);
        for _ in 0..units {
            let at = usize::try_from(below_u64(&mut d, len)).unwrap_or(0);
            if let Some(w) = shard.get_mut(at) {
                *w = fold(*w, mix64(*w));
            }
        }
    });
}

/// Sequential reads of a slice of every store given: the audit's rolling slice, and the agenda's gather.
fn sweep(stores: &[&Vec<u64>], share: u64, offset: u64) -> u64 {
    let mut acc = 0_u64;
    for s in stores {
        let n = s.len() / usize::try_from(share).unwrap_or(1);
        let from = if s.is_empty() { 0 } else { (usize::try_from(offset).unwrap_or(0) * n) % s.len() };
        for w in s.iter().skip(from).take(n) {
            acc = fold(acc, *w);
        }
    }
    acc
}

/// What one day's work took, kind by kind.
struct DayRecord {
    index: usize,
    kind: DayType,
    walls: Vec<(String, u64)>,
    total_ns: u64,
}

/// The month's turns: each business or heavy day closes a turn holding the closed days before it.
fn turns(days: &[DayRecord]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut running = 0_u64;
    for d in days {
        running += d.total_ns;
        if d.kind != DayType::Closed {
            out.push(running);
            running = 0;
        }
    }
    out
}

fn median(values: &[u64]) -> Option<u64> {
    let mut v = values.to_vec();
    v.sort_unstable();
    v.get(v.len() / 2).copied()
}

fn greatest(values: &[u64]) -> Option<u64> {
    values.iter().copied().reduce(|a, b| if b > a { b } else { a })
}

/// The stores and books the month runs over.
struct Load {
    pool: Pool,
    population: Agents,
    books: Settlement<SystemBacking>,
    stores: Vec<Vec<u64>>,
    names: Vec<String>,
}

impl Load {
    fn store(&self, name: Option<&String>) -> Option<usize> {
        name.and_then(|n| self.names.iter().position(|m| m == n))
    }

    /// A random agent.
    fn agent(&self, d: &mut Draws) -> Option<Slot> {
        let slots = &self.population.slots;
        let at = usize::try_from(below_u64(d, u64::try_from(slots.len()).unwrap_or(0))).ok()?;
        slots.get(at).copied()
    }

    /// One kind of work on one day, at its count for the day's type.
    fn work(&mut self, w: &Work, count: u64, (day, date, kind): (u32, Date, DayType)) {
        if count == 0 {
            return;
        }
        match w.kernel {
            Kernel::Hazard => {
                let population = &self.population;
                let n = piece_count(&self.pool);
                let drawn = self.pool.map(n, |p| {
                    let (first, units) = piece(count, n, p);
                    let mut d = draws(4, first, day);
                    let len = u64::try_from(population.slots.len()).unwrap_or(0);
                    let mut hits = 0_u64;
                    for _ in 0..units {
                        let at = usize::try_from(below_u64(&mut d, len)).unwrap_or(0);
                        let Some(slot) = population.slots.get(at) else { continue };
                        let booked = hazard(population, *slot, (Day::new(day), date), &mut d);
                        hits += u64::from(matches!(booked, phx_pop::hazard::Booking::Hit(_)));
                    }
                    hits
                });
                black_box(drawn);
            }
            Kernel::Outcome => {
                let mut d = draws(5, 0, day);
                for _ in 0..count {
                    let Some(slot) = self.agent(&mut d) else { continue };
                    outcome(&mut self.population, slot, date, &mut d);
                }
            }
            Kernel::Redraw => {
                if let Some(at) = self.store(w.store.as_ref())
                    && let Some(store) = self.stores.get_mut(at)
                {
                    update(&self.pool, store, count, day);
                }
            }
            Kernel::Agenda => {
                if let Some(store) = self.store(w.store.as_ref()).and_then(|at| self.stores.get(at)) {
                    black_box(sweep(&[store], AUDIT_SLICES, u64::from(day)));
                }
            }
            Kernel::Gather => {
                black_box(gather(&self.pool, &self.stores, count, w.reads.unwrap_or(1), day));
            }
            Kernel::Settle => {
                if kind == DayType::Closed {
                    return;
                }
                let Some(today) = self.books.calendar.day(date) else { return };
                let due = self.books.books.ledger.mark_due(today, &self.books.calendar);
                let draws_of = |l: phx_id::LineId| draws(7, u64::from(l.get()), day);
                let closed = phx_ledger::pending::Closed::default();
                let cal = &self.books.calendar;
                black_box(self.books.books.settle_day(&due, today, cal, &closed, &draws_of, &mut Unaudited).settled);
                // The day's book is the close's to take, as the world's is, or it grows over the month.
                black_box(self.books.books.close().dues.len());
            }
            Kernel::Audit => {
                let all: Vec<&Vec<u64>> = self.stores.iter().collect();
                black_box(sweep(&all, AUDIT_SLICES, u64::from(day)));
                let step = usize::try_from(AUDIT_SLICES).unwrap_or(1);
                let skip = usize::try_from(u64::from(day) % AUDIT_SLICES).unwrap_or(0);
                let t = &self.population.table;
                let slots = &self.population.slots;
                black_box(
                    slots.iter().skip(skip).step_by(step).fold(0, |a, s| fold(a, u64::from(t.multiplicity(*s).get()))),
                );
            }
        }
    }
}

/// The dates of the month's days from the first, and the holidays among them: the closed days that are not weekends.
fn month_dates(days: &[DayType]) -> Result<(Vec<Date>, Vec<Date>, Date), String> {
    let Some(first) = Date::new(FIRST.0, FIRST.1, FIRST.2) else { return Err("no first date".to_owned()) };
    let epoch = first;
    let dates: Vec<Date> = (0..days.len()).map(|i| Day::new(u32::try_from(i).unwrap_or(0)).date(epoch)).collect();
    let weekend = [phx_id::Weekday::Saturday, phx_id::Weekday::Sunday];
    let holidays = dates
        .iter()
        .zip(days)
        .filter(|(date, kind)| **kind == DayType::Closed && !weekend.contains(&date.weekday()))
        .map(|(date, _)| *date)
        .collect();
    let heavy = dates.iter().zip(days).find(|(_, k)| **k == DayType::Heavy).map_or(first, |(d, _)| *d);
    Ok((dates, holidays, heavy))
}

/// Builds the stores: the population, the books and the held words.
fn build(v: &Volumes, host: &dyn BenchHost, holidays: &[Date], (first, heavy): (Date, Date)) -> Result<Load, String> {
    let Some(week_before) = Date::new(WEEK_BEFORE.0, WEEK_BEFORE.1, WEEK_BEFORE.2) else {
        return Err("no Monday before the first".to_owned());
    };
    let pool = Pool::new(&PoolSpec::detect()).map_err(|e| e.0)?;
    show(host, "building", "the household agents, their persons and attachments".to_owned(), String::new(), "");
    let pv = &v.population;
    let stream = stream_key(Seed::new(3), "LOAD.bench");
    let population = agents(pv.agents, pv.persons_per_agent, pv.attachments_per_agent, pv.multiplicity, stream);
    show(host, "building", "the books' holders and their dated rows".to_owned(), String::new(), "");
    let sv = &v.settlement;
    let size = SettlementSize {
        holders: sv.holders,
        daily_rows: sv.daily_rows,
        monthly_rows: sv.monthly_rows,
        banks: sv.banks,
        holders_per_line: sv.holders_per_line,
    };
    let books = settlement::<SystemBacking>(size, calendar(holidays)?, (week_before, first, heavy))?;
    show(host, "building", "the held stores".to_owned(), String::new(), "");
    let stores: Vec<Vec<u64>> = v.stores.iter().zip(0_u64..).map(|(s, i)| held(&pool, s.bytes, i)).collect();
    let names = v.stores.iter().map(|s| s.name.clone()).collect();
    Ok(Load { pool, population, books, stores, names })
}

fn verdict(ok: bool) -> &'static str {
    if ok { "met" } else { "missed" }
}

/// The full-load bench over the volumes at `volumes_path`, saves written to and removed from `save_dir`, its report
/// written to `report_path` and returned.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, a save that cannot be written, or a report that cannot.
pub fn run(host: &dyn BenchHost, volumes_path: &str, save_dir: &str, report_path: &str) -> Result<String, String> {
    let text = measure(host, volumes_path, save_dir)?.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show(host, "report", report_path.to_owned(), String::new(), "");
    Ok(text)
}

/// The full-load bench's month, each day and save shown as it completes: the load section of the device report.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, or a save that cannot be written.
pub fn measure(host: &dyn BenchHost, volumes_path: &str, save_dir: &str) -> Result<Json, String> {
    let clock = Mono(Instant::now());
    let text = std::fs::read_to_string(volumes_path).map_err(|e| format!("{volumes_path}: {e}"))?;
    let v: Volumes = toml::from_str(&text).map_err(|e| format!("{volumes_path}: {e}"))?;
    let (dates, holidays, heavy) = month_dates(&v.month.days)?;
    let first = dates.first().copied().ok_or("a month of no days")?;
    let started = clock.now_ns();
    let mut load = build(&v, host, &holidays, (first, heavy))?;
    let built_ms = (clock.now_ns() - started) / NS_PER_MS;
    let built_peak = proc_kib("status", "VmHWM:");
    show(host, "built", format!("{built_ms} ms, peak {} MiB", built_peak.unwrap_or(0) / MIB), String::new(), "");
    let mut records: Vec<DayRecord> = Vec::new();
    let mut saves: Vec<(usize, u64, u64)> = Vec::new();
    for (i, (kind, date)) in v.month.days.iter().zip(&dates).enumerate() {
        let day = u32::try_from(i).unwrap_or(0);
        let day_start = clock.now_ns();
        let mut walls = Vec::with_capacity(v.works.len());
        for w in &v.works {
            let t0 = clock.now_ns();
            load.work(w, w.counts.get(kind.index()).copied().unwrap_or(0), (day, *date, *kind));
            walls.push((w.name.clone(), clock.now_ns() - t0));
        }
        let total_ns = clock.now_ns() - day_start;
        let peak = proc_kib("status", "VmHWM:").unwrap_or(0) / MIB;
        let name = format!("day {} ({})", i + 1, kind.name());
        show(host, &name, format!("{} ms, peak {peak} MiB", total_ns / NS_PER_MS), String::new(), "");
        records.push(DayRecord { index: i, kind: *kind, walls, total_ns });
        if v.month.saves.contains(&i) {
            let t0 = clock.now_ns();
            let path = std::path::Path::new(save_dir).join(format!("load-save-{i}.zst"));
            let bytes = save(&path, &load.stores)?;
            let ms = (clock.now_ns() - t0) / NS_PER_MS;
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let name = format!("save after day {}", i + 1);
            show(
                host,
                &name,
                format!("{ms} ms, {} MiB", bytes / MIB),
                format!("≤ {SAVE_MS} ms"),
                verdict(ms <= SAVE_MS),
            );
            saves.push((i + 1, ms, bytes));
        }
    }
    let turn_ms: Vec<u64> = turns(&records).iter().map(|ns| ns / NS_PER_MS).collect();
    let (middle, worst) = (median(&turn_ms), greatest(&turn_ms));
    let (peak, pss) = (proc_kib("status", "VmHWM:"), proc_kib("smaps_rollup", "Pss:"));
    let two_saves: u64 = saves.iter().rev().take(2).map(|(_, _, b)| b).sum();
    let show_ms = |m: Option<u64>| m.map_or("none".to_owned(), |m| format!("{m} ms"));
    show(
        host,
        "median turn",
        show_ms(middle),
        format!("≤ {MEDIAN_MS} ms"),
        verdict(middle.is_some_and(|m| m <= MEDIAN_MS)),
    );
    show(host, "worst turn", show_ms(worst), format!("≤ {WORST_MS} ms"), verdict(worst.is_some_and(|m| m <= WORST_MS)));
    let peak_text = peak.map_or("unread".to_owned(), |p| format!("{} MiB", p / MIB));
    show(
        host,
        "peak resident",
        peak_text,
        format!("≤ {MEMORY_BYTES} bytes"),
        verdict(peak.is_some_and(|p| p <= MEMORY_BYTES)),
    );
    show(
        host,
        "two saves",
        format!("{} MiB", two_saves / MIB),
        format!("≤ {} MiB", TWO_SAVES_BYTES / MIB),
        verdict(two_saves <= TWO_SAVES_BYTES),
    );
    let report = report_json(&v, &records, (built_ms, built_peak), (&turn_ms, middle, worst), (peak, pss), &saves);
    drop(load);
    Ok(report)
}

/// The load section of the device report.
fn report_json(
    v: &Volumes,
    records: &[DayRecord],
    (built_ms, built_peak): (u64, Option<u64>),
    (turn_ms, middle, worst): (&[u64], Option<u64>, Option<u64>),
    (peak, pss): (Option<u64>, Option<u64>),
    saves: &[(usize, u64, u64)],
) -> Json {
    let count = |n: usize| Json::UInt(u64::try_from(n).unwrap_or(u64::MAX));
    let days = records.iter().map(|r| {
        let works = r
            .walls
            .iter()
            .map(|(n, ns)| Json::obj([("work", Json::str(n.clone())), ("ms", Json::UInt(ns / NS_PER_MS))]));
        Json::obj([
            ("day", count(r.index + 1)),
            ("type", Json::str(r.kind.name())),
            ("ms", Json::UInt(r.total_ns / NS_PER_MS)),
            ("works", Json::Array(works.collect())),
        ])
    });
    let saves = saves
        .iter()
        .map(|(d, ms, b)| Json::obj([("after_day", count(*d)), ("ms", Json::UInt(*ms)), ("bytes", Json::UInt(*b))]));
    let stores = v.stores.iter().map(|s| {
        Json::obj([
            ("name", Json::str(s.name.clone())),
            ("line", Json::str(s.line.clone())),
            ("bytes", Json::UInt(s.bytes)),
        ])
    });
    let works =
        v.works.iter().map(|w| Json::obj([("name", Json::str(w.name.clone())), ("line", Json::str(w.line.clone()))]));
    Json::obj([
        ("load_version", Json::UInt(LOAD_VERSION)),
        ("commit", Json::str(option_env!("PHX_COMMIT").unwrap_or("unknown"))),
        ("population_agents", Json::UInt(u64::from(v.population.agents))),
        ("population_multiplicity", Json::UInt(u64::from(v.population.multiplicity))),
        ("population_persons_per_agent", Json::UInt(u64::from(v.population.persons_per_agent))),
        ("population_attachments_per_agent", Json::UInt(u64::from(v.population.attachments_per_agent))),
        ("built_ms", Json::UInt(built_ms)),
        ("built_vm_hwm_bytes", Json::opt(built_peak, Json::UInt)),
        ("days", Json::Array(days.collect())),
        ("turn_ms", Json::Array(turn_ms.iter().map(|m| Json::UInt(*m)).collect())),
        ("median_turn_ms", Json::opt(middle, Json::UInt)),
        ("worst_turn_ms", Json::opt(worst, Json::UInt)),
        ("vm_hwm_bytes", Json::opt(peak, Json::UInt)),
        ("pss_bytes", Json::opt(pss, Json::UInt)),
        ("saves", Json::Array(saves.collect())),
        (
            "criteria",
            Json::obj([
                ("median_turn_ms", Json::UInt(MEDIAN_MS)),
                ("worst_turn_ms", Json::UInt(WORST_MS)),
                ("memory_bytes", Json::UInt(MEMORY_BYTES)),
                ("save_ms", Json::UInt(SAVE_MS)),
                ("two_saves_bytes", Json::UInt(TWO_SAVES_BYTES)),
            ]),
        ),
        ("stores", Json::Array(stores.collect())),
        ("works", Json::Array(works.collect())),
    ])
}

/// A full save of the held stores through the store's compressed writer; returns the bytes written.
fn save(path: &std::path::Path, stores: &[Vec<u64>]) -> Result<u64, String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut sink = std::io::BufWriter::new(file);
    let mut w = phx_store::save::Writer::new(&mut sink).map_err(|e| e.to_string())?;
    for s in stores {
        for chunk in s.chunks(SAVE_WORDS) {
            let bytes: Vec<u8> = chunk.iter().flat_map(|x| x.to_le_bytes()).collect();
            w.bytes(&bytes);
        }
    }
    let (_, written) = w.finish().map_err(|e| e.to_string())?;
    Ok(written)
}

/// The app's entry: runs the full-load bench on the calling thread, which must not be the interface's.
#[uniffi::export]
#[expect(clippy::needless_pass_by_value, reason = "the foreign interface hands over owned values")]
pub fn run_load(
    host: std::sync::Arc<dyn BenchHost>,
    volumes_path: String,
    save_dir: String,
    report_path: String,
) -> String {
    crate::stopped::watch(&report_path);
    match run(host.as_ref(), &volumes_path, &save_dir, &report_path) {
        Ok(report) => report,
        Err(error) => {
            show(host.as_ref(), "the full-load bench stopped", error.clone(), String::new(), "");
            error
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Sender, channel};

    use super::{BenchHost, BenchLine, run};

    struct Host(Sender<BenchLine>);

    impl BenchHost for Host {
        fn thermal_status(&self) -> i32 {
            0
        }

        fn on_line(&self, line: BenchLine) {
            self.0.send(line).unwrap();
        }
    }

    /// The finished world's volumes shrunk to a few thousand of everything, the month and the kinds of work kept.
    fn small() -> String {
        let full = include_str!("../../../../perf/load/volumes.toml");
        let mut table: toml::Table = toml::from_str(full).unwrap();
        let pop = table.get_mut("population").unwrap().as_table_mut().unwrap();
        pop.insert("agents".to_owned(), 2_000.into());
        let set = table.get_mut("settlement").unwrap().as_table_mut().unwrap();
        set.insert("holders".to_owned(), 3_000.into());
        set.insert("banks".to_owned(), 3.into());
        for store in table.get_mut("store").unwrap().as_array_mut().unwrap() {
            store.as_table_mut().unwrap().insert("bytes".to_owned(), 1_048_576.into());
        }
        for work in table.get_mut("work").unwrap().as_array_mut().unwrap() {
            let counts = work.as_table_mut().unwrap().get_mut("counts").unwrap().as_array_mut().unwrap();
            for c in counts.iter_mut() {
                *c = (c.as_integer().unwrap() / 1_000 + 1).into();
            }
        }
        toml::to_string(&table).unwrap()
    }

    #[test]
    #[ignore = "the full-load bench end to end at a small size: a minute of work, run by hand"]
    fn load_runs_end_to_end() {
        let dir = std::env::temp_dir();
        let volumes = dir.join("phx-load-volumes.toml");
        std::fs::write(&volumes, small()).unwrap();
        let (tx, rx) = channel();
        let report = dir.join("phx-load-report.json");
        let text = run(&Host(tx), volumes.to_str().unwrap(), dir.to_str().unwrap(), report.to_str().unwrap()).unwrap();
        let lines: Vec<BenchLine> = rx.try_iter().collect();
        let summary: Vec<String> =
            lines.iter().map(|l| format!("{} | {} | {} {}", l.name, l.value, l.target, l.verdict)).collect();
        std::fs::write(dir.join("phx-load-lines.txt"), summary.join("\n")).unwrap();
        assert!(text.contains("\"median_turn_ms\"") && lines.iter().any(|l| l.name == "worst turn"));
    }
}
