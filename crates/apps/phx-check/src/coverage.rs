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
    derive(&ws.architecture, &docs::steps(&ws.plan)?, &docs::map(&ws.plan)?, &docs::build_order(&ws.plan)?)
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

/// What a row of the table is keyed by: a system of the clause map, or the chain and measurement items it lists.
enum Key {
    System(String),
    Items(Vec<(String, u32)>),
}

impl Key {
    fn of(cells: &[String]) -> Option<Self> {
        match docs::items_of(cells.first()?) {
            Some(items) => Some(Self::Items(items)),
            None => Some(Self::System(cells.get(1)?.clone())),
        }
    }

    fn covers(&self, row: &MapRow) -> bool {
        match self {
            Self::System(system) => row.system == *system,
            Self::Items(items) => row.numbers.iter().any(|n| items.contains(&(row.system.clone(), *n))),
        }
    }

    fn named_by(&self, named: &[String]) -> bool {
        match self {
            Self::System(system) => named.contains(system),
            Self::Items(items) => items.iter().any(|(system, n)| named.contains(&docs::clause_id(system, *n))),
        }
    }
}

pub fn derive(architecture: &str, steps: &[Step], map: &[MapRow], order: &[u32]) -> Result<Coverage, String> {
    let mention = Regex::new(r"\b(?:([A-Z]{2,4})\.\d+|([LN])(\d+))").map_err(|e| e.to_string())?;
    let named: Vec<(&Step, Vec<String>)> = steps
        .iter()
        .map(|s| {
            let names = mention
                .captures_iter(&s.clauses)
                .filter_map(|c| match (c.get(1), c.get(2), c.get(3)) {
                    (Some(system), _, _) => Some(system.as_str().to_owned()),
                    (None, Some(item), Some(n)) => Some(format!("{}{}", item.as_str(), n.as_str())),
                    _ => None,
                })
                .collect();
            (s, names)
        })
        .collect();
    let rank = |stage: u32, what: &str| {
        order
            .iter()
            .position(|s| *s == stage)
            .ok_or_else(|| format!("{what}'s stage {stage} is not in the build table"))
    };
    for step in steps {
        rank(step.stage, &step.id)?;
    }
    for row in map {
        rank(row.stage, &row.step)?;
    }
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
                && let Some(row) = derive_row(line, &named, map, order)
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

/// A row keyed by a system of the clause map or by chain and measurement items, with its stages and status derived;
/// `None` for other rows and for a key no row of the map completes. Stages are ranked by the build order, which the
/// caller has checked names every stage.
fn derive_row(line: &str, named: &[(&Step, Vec<String>)], map: &[MapRow], order: &[u32]) -> Option<String> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    let mut cells: Vec<String> = inner.split('|').map(|c| c.trim().to_owned()).collect();
    let key = Key::of(&cells)?;
    let rank = |stage: &u32| order.iter().position(|s| s == stage);
    let rows: Vec<&MapRow> = map.iter().filter(|r| key.covers(r)).collect();
    let complete = rows.iter().map(|r| r.stage).fold(None, |best: Option<u32>, s| match best {
        Some(b) if rank(&b) >= rank(&s) => Some(b),
        _ => Some(s),
    })?;
    let naming: Vec<&Step> = named.iter().filter(|(_, names)| key.named_by(names)).map(|(step, _)| *step).collect();
    let first = naming.iter().map(|s| s.stage).chain(rows.iter().map(|r| r.stage)).fold(
        None,
        |best: Option<u32>, s| match best {
            Some(b) if rank(&b) <= rank(&s) => Some(b),
            _ => Some(s),
        },
    )?;
    // A done step leaves the plan, so a completing step the plan no longer holds is done.
    let status_of = |id: &str| match named.iter().find(|(s, _)| s.id == id) {
        Some((s, _)) => s.status.clone(),
        None => Some("done".to_owned()),
    };
    let completing: Vec<Option<String>> = rows.iter().map(|r| status_of(&r.step)).collect();
    let involved = naming.iter().map(|s| s.status.clone()).chain(completing.iter().cloned());
    // Done steps leave the plan, so what the table recorded of them stands: a system's first stage never moves later
    // in the build and a system begun is never planned again.
    let recorded = cells.get(5).map(String::as_str);
    let status = if completing.iter().all(|s| s.as_deref() == Some("done")) {
        "done"
    } else if matches!(recorded, Some("building" | "done"))
        || involved.into_iter().any(|s| matches!(s.as_deref(), Some("building" | "awaiting" | "held" | "done")))
    {
        "building"
    } else {
        "planned"
    };
    let first = match cells.get(3).and_then(|c| c.parse::<u32>().ok()) {
        Some(was) if rank(&was).is_some_and(|w| rank(&first).is_some_and(|f| w < f)) => was,
        _ => first,
    };
    for (index, value) in [(3, first.to_string()), (4, complete.to_string()), (5, status.to_owned())] {
        *cells.get_mut(index)? = value;
    }
    Some(format!("| {} |", cells.join(" | ")))
}

