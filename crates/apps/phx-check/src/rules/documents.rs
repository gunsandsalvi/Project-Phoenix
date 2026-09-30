use super::Breach;
use crate::docs::{self, KINDS, SCRATCH, SECTIONS, STATUSES, Step, WRITERS};
use crate::workspace::{ARCHITECTURE, PLAN, SPEC, Workspace};

const RULE: &str = "PC-09";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let (listed, steps) = match (docs::architecture_crates(&ws.architecture), docs::steps(&ws.plan)) {
        (Ok(l), Ok(s)) => (l, s),
        (Err(error), _) | (_, Err(error)) => return vec![Breach::new(RULE, PLAN, 1, error)],
    };
    let mut breaches = Vec::new();
    for c in &ws.crates {
        if !listed.contains(&c.name) {
            let message = format!("`{}` is not in the architecture's crate lists", c.name);
            breaches.push(Breach::new(RULE, ARCHITECTURE, 1, message));
        }
        // A system's crate is begun by its system's first step; a crate no step names, one whose system the coverage
        // table records as begun, and a crate of no system were made by steps done and gone from the plan.
        if let Some(step) = steps.iter().find(|s| s.crates.contains(&c.name))
            && !matches!(step.status.as_deref(), Some("building" | "awaiting" | "held" | "done"))
            && begun(&ws.architecture, &c.name) == Some(false)
        {
            let message = format!("`{}` exists before its step {} is building", c.name, step.id);
            breaches.push(Breach::new(RULE, &c.manifest_path(), 1, message));
        }
    }
    let mut building = 0_usize;
    for step in &steps {
        breaches.extend(shape(step));
        if step.status.as_deref() == Some("building") {
            building += 1;
            if building > 1 {
                breaches.push(Breach::new(RULE, PLAN, step.line, format!("{} is a second step building", step.id)));
            }
        }
    }
    breaches.extend(extension_points(&steps));
    match bench_reads(&steps) {
        Ok(found) => breaches.extend(found),
        Err(error) => breaches.push(Breach::new(RULE, PLAN, 1, error)),
    }
    match edge_cases(ws, &steps) {
        Ok(found) => breaches.extend(found),
        Err(error) => breaches.push(Breach::new(RULE, PLAN, 1, error)),
    }
    match docs::clauses(&ws.spec) {
        Ok(clauses) => breaches.extend(clause_forms(&steps, &clauses)),
        Err(error) => breaches.push(Breach::new(RULE, SPEC, 1, error)),
    }
    breaches.extend(texts(ws));
    breaches
}

/// Every standing step's **Clauses** line in its one form, each clause with the type its own bullet declares.
fn clause_forms(steps: &[Step], clauses: &[docs::Clause]) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for step in steps.iter().filter(|s| s.status.as_deref() != Some("retired")) {
        let items = match docs::clauses_form(&step.clauses) {
            Ok(items) => items,
            Err(error) => {
                breaches.push(Breach::new(RULE, PLAN, step.line, format!("{}'s Clauses: {error}", step.id)));
                continue;
            }
        };
        for item in items.iter().filter(|i| !i.id.starts_with("Law ")) {
            let spec = clauses.iter().find(|c| docs::clause_id(&c.system, c.number) == item.id);
            let message = match (spec, &item.kind) {
                (None, _) => format!("{}'s Clauses name {}, which is not a clause of the spec", step.id, item.id),
                (Some(c), Some(kind)) if c.kind.as_ref() != Some(kind) => format!(
                    "{}'s Clauses list {} as {kind}; the spec lists it as {}",
                    step.id,
                    item.id,
                    c.kind.as_deref().unwrap_or("retired")
                ),
                _ => continue,
            };
            breaches.push(Breach::new(RULE, PLAN, step.line, message));
        }
    }
    breaches
}

