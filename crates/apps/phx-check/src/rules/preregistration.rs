//! Pre-registration: every measure is defined before any read of the world uses it. A definition is never edited
//! but by a new version citing why, never deleted; a report or read is never edited or deleted; and every report
//! names definitions each registered in an ancestor of the commit it was computed at.

use std::path::Path;

use regex::Regex;

use super::Breach;
use crate::git::{Git, trailers};
use crate::workspace::Workspace;

const RULE: &str = "PC-90";
/// The rule's own file: the commit that added it is the registration.
const REGISTRATION: &str = "crates/apps/phx-check/src/rules/preregistration.rs";
const DEFINITIONS: &str = "data/measure/";
/// Where the reads of the world are kept, never edited or deleted.
const REPORTS: &[&str] = &["perf/realism", "perf/chains", "perf/reads"];
/// The files that are not one measure's definition.
const SHARED: &[&str] = &["data/measure/CREDIT.toml", "data/measure/estimators.toml", "data/measure/N4/BREAKS.toml"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches: Vec<Breach> = ws
        .data
        .iter()
        .filter(|(path, _)| path.starts_with(DEFINITIONS) && !SHARED.contains(&path.as_str()))
        .flat_map(|(path, text)| malformed(path, text).into_iter().map(move |m| Breach::new(RULE, path, 1, m)))
        .collect();
    let git = Git::at(&ws.root);
    let fail = |e: String| Breach::new(RULE, REGISTRATION, 1, e);
    if let Err(e) = git.complete() {
        breaches.push(fail(e));
        return breaches;
    }
    let registered = match git.added(REGISTRATION) {
        Ok(Some(c)) => c,
        Ok(None) => return breaches,
        Err(e) => {
            breaches.push(fail(e));
            return breaches;
        }
    };
    match history(&git, &registered) {
        Ok(found) => breaches.extend(found.into_iter().map(|(file, m)| Breach::new(RULE, &file, 1, m))),
        Err(e) => breaches.push(fail(e)),
    }
    breaches.extend(reports(ws, &git));
    breaches
}

/// A definition's refusals: the fields every one carries, and a benchmark with its range and source.
pub fn malformed(path: &str, text: &str) -> Vec<String> {
    let table: toml::Table = match toml::from_str(text) {
        Ok(t) => t,
        Err(e) => return vec![format!("does not parse: {e}")],
    };
    let mut refusals = Vec::new();
    let stem = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    if table.get("id").and_then(toml::Value::as_str) != Some(stem) {
        refusals.push(format!("its `id` is not its file's name, `{stem}`"));
    }
    if table.get("version").and_then(toml::Value::as_integer).is_none_or(|v| v < 1) {
        refusals.push("no `version` of one or more".to_owned());
    }
    if table.get("min_years").and_then(toml::Value::as_integer).is_none_or(|v| v < 1) {
        refusals.push("no `min_years`: the years of play its statistic needs".to_owned());
    }
    if table.get("suspects").and_then(toml::Value::as_array).is_none_or(Vec::is_empty) {
        refusals.push("no `suspects`, named before any read".to_owned());
    }
    let benchmarks = table.get("benchmark").and_then(toml::Value::as_array).cloned().unwrap_or_default();
    if benchmarks.is_empty() {
        refusals.push("no `[[benchmark]]`".to_owned());
    }
    for (i, b) in benchmarks.iter().enumerate() {
        let range: Vec<f64> = b
            .get("range")
            .and_then(toml::Value::as_array)
            .map(|r| r.iter().filter_map(number).collect())
            .unwrap_or_default();
        if !matches!(range.as_slice(), [lo, hi] if lo < hi) {
            refusals.push(format!("benchmark {i} has no numerical `range` [low, high]"));
        }
        for key in ["statistic", "source", "source_ref"] {
            if b.get(key).and_then(toml::Value::as_str).is_none_or(|s| s.trim().is_empty()) {
                refusals.push(format!("benchmark {i} has no `{key}`"));
            }
        }
    }
    refusals
}

