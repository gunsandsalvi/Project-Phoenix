//! No tuning: from this rule's registration on, every primitive, opening distribution or rule form a commit adds,
//! changes or removes is cited by a trailer saying where its value comes from, and no citation names a result.

use std::collections::BTreeMap;

use regex::Regex;

use super::Breach;
use crate::git::{Git, trailers};
use crate::workspace::Workspace;

const RULE: &str = "PC-91";
/// The rule's own file: the commit that added it is the registration.
const REGISTRATION: &str = "crates/apps/phx-check/src/rules/no_tuning.rs";
const DATA: &str = "data";
/// Where the budget's measurements are, which a resolution change may cite.
const BUDGET_REPORTS: &[&str] = &["perf/device/", "perf/measure/"];
/// A resolution changed by the owner cites the decision's row in the plan's owner decisions instead.
const OWNER: &str = "plan §12, ";
const PLAN: &str = "docs/IMPLEMENTATION.md";
/// A later commit's trailer naming an earlier commit whose citations it carries.
const CITED_FOR: &str = "Cited-For";
/// The fewest hex digits a cited commit is named by.
const SHORT_HASH: usize = 7;
/// What no citation may name: a realism or chain result, a finding, or a measure's definition.
const RESULTS: &[&str] = &["perf/realism", "perf/chains", "data/measure"];

/// One entry of the register as a data file declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: String,
    pub value: String,
    pub source_ref: String,
}

/// The entries of the world's data by (file, id).
pub type Dump = BTreeMap<(String, String), Entry>;

/// An entry that differs: its (file, id), and the entry before and after, either absent.
pub type Change<'a> = (&'a (String, String), Option<&'a Entry>, Option<&'a Entry>);

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let git = Git::at(&ws.root);
    let fail = |e: String| vec![Breach::new(RULE, REGISTRATION, 1, e)];
    if let Err(e) = git.complete() {
        return fail(e);
    }
    let registered = match git.added(REGISTRATION) {
        Ok(Some(c)) => c,
        Ok(None) => return Vec::new(),
        Err(e) => return fail(e),
    };
    let commits = match git.since(&registered) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let mut later: Vec<(String, String)> = Vec::new();
    for commit in &commits {
        let Ok(message) = git.message(commit) else { continue };
        for cited in trailers(&message, CITED_FOR) {
            later.push((cited.to_owned(), message.clone()));
        }
    }
    let mut breaches = Vec::new();
    for commit in commits {
        match commit_breaches(&git, &commit, &later) {
            Ok(found) => breaches.extend(found.into_iter().map(|m| Breach::new(RULE, DATA, 1, m))),
            Err(e) => breaches.extend(fail(e)),
        }
    }
    breaches
}

fn commit_breaches(git: &Git, commit: &str, later: &[(String, String)]) -> Result<Vec<String>, String> {
    let touched: Vec<String> =
        git.changed(commit, DATA)?.into_iter().map(|t| t.path).filter(|p| is_register_file(p)).collect();
    if touched.is_empty() {
        return Ok(Vec::new());
    }
    let parent = git.parent(commit);
    let mut before = Dump::new();
    let mut after = Dump::new();
    for path in &touched {
        if let Some(p) = &parent
            && let Some(text) = git.show(p, path)
        {
            before.extend(dump(path, &text)?);
        }
        if let Some(text) = git.show(commit, path) {
            after.extend(dump(path, &text)?);
        }
    }
    let mut message = git.message(commit)?;
    for (cited, carried) in later {
        if cited.len() >= SHORT_HASH && commit.starts_with(cited.as_str()) {
            message.push('\n');
            message.push_str(carried);
        }
    }
    let short: String = commit.chars().take(12).collect();
    let plan = git.show(commit, PLAN).unwrap_or_default();
    let exists = |report: &str| match report.strip_prefix(OWNER) {
        Some(decision) => plan.lines().any(|l| l.starts_with(&format!("| {decision}"))),
        None => git.show(commit, report).is_some(),
    };
    Ok(judge(&before, &after, &message, &exists).into_iter().map(|m| format!("{short}: {m}")).collect())
}

/// The data files the register reads: the world's, the shared ones, and the levels' templates and openings.
fn is_register_file(path: &str) -> bool {
    std::path::Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("toml"))
        && (path == "data/world.toml" || path.starts_with("data/shared/") || path.starts_with("data/profiles/"))
}