/// Every standing step that changes code ends on the fast checks and the bench's read of the budget: a base, kernel
/// or index its `-F` measure, a gate the gate's `-g` run (its own or the gate template's), any other its bench run; a
/// `docs` step changes no code and reads nothing.
fn bench_reads(steps: &[Step]) -> Result<Vec<Breach>, String> {
    let re = |p: &str| regex::Regex::new(p).map_err(|e| e.to_string());
    let (fast, measure, gate, bench) =
        (re(r"(?i)fast checks|fmt`?, `?clippy")?, re(r"-F\b")?, re(r"-g\b|§2\.24's Done when")?, re(r"\bbench\b")?);
    let mut breaches = Vec::new();
    for step in steps.iter().filter(|s| s.status.as_deref() != Some("retired")) {
        let kind = step.kind.as_deref().unwrap_or_default();
        if kind == "docs" {
            continue;
        }
        let done = step.done_when.split_whitespace().collect::<Vec<_>>().join(" ");
        let (read, wanted) = match kind {
            "base" | "kernel" | "index" => (measure.is_match(&done), "its `tools/bench.sh -F` measure"),
            "gate" => (gate.is_match(&done), "the gate's `tools/bench.sh -g` run"),
            _ => (bench.is_match(&done), "the bench's run"),
        };
        if !read {
            let message = format!("{} ({kind}) is done without {wanted}", step.id);
            breaches.push(Breach::new(RULE, PLAN, step.line, message));
        }
        if kind != "gate" && !fast.is_match(&done) {
            let message = format!("{} ({kind}) is done without the fast checks", step.id);
            breaches.push(Breach::new(RULE, PLAN, step.line, message));
        }
    }
    Ok(breaches)
}

/// What an edge case may cite as its evidence: the workspace's tests already written and the live checks its code
/// already runs or a step lists.
struct Evidence {
    functions: std::collections::BTreeSet<String>,
    live_checks: std::collections::BTreeSet<String>,
}

/// Every edge case of a standing step names its evidence: a test its unit tests list (or, where they say they hold the
/// edge cases' tests, any test it names), a test another step it names lists, a test the workspace already holds, a
/// live check, a `-F` case its budget names, or `n/a:` with the reason.
fn edge_cases(ws: &Workspace, steps: &[Step]) -> Result<Vec<Breach>, String> {
    let re = |p: &str| regex::Regex::new(p).map_err(|e| e.to_string());
    let (name, check, function, reason, step_id, by_edge) = (
        re(r"`([a-z][a-z0-9_]*)`")?,
        re(r"LC-\d+-\d+")?,
        re(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn ([a-z][a-z0-9_]*)")?,
        re(r"\bn/a(?: here)?:\s*\S")?,
        re(r"\bS\d+\.\d{2,3}\b")?,
        re(r"(?i)edge[ -]cases?'? tests|edge cases:")?,
    );
    let mut evidence = Evidence {
        functions: std::collections::BTreeSet::default(),
        live_checks: std::collections::BTreeSet::default(),
    };
    for text in ws.crates.iter().flat_map(|c| c.sources.iter().map(|s| s.text.as_str())) {
        evidence.functions.extend(function.captures_iter(text).filter_map(|c| c.get(1)).map(|m| m.as_str().to_owned()));
        evidence.live_checks.extend(check.find_iter(text).map(|m| m.as_str().to_owned()));
    }
    for step in steps {
        evidence.live_checks.extend(check.find_iter(&step.live_checks).map(|m| m.as_str().to_owned()));
    }
    let names = |text: &str| -> Vec<String> {
        name.captures_iter(text).filter_map(|c| c.get(1)).map(|m| m.as_str().to_owned()).collect()
    };
    let lists = |step: &Step, test: &str| {
        names(&step.unit_tests).iter().any(|n| n == test)
            || (by_edge.is_match(&step.unit_tests)
                && step.edge_items.iter().any(|i| names(i).iter().any(|n| n == test)))
    };
    let mut breaches = Vec::new();
    for step in steps.iter().filter(|s| s.status.as_deref() != Some("retired")) {
        for item in &step.edge_items {
            if item.trim().is_empty() || reason.is_match(item) {
                continue;
            }
            let tests = names(item);
            let checks: Vec<&str> = check.find_iter(item).map(|m| m.as_str()).collect();
            let others: Vec<&Step> =
                step_id.find_iter(item).filter_map(|m| steps.iter().find(|s| s.id == m.as_str())).collect();
            let shown = tests.iter().any(|t| {
                lists(step, t)
                    || evidence.functions.contains(t)
                    || others.iter().any(|o| lists(o, t))
                    || (item.contains("-F") && names(&step.budget).contains(t))
            }) || checks.iter().any(|c| evidence.live_checks.contains(*c));
            if !shown {
                let quoted: String = item.chars().take(60).collect();
                let message =
                    format!("{}'s edge case `{quoted}` names no test, live check or `n/a:` reason it has", step.id);
                breaches.push(Breach::new(RULE, PLAN, step.line, message));
            }
        }
    }
    Ok(breaches)
}

/// The kinds of step whose Extension points are the index of the later steps that depend on them.
const INDEXED: &[&str] = &["base", "kernel", "index"];

/// A base's Extension points name exactly the later steps whose Depends on names it; a step done and gone from the
/// plan is neither read nor required.
fn extension_points(steps: &[Step]) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let place = |id: &str| steps.iter().position(|s| s.id == id);
    for (at, base) in steps.iter().enumerate() {
        if !base.kind.as_deref().is_some_and(|k| INDEXED.contains(&k)) || base.status.as_deref() == Some("retired") {
            continue;
        }
        for dependent in steps.iter().filter(|s| s.depends.contains(&base.id) && !base.extends.contains(&s.id)) {
            let message = format!("{}'s Extension points omit {}, which depends on it", base.id, dependent.id);
            breaches.push(Breach::new(RULE, PLAN, base.line, message));
        }
        for named in &base.extends {
            let refused = match place(named).and_then(|p| steps.get(p).map(|s| (p, s))) {
                None => Some("which is not a step of the plan"),
                Some((p, _)) if p <= at => Some("which does not come after it"),
                Some((_, s)) if !s.depends.contains(&base.id) => Some("which does not depend on it"),
                Some(_) => None,
            };
            if let Some(why) = refused {
                let message = format!("{}'s Extension points name {named}, {why}", base.id);
                breaches.push(Breach::new(RULE, PLAN, base.line, message));
            }
        }
    }
    breaches
}