/// A benchmark's bound, written as a float or a whole number.
fn number(v: &toml::Value) -> Option<f64> {
    v.as_float().or_else(|| v.as_integer().and_then(|n| i32::try_from(n).ok()).map(f64::from))
}

/// Every commit since the registration: no report or read edited or deleted, no definition deleted, and every
/// definition edited carrying its `Measure-Change` with a higher version.
fn history(git: &Git, registered: &str) -> Result<Vec<(String, String)>, String> {
    let mut found = Vec::new();
    let finding = Regex::new(r"\bF-\d{3}\b").map_err(|e| e.to_string())?;
    for commit in git.since(registered)? {
        let short: String = commit.chars().take(12).collect();
        let message = git.message(&commit)?;
        let cited = trailers(&message, "Measure-Change");
        for t in &cited {
            if t.contains("perf/") || finding.is_match(t) {
                found.push((
                    REGISTRATION.to_owned(),
                    format!("{short}: `Measure-Change: {t}` names a result or a finding"),
                ));
            }
        }
        for dir in REPORTS {
            for t in git.changed(&commit, dir)? {
                if t.status != 'A' {
                    found.push((t.path.clone(), format!("{short}: a report or read edited or deleted")));
                }
            }
        }
        for t in git.changed(&commit, DEFINITIONS)? {
            if SHARED.contains(&t.path.as_str()) || t.status == 'A' {
                continue;
            }
            if t.status == 'D' {
                found.push((t.path.clone(), format!("{short}: a definition deleted")));
                continue;
            }
            let parent = git.parent(&commit).unwrap_or_default();
            let version = |c: &str| -> Option<(String, i64)> {
                let table: toml::Table = toml::from_str(&git.show(c, &t.path)?).ok()?;
                Some((table.get("id")?.as_str()?.to_owned(), table.get("version")?.as_integer()?))
            };
            if let Err(why) = superseded(version(&parent), version(&commit), &cited) {
                found.push((t.path.clone(), format!("{short}: {why}")));
            }
        }
    }
    Ok(found)
}

/// An edit of a definition: a higher version, and `Measure-Change: defect|precision <id>@<version superseded>`.
pub fn superseded(before: Option<(String, i64)>, after: Option<(String, i64)>, cited: &[&str]) -> Result<(), String> {
    let (Some((id, old)), Some((_, new))) = (before, after) else {
        return Err("a definition edited whose id or version cannot be read".to_owned());
    };
    if new <= old {
        return Err(format!("`{id}` edited without a new version above {old}"));
    }
    let wanted = [format!("defect {id}@{old}"), format!("precision {id}@{old}")];
    if !cited.iter().any(|t| wanted.iter().any(|w| t == w)) {
        return Err(format!("`{id}` edited without `Measure-Change: defect|precision {id}@{old}`"));
    }
    Ok(())
}

/// Every report's definitions, each registered in an ancestor of the report's commit.
fn reports(ws: &Workspace, git: &Git) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for dir in REPORTS {
        for path in json_files(&ws.root.join(dir)) {
            let rel = path.strip_prefix(&ws.root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            let report: serde_json::Value = match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))
            {
                Ok(v) => v,
                Err(e) => {
                    breaches.push(Breach::new(RULE, &rel, 1, format!("does not read: {e}")));
                    continue;
                }
            };
            let Some(commit) = report.get("commit").and_then(serde_json::Value::as_str) else {
                breaches.push(Breach::new(RULE, &rel, 1, "no `commit` it was computed at"));
                continue;
            };
            let Some(defs) = report.get("definitions").and_then(serde_json::Value::as_object) else {
                breaches.push(Breach::new(RULE, &rel, 1, "no `definitions` with their hashes"));
                continue;
            };
            for (def, hash) in defs {
                let hash = hash.as_str().unwrap_or_default();
                let introduced = git.touching(def).map(|commits| {
                    commits
                        .into_iter()
                        .filter_map(|c| Some((canonical_hash(&git.show(&c, def)?).ok()?, c)))
                        .collect::<Vec<_>>()
                });
                let outcome =
                    introduced.and_then(|h| registered_before(&h, hash, commit, &|a, b| git.is_ancestor(a, b)));
                if let Err(why) = outcome {
                    breaches.push(Breach::new(RULE, &rel, 1, format!("`{def}`: {why}")));
                }
            }
        }
    }
    breaches
}

