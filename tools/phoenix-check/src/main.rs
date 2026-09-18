//! THE LAWS, AS A CHECK. `tools/eslint-rules` for the Rust kernel.
//!
//! **A law that stops being checkable is a law that stops holding**, and that is the whole reason
//! this exists before the other forty-six modules are ported rather than after. Clippy cannot
//! express any of these: they are this project's, not the language's.
//!
//! Run: `cargo run --release --manifest-path tools/phoenix-check/Cargo.toml`
//!
//! What is EXEMPT, and why:
//!   - `src/bin/**` — the benches. They time things, so they hold a clock and print; and they
//!     construct inputs, so a bound on a loop counter is arithmetic rather than a damper.
//!   - `#[cfg(test)]` blocks — a test states the numbers it is a test of.
//!   - `ids`, `params`, `calendar` — the kernel's own conventions live there, which is what
//!     `core/` and `registry/` are exempt for in the TypeScript rules.

mod spec;

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

/// **Part II: a VERIFY that cannot fail is worse than none.** Written after the same defect was
/// written three times in one session — in `trade_credit` (receivables summed against payables that
/// were the same field), in `cds` (protection paid against protection received, one number), and in
/// `irs` (`p.amount - p.amount`). Each looked like a conservation check and each compared a quantity
/// with itself, so each reported green about a thing it had not measured.
///
/// What is detectable textually is an expression subtracted from, or compared with, ITSELF. That is
/// the shape all three took once reduced. It cannot catch the subtler version — two sums that are
/// equal by construction over different names — which is why the rule the record states is a rule
/// for the writer: **before a VERIFY is written, name the input that makes it answer false.**
fn compares_with_itself(line: &str) -> Option<String> {
    let bytes: Vec<char> = line.chars().collect();
    for op in [" - ", " == ", " != "] {
        let mut from = 0usize;
        while let Some(rel) = line[from..].find(op) {
            let at = from + rel;
            from = at + op.len();
            let (Some(left), Some(right)) = (operand_before(&bytes, at), operand_after(&bytes, at + op.len()))
            else {
                continue;
            };
            // Only an expression that READS something — a field, a call, an index — can be a check
            // pretending to measure. Two bare identifiers are ordinary arithmetic, and `50.0 - 50.0`
            // is a literal spelling out what a test expects rather than a quantity being compared
            // with itself, so a name has to appear in it.
            let names_something = left.chars().any(|c| c.is_alphabetic());
            let reads = names_something && (left.contains('.') || left.contains('(') || left.contains('['));
            if reads && left == right {
                return Some(format!("an expression compared with itself: {left}{op}{right}"));
            }
        }
    }
    None
}

/// The operand ending just before `at`, with balanced parentheses walked through so a call is taken
/// whole. `None` where there is nothing readable there.
fn operand_before(chars: &[char], at: usize) -> Option<String> {
    let mut end = at;
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    let mut start = end;
    let mut depth = 0i32;
    while start > 0 {
        let c = chars[start - 1];
        match c {
            ')' | ']' => depth += 1,
            '(' | '[' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ if depth > 0 => {}
            c if c.is_alphanumeric() || c == '_' || c == '.' => {}
            _ => break,
        }
        start -= 1;
    }
    let taken: String = chars[start..end].iter().collect();
    if taken.is_empty() { None } else { Some(taken) }
}

/// And the operand beginning at `at`, the same way.
fn operand_after(chars: &[char], at: usize) -> Option<String> {
    let mut start = at;
    while start < chars.len() && chars[start] == ' ' {
        start += 1;
    }
    let mut end = start;
    let mut depth = 0i32;
    while end < chars.len() {
        let c = chars[end];
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ if depth > 0 => {}
            c if c.is_alphanumeric() || c == '_' || c == '.' => {}
            _ => break,
        }
        end += 1;
    }
    let taken: String = chars[start..end].iter().collect();
    if taken.is_empty() { None } else { Some(taken) }
}