/// The documents name only what a builder has: no working paper, no writer's name for a range of steps; and a step
/// cited as completing a clause or retiring a placeholder is the one that does.
fn texts(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for (path, text) in [(SPEC, &ws.spec), (ARCHITECTURE, &ws.architecture), (PLAN, &ws.plan)] {
        if text.trim().is_empty() {
            breaches.push(Breach::new(RULE, path, 1, "the document is missing or empty"));
            continue;
        }
        let words: Vec<&str> =
            if path == PLAN { SCRATCH.iter().chain(WRITERS).copied().collect() } else { SCRATCH.to_vec() };
        for (line, word) in docs::whole_words(text, &words) {
            breaches.push(Breach::new(
                RULE,
                path,
                line,
                format!("`{word}` names a working paper of the plan's writing"),
            ));
        }
    }
    match (
        docs::map(&ws.plan),
        docs::completion_citations(&ws.plan),
        docs::retirer_citations(&ws.plan),
        docs::step_texts(&ws.plan),
    ) {
        (Ok(map), Ok(completions), Ok(retirers), Ok(steps)) => {
            breaches.extend(completions_match(&map, &completions));
            breaches.extend(retirers_name(&steps, &retirers));
        }
        (Err(error), ..) | (_, Err(error), ..) | (_, _, Err(error), _) | (.., Err(error)) => {
            breaches.push(Breach::new(RULE, PLAN, 1, error));
        }
    }
    breaches
}

/// Each clause cited as completed at steps that the clause map does not give it to.
fn completions_match(map: &[docs::MapRow], cited: &[(usize, String, Vec<String>)]) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for (line, clause, steps) in cited {
        let completing =
            map.iter().find(|r| r.numbers.iter().any(|n| docs::clause_id(&r.system, *n) == *clause)).map(|r| &r.step);
        if completing.is_none_or(|s| !steps.contains(s)) {
            let at = completing.map_or("no step", String::as_str);
            let message =
                format!("{clause} is cited as completed at {}; the clause map completes it at {at}", steps.join(", "));
            breaches.push(Breach::new(RULE, PLAN, *line, message));
        }
    }
    breaches
}

