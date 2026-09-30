use regex::Regex;

/// A clause of the world's document: a bullet whose bold opening is a system code, a number and a type, or a code
/// and a number followed by `_Retired_`; or an item of the transmission chains (system `L`) or of measurement
/// (system `N`), a heading of its part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub system: String,
    pub number: u32,
    pub retired: bool,
    pub line: usize,
}

/// A step of the plan, from its heading to the next step or top-level heading.
#[derive(Debug, Clone)]
pub struct Step {
    pub id: String,
    pub stage: u32,
    pub line: usize,
    pub status: Option<String>,
    /// The first word of its **Kind** section.
    pub kind: Option<String>,
    pub sections: Vec<String>,
    /// The text of its **Clauses** section.
    pub clauses: String,
    /// The crates its **Files** table names by full path, in order.
    pub crates: Vec<String>,
}

/// A row of the clause map: the step that completes the listed clauses of one system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapRow {
    pub system: String,
    pub step: String,
    pub stage: u32,
    pub numbers: Vec<u32>,
    pub line: usize,
}

/// The sections every step has, in this order.
pub const SECTIONS: &[&str] = &[
    "Status",
    "Kind",
    "Clauses",
    "Architecture",
    "Depends on",
    "Goal",
    "Files",
    "Design",
    "Edge cases",
    "Extension points",
    "Unit tests",
    "Live checks",
    "Budget",
    "Guards",
    "Not allowed",
    "Done when",
];

/// A step's statuses; `awaiting` (written `awaiting owner`) is a step whose remaining items are the owner's alone, and
/// `held` a step begun and set aside while an earlier one, reopened, is building.
pub const STATUSES: &[&str] = &["planned", "building", "awaiting", "held", "done", "retired"];

/// What a step builds; every step but a retired one names one.
pub const KINDS: &[&str] =
    &["base", "kernel", "index", "migration", "mechanism", "data", "tool", "repair", "docs", "gate"];

/// The parts whose headings are items, by their heading and their items' system.
const ITEM_PARTS: &[(&str, &str)] = &[("# PART L", "L"), ("# PART N", "N")];

/// How a clause is named: `SYS.n` for a system's clause, `Ln` or `Nn` for a chain or a measurement item.
pub fn clause_id(system: &str, number: u32) -> String {
    if system.len() == 1 { format!("{system}{number}") } else { format!("{system}.{number}") }
}

fn regex(pattern: &str) -> Result<Regex, String> {
    Regex::new(pattern).map_err(|e| e.to_string())
}

fn number<T: std::str::FromStr>(text: Option<regex::Match<'_>>) -> Option<T> {
    text.and_then(|m| m.as_str().parse().ok())
}

pub fn clauses(text: &str) -> Result<Vec<Clause>, String> {
    let re = regex(r"^- \*\*([A-Z]{2,4})\.(\d+)(?: [A-Z]+)?\*\* — (_Retired_)?")?;
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(caps) = re.captures(line) else {
            continue;
        };
        let (Some(system), Some(n)) = (caps.get(1), number(caps.get(2))) else {
            continue;
        };
        found.push(Clause {
            system: system.as_str().to_owned(),
            number: n,
            retired: caps.get(3).is_some(),
            line: index + 1,
        });
    }
    found.extend(part_items(text)?);
    Ok(found)
}

/// The items of the parts whose headings are items: `## L<n>. <title>` within Part L and `## N<n>. <title>` within
/// Part N, each retired when the first line after its heading that is not blank opens with `_Retired_`.
fn part_items(text: &str) -> Result<Vec<Clause>, String> {
    let heading = regex(r"^## ([LN])(\d+)\. ")?;
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut part: Option<&str> = None;
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("# ") {
            part = ITEM_PARTS.iter().find(|(h, _)| line.starts_with(h)).map(|(_, system)| *system);
            continue;
        }
        let Some(caps) = heading.captures(line) else {
            continue;
        };
        let (Some(system), Some(n)) = (caps.get(1).map(|m| m.as_str()), number(caps.get(2))) else {
            continue;
        };
        if part != Some(system) {
            continue;
        }
        let next = lines.iter().skip(index + 1).find(|l| !l.trim().is_empty());
        found.push(Clause {
            system: system.to_owned(),
            number: n,
            retired: next.is_some_and(|l| l.starts_with("_Retired_")),
            line: index + 1,
        });
    }
    Ok(found)
}

