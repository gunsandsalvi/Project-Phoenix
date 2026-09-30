//! PC-09's names: the architecture never names a type, function or file the code does not have, save in a section
//! still planned, so what it says stands is what stands. A backticked token is a code name when it is a CamelCase
//! identifier, a `snake_case(` call, a `Type::item` or `crate::path`, or a path ending `.rs`.

use super::Breach;
use crate::names::Names;
use crate::workspace::{ARCHITECTURE, Workspace};

const RULE: &str = "PC-09";

/// Names of the language, its library and the external crates the architecture speaks of, which no crate declares.
const OUTSIDE: &[&str] = &[
    "Vec",
    "Option",
    "Some",
    "None",
    "Result",
    "Ok",
    "Err",
    "String",
    "Box",
    "Arc",
    "Rc",
    "BTreeMap",
    "BTreeSet",
    "HashMap",
    "HashSet",
    "Mutex",
    "RwLock",
    "OnceLock",
    "LazyLock",
    "Cell",
    "RefCell",
    "Instant",
    "Duration",
    "SystemTime",
    "Default",
    "Copy",
    "Clone",
    "Debug",
    "Display",
    "Send",
    "Sync",
    "Sized",
    "Fn",
    "FnMut",
    "FnOnce",
    "Iterator",
    "IntoIterator",
    "PhantomData",
    "Cow",
    "Ordering",
    "AtomicU64",
    "AtomicUsize",
    "AtomicBool",
    "GlobalAlloc",
    "System",
    "Layout",
    "Path",
    "PathBuf",
    "File",
    "Read",
    "Write",
    "Hash",
    "Hasher",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "From",
    "Into",
    "TryFrom",
    "AsRef",
    "Deref",
    "Drop",
    "Any",
    "TypeId",
    "NonZeroU32",
    "NonZeroU64",
    "ThreadPool",
    "ThreadPoolBuilder",
    "Deserialize",
    "Serialize",
    "Value",
    "Table",
    "TokenStream",
    "Span",
    "Ident",
    "Visit",
    "Attribute",
    "ItemFn",
    "SmallVec",
    "ArrayVec",
    "Error",
    "Self",
    "Sum",
    "Add",
    "Sub",
    "Mul",
    "Div",
    "Neg",
    "RandomState",
    "VmHWM",
    "VmRSS",
    "allow",
    "expect",
    "cfg",
    "derive",
    "unwrap_or",
    "unwrap_or_default",
    "map_or",
    "collect",
    "to_vec",
    "to_owned",
    "to_string",
    "clone",
    "with_capacity",
    "new",
];
/// Paths that lead outside the workspace.
const OUTSIDE_ROOTS: &[&str] = &[
    "std",
    "core",
    "alloc",
    "libc",
    "syn",
    "serde",
    "toml",
    "rayon_core",
    "rayon",
    "clap",
    "cargo_metadata",
    "zstd",
    "libm",
    "hashbrown",
    "foldhash",
    "trybuild",
    "proptest",
    "env",
    "fs",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "f32",
    "f64",
];
/// A token followed on its line by this is a name to come.
const PLANNED: &str = "(planned, S";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let names = Names::of(ws);
    missing(&ws.architecture, &names)
        .into_iter()
        .map(|(line, token)| {
            Breach::new(RULE, ARCHITECTURE, line, format!("`{token}` names nothing the code has; the section is built"))
        })
        .collect()
}

/// A code name's parts to find, or none for a token that is no code name.
fn code_name(token: &str) -> Option<Vec<String>> {
    let base = token.split('<').next().unwrap_or(token);
    let word = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if let Some(path) = token.strip_suffix(".rs") {
        return (path.chars().all(|c| c.is_ascii_alphanumeric() || "_/-.{},".contains(c)) && !path.contains(' '))
            .then(|| vec![token.to_owned()]);
    }
    if base.contains("::") {
        let head = base.split('(').next().unwrap_or(base);
        let segments: Vec<&str> = head.split("::").collect();
        return segments.iter().all(|s| word(s)).then(|| segments.iter().map(|s| (*s).to_owned()).collect());
    }
    if let Some((name, _)) = base.split_once('(') {
        let snake = name.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c == '_')
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        return (snake && word(name)).then(|| vec![name.to_owned()]);
    }
    // Two letters are notation, a clause's form, never a type.
    let camel = base.len() > 2
        && base.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && base.chars().any(|c| c.is_ascii_lowercase())
        && word(base)
        && !base.contains('_');
    camel.then(|| vec![base.to_owned()])
}