/// A data file's primitives and forms by id.
///
/// # Errors
/// When the file does not parse.
pub fn dump(path: &str, text: &str) -> Result<Dump, String> {
    let table: toml::Table = toml::from_str(text).map_err(|e| format!("{path}: {e}"))?;
    let mut out = Dump::new();
    let entries = |key: &str| table.get(key).and_then(toml::Value::as_array).cloned().unwrap_or_default();
    for e in entries("primitive") {
        let field = |k: &str| e.get(k).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned));
        let Some(id) = field("id") else { continue };
        let kind = field("kind").unwrap_or_default();
        let mut value = field("value").unwrap_or_default();
        for extra in ["unit", "period", "decided_by", "shape"] {
            if let Some(v) = field(extra) {
                value = format!("{value} {extra}={v}");
            }
        }
        out.insert((path.to_owned(), id), Entry { kind, value, source_ref: field("source_ref").unwrap_or_default() });
    }
    for f in entries("form") {
        let field = |k: &str| f.get(k).and_then(toml::Value::as_str).map(str::to_owned);
        let Some(id) = field("id") else { continue };
        let value = field("reason").unwrap_or_default();
        out.insert(
            (path.to_owned(), id),
            Entry { kind: "FORM".to_owned(), value, source_ref: field("source_ref").unwrap_or_default() },
        );
    }
    Ok(out)
}

