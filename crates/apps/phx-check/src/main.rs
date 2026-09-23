mod clauses;
mod comments;
mod coverage;
mod docs;
mod rules;
mod workspace;

use std::fs;
use std::process::{Command as Process, ExitCode};

use clap::{Parser, Subcommand};

use crate::rules::Breach;
use crate::workspace::{ARCHITECTURE, Workspace};

#[derive(Debug, Parser)]
#[command(name = "phx-check", about = "The workspace's law, layering and document checks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Every check below.
    All,
    /// The dependency rules.
    Layering,
    /// Every rule of the table.
    Rules,
    /// The documents' shape and the coverage table.
    Docs,
    /// The clause map.
    Clauses,
    /// Derives the coverage table; `--write` rewrites it in place.
    Coverage {
        #[arg(long)]
        write: bool,
    },
    /// Format, lint and test as CI does, then every check.
    CheckAll,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.command, Command::CheckAll) && !cargo_steps() {
        return ExitCode::FAILURE;
    }
    let ws = match workspace::load() {
        Ok(ws) => ws,
        Err(error) => {
            eprintln!("phx-check: {error}");
            return ExitCode::FAILURE;
        }
    };
    let breaches = match cli.command {
        Command::All | Command::CheckAll => all(&ws),
        Command::Layering => rules::run(&ws, Some(rules::LAYERING)),
        Command::Rules => rules::run(&ws, None),
        Command::Docs => {
            let mut b = rules::run(&ws, Some(&["PC-09"]));
            b.extend(coverage::check(&ws));
            b
        }
        Command::Clauses => clauses::run(&ws),
        Command::Coverage { write } => return coverage_command(&ws, write),
    };
    report(&breaches)
}

fn all(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = rules::run(ws, None);
    breaches.extend(coverage::check(ws));
    breaches.extend(clauses::run(ws));
    breaches
}

fn report(breaches: &[Breach]) -> ExitCode {
    for b in breaches {
        println!("{b}");
    }
    if breaches.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("phx-check: {} breaches", breaches.len());
        ExitCode::FAILURE
    }
}

fn coverage_command(ws: &Workspace, write: bool) -> ExitCode {
    let derived = match coverage::derive_from(ws) {
        Ok(d) => d,
        Err(error) => {
            eprintln!("phx-check: {error}");
            return ExitCode::FAILURE;
        }
    };
    if !write {
        return report(&coverage::check(ws));
    }
    match fs::write(ws.root.join(ARCHITECTURE), derived.text) {
        Ok(()) => {
            println!("{ARCHITECTURE}: {} rows rewritten", derived.changed.len());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("phx-check: {ARCHITECTURE}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// CI's per-push cargo jobs, in its order, stopping at the first that fails.
fn cargo_steps() -> bool {
    let steps: [&[&str]; 3] = [
        &["fmt", "--all", "--check"],
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        &["test", "--workspace"],
    ];
    steps.iter().all(|args| match Process::new("cargo").args(*args).status() {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("phx-check: cargo {}: {error}", args.join(" "));
            false
        }
    })
}
