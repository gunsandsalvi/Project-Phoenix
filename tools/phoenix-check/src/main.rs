//! The laws, as a check. Clippy cannot express any of these: they are this project's.
//!
//! Exempt, and this is the one writer of that list: `src/bin/**`, which holds a clock and prints
//! and constructs its own inputs; `#[cfg(test)]` blocks, for NUMBERS only — a test may write `2.5`
//! undeclared, and may not build a world; and `ids`, `params`, `calendar`, where a number is the
//! subject.

mod spec;

use std::fs;
use std::path::{Path, PathBuf};

struct Finding {
    file: String,
    line: usize,
    law: &'static str,
    what: String,
}

/// NO BOUND OF ANY KIND.
const BOUNDS: &[&str] = &[".min(", ".max(", ".clamp(", "::max(", "::min("];

/// MISSING IS MISSING.
const DEFAULTS: &[&str] = &["unwrap_or(0", "unwrap_or(0.0", "unwrap_or_default()"];

/// No `Date`, no `Math.random`, no `console` in the engine.
const CLOCKS: &[&str] = &["std::time", "SystemTime", "Instant::now", "rand::", "println!", "eprintln!"];

/// No mechanism branches on industry, sector, entity type or product id.
const KINDS: &[&str] = &[".industry", ".sector", ".entity_type", ".product_id", ".party_kind ==", ".kind =="];

/// THE TESTING RULE: no test is ever run against a test world. Absolute — the ratchet reached
/// zero at 0m3 and its row is gone, so a single construction inside a test fails the check.
const WORLD_BUILDING: &[&str] = &[
    "Parties::new(",
    "Register::new(",
    "Instruments::new(",
    "Prints::new(",
    "Settlement::new(",
    "Journal::new(",
    "Registry::new(",
    "Stores::new(",
    "Audit::new(",
    "World::empty(",
    "World::new(",
];

/// ONE SYSTEM, ONE FILE: a system's BEHAVIOUR lives in the system's own module.
const BEHAVIOUR_OUTSIDE_ITS_MODULE: &[&str] =
    &["impl Mechanism for ", "impl Participant for ", "impl crate::module::Participant for "];

/// A RATCHET: the count must fall and must never rise.
struct Ratchet {
    law: &'static str,
    /// The item that drives it to zero, so a reader knows where the work is.
    item: &'static str,
    /// What it stands at today.
    allowed: usize,
}

const RATCHETS: &[Ratchet] = &[
    Ratchet { law: "One system, one file", item: "0m2", allowed: 9 },
];

/// Part II: a FORBID that holds is as valuable as a mechanism that works, and it breaks in perfect
/// silence — guard it.
struct Forbid {
    /// The clause, so a finding cites what it is about rather than restating it.
    clause: &'static str,
    /// The absence, in the clause's own words.
    says: &'static str,
    words: &'static [&'static str],
    scope: Scope,
    /// Path fragments.
    files: &'static [&'static str],
}

enum Scope {
    /// The word belongs in these files and nowhere else.
    Only,
    /// The word does not belong in these files.
    Never,
}

const FORBIDS: &[Forbid] = &[
    Forbid {
        clause: "Corporate Credit D8",
        says: "no derived measure may set the price",
        words: &["yield", "spread", "oas", "multiple", "discount"],
        scope: Scope::Never,
        files: &["/prices.rs", "/clearing.rs"],
    },
    // The consensus is a READ, computed when somebody looks; nothing may take it AS its outlook, and
    // there is no variable in this world called the market's expectation.
    Forbid {
        clause: "Reporting E2, E3",
        says: "no consensus a decision consults, and none stored",
        words: &["consensus"],
        scope: Scope::Only,
        files: &["/mechanisms/reporting.rs", "/mechanisms/observer.rs"],
    },
    Forbid {
        clause: "Reporting F2.a",
        says: "no price reaction rule — no stated move per unit of surprise",
        words: &["surprise"],
        scope: Scope::Only,
        files: &["/mechanisms/reporting.rs", "/mechanisms/expectations.rs"],
    },
];

/// Whether this file is one the rule watches.
fn watches(f: &Forbid, file: &str) -> bool {
    let named = f.files.iter().any(|p| file.contains(p));
    match f.scope {
        Scope::Only => !named,
        Scope::Never => named,
    }
}

/// Whether a line of code NAMES this word.
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

/// Part II: a VERIFY that cannot fail is worse than none.
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
            // pretending to measure.
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
/// whole.
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

/// A magnitude compared against a NUMBER.
fn fixed_tolerance(line: &str) -> Option<String> {
    for op in [".abs() < ", ".abs() <= "] {
        let Some(at) = line.find(op) else { continue };
        let rest = line[at + op.len()..].trim_start();
        let first = rest.chars().next()?;
        // A derived dust is a call or a name; only a number written out is a band.
        if first.is_ascii_digit() && !rest.replace(' ', "").contains("*f64::EPSILON") {
            let band: String = rest.chars().take_while(|c| !c.is_whitespace() && *c != ')' && *c != ',').collect();
            return Some(format!("{}{}", op.trim_end(), band));
        }
    }
    None
}

/// A behaviour-shaping number reaches a mechanism only via `params`.
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
        // It ends where a field ends.
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

/// Every construction this line makes, of `WORLD_BUILDING`'s.
fn builds_a_world(line: &str) -> Vec<&'static str> {
    WORLD_BUILDING.iter().filter(|w| line.contains(**w)).copied().collect()
}

/// The type whose behaviour this line declares, where it declares one.
fn declares_behaviour(line: &str) -> Option<&str> {
    for b in BEHAVIOUR_OUTSIDE_ITS_MODULE {
        if let Some(rest) = line.strip_prefix(b) {
            return rest.split_whitespace().next().or(Some(rest));
        }
    }
    None
}

