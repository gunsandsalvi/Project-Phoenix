use regex::Regex;

use crate::docs::{self, MapRow, Step};
use crate::rules::Breach;
use crate::workspace::{ARCHITECTURE, Workspace};

const RULE: &str = "coverage";

/// The architecture text with its coverage table derived, and the lines of the rows that changed.
#[derive(Debug)]
pub struct Coverage {
    pub text: String,
    pub changed: Vec<usize>,
}

pub fn derive_from(ws: &Workspace) -> Result<Coverage, String> {
    derive(&ws.architecture, &docs::steps(&ws.plan)?, &docs::map(&ws.plan)?)
}

pub fn check(ws: &Workspace) -> Vec<Breach> {
    match derive_from(ws) {
        Ok(c) => c
            .changed
            .into_iter()
            .map(|line| Breach::new(RULE, ARCHITECTURE, line, "differs from the table `coverage --write` derives"))
            .collect(),
        Err(error) => vec![Breach::new(RULE, ARCHITECTURE, 1, error)],
    }
}

pub fn derive(architecture: &str, steps: &[Step], map: &[MapRow]) -> Result<Coverage, String> {
    let mention = Regex::new(r"\b([A-Z]{2,4})\.\d+").map_err(|e| e.to_string())?;
    let named: Vec<(&Step, Vec<String>)> = steps
        .iter()
        .map(|s| {
            (s, mention.captures_iter(&s.clauses).filter_map(|c| c.get(1)).map(|m| m.as_str().to_owned()).collect())
        })
        .collect();
    let mut lines: Vec<String> = Vec::new();
    let mut changed = Vec::new();
    let mut in_section = false;
    let mut table_rows = 0_usize;
    for (index, line) in architecture.lines().enumerate() {
        if line.starts_with("## ") {
            in_section = line.starts_with("## 19. ");
        }
        let mut out = line.to_owned();
        if in_section && line.starts_with('|') {
            table_rows += 1;
            if table_rows > 2
                && let Some(row) = derive_row(line, &named, map)
                && row != line
            {
                changed.push(index + 1);
                out = row;
            }
        }
        lines.push(out);
    }
    let mut text = lines.join("\n");
    if architecture.ends_with('\n') {
        text.push('\n');
    }
    Ok(Coverage { text, changed })
}

/// A row keyed by a system of the clause map, with its stages and status derived; `None` for other rows.
fn derive_row(line: &str, named: &[(&Step, Vec<String>)], map: &[MapRow]) -> Option<String> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    let mut cells: Vec<String> = inner.split('|').map(|c| c.trim().to_owned()).collect();
    let system = cells.get(1)?.clone();
    let rows: Vec<&MapRow> = map.iter().filter(|r| r.system == system).collect();
    let mut completions: Vec<u32> = rows.iter().map(|r| r.stage).collect();
    completions.sort_unstable();
    let complete = *completions.last()?;
    let naming: Vec<&Step> =
        named.iter().filter(|(_, systems)| systems.contains(&system)).map(|(step, _)| *step).collect();
    let mut stages: Vec<u32> = naming.iter().map(|s| s.stage).chain(completions).collect();
    stages.sort_unstable();
    let first = *stages.first()?;
    let status_of = |id: &str| named.iter().find(|(s, _)| s.id == id).and_then(|(s, _)| s.status.clone());
    let completing: Vec<Option<String>> = rows.iter().map(|r| status_of(&r.step)).collect();
    let involved = naming.iter().map(|s| s.status.clone()).chain(completing.iter().cloned());
    let status = if completing.iter().all(|s| s.as_deref() == Some("done")) {
        "done"
    } else if involved.into_iter().any(|s| matches!(s.as_deref(), Some("building" | "awaiting" | "held" | "done"))) {
        "building"
    } else {
        "planned"
    };
    for (index, value) in [(3, first.to_string()), (4, complete.to_string()), (5, status.to_owned())] {
        *cells.get_mut(index)? = value;
    }
    Some(format!("| {} |", cells.join(" | ")))
}

#[cfg(test)]
mod tests {
    use super::derive;
    use crate::docs::{map, steps};

    const PLAN: &str = "### S0.02 — a\n**Status**: done\n**Clauses**:\n- ABC.1 *(part)*.\n\
                        ### S2.01 — b\n**Status**: planned\n**Clauses**:\n- ABC.1.\n\
                        ## 13. The clause map\n| ABC | S2.01 | 1 |";
    const ARCH: &str = "## 19. Coverage\n\n| Spec | System | Crate | First | Complete | Status |\n\
                        | --- | --- | --- | --- | --- | --- |\n\
                        | A1 | ABC | `phx-x` | 1 | 1 | planned |\n| L3 | estates | `sys-est` | 0 | 2 | planned |\n";

    #[test]
    fn coverage_derives_stages_and_status() {
        let c = derive(ARCH, &steps(PLAN).unwrap(), &map(PLAN).unwrap()).unwrap();
        assert!(c.text.contains("| A1 | ABC | `phx-x` | 0 | 2 | building |"));
        assert!(c.text.contains("| L3 | estates | `sys-est` | 0 | 2 | planned |"));
        assert_eq!(c.changed, vec![5]);
        let again = derive(&c.text, &steps(PLAN).unwrap(), &map(PLAN).unwrap()).unwrap();
        assert!(again.changed.is_empty());
    }
}
