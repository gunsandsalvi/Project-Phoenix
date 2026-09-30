//! The run's trace printed as it happens: each span as it begins and ends with its own time and what else it spent,
//! each note with its counts, every line with the time since the run began and the memory held then, indented by
//! nesting. Every span's totals are gathered by name for the run's report.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::time::Instant;

use phx_exec::Clock;
use phx_exec::trace::{Mark, Tracer};

#[derive(Debug)]
pub struct Printer {
    began: Instant,
    spans: Mutex<Spans>,
}

impl Clock for Printer {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.began.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

impl Tracer for Printer {
    fn mark(&self, mark: Mark<'_>, depth: usize) {
        let since = self.began.elapsed().as_secs_f64();
        let head = format!("{since:>10.3}s {:>7} MiB {}", resident_mib(), "  ".repeat(depth));
        match mark {
            Mark::Begun(name) => println!("{head}> {name}"),
            Mark::Ended(name, ns, counts) => {
                let wall = ns.map_or_else(|| "?".to_owned(), |ns| format!("{:.1}", ms(ns)));
                println!("{head}< {name} {wall} ms{}", spent(counts));
                if let Ok(mut spans) = self.spans.lock() {
                    spans.add(name, ns, counts);
                }
            }
            Mark::Note(name, counts) => println!("{head}= {name}{}", fields(counts)),
        }
    }
}

impl Printer {
    /// Spans of many items that kept fewer cores busy than such a pass should.
    pub fn below_busy(&self) -> Option<u64> {
        self.spans.lock().ok().map(|s| s.below_busy)
    }

    /// Every span's totals so far, by name.
    pub fn spans(&self) -> serde_json::Value {
        self.spans.lock().map_or(serde_json::Value::Null, |s| s.report())
    }
}

/// A span's totals over every time it ran: a sum is missing once any of its parts was, never read as a smaller one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpanTotals {
    pub count: u64,
    pub wall_ns: Option<u64>,
    pub items: Option<u64>,
    pub cpu_ns: Option<u64>,
    pub faults: Option<u64>,
    pub chunks: Option<u64>,
    pub barriers: Option<u64>,
    pub spun: Option<u64>,
    pub allocs: Option<u64>,
    pub alloc_bytes: Option<u64>,
}

impl SpanTotals {
    fn add(&mut self, ns: Option<u64>, counts: &[(&'static str, i64)]) {
        let first = self.count == 0;
        let read = |name: &str| counts.iter().find(|(k, _)| *k == name).and_then(|(_, v)| u64::try_from(*v).ok());
        let sum = |total: Option<u64>, part: Option<u64>| if first { part } else { total?.checked_add(part?) };
        self.wall_ns = sum(self.wall_ns, ns);
        self.items = sum(self.items, read("items"));
        self.cpu_ns = sum(self.cpu_ns, read("cpu_ns"));
        self.faults = sum(self.faults, read("faults"));
        self.chunks = sum(self.chunks, read("chunks"));
        self.barriers = sum(self.barriers, read("barriers"));
        self.spun = sum(self.spun, read("spun"));
        self.allocs = sum(self.allocs, read("allocs"));
        self.alloc_bytes = sum(self.alloc_bytes, read("alloc_bytes"));
        self.count += 1;
    }

    /// CPU nanoseconds an item; none for a span with no items, never a rate of zero.
    #[must_use]
    pub fn ns_per_item(&self) -> Option<u64> {
        self.cpu_ns?.checked_div(self.items.filter(|n| *n > 0)?)
    }
}

/// Every span's totals by name, and how many spans of many items ran on too few cores.
#[derive(Debug, Default)]
pub struct Spans {
    by_name: BTreeMap<&'static str, SpanTotals>,
    below_busy: u64,
}

/// A span this many items long should keep this many cores busy: a pass that large is the pool's to share.
const MANY_ITEMS: i64 = 65_536;
const BUSY_CORES: f64 = 2.5;

impl Spans {
    pub fn add(&mut self, name: &'static str, ns: Option<u64>, counts: &[(&'static str, i64)]) {
        self.by_name.entry(name).or_default().add(ns, counts);
        let read = |key: &str| counts.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        if let (Some(items), Some(cpu), Some(wall)) = (read("items"), read("cpu_ns"), ns)
            && items >= MANY_ITEMS
            && let (Ok(cpu), Ok(wall)) = (cpu.to_string().parse::<f64>(), wall.to_string().parse::<f64>())
            && cpu < BUSY_CORES * wall
        {
            self.below_busy += 1;
        }
    }

