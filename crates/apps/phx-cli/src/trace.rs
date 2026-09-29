//! The run's trace printed as it happens: each span as it begins and ends with its own time, each note with its
//! counts, every line with the time since the run began and the memory held then, indented by nesting.

use std::time::Instant;

use phx_exec::Clock;
use phx_exec::trace::{Mark, Tracer};

#[derive(Debug)]
struct Printer {
    began: Instant,
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
            Mark::Ended(name, Some(ns)) => println!("{head}< {name} {:.1} ms", ms(ns)),
            Mark::Ended(name, None) => println!("{head}< {name} ? ms"),
            Mark::Note(name, counts) => {
                let fields: Vec<String> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
                println!("{head}= {name} {}", fields.join(" "));
            }
        }
    }
}

/// Nanoseconds read as milliseconds.
fn ms(ns: u64) -> f64 {
    std::time::Duration::from_nanos(ns).as_secs_f64() * 1e3
}

/// Every mark printed from now on.
pub fn start() {
    phx_exec::trace::set(Box::leak(Box::new(Printer { began: Instant::now() })));
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
