//! The run's report, in JSON for the kept record and as a summary for the terminal.

use std::fmt::Write;

use crate::compose::{Days, Line, Source};

/// What a `-F` run found.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    /// The bases filled and measured.
    pub bases: Vec<String>,
    /// Each operation measured: base, name, items and nanoseconds an item.
    pub ops: Vec<(String, String, u64, Option<u64>)>,
    /// Each base's rows and the MiB it holds once filled.
    pub sizes: Vec<(String, u64, u64)>,
    pub lines: Vec<Line>,
    pub days: Days,
    /// Every refusal: a capacity short of the design point, a ratchet missed.
    pub misses: Vec<String>,
}

impl Report {
    /// Whether every check held.
    #[must_use]
    pub fn clean(&self) -> bool {
        self.misses.is_empty()
    }

    /// The report for the terminal.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let declared = self.lines.iter().filter(|l| l.source == Source::Declared).count();
        let partly = self.lines.iter().filter(|l| l.source == Source::Partly).count();
        let _ = writeln!(
            out,
            "fin: {} bases measured; {} of {} lines declared, not measured; {partly} declared with today's kernels in \
             place of their unit costs",
            self.bases.len(),
            declared,
            self.lines.len()
        );
        let [b, nb, h, bc] = self.days.core_ms;
        let _ = writeln!(out, "days: B {b:.1}, NB {nb:.1}, H {h:.1}, B' {bc:.1} core-ms");
        let [first, second] = self.days.turns_ms;
        let _ = writeln!(out, "turns: 4 NB + B' {first:.0} ms, 3 NB + H {second:.0} ms");
        for (base, rows, mb) in &self.sizes {
            let _ = writeln!(out, "  {base}: {rows} rows filled, {mb} MiB");
        }
        for (base, op, items, ns) in &self.ops {
            let ns = ns.map_or_else(|| "no".to_owned(), |n| n.to_string());
            let _ = writeln!(out, "  {base}.{op}: {items} items, {ns} ns an item");
        }
        for m in &self.misses {
            let _ = writeln!(out, "  missed: {m}");
        }
        out
    }
}