fn json_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            out.extend(json_files(&p));
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// Whether the commit that introduced a definition's content, found in its history as (hash, commit) oldest first,
/// is an ancestor of the report's commit.
pub fn registered_before(
    history: &[(String, String)],
    hash: &str,
    report: &str,
    is_ancestor: &dyn Fn(&str, &str) -> bool,
) -> Result<(), String> {
    let Some((_, introduced)) = history.iter().find(|(h, _)| h == hash) else {
        return Err(format!("no commit ever held a definition of hash {hash}"));
    };
    if is_ancestor(introduced, report) {
        Ok(())
    } else {
        Err(format!("registered at {introduced}, which is not an ancestor of the report's {report}"))
    }
}

/// A definition's hash, blind to the order of its keys and to its layout: its values as sorted JSON, FNV-1a.
///
/// # Errors
/// When the text does not parse.
pub fn canonical_hash(text: &str) -> Result<String, String> {
    let value: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let canonical = serde_json::to_string(&json).map_err(|e| e.to_string())?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in canonical.bytes() {
        let product = (u128::from(h ^ u64::from(b)) * 0x0100_0000_01b3) & u128::from(u64::MAX);
        h = u64::try_from(product).map_err(|e| e.to_string())?;
    }
    Ok(format!("{h:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
id = "F01"
version = 1
min_years = 20
suspects = ["TEC"]
[[benchmark]]
statistic = "mean annual growth"
range = [1.0, 3.5]
source = "measured"
source_ref = "a table"
"#;

    #[test]
    fn definition_hash_canonical() {
        let reordered = "version = 1\nid = \"F01\"\nsuspects = [ \"TEC\" ]\nmin_years = 20\n\n[[benchmark]]\nsource_ref = \"a table\"\nsource = \"measured\"\nrange = [1.0, 3.5]\nstatistic = \"mean annual growth\"\n";
        assert_eq!(canonical_hash(GOOD), canonical_hash(reordered));
        assert_ne!(canonical_hash(GOOD), canonical_hash(&GOOD.replace("3.5", "3.6")));
    }

    #[test]
    fn definitions_carry_their_fields() {
        assert!(malformed("data/measure/N3/F01.toml", GOOD).is_empty());
        assert!(!malformed("data/measure/N3/F02.toml", GOOD).is_empty());
        assert_eq!(malformed("data/measure/N3/F01.toml", &GOOD.replace("[1.0, 3.5]", "[3.5, 1.0]")).len(), 1);
        assert_eq!(
            malformed("data/measure/N3/F01.toml", &GOOD.replace("suspects = [\"TEC\"]", "suspects = []")).len(),
            1
        );
    }

    #[test]
    fn ancestry_on_given_commit_graph() {
        // a ← b ← c on one line; d on another.
        let ancestors =
            |x: &str, y: &str| matches!((x, y), ("a", "a" | "b" | "c") | ("b", "b" | "c") | ("c", "c") | ("d", "d"));
        let history = vec![("h1".to_owned(), "a".to_owned()), ("h2".to_owned(), "c".to_owned())];
        assert!(registered_before(&history, "h1", "b", &ancestors).is_ok());
        assert!(registered_before(&history, "h2", "b", &ancestors).is_err());
        assert!(registered_before(&history, "h2", "d", &ancestors).is_err());
        assert!(registered_before(&history, "h9", "c", &ancestors).is_err());
    }

    #[test]
    fn a_new_version_cites_what_it_supersedes() {
        let v = |n| Some(("F01".to_owned(), n));
        assert!(superseded(v(1), v(2), &["defect F01@1"]).is_ok());
        assert!(superseded(v(1), v(2), &["precision F01@1"]).is_ok());
        assert!(superseded(v(1), v(1), &["defect F01@1"]).is_err());
        assert!(superseded(v(1), v(2), &["defect F01@2"]).is_err());
        assert!(superseded(v(1), v(2), &[]).is_err());
    }
}
