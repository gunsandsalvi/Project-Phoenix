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
        match steps.iter().find(|s| s.crates.contains(&c.name)) {
            None => breaches.push(Breach::new(RULE, &c.manifest_path(), 1, "no step creates this crate")),
            Some(step) if !matches!(step.status.as_deref(), Some("building" | "awaiting" | "done")) => {
                let message = format!("`{}` exists before its step {} is building", c.name, step.id);
                breaches.push(Breach::new(RULE, &c.manifest_path(), 1, message));
            }
            Some(_) => {}
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

    const ARCH: &str = "## 3. Layers and crates\n`phx-check` `phx-core`\n## 4. Next\n`phx-cli`\n";

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
            + &step("S0.02", "planned", "", "crates/apps/phx-cli/src/main.rs");
        assert!(run(&ws(plan.clone(), &["phx-check"])).is_empty());
        let breaches = run(&ws(plan, &["phx-check", "phx-cli"]));
        assert_eq!(breaches.len(), 2, "{breaches:?}");
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
    fn docs_accept_retired_step_with_status_only() {
        let plan = "### S7.03 — x\n\n**Status**: retired. The world runs once.\n".to_owned();
        assert!(run(&ws(plan, &[])).is_empty());
    }
}
