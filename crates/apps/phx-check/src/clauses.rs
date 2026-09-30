use std::collections::BTreeSet;

use crate::docs::{self, Clause, MapRow, Step};
use crate::rules::Breach;
use crate::workspace::{PLAN, SPEC, Workspace};

const RULE: &str = "clauses";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    match (docs::clauses(&ws.spec), docs::map(&ws.plan), docs::steps(&ws.plan)) {
        (Ok(clauses), Ok(map), Ok(steps)) => {
            let mut breaches = check(&clauses, &map);
            breaches.extend(listed(&steps, &map));
            // A done step leaves the plan, so a step the map names that the plan no longer holds is done.
            let done: Vec<&str> = map
                .iter()
                .map(|r| r.step.as_str())
                .filter(|id| steps.iter().find(|s| s.id == *id).is_none_or(|s| s.status.as_deref() == Some("done")))
                .collect();
            let texts = ws
                .crates
                .iter()
                .flat_map(|c| c.sources.iter().filter(|s| !s.is_test_or_bench()).map(|s| s.text.as_str()))
                .chain(ws.data.iter().map(|(_, text)| text.as_str()));
            match carriers(texts) {
                Ok(carried) => breaches.extend(uncarried(&map, &done, &carried)),
                Err(error) => breaches.push(Breach::new(RULE, PLAN, 1, error)),
            }
            breaches
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => vec![Breach::new(RULE, PLAN, 1, error)],
    }
}

/// Every clause the code or data names as carried: in a `#[clause(..)]` attribute, a declaration's `clause` field, a
/// data file's `clause` key, or a contract's `violation!(clause = ..)`.
pub fn carriers<'a>(texts: impl Iterator<Item = &'a str>) -> Result<std::collections::BTreeSet<String>, String> {
    let re = |p: &str| regex::Regex::new(p).map_err(|e| e.to_string());
    let (attribute, field, id) =
        (re(r"#\[clause\(([^)]*)\)\]")?, re(r#"clause\s*[:=]\s*"([A-Z]{2,4}\.\d+)""#)?, re(r"[A-Z]{2,4}\.\d+")?);
    let mut carried = std::collections::BTreeSet::new();
    for text in texts {
        for caps in attribute.captures_iter(text) {
            if let Some(list) = caps.get(1) {
                carried.extend(id.find_iter(list.as_str()).map(|m| m.as_str().to_owned()));
            }
        }
        carried.extend(field.captures_iter(text).filter_map(|c| c.get(1)).map(|m| m.as_str().to_owned()));
    }
    Ok(carried)
}

/// Each clause a done step completes that nothing carries.
pub fn uncarried(map: &[MapRow], done: &[&str], carried: &std::collections::BTreeSet<String>) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for row in map.iter().filter(|r| done.contains(&r.step.as_str())) {
        for n in &row.numbers {
            let id = format!("{}.{n}", row.system);
            if !carried.contains(&id) {
                let message = format!("{id} is completed by {}, which is done, and nothing carries it", row.step);
                breaches.push(Breach::new(RULE, PLAN, row.line, message));
            }
        }
    }
    breaches
}

/// Each step still standing lists as complete exactly the clauses the map gives it.
pub fn listed(steps: &[Step], map: &[MapRow]) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for step in steps.iter().filter(|s| s.status.as_deref() != Some("retired")) {
        let completed = match docs::completed_clauses(&step.clauses) {
            Ok(c) => c,
            Err(error) => {
                breaches.push(Breach::new(RULE, PLAN, step.line, error));
                continue;
            }
        };
        let mapped: BTreeSet<String> = map
            .iter()
            .filter(|r| r.step == step.id)
            .flat_map(|r| r.numbers.iter().map(|n| format!("{}.{n}", r.system)))
            .collect();
        for id in completed.difference(&mapped) {
            let message =
                format!("{} lists {id} as complete, and the clause map does not give it to the step", step.id);
            breaches.push(Breach::new(RULE, PLAN, step.line, message));
        }
        for id in mapped.difference(&completed) {
            let message = format!("the clause map gives {id} to {}, which does not list it as complete", step.id);
            breaches.push(Breach::new(RULE, PLAN, step.line, message));
        }
    }
    breaches
}