/// A line with its string literals emptied.
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

    // The specification is what a citation is checked against, so it is READ rather than restated
    // here.
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
            // Asset}`, whose brace is not a block.
            if line.starts_with("//") || line.starts_with("*") || line.starts_with("/*") {
                continue;
            }
            // Track `#[cfg(test)]` blocks so a test may state its own numbers.
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

            // Law 6 is about the ENGINE.
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
            // The testing rule, and it is the one rule that fires ONLY inside a test: building the
            // world is what the engine is for, and what a test may never do.
            if in_test {
                for w in builds_a_world(line) {
                    found.push(say("Testing rule", format!("a test builds a world: `{w}`")));
                }
            }
            // One system, one file.
            if !is_mechanism && !is_bench && !in_test && stem != "module" {
                if let Some(what) = declares_behaviour(line) {
                    found.push(say(
                        "One system, one file",
                        format!("`{what}` decides what a system does, outside that system's module"),
                    ));
                }
            }
            // Part II: a VERIFY that cannot fail is worse than none.
            if !in_test {
                if let Some(what) = compares_with_itself(line) {
                    found.push(say("Part II", what));
                }
            }
            // A module never imports another module.
            if is_mechanism && line.starts_with("use crate::mechanisms::") {
                found.push(say("Law 15", "a module imports another module".to_string()));
            }
            // Tolerance is arithmetic dust, derived per check — never a band somebody picked.
            if let Some(band) = fixed_tolerance(line) {
                found.push(say("Law 7", format!("a tolerance nobody derived: {band}")));
            }
            // And every behaviour-shaping number reaches a mechanism through `params`.
            if (is_mechanism || stem == "systems" || stem == "running") && !in_test {
                if let Some(what) = undeclared_number(line) {
                    found.push(say("XI-14", what));
                }
            }
            // Part II: the absences that break in perfect silence.
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

        // And every citation names a clause that is there.
        for (n, citation) in spec::citations(&text) {
            if let Some(why) = spec.resolve(&citation) {
                found.push(Finding { file: name.clone(), line: n, law: "Law 16", what: why });
            }
        }
    }

    println!("phoenix-check: {checked} files");

    // The ratcheted laws report a COUNT against what their item has left to do; everything else
    // names its site, because everything else is fixed where it stands.
    let mut failed = false;
    for r in RATCHETS {
        let at = found.iter().filter(|f| f.law == r.law).count();
        let verdict = if at > r.allowed {
            failed = true;
            format!("ROSE by {} — a ratchet only falls", at - r.allowed)
        } else if at < r.allowed {
            failed = true;
            format!("has fallen to {at}: lower the allowance in main.rs, or the check goes slack")
        } else {
            format!("{at} left, and {} is what closes them", r.item)
        };
        println!("  [{}] allowance {} · {verdict}", r.law, r.allowed);
    }

    let loose: Vec<&Finding> =
        found.iter().filter(|f| !RATCHETS.iter().any(|r| r.law == f.law)).collect();
    if loose.is_empty() && !failed {
        println!("  every law holds.");
        return;
    }
    for f in &loose {
        println!("  {}:{}  [{}] {}", f.file, f.line, f.law, f.what);
    }
    if !loose.is_empty() {
        println!(
            "\n{} findings. A law that stops being checkable is a law that stops holding.",
            loose.len()
        );
    }
    std::process::exit(1);
}

/// A guard is proved to BITE before it is trusted.
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
        // The dealer earns the bid-offer, so the word is required in its own module.
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
        // A surface decides nothing, which is why it is the one exception.
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
    fn a_test_that_constructs_a_store_is_a_test_that_builds_a_world() {
        assert_eq!(builds_a_world("        let mut reg = Register::new();"), ["Register::new("]);
        // Every construction on the line, so breaking it in two cannot make the count rise.
        assert_eq!(
            builds_a_world("let (r, p) = (Register::new(), Prints::new());"),
            ["Register::new(", "Prints::new("]
        );
        assert_eq!(builds_a_world("    let w = World::empty();"), ["World::empty("]);
        // A date is arithmetic, a declared number is a declaration, an id is an allocation: a test
        // over any of the three is already values in, values out.
        assert!(builds_a_world("let c = Calendar::new(Day(0), 7, 3);").is_empty());
        assert!(builds_a_world("let p = Params::new(100.0, 60.0);").is_empty());
        assert!(builds_a_world("assert_eq!(waterfall(500.0, &claims), 500.0);").is_empty());
    }

    #[test]
    fn behaviour_is_declared_where_the_system_lives_and_the_finding_names_which() {
        assert_eq!(declares_behaviour("impl Mechanism for Protection {"), Some("Protection"));
        assert_eq!(declares_behaviour("impl Participant for ForcedSeller {"), Some("ForcedSeller"));
        assert_eq!(declares_behaviour("impl crate::module::Participant for Builder {"), Some("Builder"));
        // An ordinary impl is not a system deciding anything.
        assert_eq!(declares_behaviour("impl Instruments {"), None);
        assert_eq!(declares_behaviour("impl Default for Journal {"), None);
    }

    #[test]
    fn a_ratchet_names_the_item_that_closes_it_and_never_stands_at_nothing() {
        for r in RATCHETS {
            assert!(!r.item.is_empty(), "{} names no item, so nobody owns driving it down", r.law);
            // Zero is not an allowance, it is a rule: delete the row and the law is absolute.
            assert!(r.allowed > 0, "{} allows nothing — delete the ratchet instead", r.law);
        }
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
