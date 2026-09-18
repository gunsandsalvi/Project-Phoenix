//! THE LAWS, AS A CHECK. `tools/eslint-rules` for the Rust kernel.
//!
//! **A law that stops being checkable is a law that stops holding**, and that is the whole reason
//! this exists before the other forty-six modules are ported rather than after. Clippy cannot
//! express any of these: they are this project's, not the language's.
//!
//! Run: `cargo run --release --manifest-path tools/phoenix-check/Cargo.toml`
//!
//! What is EXEMPT, and why:
//!   - `src/bin/**` — the benches. They time things, so they hold a clock and print.
//!   - `#[cfg(test)]` blocks — a test states the numbers it is a test of.
//!   - `ids`, `params`, `calendar` — the kernel's own conventions live there, which is what
//!     `core/` and `registry/` are exempt for in the TypeScript rules.

use std::fs;
use std::path::{Path, PathBuf};

struct Finding {
    file: String,
    line: usize,
    law: &'static str,
    what: String,
}

/// Law 6: NO BOUND OF ANY KIND. Only arithmetic impossibility. If a number explodes the
/// compensating mechanism is missing — build it and delete the bound in the same change.
const BOUNDS: &[&str] = &[".min(", ".max(", ".clamp(", "::max(", "::min("];

/// Appendix A: MISSING IS MISSING. No `?? 0`, no `|| 0`, no numeric defaults.
const DEFAULTS: &[&str] = &["unwrap_or(0", "unwrap_or(0.0", "unwrap_or_default()"];

/// No `Date`, no `Math.random`, no `console` in the engine.
const CLOCKS: &[&str] = &["std::time", "SystemTime", "Instant::now", "rand::", "println!", "eprintln!"];

/// Law 15: no mechanism branches on industry, sector, entity type or product id.
const KINDS: &[&str] = &[".industry", ".sector", ".entity_type", ".product_id", ".party_kind ==", ".kind =="];

fn rust_files(at: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(at) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn main() {
    let root = PathBuf::from("packages/kernel-rs/src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    files.sort();

    let mut found: Vec<Finding> = Vec::new();
    let mut checked = 0usize;

    for path in &files {
        let Ok(text) = fs::read_to_string(path) else { continue };
        let name = path.to_string_lossy().to_string();
        let is_bench = name.contains("/bin/");
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        // The kernel's own conventions: where a number IS the subject.
        let is_convention = matches!(stem.as_str(), "ids" | "params" | "calendar");
        let is_mechanism = name.contains("/mechanisms/");
        checked += 1;

        let mut in_test = false;
        let mut depth_at_test: i32 = -1;
        let mut depth: i32 = 0;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            // Track `#[cfg(test)]` blocks so a test may state its own numbers.
            if line.starts_with("#[cfg(test)]") {
                in_test = true;
                depth_at_test = depth;
            }
            depth += raw.matches('{').count() as i32 - raw.matches('}').count() as i32;
            if in_test && depth <= depth_at_test {
                in_test = false;
            }
            // A comment is prose, and prose may say the word "max".
            if line.starts_with("//") || line.starts_with("*") || line.starts_with("/*") {
                continue;
            }
            let say = |law: &'static str, what: String| Finding {
                file: name.clone(),
                line: n + 1,
                law,
                what,
            };

            if !is_convention {
                for b in BOUNDS {
                    if line.contains(b) {
                        found.push(say("Law 6", format!("a bound: {b}")));
                    }
                }
            }
            for d in DEFAULTS {
                if line.contains(d) {
                    found.push(say("Appendix A", format!("a numeric default: {d}")));
                }
            }
            if !is_bench && !in_test {
                for c in CLOCKS {
                    if line.contains(c) {
                        found.push(say("Error discipline", format!("the engine reaches for {c}")));
                    }
                }
            }
            if is_mechanism && !in_test {
                for k in KINDS {
                    if line.contains(k) {
                        found.push(say("Law 15", format!("a kind branch: {k}")));
                    }
                }
            }
            // Law 15: a module never imports another module.
            if is_mechanism && line.starts_with("use crate::mechanisms::") {
                found.push(say("Law 15", "a module imports another module".to_string()));
            }
        }

        // Every module cites the clauses it implements.
        if is_mechanism && stem != "mod" && !text.contains("@spec") {
            found.push(Finding {
                file: name.clone(),
                line: 1,
                law: "Law 16",
                what: "a module with no @spec citation".to_string(),
            });
        }
    }

    println!("phoenix-check: {checked} files");
    if found.is_empty() {
        println!("  every law holds.");
        return;
    }
    for f in &found {
        println!("  {}:{}  [{}] {}", f.file, f.line, f.law, f.what);
    }
    println!("\n{} findings. A law that stops being checkable is a law that stops holding.", found.len());
    std::process::exit(1);
}
