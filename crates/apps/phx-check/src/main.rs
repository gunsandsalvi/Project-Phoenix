mod clauses;
mod comments;
mod coverage;
mod docs;
mod exceptions;
mod git;
mod ratchets;
mod rules;
mod workspace;

use std::fs;
use std::process::{Command as Process, ExitCode};

use clap::{Parser, Subcommand};

use crate::rules::Breach;
use crate::workspace::{API_SNAPSHOT, ARCHITECTURE, VERSIONS, Workspace};

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
    /// Compares each kernel and interface crate's public API, read by the pinned cargo-public-api, with its committed
    /// snapshot; `--write` records the current API instead.
    PublicApi {
        #[arg(long)]
        write: bool,
    },
    /// Lists every site a rule finds as it would admit them; `--write` records them as the rule's exceptions file,
    /// which is done once, when the rule is first widened over the code as it stands.
    Exceptions {
        rule: String,
        #[arg(long)]
        write: bool,
    },
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
        Command::PublicApi { write } => return public_api_command(&ws, write),
        Command::Exceptions { rule, write } => return exceptions_command(&ws, &rule, write),
    };
    report(&breaches)
}

/// The rules whose sites an exceptions file can admit, with how each finds them.
const FINDERS: &[(&str, exceptions::Finder)] = &[(rules::hot_paths::RULE, rules::hot_paths::found)];

fn exceptions_command(ws: &Workspace, rule: &str, write: bool) -> ExitCode {
    let Some((id, find)) = FINDERS.iter().find(|(id, _)| *id == rule) else {
        eprintln!("phx-check: no exceptions file is kept for `{rule}`");
        return ExitCode::FAILURE;
    };
    let (found, breaches) = find(ws);
    if !breaches.is_empty() {
        return report(&breaches);
    }
    let text = exceptions::write(id, &found);
    if !write {
        print!("{text}");
        return ExitCode::SUCCESS;
    }
    let path = ws.root.join(exceptions::path(id));
    if let Some(dir) = path.parent()
        && let Err(e) = fs::create_dir_all(dir)
    {
        eprintln!("phx-check: {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    match fs::write(&path, text) {
        Ok(()) => {
            println!("{}: {} sites", path.display(), found.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("phx-check: {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn public_api(nightly: &str, krate: &str) -> Result<String, String> {
    let output = Process::new("cargo")
        .args([&format!("+{nightly}"), "public-api", "-p", krate, "-sss", "--color", "never"])
        .output()
        .map_err(|e| format!("cargo public-api: {e}"))?;
    if !output.status.success() {
        return Err(format!("cargo public-api -p {krate}: {}", String::from_utf8_lossy(&output.stderr)));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo public-api -p {krate}: {e}"))
}

fn public_api_command(ws: &Workspace, write: bool) -> ExitCode {
    let nightly = fs::read_to_string(ws.root.join(VERSIONS))
        .map_err(|e| format!("{VERSIONS}: {e}"))
        .and_then(|text| text.parse::<toml::Table>().map_err(|e| format!("{VERSIONS}: {e}")))
        .and_then(|v| {
            v.get("nightly")
                .and_then(|n| n.get("public_api"))
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("{VERSIONS}: no [nightly] public_api"))
        });
    let nightly = match nightly {
        Ok(n) => n,
        Err(error) => {
            eprintln!("phx-check: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut breaches = Vec::new();
    for c in ws.crates.iter().filter(|c| rules::api_snapshot::needs_snapshot(c)) {
        let current = match public_api(&nightly, &c.name) {
            Ok(api) => api,
            Err(error) => {
                eprintln!("phx-check: {error}");
                return ExitCode::FAILURE;
            }
        };
        let path = format!("{}/{API_SNAPSHOT}", c.dir);
        if write {
            if let Err(error) = fs::write(ws.root.join(&path), &current) {
                eprintln!("phx-check: {path}: {error}");
                return ExitCode::FAILURE;
            }
            println!("{path}: recorded");
        } else if let Some((line, message)) =
            rules::api_snapshot::first_difference(c.api_snapshot.as_deref().unwrap_or_default(), &current)
        {
            breaches.push(Breach::new("PC-15", &path, line, message));
        }
    }
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
