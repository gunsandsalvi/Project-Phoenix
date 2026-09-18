//! `check:opening` (22b.2): **draw a world, live its past, take the census, accept it or throw it
//! away** — and write every world thrown away to `docs/rejections.log` with the reason.
//!
//! It prints the census of the world it opened (or of the last one it drew), because a number nobody
//! looks at is not a check. **It exits non-zero only when no world was accepted**, and the log is
//! then the whole point: a property that failed on every attempt is a missing mechanism with a name,
//! and the name is in the file.
//!
//! Law 11: this is a MEASUREMENT of a half-built world. What it names goes in `docs/IMPLEMENTATION.md`
//! under the item that should fix it. It is not chased here.

use phoenix_kernel::calendar::Day;
use phoenix_kernel::chronicle::Census;
use phoenix_kernel::opening::{open, Shape};
use phoenix_kernel::snapshot::{from_text, regenerates, to_text, Regenerated, Snapshot};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

/// SHAPE (Law 2): how many of each a world this size has, until the mechanisms that decide them are
/// built. Small enough to draw in a moment, large enough that every property has something to say.
const BANKS: usize = 4;
const BILLS: usize = 8;
const FIRMS: usize = 12;
const CELLS: usize = 24;
const WEEKS: i64 = 52;
/// One calendar: a 7-day period (Calendar A1).
const WEEK: u32 = 7;
/// The run's budget — how many worlds this invocation draws before it reports. Not a bound on any
/// number in the model (Law 6): it changes nothing about the worlds it draws.
const ATTEMPTS: usize = 8;
const FIRST_SEED: u64 = 1;

fn main() {
    let shape = Shape {
        banks: BANKS,
        bills: BILLS,
        firms: FIRMS,
        cells: CELLS,
        weeks: WEEKS,
        opens_on: Day(0),
        days_per_period: WEEK,
    };
    let run = open(FIRST_SEED, ATTEMPTS, shape);

    let mut log = String::new();
    let _ = writeln!(log, "# Worlds thrown away, and why (22b.2)");
    let _ = writeln!(log);
    let _ = writeln!(
        log,
        "Written by `check:opening`. A property that fails on EVERY seed value is not a run of bad luck:"
    );
    let _ = writeln!(log, "it is a missing mechanism, and this file names it.");
    let _ = writeln!(log);
    for r in &run.rejections {
        let _ = writeln!(log, "- seed {} — {:?}: {}", r.seed_value, r.failed, r.why);
    }
    if run.rejections.is_empty() {
        let _ = writeln!(log, "- none: the first world drawn was accepted.");
    }
    let _ = writeln!(log);

    if run.accepted() {
        let _ = writeln!(log, "Accepted after {} rejection(s).", run.rejections.len());
        println!("check:opening — a world opened after {} rejection(s).", run.rejections.len());
    } else {
        let _ = writeln!(log, "No world accepted in {ATTEMPTS} attempts.");
        println!("check:opening — NO WORLD ACCEPTED in {ATTEMPTS} attempts.");
        for r in &run.rejections {
            println!("  seed {} — {:?}: {}", r.seed_value, r.failed, r.why);
        }
    }
    // The census of whatever it ended with, accepted or not: a run that rejected everything still
    // has a world to show, and what it shows is where the next mechanism is missing.
    say(&run.outcome.census);
    println!("  {} moments of the past settled.", run.outcome.replayed.settled);
    for v in &run.outcome.violations {
        let line = format!("{} — {} ({} {}) — {}", v.spec, v.owner, v.size, v.unit, v.message);
        println!("  audit: {line}");
        let _ = writeln!(log, "- audit: {line}");
    }

    let at = log_path();
    if let Err(e) = fs::write(&at, log) {
        // The log is the deliverable, so a failure to write it is a failure of the check.
        eprintln!("check:opening could not write {}: {e}", at.display());
        std::process::exit(1);
    }
    println!("  reasons written to {}", at.display());

    if !run.accepted() {
        std::process::exit(1);
    }

    // 22b.7: **the snapshot, and the regeneration check that keeps it an optimisation.** The
    // committed snapshot is what an artifact opens from; if drawing from its own seed value no longer
    // reproduces it, it is a SECOND STATEMENT of the opening (Law 4) and the check says so rather than
    // quietly rewriting it. `--write` is how somebody who changed the draw on purpose replaces it.
    let fresh = Snapshot::of(&run.outcome);
    let at = snapshot_path();
    if std::env::args().any(|a| a == "--write") {
        if let Err(e) = fs::write(&at, to_text(&fresh)) {
            eprintln!("check:opening could not write {}: {e}", at.display());
            std::process::exit(1);
        }
        println!("  snapshot written to {}", at.display());
        return;
    }
    match fs::read_to_string(&at) {
        Err(_) => {
            println!("  NO SNAPSHOT at {} — run `npm run check:opening:write`.", at.display());
            std::process::exit(1);
        }
        Ok(text) => {
            let held = from_text(&text);
            match regenerates(&held) {
                Regenerated::Same => println!("  snapshot regenerates from seed {}.", held.seed_value()),
                Regenerated::Differs(why) => {
                    println!("  SNAPSHOT DOES NOT REGENERATE: {why}");
                    println!("  The draw changed. `npm run check:opening:write` replaces it.");
                    std::process::exit(1);
                }
            }
        }
    }
}

/// `docs/opening.snapshot` — the world an artifact opens from, beside the log that says what was
/// thrown away to get it.
fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/opening.snapshot")
}

fn say(c: &Census) {
    println!("  markets traded         {} of {}", c.markets_that_traded, c.markets);
    println!("  parties with an outlook {} of {}", c.parties_with_an_outlook, c.parties_alive);
    println!("  firms through the cycle {} of {}", c.firms_that_did_all_three, c.firms);
    println!("  banks lent and repaid   {} of {}", c.banks_that_lent_and_were_repaid, c.banks);
    println!("  kinds held by choice    {} of {}", c.kinds_held_by_choice, c.kinds_declared);
    println!("  maturity days {} · issue days {} · ages {}", c.distinct_maturity_days, c.distinct_issue_days, c.distinct_ages);
    println!("  moments refused         {}", c.refused);
    println!("  audit built {} · violations {}", c.audit_built, c.audit_violations);
    println!("  holdings younger than the world {}", c.rows_younger_than_the_world);
}

/// `docs/rejections.log`, relative to this crate rather than to wherever the check was invoked.
fn log_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/rejections.log")
}
