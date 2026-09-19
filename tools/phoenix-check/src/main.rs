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

/// **Part II: a FORBID that holds is as valuable as a mechanism that works, and it breaks in
/// perfect silence — guard it.**
///
/// A world that broke one of these would run, settle, print, balance and look exactly like one that
/// did not. That is the case for a GUARD rather than a test, and it is what `tools/check-forbids.ts`
/// existed for before the port. It was deleted with the TypeScript and nothing replaced it, so eight
/// COVERAGE rows went on naming it as the reason their absence held (0j.6). Three of the eight are
/// expressible here; the other five say in the row itself that nothing guards them and why.
///
/// **The SCOPE is the rule, not the word.** `spread` is what a dealer earns on its flow (§7 D4) and
/// what a price may never be set from (D8) — the same word, forbidden in one place and required in
/// another. So each entry names the files it is about and whether the word belongs ONLY there or
/// NEVER there.
struct Forbid {
    /// The clause, so a finding cites what it is about rather than restating it.
    clause: &'static str,
    /// The absence, in the clause's own words.
    says: &'static str,
    words: &'static [&'static str],
    scope: Scope,
    /// Path fragments. A file is named when its path contains one of them.
    files: &'static [&'static str],
}

enum Scope {
    /// The word belongs in these files and nowhere else. A use elsewhere is a caller.
    Only,
    /// The word does not belong in these files. A use there is the forbidden input arriving.
    Never,
}

const FORBIDS: &[Forbid] = &[
    // §7 D8. The two files that WRITE a price are the whole scope: a derived measure is forbidden
    // where the price is set, not in the world, because a spread derived FROM a price is what §7 D2
    // and Law 3 require to exist.
    Forbid {
        clause: "Corporate Credit D8",
        says: "no derived measure may set the price",
        words: &["yield", "spread", "oas", "multiple", "discount"],
        scope: Scope::Never,
        files: &["/prices.rs", "/clearing.rs"],
    },
    // §48 E2, E3. The consensus is a READ, computed when somebody looks; nothing may take it AS its
    // outlook, and there is no variable in this world called the market's expectation. The observer
    // is the one exception and it is the one §45 B2.a names: a surface decides nothing.
    Forbid {
        clause: "Reporting E2, E3",
        says: "no consensus a decision consults, and none stored",
        words: &["consensus"],
        scope: Scope::Only,
        files: &["/mechanisms/reporting.rs", "/mechanisms/observer.rs"],
    },
    // §48 F2.a. A stated move per unit of surprise is a written price path (Law 3) and it deletes
    // F2. What a surprise may reach is a party's own outlook — §46's, which is why `expectations` is
    // named beside `reporting` — and from there a schedule, and from the schedules a price that
    // cleared. Anywhere else, a surprise is being wired to a number directly.
    Forbid {
        clause: "Reporting F2.a",
        says: "no price reaction rule — no stated move per unit of surprise",
        words: &["surprise"],
        scope: Scope::Only,
        files: &["/mechanisms/reporting.rs", "/mechanisms/expectations.rs"],
    },
];

/// Whether this file is one the rule watches. `Only` watches everywhere BUT its files; `Never`
/// watches its files.
fn watches(f: &Forbid, file: &str) -> bool {
    let named = f.files.iter().any(|p| file.contains(p));
    match f.scope {
        Scope::Only => !named,
        Scope::Never => named,
    }
}

/// Whether a line of code NAMES this word.
///
/// The boundary is at the FRONT only: `spread` must not match `bid_spread`, because that is a
/// different identifier and the rule would be about a word rather than a thing. It does match
/// `spreads` and `surprises`, because a plural of a forbidden noun is the forbidden noun — and a
/// FORBID is better too loud than too quiet, since the one that fails silently is the defect this
/// exists for.
fn names(line: &str, word: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find(word) {
        let at = from + rel;
        from = at + word.len();
        let before = lower[..at].chars().next_back();
        if !before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return true;
        }
    }
    false
}

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

