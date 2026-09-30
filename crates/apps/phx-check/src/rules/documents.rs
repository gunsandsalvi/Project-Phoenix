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
    breaches.extend(texts(ws));
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
    use crate::workspace::fixture::krate;
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
}