/// Whether a code name's parts are all the workspace's: a file by its path, a path by its last item and its first part.
fn found(parts: &[String], names: &Names) -> bool {
    let known = |s: &str| names.items.contains(s) || names.crates.contains(s) || OUTSIDE.contains(&s);
    match parts {
        [one] if std::path::Path::new(one).extension().is_some_and(|e| e.eq_ignore_ascii_case("rs")) => {
            // A path with alternatives, `{a,b}.rs`, is each of them.
            let path = one.as_str();
            match (path.find('{'), path.find('}')) {
                (Some(open), Some(close)) if open < close => {
                    let (head, tail) = (&path[..open], &path[close + 1..]);
                    path[open + 1..close].split(',').all(|alt| names.has_file(&format!("{head}{}{tail}", alt.trim())))
                }
                _ => names.has_file(path),
            }
        }
        [one] => known(one),
        [first, .., last] => {
            OUTSIDE_ROOTS.contains(&first.as_str()) || OUTSIDE.contains(&first.as_str()) || known(first) && known(last)
        }
        [] => true,
    }
}

/// Every code name the architecture holds outside planned sections that the workspace lacks, with its line.
fn missing(text: &str, names: &Names) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut exempt = false;
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") || line.starts_with("### ") {
            // A subsection of the core's crates is exempt while its status line, the first after its heading, reads
            // planned or building.
            exempt = line.starts_with("### 7.")
                && lines
                    .iter()
                    .skip(i + 1)
                    .find(|l| !l.trim().is_empty())
                    .is_some_and(|l| l.starts_with("Status: planned") || l.starts_with("Status: building"));
            continue;
        }
        if exempt {
            continue;
        }
        for (at, token) in backticked(line) {
            let Some(parts) = code_name(token) else { continue };
            if line[at..].contains(PLANNED) || found(&parts, names) {
                continue;
            }
            out.push((i + 1, token.to_owned()));
        }
    }
    out
}

/// Every single-backticked token of a line, with where it ends.
fn backticked(line: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut rest = line;
    let mut offset = 0;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let token = &after[..close];
        let end = offset + open + 1 + close + 1;
        if !token.is_empty() {
            out.push((end, token));
        }
        offset = end;
        rest = &line[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{code_name, missing};
    use crate::names::Names;

    fn names() -> Names {
        let mut n = Names::default();
        for item in ["KindStore", "rebuild_skipped", "Saved", "span_items", "trace"] {
            n.items.insert(item.to_owned());
        }
        n.crates.insert("phx_exec".to_owned());
        n.files.insert("crates/kernel/phx-exec/src/trace.rs".to_owned());
        n
    }

    #[test]
    fn architecture_names_must_exist() {
        let text = "## 3. Crates\n`KindStore` and `Saved::rebuild_skipped` and `phx_exec::trace::span_items` and \
                    `trace.rs` stand; `ColumnFacts`, `column_facts.rs`, `record_facts(` and `Core::gone` do not.\n";
        let found: Vec<String> = missing(text, &names()).into_iter().map(|(_, t)| t).collect();
        assert_eq!(found, ["ColumnFacts", "column_facts.rs", "record_facts(", "Core::gone"]);
    }

    #[test]
    fn planned_sections_are_exempt() {
        let text = "### 7.2 phx-exec\n\nStatus: planned (S1.169)\n\n`ChunkPlan` to come.\n\n## 8. Next\n\
                    `DayCached` (planned, S1.211) and `Gone`.\n";
        let found: Vec<String> = missing(text, &names()).into_iter().map(|(_, t)| t).collect();
        assert_eq!(found, ["Gone"]);
    }

    #[test]
    fn config_keys_are_not_names() {
        for token in
            ["fin.retail.match_ns", "perf/budget.toml", "--features", "SAVE_FORMAT", "phx-core", "SET.15", "N8.8"]
        {
            assert_eq!(code_name(token), None, "{token}");
        }
    }

    #[test]
    fn prose_sections_pass() {
        assert!(missing("## 1. Why\nThe world runs once, *on the phone*.\n", &names()).is_empty());
    }

    #[test]
    fn test_only_items_do_not_count() {
        use crate::workspace::fixture::{krate, with_source};
        use crate::workspace::{Layer, Workspace};
        let c = with_source(
            krate("phx-core", Layer::Kernel),
            "src/a.rs",
            "pub struct Real;\n#[cfg(test)] mod tests { pub struct Fixture; }",
        );
        let n = Names::of(&Workspace::new(vec![c]));
        assert!(n.items.contains("Real") && !n.items.contains("Fixture"));
    }
}