    /// The report's `spans`: each name's count, wall and CPU in ms, items and ns an item, faults, allocations,
    /// chunks, barriers and spin.
    #[must_use]
    pub fn report(&self) -> serde_json::Value {
        let ms = |ns: Option<u64>| ns.map(|n| n / NS_PER_MS);
        self.by_name
            .iter()
            .map(|(name, t)| {
                let row = serde_json::json!({
                    "count": t.count,
                    "wall_ms": ms(t.wall_ns),
                    "cpu_ms": ms(t.cpu_ns),
                    "items": t.items,
                    "ns_per_item": t.ns_per_item(),
                    "faults": t.faults,
                    "allocs": t.allocs,
                    "alloc_bytes": t.alloc_bytes,
                    "chunks": t.chunks,
                    "barriers": t.barriers,
                    "spun": t.spun,
                });
                ((*name).to_owned(), row)
            })
            .collect::<serde_json::Map<_, _>>()
            .into()
    }
}

const NS_PER_MS: u64 = 1_000_000;

/// What a span spent as its line prints it: its items, CPU in ms, faults and allocations, each where it was read.
fn spent(counts: &[(&'static str, i64)]) -> String {
    let read = |name: &str| counts.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    let mut out = String::new();
    if let Some(items) = read("items") {
        let _ = write!(out, " items={items}");
    }
    if let Some(cpu) = read("cpu_ns").and_then(|c| u64::try_from(c).ok()) {
        let _ = write!(out, " cpu_ms={:.1}", ms(cpu));
    }
    for (label, name) in [("faults", "faults"), ("allocs", "allocs")] {
        if let Some(v) = read(name) {
            let _ = write!(out, " {label}={v}");
        }
    }
    out
}

/// Named counts as the trace prints them, each after a space.
fn fields(counts: &[(&'static str, i64)]) -> String {
    counts.iter().fold(String::new(), |mut out, (k, v)| {
        let _ = write!(out, " {k}={v}");
        out
    })
}

/// Nanoseconds read as milliseconds.
fn ms(ns: u64) -> f64 {
    std::time::Duration::from_nanos(ns).as_secs_f64() * 1e3
}

/// Every mark printed from now on, and every span gathered by the printer returned.
pub fn start() -> &'static Printer {
    let printer: &'static Printer =
        Box::leak(Box::new(Printer { began: Instant::now(), spans: Mutex::new(Spans::default()) }));
    phx_exec::trace::set(printer);
    printer
}

/// The memory the process holds now, from the kernel's own account; none where it gives none.
fn resident_mib() -> String {
    let read = || -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kib >> 10)
    };
    read().map_or_else(|| "?".to_owned(), |m| m.to_string())
}

#[cfg(test)]
mod tests {
    use super::{SpanTotals, Spans};

    #[test]
    fn span_without_items_has_no_rate() {
        let mut t = SpanTotals::default();
        t.add(Some(5_000_000), &[("cpu_ns", 9_000_000)]);
        assert_eq!((t.items, t.ns_per_item()), (None, None), "no items, no rate, never 0 ns an item");
        t.add(Some(1_000_000), &[("items", 3), ("cpu_ns", 1_000)]);
        assert_eq!(t.items, None, "a sum stays missing once a part of it was");
    }

    #[test]
    fn report_carries_span_counters() {
        let mut spans = Spans::default();
        let counts = [("items", 1_000), ("cpu_ns", 4_000_000), ("faults", 7), ("allocs", 2), ("chunks", 8)];
        spans.add("goods.meet", Some(2_000_000), &counts);
        spans.add("goods.meet", Some(3_000_000), &counts);
        spans.add("audit", None, &[("cpu_ns", 1)]);
        let report = spans.report();
        let meet = &report["goods.meet"];
        assert_eq!(
            (meet["count"].as_u64(), meet["wall_ms"].as_u64(), meet["cpu_ms"].as_u64()),
            (Some(2), Some(5), Some(8))
        );
        assert_eq!((meet["items"].as_u64(), meet["ns_per_item"].as_u64()), (Some(2_000), Some(4_000)));
        assert_eq!(
            (meet["faults"].as_u64(), meet["allocs"].as_u64(), meet["chunks"].as_u64()),
            (Some(14), Some(4), Some(16))
        );
        assert!(meet["alloc_bytes"].is_null(), "a count never read is missing");
        assert!(report["audit"]["wall_ms"].is_null() && report["audit"]["ns_per_item"].is_null());
    }
}
