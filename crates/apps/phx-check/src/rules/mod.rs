use std::fmt;

use crate::workspace::Workspace;

pub mod api_snapshot;
mod audit_reads;
mod batches;
mod borders;
mod clippy_files;
mod comment_refs;
mod day_arithmetic;
mod dependencies;
mod documents;
mod draws;
mod expect_count;
mod expect_reason;
mod hand_pod;
mod id_default;
mod interfaces;
mod layering;
mod ledger_writes;
mod literals;
mod live_checks;
mod money_moves;
mod opening_writes;
mod places;
mod prints;
mod random_crates;
mod rayon_libc;
mod register_reads;
mod statics;
mod substeps;
mod unsafe_code;

pub mod attrs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breach {
    pub rule: &'static str,
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl Breach {
    pub fn new(rule: &'static str, file: &str, line: usize, message: impl Into<String>) -> Breach {
        Breach { rule, file: file.to_owned(), line, message: message.into() }
    }
}

impl fmt::Display for Breach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match RULES.iter().find(|r| r.id == self.rule) {
            Some(r) => {
                write!(f, "{}:{}: {} ({}, since {}): {}", self.file, self.line, r.id, r.title, r.since, self.message)
            }
            None => write!(f, "{}:{}: {}: {}", self.file, self.line, self.rule, self.message),
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    pub since: &'static str,
    pub run: fn(&Workspace) -> Vec<Breach>,
}

/// The one table of rules; later steps add theirs here, numbered on.
pub const RULES: &[Rule] = &[
    Rule { id: "PC-01", title: "layering", since: "S0.01", run: layering::run },
    Rule { id: "PC-02", title: "direct external dependencies", since: "S0.01", run: dependencies::run },
    Rule { id: "PC-03", title: "allow(unsafe_code) only where declared", since: "S0.01", run: unsafe_code::run },
    Rule { id: "PC-04", title: "rayon, rayon-core and libc only where allowed", since: "S0.01", run: rayon_libc::run },
    Rule { id: "PC-05", title: "no static items in world crates", since: "S0.01", run: statics::run },
    Rule { id: "PC-06", title: "no numeric literals in mechanisms", since: "S0.01", run: literals::run },
    Rule { id: "PC-07", title: "no references in comments", since: "S0.01", run: comment_refs::run },
    Rule { id: "PC-08", title: "interface crates without behaviour", since: "S0.01", run: interfaces::run },
    Rule { id: "PC-09", title: "documents", since: "S0.01", run: documents::run },
    Rule {
        id: "PC-10",
        title: "allow and expect counts within their ratchets",
        since: "S0.01",
        run: expect_count::run,
    },
    Rule { id: "PC-11", title: "every expect has a reason", since: "S0.01", run: expect_reason::run },
    Rule {
        id: "PC-12",
        title: "storable only through the derive, and never a float",
        since: "S0.03",
        run: hand_pod::run,
    },
    Rule { id: "PC-14", title: "no default identity", since: "S0.05", run: id_default::run },
    Rule { id: "PC-13", title: "no random-number crates or other hashers", since: "S0.01", run: random_crates::run },
    Rule {
        id: "PC-15",
        title: "kernel and interface public APIs match their snapshots",
        since: "S0.06",
        run: api_snapshot::run,
    },
    Rule { id: "PC-16", title: "per-crate clippy files", since: "S0.01", run: clippy_files::run },
    Rule { id: "PC-17", title: "days placed only by the calendar", since: "S0.08", run: day_arithmetic::run },
    Rule { id: "PC-18", title: "numbers read only through the register", since: "S0.09", run: register_reads::run },
    Rule { id: "PC-19", title: "draws only from the run's streams", since: "S0.10", run: draws::run },
    Rule {
        id: "PC-20",
        title: "the live-check suite is complete and the inspector reads only",
        since: "S0.11",
        run: live_checks::run,
    },
    Rule { id: "PC-21", title: "handlers name only the table's sub-steps", since: "S0.11", run: substeps::run },
    Rule { id: "PC-22", title: "the audit reads the world only", since: "S0.12", run: audit_reads::run },
    Rule { id: "PC-23", title: "places belong to phx-geo", since: "S0.13", run: places::run },
    Rule {
        id: "PC-24",
        title: "holdings, rows, lines, liens and issued amounts are written by phx-ledger alone",
        since: "S0.14",
        run: ledger_writes::run,
    },
    Rule {
        id: "PC-25",
        title: "money moves only through the apply routine, which alone writes a balance",
        since: "S0.15",
        run: money_moves::run,
    },
    Rule {
        id: "PC-26",
        title: "the opening writes only through its contributions' opening writes",
        since: "S0.16",
        run: opening_writes::run,
    },
    Rule { id: "PC-27", title: "no materialised batch in stage 7's passes", since: "S0.17", run: batches::run },
    Rule {
        id: "PC-28",
        title: "prints made by the markets alone, and nothing converted into one",
        since: "S0.18",
        run: prints::run,
    },
    Rule { id: "PC-75", title: "a border closed in the markets' reach alone", since: "S0.18", run: borders::run },
];

/// The dependency rules, which `layering` runs alone.
pub const LAYERING: &[&str] = &["PC-01", "PC-02", "PC-03", "PC-04"];

pub fn run(ws: &Workspace, ids: Option<&[&str]>) -> Vec<Breach> {
    RULES.iter().filter(|r| ids.is_none_or(|ids| ids.contains(&r.id))).flat_map(|r| (r.run)(ws)).collect()
}

/// A breach for a source that does not parse, which every syntax rule reports rather than skips.
pub fn unparsed(rule: &'static str, path: &str, error: &str) -> Breach {
    Breach::new(rule, path, 1, format!("does not parse: {error}"))
}
