//! The design point's figure set holds together: its tables stand, each total is its parts, and every key the plan
//! cites is in it.

use regex::Regex;
use toml::{Table, Value};

use super::Breach;
use crate::workspace::{DESIGN, PLAN, Workspace};

const RULE: &str = "PC-09";

/// The tables every step and `phx-fin` read.
const TABLES: &[&str] = &[
    "point",
    "phone",
    "store",
    "store.contracts",
    "day.b",
    "day.nb",
    "day.h",
    "day.bc",
    "unit",
    "bytes",
    "stage",
    "fin.decide.additions",
    "fin.fixed",
    "fin.calendar.joins_h",
    "fin.calendar.joins_bprime",
    "fin.calendar.campaign",
    "fin.calendar.mass_default",
    "fin.calendar.events",
    "resolution",
    "ledger",
    "save",
];
/// How far a total may stand from its parts: each figure is written to a hundredth, so rounding alone moves a sum by
/// no more than this.
const ROUNDING: f64 = 0.05;
/// The fixed line that does not grow with the persons, so its shares are its totals.
const UNSCALED: &str = "barriers";
/// A calendar list's keys that are not its items.
const TOTALS: &[&str] = &["sum", "core_ms", "ratchet"];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    if ws.design.trim().is_empty() {
        return vec![Breach::new(RULE, DESIGN, 1, "the design point's figure set is missing or empty")];
    }
    let mut breaches: Vec<Breach> = check(&ws.design).into_iter().map(|m| Breach::new(RULE, DESIGN, 1, m)).collect();
    if let Ok(table) = ws.design.parse::<Table>() {
        breaches.extend(cited(&table, &ws.plan).into_iter().map(|m| Breach::new(RULE, PLAN, 1, m)));
    }
    breaches
}

fn at<'a>(table: &'a Table, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let mut value = table.get(parts.next()?)?;
    for part in parts {
        value = value.get(part)?;
    }
    Some(value)
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Float(f) => Some(*f),
        Value::Integer(i) => i.to_string().parse().ok(),
        _ => None,
    }
}

/// A day's three figures, business, non-business and heavy.
fn triple(value: &Value) -> Option<[f64; 3]> {
    let list = value.as_array()?;
    let mut out = [0.0; 3];
    for (slot, v) in out.iter_mut().zip(list) {
        *slot = number(v)?;
    }
    (list.len() == out.len()).then_some(out)
}

fn apart(a: f64, b: f64) -> bool {
    (a - b).abs() > ROUNDING
}

/// Every refusal of the figure set's own consistency.
pub fn check(text: &str) -> Vec<String> {
    let table = match text.parse::<Table>() {
        Ok(t) => t,
        Err(e) => return vec![format!("the design point's figure set does not parse: {e}")],
    };
    let mut refused: Vec<String> =
        TABLES.iter().filter(|t| at(&table, t).is_none()).map(|t| format!("the table `[{t}]` is missing")).collect();
    if !refused.is_empty() {
        return refused;
    }
    let point = |key: &str| at(&table, &format!("point.{key}")).and_then(number);
    let Some(scale) = point("scale") else {
        return vec!["`[point] scale` is missing".to_owned()];
    };
    if let (Some(persons), Some(steps)) = (point("persons"), point("steps_persons"))
        && apart(persons / steps, scale)
    {
        refused.push(format!("`[point] scale` {scale} is not persons over the steps' persons"));
    }
    if let Some(Value::Table(fixed)) = at(&table, "fin.fixed") {
        for (name, line) in fixed {
            let factor = if name == UNSCALED { 1.0 } else { scale };
            let mut sum = [0.0; 3];
            for share in line.get("shares").and_then(Value::as_table).into_iter().flat_map(|t| t.values()) {
                for (s, v) in sum.iter_mut().zip(triple(share).unwrap_or_default()) {
                    *s += v;
                }
            }
            for (key, s) in ["b_core_ms", "nb_core_ms", "h_core_ms"].into_iter().zip(sum) {
                match line.get(key).and_then(number) {
                    Some(total) if !apart(total, s * factor) => {}
                    total => refused.push(format!("`fin.fixed.{name}.{key}` {total:?} is not its shares' sum × scale")),
                }
            }
        }
    }
    if let Some(Value::Table(calendar)) = at(&table, "fin.calendar") {
        for (name, list) in calendar.iter().filter(|(n, _)| *n != "events") {
            let items: Vec<f64> = list
                .as_table()
                .into_iter()
                .flat_map(|t| t.iter())
                .filter(|(k, _)| !TOTALS.contains(&k.as_str()))
                .filter_map(|(_, v)| number(v))
                .collect();
            let sum = list.get("sum").and_then(number);
            if !items.is_empty() && sum.is_none_or(|s| apart(s, items.iter().sum())) {
                refused.push(format!("`fin.calendar.{name}.sum` is not its items' sum"));
            }
            if sum.zip(list.get("core_ms").and_then(number)).is_none_or(|(s, c)| apart(c, s * scale)) {
                refused.push(format!("`fin.calendar.{name}.core_ms` is not its sum × scale"));
            }
        }
    }
    if let Some(Value::Table(stage)) = at(&table, "stage") {
        let mut sum = [0.0; 3];
        for part in stage.iter().filter(|(k, _)| *k != "day").filter_map(|(_, v)| triple(v)) {
            for (s, v) in sum.iter_mut().zip(part) {
                *s += v;
            }
        }
        if stage.get("day").and_then(triple).is_none_or(|day| day.iter().zip(sum).any(|(d, s)| apart(*d, s))) {
            refused.push("`[stage] day` is not its stages' sum".to_owned());
        }
    }
    if let Some(Value::Table(ledger)) = at(&table, "ledger") {
        let sum: f64 = ledger.values().filter_map(|l| l.get("mb").and_then(number)).sum();
        if ledger.get("total_mb").and_then(number).is_none_or(|t| apart(t, sum)) {
            refused.push("`[ledger] total_mb` is not its lines' sum".to_owned());
        }
    }
    refused
}

