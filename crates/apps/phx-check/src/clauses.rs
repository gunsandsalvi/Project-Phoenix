use crate::docs::{self, Clause, MapRow};
use crate::rules::Breach;
use crate::workspace::{PLAN, SPEC, Workspace};

const RULE: &str = "clauses";

pub fn run(ws: &Workspace) -> Vec<Breach> {
    match (docs::clauses(&ws.spec), docs::map(&ws.plan)) {
        (Ok(clauses), Ok(map)) => check(&clauses, &map),
        (Err(error), _) | (_, Err(error)) => vec![Breach::new(RULE, PLAN, 1, error)],
    }
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
    use super::check;
    use crate::docs::{clauses, map};

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