/// **XI-14, Law 2, 21g: a behaviour-shaping number reaches a mechanism only via `params`.** The
/// route by which one reaches a mechanism is its CONSTRUCTION — `Perishing { share: 0.01 }` — so a
/// numeric literal in a field position is a number somebody handed a mechanism without saying what
/// kind of number it is, who owns it, or what unit it is in. Declared, the same number is a row in
/// the register and the mechanism holds only its id.
///
/// 0, 1, -1 and 2 are exempt because they are arithmetic rather than declarations: an empty count,
/// a single one, the other side, a halving. That is the same exemption the TypeScript rule carried.
fn undeclared_number(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut from = 0usize;
    while let Some(rel) = line[from..].find(": ") {
        let at = from + rel;
        from = at + 2;
        // A field position, not a type annotation or a match arm: what follows must be a number.
        let value: String = chars[at + 2..]
            .iter()
            .take_while(|c| c.is_ascii_digit() || **c == '.' || **c == '_' || **c == '-')
            .collect();
        if value.is_empty() || !value.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            continue;
        }
        // It ends where a field ends. `x: 1.0e9` and `x: 3u32` are numbers too, but what follows a
        // number here must not be a letter, or `weight: 1u32` reads as the literal `1`.
        let after = chars.get(at + 2 + value.chars().count());
        if after.is_some_and(|c| c.is_alphabetic()) {
            continue;
        }
        let bare = value.replace('_', "");
        let Ok(n) = bare.parse::<f64>() else { continue };
        if n == 0.0 || n == 1.0 || n == -1.0 || n == 2.0 {
            continue;
        }
        let mut back: Vec<char> =
            chars[..at].iter().rev().take_while(|c| c.is_alphanumeric() || **c == '_').copied().collect();
        back.reverse();
        let field: String = back.into_iter().collect();
        if field.is_empty() {
            continue;
        }
        return Some(format!("{field} is handed {value} rather than a declared id"));
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
            // XI-14: and every behaviour-shaping number reaches a mechanism through `params`.
            if (is_mechanism || stem == "systems" || stem == "running") && !in_test {
                if let Some(what) = undeclared_number(line) {
                    found.push(say("XI-14", what));
                }
            }
            // Part II: the absences that break in perfect silence. **A test is not exempt** — a
            // test in another module that calls `consensus` is a caller, which is the whole of what
            // E2 forbids, and exempting it would be the escape hatch the rule exists to close.
            for f in FORBIDS {
                if !watches(f, &name) {
                    continue;
                }
                for w in f.words {
                    if names(line, w) {
                        found.push(say("Part II", format!("{} — {}: `{w}` is written here", f.clause, f.says)));
                    }
                }
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

/// **A guard is proved to BITE before it is trusted.** The discipline is the one the record set when
/// the first silent FORBID was guarded: a probe was inserted, the check failed with the clause, and
/// the probe was removed. These are that probe, kept — because a guard that has never refused
/// anything is indistinguishable from one that cannot.
#[cfg(test)]
mod forbids {
    use super::*;

    fn rule(clause: &str) -> &'static Forbid {
        FORBIDS.iter().find(|f| f.clause == clause).expect("the rule is in the table")
    }

    #[test]
    fn a_derived_measure_is_refused_where_the_price_is_written_and_nowhere_else() {
        let d8 = rule("Corporate Credit D8");
        assert!(watches(d8, "packages/kernel-rs/src/prices.rs"));
        assert!(watches(d8, "packages/kernel-rs/src/clearing.rs"));
        // §7 D4: the dealer earns the bid-offer, so the word is required in its own module.
        assert!(!watches(d8, "packages/kernel-rs/src/mechanisms/dealing.rs"));
        assert!(names("let p = par / (1.0 + yield_to(m));", "yield"));
        assert!(names("        spread_of(b) + risk_free", "spread"));
    }

    #[test]
    fn nothing_outside_reporting_may_consult_the_consensus_and_the_observer_may_look() {
        let e2 = rule("Reporting E2, E3");
        assert!(watches(e2, "packages/kernel-rs/src/running.rs"));
        assert!(watches(e2, "packages/kernel-rs/src/mechanisms/equity.rs"));
        assert!(!watches(e2, "packages/kernel-rs/src/mechanisms/reporting.rs"));
        // §45 B2.a: a surface decides nothing, which is why it is the one exception.
        assert!(!watches(e2, "packages/kernel-rs/src/mechanisms/observer.rs"));
        assert!(names("let want = consensus(of, &estimates);", "consensus"));
    }

    #[test]
    fn a_surprise_reaches_an_outlook_and_never_a_price() {
        let f2a = rule("Reporting F2.a");
        assert!(watches(f2a, "packages/kernel-rs/src/prices.rs"));
        assert!(watches(f2a, "packages/kernel-rs/src/mechanisms/equity.rs"));
        assert!(!watches(f2a, "packages/kernel-rs/src/mechanisms/reporting.rs"));
        // §46's surprise is the same noun, so the module that holds outlooks is named beside it.
        assert!(!watches(f2a, "packages/kernel-rs/src/mechanisms/expectations.rs"));
        assert!(names("let move_by = surprise * sensitivity;", "surprise"));
        // The plural of a forbidden noun is the forbidden noun.
        assert!(names("for s in self.surprises.iter() {", "surprise"));
    }

    #[test]
    fn the_boundary_is_at_the_front_so_a_different_identifier_is_a_different_thing() {
        assert!(!names("let bid_spread = q.offer - q.bid;", "spread"));
        assert!(!names("let no_surprise = 0.0;", "surprise"));
        assert!(names("Surprise { about, period }", "surprise"));
    }

    #[test]
    fn every_rule_names_a_word_and_a_file_it_is_about() {
        for f in FORBIDS {
            assert!(!f.words.is_empty(), "{} forbids no word", f.clause);
            assert!(!f.files.is_empty(), "{} names no file, so its scope is the world", f.clause);
            assert!(!f.says.is_empty(), "Law 16: {} says what it is without saying why", f.clause);
        }
    }
}