/// Every live clause is completed by exactly one row of the map, and no row names a retired or unknown clause.
pub fn check(clauses: &[Clause], map: &[MapRow]) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for row in map {
        for n in &row.numbers {
            let key = (row.system.as_str(), *n);
            let id = format!("{}.{n}", row.system);
            match clauses.iter().find(|c| (c.system.as_str(), c.number) == key) {
                None => breaches.push(Breach::new(RULE, PLAN, row.line, format!("{id} is not a clause"))),
                Some(c) if c.retired => {
                    breaches.push(Breach::new(RULE, PLAN, row.line, format!("{id} is retired and mapped")));
                }
                Some(_) if seen.contains(&key) => {
                    breaches.push(Breach::new(RULE, PLAN, row.line, format!("{id} is completed by two steps")));
                }
                Some(_) => seen.push(key),
            }
        }
    }
    for c in clauses.iter().filter(|c| !c.retired) {
        if !seen.contains(&(c.system.as_str(), c.number)) {
            let message = format!("{}.{} is in no row of the clause map", c.system, c.number);
            breaches.push(Breach::new(RULE, SPEC, c.line, message));
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use super::{carriers, check, listed, uncarried};
    use crate::docs::{clauses, map, steps};

    #[test]
    fn a_steps_complete_clauses_match_its_map_rows() {
        let plan = "## 4. Stage 1\n### S1.140 — a\n**Status**: planned\n**Clauses**:\n- STATE: ABC.1, ABC.2 *(part)*.\n\
                    - PROCESS: DEF.3.\n### S1.141 — b\n**Status**: retired. Dealt out.\n**Clauses**: ABC.9.\n\
                    ## 13. The clause map\n| ABC | S1.140 | 1, 4 |\n| ABC | S1.141 | 5 |";
        let breaches = listed(&steps(plan).unwrap(), &map(plan).unwrap());
        let found: Vec<(usize, &str)> = breaches.iter().map(|b| (b.line, b.message.as_str())).collect();
        assert_eq!(
            found,
            vec![
                (2, "S1.140 lists DEF.3 as complete, and the clause map does not give it to the step"),
                (2, "the clause map gives ABC.4 to S1.140, which does not list it as complete"),
            ],
            "a part is not complete, and a retired step is not read"
        );
    }

    #[test]
    fn a_done_steps_clauses_need_a_carrier() {
        let code = [
            r#"#[clause("ABC.1", "Law 6")] fn a() {} violation!(clause = "ABC.2", "x");"#,
            r#"declare_family! { F = "f" { clause: "ABC.3" } }"#,
            "clause = \"ABC.5\"",
        ];
        let carried = carriers(code.into_iter()).unwrap();
        let plan = "## 13. The clause map\n| ABC | S0.01 | 1, 2, 3, 4, 5 |\n| ABC | S0.02 | 6 |";
        let breaches = uncarried(&map(plan).unwrap(), &["S0.01"], &carried);
        let messages: Vec<&str> = breaches.iter().map(|b| b.message.as_str()).collect();
        assert_eq!(messages, vec!["ABC.4 is completed by S0.01, which is done, and nothing carries it"]);
    }

    #[test]
    fn clauses_refuse_unmapped_twice_and_retired() {
        let spec = "- **ABC.1 STATE** — a\n- **ABC.2 STATE** — b\n- **ABC.3** — _Retired_: c\n- **ABC.4 STATE** — d";
        let plan = "## 13. The clause map\n| ABC | S0.01 | 1, 3 |\n| ABC | S0.02 | 1, 4, 9 |";
        let breaches = check(&clauses(spec).unwrap(), &map(plan).unwrap());
        let messages: Vec<&str> = breaches.iter().map(|b| b.message.as_str()).collect();
        assert_eq!(
            messages,
            vec![
                "ABC.3 is retired and mapped",
                "ABC.1 is completed by two steps",
                "ABC.9 is not a clause",
                "ABC.2 is in no row of the clause map",
            ]
        );
    }
}
