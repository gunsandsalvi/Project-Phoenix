use regex::Regex;

/// A clause of the world's document: a bullet whose bold opening is a system code, a number and a type, or a code
/// and a number followed by `_Retired_`.
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
    "Clauses",
    "Architecture",
    "Depends on",
    "Goal",
    "Files",
    "Design",
    "Unit tests",
    "Live checks",
    "Budget",
    "Guards",
    "Not allowed",
    "Done when",
];

pub const STATUSES: &[&str] = &["planned", "building", "done", "retired"];

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
    Ok(found)
}

pub fn steps(plan: &str) -> Result<Vec<Step>, String> {
    let heading = regex(r"^### (S(\d+)\.\d{2}) — ")?;
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
            if *known == "Status" {
                let rest = line.trim_start_matches("**Status**").trim_start_matches([':', ' ']);
                let word: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
                step.status = Some(word);
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

pub fn map(plan: &str) -> Result<Vec<MapRow>, String> {
    let row = regex(r"^\| ([A-Z]{2,4}) \| (S(\d+)\.\d{2}) \| ([^|]*) \|")?;
    let digits = regex(r"\d+")?;
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
        let numbers = digits.find_iter(list.as_str()).filter_map(|m| m.as_str().parse().ok()).collect();
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
    use super::{Clause, clauses, map, steps};

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