pub fn steps(plan: &str) -> Result<Vec<Step>, String> {
    let heading = regex(r"^### (S(\d+)\.\d{2,3}) — ")?;
    let section = regex(r"^\*\*([A-Za-z ]+)\*\*")?;
    let crate_path = regex(r"crates/(?:foundation|kernel|interfaces|systems|assembly|apps)/([a-z0-9-]+)/")?;
    let mut steps: Vec<Step> = Vec::new();
    let mut open = false;
    let mut current = "";
    for (index, line) in plan.lines().enumerate() {
        if let Some(caps) = heading.captures(line) {
            let (Some(id), Some(stage)) = (caps.get(1), number(caps.get(2))) else {
                continue;
            };
            steps.push(Step {
                id: id.as_str().to_owned(),
                stage,
                line: index + 1,
                status: None,
                kind: None,
                sections: Vec::new(),
                clauses: String::new(),
                crates: Vec::new(),
            });
            open = true;
            current = "";
            continue;
        }
        if line.starts_with("## ") {
            open = false;
        }
        let Some(step) = steps.last_mut().filter(|_| open) else {
            continue;
        };
        if let Some(name) = section.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str())
            && let Some(known) = SECTIONS.iter().find(|s| **s == name)
        {
            step.sections.push((*known).to_owned());
            current = known;
            match *known {
                "Status" => step.status = Some(first_word(line, "**Status**")),
                "Kind" => step.kind = Some(first_word(line, "**Kind**")),
                _ => {}
            }
        }
        match current {
            "Clauses" => {
                step.clauses.push_str(line);
                step.clauses.push('\n');
            }
            "Files" => {
                for caps in crate_path.captures_iter(line) {
                    if let Some(name) = caps.get(1).map(|m| m.as_str().to_owned())
                        && !step.crates.contains(&name)
                    {
                        step.crates.push(name);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(steps)
}

fn first_word(line: &str, heading: &str) -> String {
    let rest = line.trim_start_matches(heading).trim_start_matches([':', ' ']);
    rest.chars().take_while(char::is_ascii_alphabetic).collect()
}

/// An item of a **Clauses** list: its whole text, and its text outside parentheses, where the ids it lists stand.
struct Item {
    whole: String,
    listed: String,
}

/// The items of a **Clauses** text: its bullets, and the inline text before them, each split at the commas and
/// semicolons that stand outside parentheses.
fn items(text: &str) -> Vec<Item> {
    let body = text.trim_start().trim_start_matches("**Clauses**").trim_start_matches(':');
    let mut units = vec![String::new()];
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(bullet) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            units.push(bullet.to_owned());
        } else if let Some(last) = units.last_mut() {
            last.push(' ');
            last.push_str(trimmed);
        }
    }
    let mut found = Vec::new();
    for unit in &units {
        let mut depth = 0_usize;
        let mut item = Item { whole: String::new(), listed: String::new() };
        for c in unit.chars() {
            match c {
                '(' => depth += 1,
                ')' => {
                    if let Some(less) = depth.checked_sub(1) {
                        depth = less;
                    }
                    item.whole.push(c);
                    continue;
                }
                ',' | ';' if depth == 0 => {
                    found.push(std::mem::replace(&mut item, Item { whole: String::new(), listed: String::new() }));
                    continue;
                }
                _ => {}
            }
            item.whole.push(c);
            if depth == 0 {
                item.listed.push(c);
            }
        }
        found.push(item);
    }
    found
}

/// The clause ids a step's **Clauses** text completes: every id an item lists outside its parentheses, unless the
/// item is marked as only starting them. A range of two ids lists every id between. A chain or measurement item is
/// listed bare, a letter and its number; a sub-item, the item's number and a point and another, is text, never
/// the item.
pub fn completed_clauses(text: &str) -> Result<std::collections::BTreeSet<String>, String> {
    let id = regex(r"\b([A-Z]{2,4})\.(\d+)\b(?:\s*[–-]\s*([A-Z]{2,4})\.(\d+)\b)?")?;
    // A chain or measurement item is a letter and its number; one followed by a point and a number is a sub-item.
    let chain = regex(r"\b([LN])(\d+)(\.\d+)?\b(?:\s*[–-]\s*([LN])(\d+)\b)?")?;
    let part = regex(r"\(part\b")?;
    let mut found = std::collections::BTreeSet::new();
    for item in items(text).iter().filter(|i| !part.is_match(&i.whole)) {
        for caps in id.captures_iter(&item.listed) {
            let (Some(system), Some(from)) = (caps.get(1).map(|m| m.as_str()), number::<u32>(caps.get(2))) else {
                continue;
            };
            found.insert(format!("{system}.{from}"));
            let (Some(other), Some(to)) = (caps.get(3).map(|m| m.as_str()), number::<u32>(caps.get(4))) else {
                continue;
            };
            if other == system {
                found.extend((from..=to).map(|n| format!("{system}.{n}")));
            } else {
                found.insert(format!("{other}.{to}"));
            }
        }
        for caps in chain.captures_iter(&item.listed).filter(|c| c.get(3).is_none()) {
            let (Some(system), Some(from)) = (caps.get(1).map(|m| m.as_str()), number::<u32>(caps.get(2))) else {
                continue;
            };
            found.insert(clause_id(system, from));
            let (Some(other), Some(to)) = (caps.get(4).map(|m| m.as_str()), number::<u32>(caps.get(5))) else {
                continue;
            };
            if other == system {
                found.extend((from..=to).map(|n| clause_id(system, n)));
            } else {
                found.insert(clause_id(other, to));
            }
        }
    }
    Ok(found)
}

pub fn map(plan: &str) -> Result<Vec<MapRow>, String> {
    let row = regex(r"^\| ([A-Z]{2,4}|[LN]) \| (S(\d+)\.\d{2,3}) \| ([^|]*) \|")?;
    let mut rows = Vec::new();
    let mut inside = false;
    for (index, line) in plan.lines().enumerate() {
        if line.starts_with("## ") {
            inside = line.starts_with("## 13. The clause map");
            continue;
        }
        if !inside {
            continue;
        }
        let Some(caps) = row.captures(line) else {
            continue;
        };
        let (Some(system), Some(step), Some(stage), Some(list)) =
            (caps.get(1), caps.get(2), number(caps.get(3)), caps.get(4))
        else {
            continue;
        };
        let numbers = numbers(list.as_str()).map_err(|e| format!("the clause map's row at line {}: {e}", index + 1))?;
        rows.push(MapRow {
            system: system.as_str().to_owned(),
            step: step.as_str().to_owned(),
            stage,
            numbers,
            line: index + 1,
        });
    }
    Ok(rows)
}

/// The numbers a row's cell lists: each number, or a range `a–b` (an en dash or a hyphen) standing for a to b.
///
/// # Errors
/// When a range's end is below its start, or an entry is not a number.
fn numbers(cell: &str) -> Result<Vec<u32>, String> {
    let mut found = Vec::new();
    for entry in cell.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let parse = |t: &str| t.trim().parse::<u32>().map_err(|_| format!("`{entry}` is not a number or a range"));
        match entry.split_once(['–', '-']) {
            Some((from, to)) => {
                let (from, to) = (parse(from)?, parse(to)?);
                if to < from {
                    return Err(format!("the range `{entry}` runs backwards"));
                }
                found.extend(from..=to);
            }
            None => found.push(parse(entry)?),
        }
    }
    Ok(found)
}

/// The stages in the order the plan builds them: the section `## 1.`'s table rows, each row's first cell's first
/// number, a stage named by several rows counted once where it first appears.
///
/// # Errors
/// When the table names no stage.
pub fn build_order(plan: &str) -> Result<Vec<u32>, String> {
    let row = regex(r"^\|[^|\d]*(\d+)")?;
    let mut order: Vec<u32> = Vec::new();
    for line in section(plan, "1").lines().filter(|l| l.starts_with('|')) {
        if let Some(stage) = number::<u32>(row.captures(line).and_then(|c| c.get(1)))
            && !order.contains(&stage)
        {
            order.push(stage);
        }
    }
    if order.is_empty() {
        return Err("the plan's build table names no stage".to_owned());
    }
    Ok(order)
}

/// The chain and measurement items a cell lists, each an item or a range of items, comma-separated; `None` when the
/// cell is anything else.
pub fn items_of(cell: &str) -> Option<Vec<(String, u32)>> {
    let mut found = Vec::new();
    for entry in cell.split(',').map(str::trim) {
        let mut ends = entry.split(['–', '-']).map(str::trim);
        let first = ends.next()?;
        let system = first.get(..1).filter(|s| matches!(*s, "L" | "N"))?;
        let from: u32 = first.get(1..)?.parse().ok()?;
        let to = match ends.next() {
            Some(last) => last.strip_prefix(system)?.parse().ok()?,
            None => from,
        };
        if ends.next().is_some() || to < from {
            return None;
        }
        found.extend((from..=to).map(|n| (system.to_owned(), n)));
    }
    Some(found)
}

/// The text of the section headed `## <number>.`, up to the next top-level heading.
pub fn section<'a>(text: &'a str, number: &str) -> &'a str {
    let start = format!("## {number}. ");
    let begin = if text.starts_with(&start) { Some(0) } else { text.find(&format!("\n{start}")).map(|i| i + 1) };
    let Some(rest) = begin.and_then(|b| text.get(b..)) else {
        return "";
    };
    let end = rest.get(1..).and_then(|r| r.find("\n## ")).map_or(rest.len(), |e| e + 1);
    rest.get(..end).unwrap_or(rest)
}