/// **Law 7: a magnitude compared against a NUMBER.** `x.abs() < 1e-12` is a band, and a band is what
/// `num::dust` exists to replace — `terms × ε × Σ|magnitudes|`, derived from the check's own terms.
/// What is detectable is the literal: `.abs() <= dust(...)` names a derivation and `.abs() <= 1e-9`
/// names a hope. Returns what was written, because the writer needs to see it to replace it.
fn fixed_tolerance(line: &str) -> Option<String> {
    for op in [".abs() < ", ".abs() <= "] {
        let Some(at) = line.find(op) else { continue };
        let rest = line[at + op.len()..].trim_start();
        let first = rest.chars().next()?;
        // A derived dust is a call or a name; only a number written out is a band. **`6.0 *
        // f64::EPSILON * magnitude` is not one** — it is Law 7's own formula, `terms × ε × Σ|m|`,
        // written where a reader can see the terms, and the leading number is the term COUNT.
        if first.is_ascii_digit() && !rest.replace(' ', "").contains("*f64::EPSILON") {
            let band: String = rest.chars().take_while(|c| !c.is_whitespace() && *c != ')' && *c != ',').collect();
            return Some(format!("{}{}", op.trim_end(), band));
        }
    }
    None
}

/// A line with its string literals emptied. **Braces inside a format string are not code**, and
/// counting them walked the `#[cfg(test)]` tracker out of step in every file carrying a message like
/// `"declared {:?} and its legs are {shape:?}"` — which silently un-exempted that file's tests. The
/// same emptying stops a word inside a message being read as the code it names.
fn without_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
                out.push('"');
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push('"');
            continue;
        }
        out.push(c);
    }
    out
}

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

    // Law 16: the specification is what a citation is checked against, so it is READ rather than
    // restated here. A check carrying its own copy of the clause list would be the second writer.
    let spec_text = fs::read_to_string("docs/spec/PROJECT_PHOENIX.md")
        .expect("Law 19: the specification is the source, and it is not where it is expected");
    let spec = spec::Spec::read(&spec_text);

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
        let mut inside = false;
        let mut depth_at_test: i32 = -1;
        let mut depth: i32 = 0;
        for (n, source) in text.lines().enumerate() {
            // Everything below reads CODE, never the inside of a message.
            let raw = without_strings(source);
            let line = raw.trim();
            // A comment is prose, and prose may say the word "max" — and may write `Leg::{Money,
            // Asset}`, whose brace is not a block. Counting those walked the tracker out of step and
            // silently un-exempted the tests of every file with such a line, so the skip comes FIRST.
            if line.starts_with("//") || line.starts_with("*") || line.starts_with("/*") {
                continue;
            }
            // Track `#[cfg(test)]` blocks so a test may state its own numbers.
            //
            // The block is left when the depth comes back DOWN to where the attribute stood — but it
            // stands at that depth on its own line too, so leaving on `depth <= depth_at_test` ended
            // the block the instant it began, and this exemption never once applied. `inside` is
            // what tells the two apart: the block is only left after it has been entered.
            if line.starts_with("#[cfg(test)]") {
                in_test = true;
                inside = false;
                depth_at_test = depth;
            }
            depth += raw.matches('{').count() as i32 - raw.matches('}').count() as i32;
            if in_test {
                if depth > depth_at_test {
                    inside = true;
                } else if inside {
                    in_test = false;
                }
            }
            let say = |law: &'static str, what: String| Finding {
                file: name.clone(),
                line: n + 1,
                law,
                what,
            };

            // Law 6 is about the ENGINE. A bench is not the engine: it CONSTRUCTS inputs, and
            // "build no more legs than remain to build" is arithmetic about a loop rather than a
            // damper on a number the world decided. The TypeScript config turns `no-bounds` off
            // for the same kind of file and for the same reason.
            if !is_convention && !is_bench {
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
            // Part II: a VERIFY that cannot fail is worse than none. A test may compare a thing
            // with itself to show that it does not move; the engine has no such reason.
            if !in_test {
                if let Some(what) = compares_with_itself(line) {
                    found.push(say("Part II", what));
                }
            }
            // Law 15: a module never imports another module.
            if is_mechanism && line.starts_with("use crate::mechanisms::") {
                found.push(say("Law 15", "a module imports another module".to_string()));
            }
            // **Law 7: tolerance is arithmetic dust, derived per check — never a band somebody
            // picked.** A comparison of a magnitude against a literal is the shape a band takes, and
            // a check that only passes with one is reporting a defect. It applies inside tests too,
            // and that is where it was found: `1e-12` on a year fraction that is exactly one.
            if let Some(band) = fixed_tolerance(line) {
                found.push(say("Law 7", format!("a tolerance nobody derived: {band}")));
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

        // **And every citation names a clause that is there** (21.8, 0c.3). A citation to a clause
        // that does not exist reads as evidence and is not any: `check:existence` counts it, and
        // nobody can tell it from the real thing without opening the document at that line.
        for (n, citation) in spec::citations(&text) {
            if let Some(why) = spec.resolve(&citation) {
                found.push(Finding { file: name.clone(), line: n, law: "Law 16", what: why });
            }
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
