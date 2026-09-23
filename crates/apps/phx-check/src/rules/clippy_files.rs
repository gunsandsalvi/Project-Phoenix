use toml::Value;

use super::Breach;
use crate::workspace::{Crate, Workspace};

const RULE: &str = "PC-16";

const LISTS: &[&str] = &["disallowed-methods", "disallowed-types", "disallowed-macros"];

const INTEGERS: &[&str] = &["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize"];
const ATOMICS: &[&str] = &[
    "AtomicBool",
    "AtomicU8",
    "AtomicU16",
    "AtomicU32",
    "AtomicU64",
    "AtomicUsize",
    "AtomicI8",
    "AtomicI16",
    "AtomicI32",
    "AtomicI64",
    "AtomicIsize",
    "AtomicPtr",
];
const PRINTING: &[&str] = &["std::println", "std::eprintln", "std::print", "std::eprint"];

/// The entries a crate's own clippy file drops from the root lists: modular arithmetic where an algorithm is
/// modular, threads in the pool, the clock and printing in the applications.
pub fn exemptions(krate: &str) -> Vec<String> {
    let modular = || {
        INTEGERS.iter().flat_map(|t| ["wrapping_add", "wrapping_sub", "wrapping_mul"].map(|op| format!("{t}::{op}")))
    };
    let owned = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    match krate {
        "phx-exec" => {
            let mut e: Vec<String> = ATOMICS.iter().map(|a| format!("std::sync::atomic::{a}")).collect();
            e.extend(owned(&["std::thread::spawn", "std::thread_local"]));
            e.extend(modular());
            e
        }
        "phx-rand" | "phx-store" => modular().collect(),
        "phx-cli" => {
            let mut e = owned(&["std::time::Instant::now", "std::env::var"]);
            e.extend(owned(PRINTING));
            e
        }
        "phx-check" => owned(PRINTING),
        "phx-ffi" => owned(&["std::time::Instant::now"]),
        _ => Vec::new(),
    }
}

/// The root clippy file with a crate's exemptions removed from its lists.
fn expected(root: &Value, exempt: &[String]) -> Value {
    let mut table = root.clone();
    if let Value::Table(t) = &mut table {
        for key in LISTS {
            if let Some(Value::Array(entries)) = t.get_mut(*key) {
                entries.retain(|e| e.as_str().is_none_or(|s| !exempt.iter().any(|x| x == s)));
            }
        }
    }
    table
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let root: Value = match toml::from_str(&ws.root_clippy) {
        Ok(v) => v,
        Err(error) => return vec![Breach::new(RULE, "clippy.toml", 1, format!("does not parse: {error}"))],
    };
    ws.crates.iter().filter_map(|c| check(&root, c)).collect()
}

fn check(root: &Value, c: &Crate) -> Option<Breach> {
    let exempt = exemptions(&c.name);
    let path = format!("{}/clippy.toml", c.dir);
    match (&c.clippy, exempt.is_empty()) {
        (None, true) => None,
        (Some(_), true) => Some(Breach::new(RULE, &path, 1, "a clippy file for a crate with no exemptions")),
        (None, false) => Some(Breach::new(RULE, &path, 1, "missing: the crate has declared exemptions")),
        (Some(text), false) => match toml::from_str::<Value>(text) {
            Err(error) => Some(Breach::new(RULE, &path, 1, format!("does not parse: {error}"))),
            Ok(own) if own == expected(root, &exempt) => None,
            Ok(_) => Some(Breach::new(RULE, &path, 1, "differs from the root file minus the crate's exemptions")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::krate;
    use crate::workspace::{Layer, Workspace};

    const ROOT: &str = "allow-unwrap-in-tests = true\n\
                        disallowed-methods = [\"std::time::Instant::now\", \"core::cmp::min\"]\n\
                        disallowed-macros = [\"std::println\", \"std::dbg\"]\n";

    #[test]
    fn per_crate_clippy_matches_root_minus_exemptions() {
        let mut check = krate("phx-check", Layer::Apps);
        check.clippy = Some(
            "allow-unwrap-in-tests = true\n\
             disallowed-methods = [\"std::time::Instant::now\", \"core::cmp::min\"]\n\
             disallowed-macros = [\"std::dbg\"]\n"
                .to_owned(),
        );
        let mut ws = Workspace::new(vec![check.clone()]);
        ws.root_clippy = ROOT.to_owned();
        assert!(run(&ws).is_empty());
        check.clippy = Some(ROOT.to_owned());
        let mut core = krate("phx-core", Layer::Kernel);
        core.clippy = Some(ROOT.to_owned());
        ws.crates = vec![check, core, krate("phx-ffi", Layer::Apps)];
        assert_eq!(run(&ws).len(), 3);
    }
}