/// What differs between two dumps, by (file, id): the entry before and after, either absent.
pub fn diff<'a>(before: &'a Dump, after: &'a Dump) -> Vec<Change<'a>> {
    let mut keys: Vec<&(String, String)> = before.keys().chain(after.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter_map(|k| {
            let (b, a) = (before.get(k), after.get(k));
            (b != a).then_some((k, b, a))
        })
        .collect()
}

/// Every refusal of one commit's changes under its message's trailers.
pub fn judge(before: &Dump, after: &Dump, message: &str, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut refusals = Vec::new();
    let changes = trailers(message, "Primitive-Change");
    let fixes = trailers(message, "Transcription-Fix");
    let resolutions = trailers(message, "Resolution-Change");
    let rename_pairs: Vec<(&str, &str)> = trailers(message, "Primitive-Rename")
        .into_iter()
        .filter_map(|t| t.split_once(" → "))
        .map(|(a, b)| (a.trim(), b.trim()))
        .collect();
    let finding = Regex::new(r"\bF-\d{3}\b").ok();
    for t in changes.iter().chain(&fixes).chain(&resolutions) {
        if RESULTS.iter().any(|r| t.contains(r)) || finding.as_ref().is_some_and(|f| f.is_match(t)) {
            refusals.push(format!("a citation names a result, a finding or a measure: `{t}`"));
        }
    }
    let cites = |list: &[&str], id: &str, source_ref: &str| list.iter().any(|t| *t == format!("{id} — {source_ref}"));
    let is_rename = |(path, id): &(String, String), entry: &Entry, removed: bool| {
        rename_pairs.iter().any(|(old, new)| {
            let (mine, other) = if removed { (*old, *new) } else { (*new, *old) };
            let twin = (path.clone(), other.to_owned());
            let other_entry = if removed { after.get(&twin) } else { before.get(&twin) };
            id == mine && other_entry == Some(entry)
        })
    };
    for (key, b, a) in diff(before, after) {
        let (_, id) = key;
        let Some(entry) = a.or(b) else { continue };
        // A resolution's first value and its retirement are cited like any entry; changing it coarsens or refines.
        if entry.kind == "RESOLUTION" && b.is_some() && a.is_some() {
            let reported = resolutions
                .iter()
                .filter_map(|t| t.split_once(" — "))
                .any(|(i, report)| {
                    i == id
                        && (BUDGET_REPORTS.iter().any(|p| report.starts_with(p)) || report.starts_with(OWNER))
                        && exists(report)
                });
            if !reported {
                refusals.push(format!("`{id}` is a resolution changed without `Resolution-Change: {id} — <report>`"));
            }
            continue;
        }
        match (b, a) {
            (Some(old), None) if is_rename(key, old, true) => {}
            (None, Some(new)) if is_rename(key, new, false) => {}
            (Some(old), Some(new)) if old.value != new.value && old.source_ref == new.source_ref => {
                if !cites(&fixes, id, &new.source_ref) {
                    refusals.push(format!(
                        "`{id}`'s value changed under its old source; cite a new source, or `Transcription-Fix: {id} — <source_ref>`"
                    ));
                }
            }
            _ => {
                if !cites(&changes, id, &entry.source_ref) {
                    refusals.push(format!("`{id}` changed without `Primitive-Change: {id} — <its source_ref>`"));
                }
            }
        }
    }
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(entries: &[(&str, &str, &str, &str)]) -> Dump {
        let text = entries
            .iter()
            .map(|(id, kind, value, src)| {
                format!("[[primitive]]\nid = \"{id}\"\nkind = \"{kind}\"\nowner = \"X\"\nsource = \"measured\"\nsource_ref = \"{src}\"\nvalue = {value}\n")
            })
            .fold(String::new(), |mut all, e| {
                all.push_str(&e);
                all
            });
        dump("data/shared/X.toml", &text).unwrap_or_default()
    }

    const NONE: &dyn Fn(&str) -> bool = &|_| false;

    #[test]
    fn registry_diff_by_id() {
        let before =
            file(&[("X.a", "TECHNOLOGY", "1", "s1"), ("X.b", "TECHNOLOGY", "2", "s2"), ("X.old", "POLICY", "3", "s3")]);
        let after = file(&[
            ("X.a", "TECHNOLOGY", "1", "s1"),
            ("X.b", "TECHNOLOGY", "5", "s9"),
            ("X.new", "POLICY", "3", "s3"),
            ("X.c", "SHAPE", "4", "s4"),
        ]);
        let ids: Vec<(&str, bool, bool)> =
            diff(&before, &after).into_iter().map(|(k, b, a)| (k.1.as_str(), b.is_some(), a.is_some())).collect();
        assert_eq!(
            ids,
            vec![("X.b", true, true), ("X.c", false, true), ("X.new", false, true), ("X.old", true, false)]
        );
        let message =
            "Why\n\nPrimitive-Change: X.b — s9\nPrimitive-Change: X.c — s4\nPrimitive-Rename: X.old → X.new\n";
        assert_eq!(judge(&before, &after, message, NONE), Vec::<String>::new());
        assert_eq!(judge(&before, &after, "Primitive-Change: X.b — s9\n", NONE).len(), 3);
    }

    #[test]
    fn value_change_needs_new_source_or_transcription_fix() {
        let before = file(&[("X.a", "TECHNOLOGY", "1", "table 3")]);
        let same_source = file(&[("X.a", "TECHNOLOGY", "2", "table 3")]);
        assert_eq!(judge(&before, &same_source, "Primitive-Change: X.a — table 3", NONE).len(), 1);
        assert!(judge(&before, &same_source, "Transcription-Fix: X.a — table 3", NONE).is_empty());
        let new_source = file(&[("X.a", "TECHNOLOGY", "2", "table 4, 2025 edition")]);
        assert!(judge(&before, &new_source, "Primitive-Change: X.a — table 4, 2025 edition", NONE).is_empty());
    }

    #[test]
    fn trailers_refuse_result_citations() {
        let before = file(&[("X.a", "TECHNOLOGY", "1", "s")]);
        let after = file(&[("X.a", "TECHNOLOGY", "2", "perf/realism/F01/r1.json")]);
        let refused = judge(&before, &after, "Primitive-Change: X.a — perf/realism/F01/r1.json", NONE);
        assert!(refused.iter().any(|r| r.contains("names a result")));
        let after = file(&[("X.a", "TECHNOLOGY", "2", "as F-012 showed")]);
        assert!(!judge(&before, &after, "Primitive-Change: X.a — as F-012 showed", NONE).is_empty());
        let before = file(&[("X.r", "RESOLUTION", "256", "s")]);
        let after = file(&[("X.r", "RESOLUTION", "128", "s")]);
        let device = |p: &str| p == "perf/device/abc.json";
        assert!(judge(&before, &after, "Resolution-Change: X.r — perf/device/abc.json", &device).is_empty());
        assert_eq!(judge(&before, &after, "Resolution-Change: X.r — perf/realism/F01/r.json", &device).len(), 2);
        assert!(judge(&Dump::new(), &after, "Primitive-Change: X.r — s", NONE).is_empty());
        let decided = |p: &str| p == "plan §12, The factor";
        assert!(judge(&before, &after, "Resolution-Change: X.r — plan §12, The factor", &decided).is_empty());
        assert_eq!(judge(&before, &after, "Resolution-Change: X.r — plan §12, No such row", &decided).len(), 1);
    }
}