#[cfg(test)]
mod tests {
    use super::derive;
    use crate::docs::{build_order, map, steps};

    const ORDER: &[u32] = &[0, 1, 2, 3, 8, 4, 5, 6, 7];
    const PLAN: &str = "### S0.02 — a\n**Status**: done\n**Clauses**:\n- ABC.1 *(part)*.\n\
                        ### S2.01 — b\n**Status**: planned\n**Clauses**:\n- ABC.1.\n\
                        ## 13. The clause map\n| ABC | S2.01 | 1 |";
    const ARCH: &str = "## 19. Coverage\n\n| Spec | System | Crate | First | Complete | Status |\n\
                        | --- | --- | --- | --- | --- | --- |\n\
                        | A1 | ABC | `phx-x` | 1 | 1 | planned |\n| L3 | estates | `sys-est` | 0 | 2 | planned |\n";

    fn derived(arch: &str, plan: &str) -> super::Coverage {
        derive(arch, &steps(plan).unwrap(), &map(plan).unwrap(), ORDER).unwrap()
    }

    #[test]
    fn coverage_derives_stages_and_status() {
        let c = derived(ARCH, PLAN);
        assert!(c.text.contains("| A1 | ABC | `phx-x` | 0 | 2 | building |"));
        assert!(c.text.contains("| L3 | estates | `sys-est` | 0 | 2 | planned |"), "no L row in the map: kept");
        assert_eq!(c.changed, vec![5]);
        assert!(derived(&c.text, PLAN).changed.is_empty());
    }

    #[test]
    fn coverage_orders_stages_by_the_build_table() {
        let plan = "### S8.101 — a\n**Status**: planned\n**Clauses**: ABC.1\n\
                    ### S5.101 — b\n**Status**: planned\n**Clauses**: ABC.2\n\
                    ## 13. The clause map\n| ABC | S8.101 | 1 |\n| ABC | S5.101 | 2 |";
        let arch = "## 19. Coverage\n\n| S | Y | C | F | K | St |\n| --- |\n| A1 | ABC | `x` | 8 | 8 | planned |\n";
        assert!(derived(arch, plan).text.contains("| A1 | ABC | `x` | 8 | 5 | planned |"), "Stage 5 builds after 8");
    }

    #[test]
    fn build_order_reads_repeated_stage_rows_once() {
        let plan = "## 1. The build\n\n| Stage | Range |\n| --- | --- |\n| **0 F** | S0.01 |\n| **1 A** | x |\n\
                    | **1 B** | x |\n| **1 C** | x |\n| **1 D** | x |\n| **3 M** | x |\n| **8 Minds** | x |\n## 2. Next";
        assert_eq!(build_order(plan).unwrap(), vec![0, 1, 3, 8]);
        assert!(build_order("## 1. The build\n\ntext\n").is_err(), "no stage is refused");
    }

    #[test]
    fn l_and_n_rows_are_derived() {
        let plan = "### S7.103 — chains\n**Status**: planned\n**Clauses**: N4; L1–L12\n\
                    ## 13. The clause map\n| L | S7.103 | 1–12 |\n| N | S7.103 | 4 |\n| N | S0.26 | 2, 8 |";
        let arch = "## 19. Coverage\n\n| S | Y | C | F | K | St |\n| --- |\n\
                    | L1, L2, L4–L12 | chains | `x` | 2 | 2 | planned |\n| N2–N8 | measurement | `y` | 1 | 1 | planned |\n";
        let text = derived(arch, plan).text;
        assert!(text.contains("| L1, L2, L4–L12 | chains | `x` | 2 | 7 | planned |"), "{text}");
        assert!(text.contains("| N2–N8 | measurement | `y` | 0 | 7 | building |"), "{text}");
    }

    #[test]
    fn an_unknown_stage_is_refused() {
        let plan =
            "### S9.101 — a\n**Status**: planned\n**Clauses**: ABC.1\n## 13. The clause map\n| ABC | S9.101 | 1 |";
        let refused = derive(ARCH, &steps(plan).unwrap(), &map(plan).unwrap(), ORDER).unwrap_err();
        assert!(refused.contains("S9.101's stage 9 is not in the build table"), "{refused}");
    }

    #[test]
    fn a_system_unnamed_is_kept() {
        let arch = "## 19. Coverage\n\n| S | Y | C | F | K | St |\n| --- |\n| Z1 | ZZZ | `z` | 3 | 4 | planned |\n";
        assert!(derived(arch, PLAN).changed.is_empty());
    }
}