/// Every key the plan cites as `` `[table] key` `` or `fin.fixed.<line>` that the figure set lacks.
pub fn cited(table: &Table, plan: &str) -> Vec<String> {
    let mut refused = Vec::new();
    let (Ok(keyed), Ok(line)) = (
        Regex::new(r"`\[((?:store|day|point|phone|unit|bytes|resolution)(?:\.[a-z]+)?)\] ([a-z_0-9]+)`"),
        Regex::new(r"\bfin\.fixed\.([a-z_]+)"),
    ) else {
        return vec!["the citation patterns do not compile".to_owned()];
    };
    for caps in keyed.captures_iter(plan) {
        let (Some(t), Some(k)) = (caps.get(1), caps.get(2)) else {
            continue;
        };
        if at(table, &format!("{}.{}", t.as_str(), k.as_str())).is_none() {
            refused.push(format!("the plan cites `[{}] {}`, which the figure set lacks", t.as_str(), k.as_str()));
        }
    }
    for caps in line.captures_iter(plan) {
        if let Some(name) = caps.get(1)
            && at(table, &format!("fin.fixed.{}", name.as_str())).is_none()
        {
            refused.push(format!("the plan cites `fin.fixed.{}`, which the figure set lacks", name.as_str()));
        }
    }
    refused
}

#[cfg(test)]
mod tests {
    use super::{check, cited};

    const VALID: &str = r#"
[point]
persons = 6_000_000
steps_persons = 7_500_000
scale = 0.8
[phone]
cores = 3.0
[store]
persons = 6_000_000
[store.contracts]
loans = 1_000
[day.b]
sales = 10
[day.nb]
sales = 5
[day.h]
sales = 20
[day.bc]
sales = 12
[unit]
search = 300
[bytes]
household = 174
[stage]
"1 Open" = [1.0, 0.5, 2.0]
"2 Resolve" = [3.0, 0.5, 4.0]
day = [4.0, 1.0, 6.0]
[fin.decide.additions]
"core" = [1.0, 0.0, 1.0]
[fin.fixed.open]
b_core_ms = 0.8
nb_core_ms = 0.4
h_core_ms = 1.6
shares = { "S1.179" = [1, 0.5, 2] }
[fin.fixed.barriers]
b_core_ms = 90
nb_core_ms = 45
h_core_ms = 110
shares = { "S1.169" = [90, 45, 110] }
[fin.calendar.joins_h]
"S1.242 crediting" = 6
"S1.270 tax_period_close" = 4
sum = 10
core_ms = 8
[fin.calendar.joins_bprime]
sum = 5
core_ms = 4
[fin.calendar.campaign]
sum = 10
core_ms = 8
[fin.calendar.mass_default]
sum = 221.5
core_ms = 177.2
ratchet = 225
[fin.calendar.events]
"mass_default D+1" = 266.9
[resolution]
zones = 1_000
[ledger]
1 = { name = "process", mb = 300.0 }
2 = { name = "persons", mb = 400.5 }
total_mb = 700.5
[save]
frames_per_worker = 4
"#;

    #[test]
    fn design_file_parses() {
        assert_eq!(check(VALID), Vec::<String>::new());
        assert!(check("not = [toml").first().is_some_and(|m| m.contains("does not parse")));
    }

    #[test]
    fn a_missing_table_is_refused() {
        assert_eq!(check(&VALID.replace("[save]\nframes_per_worker = 4\n", "")), ["the table `[save]` is missing"]);
    }

    #[test]
    fn totals_are_their_parts() {
        let off = |from: &str, to: &str| check(&VALID.replace(from, to));
        assert_eq!(
            off("b_core_ms = 0.8", "b_core_ms = 1.0"),
            ["`fin.fixed.open.b_core_ms` Some(1.0) is not its shares' sum × scale"]
        );
        assert!(off("h_core_ms = 110", "h_core_ms = 88").len() == 1, "the barriers are not scaled");
        assert_eq!(
            off(
                "sum = 10\ncore_ms = 8\n[fin.calendar.joins_bprime]",
                "sum = 11\ncore_ms = 8.8\n[fin.calendar.joins_bprime]"
            ),
            ["`fin.calendar.joins_h.sum` is not its items' sum"]
        );
        assert_eq!(
            off("core_ms = 177.2", "core_ms = 180"),
            ["`fin.calendar.mass_default.core_ms` is not its sum × scale"]
        );
        assert_eq!(off("day = [4.0, 1.0, 6.0]", "day = [4.0, 1.0, 7.0]"), ["`[stage] day` is not its stages' sum"]);
        assert_eq!(off("total_mb = 700.5", "total_mb = 690"), ["`[ledger] total_mb` is not its lines' sum"]);
        assert_eq!(
            off("scale = 0.8", "scale = 0.9").first().map(String::as_str),
            Some("`[point] scale` 0.9 is not persons over the steps' persons")
        );
    }

    #[test]
    fn cited_keys_are_present() {
        let table = VALID.parse::<toml::Table>().unwrap();
        let plan =
            "the `[store] persons` count, `[unit] search`, `fin.fixed.open`; `[store] firms` and `fin.fixed.kinks`";
        assert_eq!(
            cited(&table, plan),
            [
                "the plan cites `[store] firms`, which the figure set lacks",
                "the plan cites `fin.fixed.kinks`, which the figure set lacks"
            ]
        );
    }
}
