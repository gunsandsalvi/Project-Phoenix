use super::Breach;
use crate::docs::{self, SECTIONS, STATUSES, Step};
use crate::workspace::{ARCHITECTURE, PLAN, Workspace};

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

/// A step has a known status and, unless retired, every section in order.
fn shape(step: &Step) -> Option<Breach> {
    let status = step.status.as_deref().unwrap_or("");
    if !STATUSES.contains(&status) {
        return Some(Breach::new(RULE, PLAN, step.line, format!("{} has no valid status", step.id)));
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
}