/// The crates the architecture's layer section names.
pub fn architecture_crates(architecture: &str) -> Result<Vec<String>, String> {
    let name = regex(r"\b((?:phx|sys|if)-[a-z]+)\b")?;
    let mut found: Vec<String> = Vec::new();
    for m in name.find_iter(section(architecture, "3")) {
        if !found.iter().any(|f| f == m.as_str()) {
            found.push(m.as_str().to_owned());
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::{Clause, clauses, completed_clauses, map, numbers, steps};

    #[test]
    fn documents_read_three_digit_steps_and_their_kind() {
        let plan = "## 4. Stage 1\n\n### S1.140 — Base\n\n**Status**: planned\n**Kind**: base, the ledger\n\
                    ### S1.1400 — Not a step\n## 13. The clause map\n| ABC | S1.140 | 3 |\n| ABC | S1.1400 | 4 |";
        let s = steps(plan).unwrap();
        assert_eq!(s.len(), 1, "a four-digit number is not a step");
        let first = s.first().unwrap();
        assert_eq!((first.id.as_str(), first.stage, first.kind.as_deref()), ("S1.140", 1, Some("base")));
        let rows = map(plan).unwrap();
        assert_eq!(rows.iter().map(|r| r.step.as_str()).collect::<Vec<_>>(), vec!["S1.140"]);
    }

    #[test]
    fn documents_read_the_clauses_a_step_completes() {
        let text = "**Clauses**: GEN.2 *(part)*, GEN.3; ACC.10–ACC.12 and FRM.17 *(moved from BNK.4)*; N8, Law 3.\n\
                    - STATE: TCR.1 DECISION.\n- PROCESS: BNK.9; BNK.10 *(part: sales, from HH.2)*; HH.7 *(part,\n  \
                    moved)*; HH.13\n  *(completes it, with MON.1)*.\n- FORBID: POP.15 *(part)*, POP.16.";
        let found: Vec<String> = completed_clauses(text).unwrap().into_iter().collect();
        assert_eq!(
            found,
            ["ACC.10", "ACC.11", "ACC.12", "BNK.9", "FRM.17", "GEN.3", "HH.13", "N8", "POP.16", "TCR.1"],
            "parts, ids in parentheses and laws are not completed"
        );
    }

    #[test]
    fn documents_read_l_and_n_items() {
        let spec = "# PART L — TRANSMISSION\n\n## L1. A loss\n\ntext\n## L2. The seller\n# PART N — MEASUREMENT\n\n\
                    ## N6. Experiments\n\n_Retired_: gone.\n## N7. Calibration\n# PART O — STAGES\n## N9. Not an item\n\
                    ## L3. Nor this";
        let found: Vec<(String, u32, bool)> =
            clauses(spec).unwrap().into_iter().map(|c| (c.system, c.number, c.retired)).collect();
        assert_eq!(
            found,
            vec![
                ("L".to_owned(), 1, false),
                ("L".to_owned(), 2, false),
                ("N".to_owned(), 6, true),
                ("N".to_owned(), 7, false)
            ],
            "items only within their own part, N6 retired"
        );
    }

    #[test]
    fn ranges_are_read_and_bounded() {
        assert_eq!(numbers("1–12").unwrap(), (1..=12).collect::<Vec<u32>>());
        assert_eq!(numbers("2, 8, 4-5").unwrap(), vec![2, 8, 4, 5]);
        assert!(numbers("4–2").unwrap_err().contains("runs backwards"));
        let plan = "## 13. The clause map\n| L | S7.103 | 1–12 |\n| N | S0.26 | 12–2 |";
        assert!(map(plan).unwrap_err().contains("line 3"), "a backward range names its row");
    }

    #[test]
    fn completed_clauses_read_l_and_n() {
        let found: Vec<String> = completed_clauses("**Clauses**: L1–L3, N4; N8.8 *(part)*; the sub-item N7.2; Law 3")
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(found, ["L1", "L2", "L3", "N4"], "a sub-item and a law are not items");
    }

    #[test]
    fn documents_read_clauses_steps_and_map() {
        let spec = "- **TIME.11 FORBID** — No second clock.\n- **REP.6** — _Retired_: gone.\n- **N8.1** — no.";
        let found = clauses(spec).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found.get(1), Some(&Clause { system: "REP".to_owned(), number: 6, retired: true, line: 2 }));

        let plan = "## 3. Stage 0\n\n### S0.01 — One\n\n**Status**: retired. Gone.\n**Clauses**:\n- REP.1.\n\
                    **Files**\n| `crates/apps/phx-check/src/main.rs` | x |\n## 13. The clause map\n| REP | S0.01 | 1, 2 |";
        let s = steps(plan).unwrap();
        let first = s.first().unwrap();
        assert_eq!((first.stage, first.status.as_deref()), (0, Some("retired")));
        assert_eq!(first.crates, vec!["phx-check".to_owned()]);
        assert!(first.clauses.contains("REP.1"));
        assert_eq!(map(plan).unwrap().first().map(|r| r.numbers.clone()), Some(vec![1, 2]));
    }
}