/// Each step cited as retiring placeholders whose own text names none of them; a step done and gone is not read.
fn retirers_name(
    steps: &std::collections::BTreeMap<String, String>,
    cited: &[(usize, String, Vec<String>)],
) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for (line, step, names) in cited {
        if let Some(text) = steps.get(step)
            && !names.iter().any(|n| text.contains(n.as_str()))
        {
            let message = format!("{step} is cited as retiring {} and its text names none of them", names.join(", "));
            breaches.push(Breach::new(RULE, PLAN, *line, message));
        }
    }
    breaches
}

/// Whether the architecture's coverage table records the system of a crate as building or done; `None` for a crate
/// of no system.
fn begun(architecture: &str, name: &str) -> Option<bool> {
    let named = format!("`{name}`");
    let mut in_section = false;
    architecture.lines().find_map(|line| {
        if line.starts_with("## ") {
            in_section = line.starts_with("## 19. ");
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        (in_section && cells.get(3).is_some_and(|c| c.starts_with(&named)))
            .then(|| cells.get(6).is_some_and(|s| matches!(*s, "building" | "done")))
    })
}

/// A step has a known status and, unless retired, a known kind and every section in order.
fn shape(step: &Step) -> Option<Breach> {
    let status = step.status.as_deref().unwrap_or("");
    if !STATUSES.contains(&status) {
        return Some(Breach::new(RULE, PLAN, step.line, format!("{} has no valid status", step.id)));
    }
    if status != "retired" && step.kind.as_deref().is_none_or(|k| !KINDS.contains(&k)) {
        return Some(Breach::new(RULE, PLAN, step.line, format!("{} has no valid kind", step.id)));
    }
    if status == "retired" || step.sections.iter().map(String::as_str).eq(SECTIONS.iter().copied()) {
        return None;
    }
    let missing: Vec<&str> = SECTIONS.iter().copied().filter(|s| !step.sections.iter().any(|x| x == s)).collect();
    let message = if missing.is_empty() {
        format!("{}'s sections are out of order or repeated", step.id)
    } else {
        format!("{} lacks {}", step.id, missing.join(", "))
    };
    Some(Breach::new(RULE, PLAN, step.line, message))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::run;
    use crate::docs::SECTIONS;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    const ARCH: &str = "## 3. Layers and crates\n`phx-check` `phx-core` `phx-cli` `sys-abc` `sys-def`\n## 4. Next\n`phx-cli`\n\
                        ## 19. Coverage\n| Spec | System | Crate | First | Complete | Status |\n| --- | --- | --- | --- | --- | --- |\n\
                        | A1 | ABC | `sys-abc` | 1 | 1 | planned |\n| A2 | DEF | `sys-def` | 0 | 1 | building |\n";

    fn step(id: &str, status: &str, skip: &str, crate_path: &str) -> String {
        let mut text = format!("### {id} — x\n\n");
        for s in SECTIONS.iter().filter(|s| **s != skip) {
            if *s == "Status" {
                let _ = writeln!(text, "**Status**: {status}");
            } else if *s == "Kind" {
                let _ = writeln!(text, "**Kind**: mechanism");
            } else if *s == "Clauses" {
                let _ = writeln!(text, "**Clauses**: none");
            } else if *s == "Done when" {
                let _ = writeln!(text, "**Done when**: the fast checks pass; the bench's run reads the budget.");
            } else {
                let _ = writeln!(text, "**{s}**");
            }
            if *s == "Files" {
                let _ = writeln!(text, "| `{crate_path}` | x |");
            }
        }
        text
    }

    fn ws(plan: String, crates: &[&str]) -> Workspace {
        let mut ws = Workspace::new(crates.iter().map(|c| krate(c, Layer::Apps)).collect());
        ws.architecture = ARCH.to_owned();
        ws.spec = "- **ABC.1 STATE** — a\n".to_owned();
        ws.plan = plan;
        ws
    }

    #[test]
    fn docs_subset_rule() {
        let plan = step("S0.01", "building", "", "crates/apps/phx-check/Cargo.toml")
            + &step("S0.02", "planned", "", "crates/systems/sys-abc/src/lib.rs")
            + &step("S0.03", "planned", "", "crates/systems/sys-def/src/lib.rs")
            + &step("S0.04", "planned", "", "crates/apps/phx-cli/src/main.rs");
        assert!(run(&ws(plan.clone(), &["phx-check"])).is_empty());
        let breaches = run(&ws(plan, &["phx-check", "sys-abc", "sys-def", "phx-cli", "phx-x"]));
        let messages: Vec<&str> = breaches.iter().map(|b| b.message.as_str()).collect();
        assert_eq!(
            messages,
            ["`sys-abc` exists before its step S0.02 is building", "`phx-x` is not in the architecture's crate lists"],
            "a begun system's crate and a crate of no system are done work's"
        );
    }

    #[test]
    fn docs_refuse_missing_section() {
        let plan = step("S0.01", "planned", "Budget", "x");
        let breaches = run(&ws(plan, &[]));
        assert_eq!(breaches.first().map(|b| b.message.as_str()), Some("S0.01 lacks Budget"));
    }

    #[test]
    fn docs_refuse_a_missing_or_unknown_kind() {
        let missing = step("S1.140", "planned", "Kind", "x");
        let unknown = step("S1.141", "planned", "", "x").replace("**Kind**: mechanism", "**Kind**: widget");
        let breaches = run(&ws(missing + &unknown, &[]));
        let messages: Vec<&str> = breaches.iter().map(|b| b.message.as_str()).collect();
        assert_eq!(messages, ["S1.140 has no valid kind", "S1.141 has no valid kind"]);
    }

    #[test]
    fn docs_refuse_two_building() {
        let plan = step("S0.01", "building", "", "x") + &step("S0.02", "building", "", "y");
        assert_eq!(run(&ws(plan, &[])).len(), 1);
    }

    #[test]
    fn docs_accept_a_step_awaiting_the_owner_beside_one_building() {
        let plan = step("S0.01", "awaiting owner", "", "x") + &step("S0.02", "building", "", "y");
        assert!(run(&ws(plan, &[])).is_empty(), "a step awaiting the owner is not building");
    }

    #[test]
    fn docs_accept_a_held_step_with_its_crate_beside_one_building() {
        let plan = step("S0.01", "building", "", "x") + &step("S1.01", "held", "", "crates/apps/phx-core/src/lib.rs");
        assert!(run(&ws(plan, &["phx-core"])).is_empty(), "a held step is not building and keeps its crate");
    }

    #[test]
    fn docs_accept_retired_step_with_status_only() {
        let plan = "### S7.03 — x\n\n**Status**: retired. The world runs once.\n".to_owned();
        assert!(run(&ws(plan, &[])).is_empty());
    }

    fn messages(ws: &Workspace) -> Vec<String> {
        run(ws).into_iter().map(|b| b.message).collect()
    }

    #[test]
    fn scratch_names_are_refused() {
        let mut w = ws(String::new(), &[]);
        let mut plan = "### S1.01 — x\n".to_owned();
        for n in crate::docs::SCRATCH.iter().chain(crate::docs::WRITERS) {
            let _ = writeln!(plan, "cites {n} here");
        }
        w.plan = plan;
        let found = messages(&w);
        let refused = found.iter().filter(|m| m.contains("names a working paper")).count();
        assert_eq!(refused, crate::docs::SCRATCH.len() + crate::docs::WRITERS.len(), "{found:?}");
        w.plan = "### S1.01 — x\n".to_owned();
        w.spec = "## C1. MKT — market forms\n".to_owned();
        assert!(
            messages(&w).iter().all(|m| !m.contains("working paper")),
            "a writer's id is refused in the plan alone"
        );
    }

    #[test]
    fn scratch_words_are_whole() {
        let mut w = ws(String::new(), &[]);
        w.plan = "### S1.01 — x\nthe S2accrual and the kind catalogue, BRIEFLY, C10\n".to_owned();
        assert!(messages(&w).iter().all(|m| !m.contains("working paper")));
    }

    #[test]
    fn a_missing_document_is_refused() {
        let mut w = ws(String::new(), &[]);
        w.spec = String::new();
        assert!(messages(&w).contains(&"the document is missing or empty".to_owned()));
    }

    #[test]
    fn completion_citation_matches_map() {
        let mut w = ws(String::new(), &[]);
        w.plan = "### S1.01 — x\nIt carries ABC.1 (done at S0.02) and ABC.2, ABC.3 (completed at S2.01, S2.02); ABC.4 (done at\n\
                  earlier steps).\n## 13. The clause map\n| ABC | S0.02 | 1, 3 |\n| ABC | S2.02 | 2 |"
            .to_owned();
        let found: Vec<String> = messages(&w).into_iter().filter(|m| m.contains("is cited as completed")).collect();
        assert_eq!(found, ["ABC.3 is cited as completed at S2.01, S2.02; the clause map completes it at S0.02"]);
    }

    #[test]
    fn retirer_names_its_placeholder() {
        let mut w = ws(String::new(), &[]);
        w.plan = "### S1.01 — a\n| Placeholder | Introduced | Retired by |\n| --- | --- | --- |\n\
                  | `ABC.held` (placeholder:ABC) | S1.01 | S2.01 (the rest at S2.03) |\n| `ABC.flat` | S1.01 | S2.02 |\n\
                  **Extension points**: S2.02 (retires `ABC.late`).\n\
                  ### S2.01 — b\nRetires `ABC.held`.\n### S2.02 — c\nReads the market.\n"
            .to_owned();
        let found: Vec<String> = messages(&w).into_iter().filter(|m| m.contains("is cited as retiring")).collect();
        assert_eq!(
            found,
            [
                "S2.02 is cited as retiring ABC.flat and its text names none of them",
                "S2.02 is cited as retiring ABC.late and its text names none of them"
            ],
            "a retirer in parentheses is text"
        );
    }

    fn indexed(id: &str, kind: &str, depends: &str, extends: &str) -> String {
        step(id, "planned", "", "x")
            .replace("**Kind**: mechanism", &format!("**Kind**: {kind}"))
            .replace("**Depends on**", &format!("**Depends on**: {depends}"))
            .replace("**Extension points**", &format!("**Extension points**: {extends}"))
    }

    fn reverse_index(plan: String) -> Vec<String> {
        messages(&ws(plan, &[])).into_iter().filter(|m| m.contains("Extension points")).collect()
    }

    #[test]
    fn a_dependent_missing_from_extension_points_is_refused() {
        let plan = indexed("S1.160", "base", "none", "S1.170 (reads it)")
            + &indexed("S1.170", "mechanism", "S1.160", "none")
            + &indexed("S1.180", "mechanism", "S1.160 (its rows)", "none");
        assert_eq!(reverse_index(plan), ["S1.160's Extension points omit S1.180, which depends on it"]);
    }

    #[test]
    fn an_extension_point_not_depending_back_is_refused() {
        let plan = indexed("S1.150", "mechanism", "S1.160", "none")
            + &indexed("S1.160", "kernel", "none", "S1.150 (x); S1.170 (y); S1.199 (z)")
            + &indexed("S1.170", "mechanism", "none", "none");
        assert_eq!(
            reverse_index(plan),
            [
                "S1.160's Extension points name S1.150, which does not come after it",
                "S1.160's Extension points name S1.170, which does not depend on it",
                "S1.160's Extension points name S1.199, which is not a step of the plan"
            ]
        );
    }

    #[test]
    fn a_base_with_no_dependent_says_so() {
        let plan = indexed("S1.160", "index", "none", "none yet") + &indexed("S1.170", "mechanism", "none", "S1.160");
        assert!(reverse_index(plan).is_empty(), "a mechanism's Extension points are not an index");
    }

    #[test]
    fn done_steps_are_not_read() {
        let plan = indexed("S1.160", "base", "S1.05 (done)", "S1.170 (x)")
            + &indexed("S1.170", "mechanism", "S1.160, S1.05", "none");
        assert!(reverse_index(plan).is_empty(), "a done step gone from the plan is neither a base nor a dependent");
    }

    fn form(clauses: &str) -> Vec<String> {
        let mut w = ws(step("S1.140", "planned", "", "x").replace("**Clauses**: none", clauses), &[]);
        w.spec = "- **ABC.1 STATE** — a\n- **ABC.2 PROCESS** — b\n- **ABC.3** — _Retired_: c\n".to_owned();
        messages(&w).into_iter().filter(|m| m.contains("Clauses")).collect()
    }

    #[test]
    fn clauses_items_are_read() {
        let items = crate::docs::clauses_form(
            "**Clauses**: ABC.1 STATE; ABC.2 PROCESS *(part: sales, from (zone, class))*;\nLaw 3 *(part)*; L3; N8 *(part)*",
        )
        .unwrap();
        let read: Vec<(&str, Option<&str>, bool)> =
            items.iter().map(|i| (i.id.as_str(), i.kind.as_deref(), i.part)).collect();
        assert_eq!(
            read,
            [
                ("ABC.1", Some("STATE"), false),
                ("ABC.2", Some("PROCESS"), true),
                ("Law 3", None, true),
                ("L3", None, false),
                ("N8", None, true)
            ],
            "a wrapped line reads as one"
        );
    }

    #[test]
    fn prose_in_clauses_is_refused() {
        for text in
            ["**Clauses**: carries ABC.1", "**Clauses**:\n- ABC.1 STATE", "**Clauses**: ABC.1", "**Clauses**: Law 3"]
        {
            assert_eq!(form(text).len(), 1, "{text}");
        }
    }

    #[test]
    fn type_must_match_the_spec() {
        assert_eq!(
            form("**Clauses**: ABC.1 PROCESS"),
            ["S1.140's Clauses list ABC.1 as PROCESS; the spec lists it as STATE"]
        );
    }

    #[test]
    fn unknown_clause_is_refused() {
        assert_eq!(
            form("**Clauses**: ABC.9 STATE; N4"),
            [
                "S1.140's Clauses name ABC.9, which is not a clause of the spec",
                "S1.140's Clauses name N4, which is not a clause of the spec"
            ]
        );
    }

    #[test]
    fn none_is_a_form() {
        assert!(form("**Clauses**: none").is_empty());
        assert_eq!(form("**Clauses**: none; ABC.1 STATE").len(), 1, "none stands alone");
    }

    fn edged(id: &str, edge: &str, tests: &str, checks: &str) -> String {
        step(id, "planned", "", "x")
            .replace("**Edge cases**", &format!("**Edge cases**:\n{edge}"))
            .replace("**Unit tests**", &format!("**Unit tests**: {tests}"))
            .replace("**Live checks**", &format!("**Live checks**: {checks}"))
    }

    fn edge_breaches(w: &Workspace) -> Vec<String> {
        messages(w).into_iter().filter(|m| m.contains("edge case")).collect()
    }

    #[test]
    fn edge_items_need_evidence() {
        let plan = edged(
            "S1.140",
            "- E1: many at once — `many_at_once`.\n- E2: none at all, and\n  nothing more.",
            "`many_at_once`",
            "none",
        );
        assert_eq!(
            edge_breaches(&ws(plan, &[])),
            [
                "S1.140's edge case `E2: none at all, and nothing more.` names no test, live check or `n/a:` reason it has"
            ]
        );
        let unlisted = edged("S1.140", "- E1: many — `many_at_once`.", "`other`", "none");
        assert_eq!(edge_breaches(&ws(unlisted, &[])).len(), 1, "a named test its unit tests lack");
        let by_edge = edged("S1.140", "- E1: many — `many_at_once`.", "the edge cases' tests, and `other`", "none");
        assert!(edge_breaches(&ws(by_edge, &[])).is_empty(), "unit tests that hold the edge cases' tests");
    }

    #[test]
    fn n_a_needs_a_reason() {
        let bare = edged("S1.140", "- E5: a heavy day — n/a:", "none", "none");
        assert_eq!(edge_breaches(&ws(bare, &[])).len(), 1);
        let reasoned = edged("S1.140", "- E5: a heavy day — n/a: a gate reads its run.", "none", "none");
        assert!(edge_breaches(&ws(reasoned, &[])).is_empty());
    }

    #[test]
    fn edge_tests_are_the_steps_own() {
        let owner = edged("S1.130", "- E6: n/a: none.", "`saves_round_trip`", "none");
        let silent = edged("S1.140", "- E6: round-trips — `saves_round_trip`.", "none", "none");
        assert_eq!(edge_breaches(&ws(owner.clone() + &silent, &[])).len(), 1, "another step's test, that step unnamed");
        let named = edged("S1.140", "- E6: round-trips — `saves_round_trip` (S1.130).", "none", "none");
        assert!(edge_breaches(&ws(owner + &named, &[])).is_empty(), "the step named lists it");
        let mut kept = ws(edged("S1.140", "- E8: stops — `stops_the_caller` (kept).", "none", "none"), &[]);
        kept.crates.push(with_source(
            krate("phx-exec", Layer::Kernel),
            "src/pool.rs",
            "#[test]\nfn stops_the_caller() {}",
        ));
        assert!(edge_breaches(&kept).is_empty(), "a test the workspace holds");
    }

    #[test]
    fn live_check_ids_count() {
        let plan =
            edged("S1.140", "- E6: a save mid-run — LC-0-35.\n- E9: growth — LC-1-99.", "none", "`LC-0-35`: saves");
        assert_eq!(
            edge_breaches(&ws(plan, &[])),
            ["S1.140's edge case `E9: growth — LC-1-99.` names no test, live check or `n/a:` reason it has"]
        );
    }

    fn bench(kind: &str, done: &str) -> Vec<String> {
        let plan =
            step("S1.140", "planned", "", "x").replace("**Kind**: mechanism", &format!("**Kind**: {kind}")).replace(
                "**Done when**: the fast checks pass; the bench's run reads the budget.",
                &format!("**Done when**:\n{done}"),
            );
        messages(&ws(plan, &[])).into_iter().filter(|m| m.contains("is done without")).collect()
    }

    #[test]
    fn bench_read_by_kind() {
        assert!(
            bench("mechanism", "- [ ] the fast checks pass.\n- [ ] The bench's run at fewer persons passes.")
                .is_empty()
        );
        assert!(bench("repair", "- [ ] fmt, clippy, tests, `phx-check`; `tools/bench.sh` within budget.").is_empty());
        assert_eq!(bench("data", "- [ ] the fast\n  checks pass."), ["S1.140 (data) is done without the bench's run"]);
        assert_eq!(
            bench("tool", "- [ ] the bench at the committed resolution."),
            ["S1.140 (tool) is done without the fast checks"]
        );
    }

    #[test]
    fn docs_steps_are_exempt() {
        assert!(bench("docs", "- [ ] §6 restated; the two reviews done.").is_empty());
    }

    #[test]
    fn bases_need_their_f_measure() {
        assert_eq!(
            bench("base", "- [ ] the fast checks pass.\n- [ ] the bench at the committed resolution."),
            ["S1.140 (base) is done without its `tools/bench.sh -F` measure"]
        );
        assert!(
            bench("index", "- [ ] the fast checks pass; `tools/bench.sh -F stalls` within `fin.stalls.*`.").is_empty()
        );
    }

    #[test]
    fn gates_need_g() {
        assert_eq!(
            bench("gate", "- [ ] the bench at the committed resolution."),
            ["S1.140 (gate) is done without the gate's `tools/bench.sh -g` run"]
        );
        assert!(
            bench("gate", "- [ ] §2.24's Done when, with the heavy days measured.").is_empty(),
            "the gate template's run"
        );
    }
}
