//! `phx fin --seed-budget`: the design point's ratchets, `[fin]`, written into `perf/budget.toml` from
//! `perf/design.toml`, so the two files cannot disagree. Each key bounds the design figure it names; a key a measure has
//! tightened below its seed keeps the measure, and no key stands looser than its seed. The frame's own keys — the kept
//! kernels', the counters' — are measures, not seeds, and stand beside the section.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use toml::{Table, Value};

use crate::FinError;

/// Where the seeded section begins and ends in the budget file.
pub const BEGIN: &str = "# ---- [fin]: seeded from perf/design.toml by `phx fin --seed-budget`; edited only by it ----";
pub const END: &str = "# ---- end of the seeded [fin] ----";
/// A mebibyte, and the day types' figures in the order a stage line holds them.
const MIB: f64 = 1_048_576.0;
const DAYS: [&str; 3] = ["b", "nb", "h"];

/// One seeded key and its bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Seeded {
    pub counter: String,
    pub value: f64,
}

fn refuse(key: &str) -> FinError {
    FinError(format!("the design point has no `{key}` to seed from"))
}

fn at<'a>(table: &'a Table, path: &str) -> Result<&'a Value, FinError> {
    let mut parts = path.split('.');
    let mut value = table.get(parts.next().unwrap_or_default()).ok_or_else(|| refuse(path))?;
    for part in parts {
        value = value.get(part).ok_or_else(|| refuse(path))?;
    }
    Ok(value)
}

fn real(value: &Value, key: &str) -> Result<f64, FinError> {
    match value {
        Value::Float(f) => Ok(*f),
        Value::Integer(i) => i.to_string().parse().map_err(|_| refuse(key)),
        _ => Err(refuse(key)),
    }
}

fn number(table: &Table, path: &str) -> Result<f64, FinError> {
    real(at(table, path)?, path)
}

fn subtable<'a>(table: &'a Table, path: &str) -> Result<&'a Table, FinError> {
    at(table, path)?.as_table().ok_or_else(|| refuse(path))
}

/// A name as a key: lower case, words joined by `_`.
fn slug(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.join("_")
}

/// Every `[fin]` key the design point seeds, in the order written.
///
/// # Errors
/// A design point that does not parse, or lacks a figure a key needs, which is refused, never written as 0.
pub fn seed(design: &str) -> Result<Vec<Seeded>, FinError> {
    let t: Table = design.parse().map_err(|e| FinError(format!("the design point does not parse: {e}")))?;
    let mut out = Vec::new();
    let mut put = |counter: String, value: f64| out.push(Seeded { counter, value });

    // The day by type, B′ its business day with its declared extra.
    let day = at(&t, "stage.day")?.as_array().ok_or_else(|| refuse("stage.day"))?;
    for (d, v) in DAYS.iter().zip(day) {
        put(format!("fin.day.{d}_core_ms"), real(v, "stage.day")?);
    }
    let b = day.first().map(|v| real(v, "stage.day")).ok_or_else(|| refuse("stage.day"))??;
    put("fin.day.bc_core_ms".to_owned(), b + number(&t, "day.bc.extra_core_ms")?);

    // The turn's two lines, each the headroom below the phone's.
    let turn = at(&t, "phone.turn_ms")?.as_array().ok_or_else(|| refuse("phone.turn_ms"))?;
    let keep = 1.0 - number(&t, "phone.turn_headroom")?;
    for (name, v) in ["median_ms", "worst_ms"].iter().zip(turn) {
        put(format!("fin.turn.{name}"), real(v, "phone.turn_ms")? * keep);
    }

    // Memory: the phone's, the process's baseline, the bytes a person, and each line of the ledger.
    let ledger = subtable(&t, "ledger")?;
    let total = number(&t, "ledger.total_mb")?;
    put("fin.mem.peak_mb".to_owned(), number(&t, "phone.memory_mb")?);
    put("fin.mem.baseline_mb".to_owned(), number(&t, "ledger.1.mb")?);
    put("fin.mem.bytes_per_person".to_owned(), total * MIB / number(&t, "point.persons")?);
    let mut lines: Vec<(u32, &Table)> =
        ledger.iter().filter_map(|(k, v)| Some((k.parse().ok()?, v.as_table()?))).collect();
    lines.sort_by_key(|(n, _)| *n);
    for (n, line) in lines {
        let name = line.get("name").and_then(Value::as_str).ok_or_else(|| refuse(&format!("ledger.{n}.name")))?;
        let mb = real(line.get("mb").ok_or_else(|| refuse(&format!("ledger.{n}.mb")))?, "ledger.mb")?;
        put(format!("fin.mem.{}_mb", slug(name)), mb);
    }

    // Each fixed line by day type.
    for (line, v) in subtable(&t, "fin.fixed")? {
        let Value::Table(fixed) = v else { continue };
        for d in DAYS {
            let key = format!("{d}_core_ms");
            let figure = fixed.get(&key).ok_or_else(|| refuse(&format!("fin.fixed.{line}.{key}")))?;
            put(format!("fin.fixed.{line}.{key}"), real(figure, "fin.fixed")?);
        }
    }

    // Each kind of work's unit, in VM ns at the compute factor until its base measures its own.
    let k = number(&t, "phone.k_compute")?;
    for (unit, v) in subtable(&t, "unit")? {
        put(format!("fin.unit.{unit}_ns"), real(v, &format!("unit.{unit}"))? / k);
    }
    put("fin.decide.spend_mind_ns".to_owned(), number(&t, "unit.spend")? / k);
    put("fin.decide.handler_mind_ns".to_owned(), number(&t, "unit.handler")? / k);

    // Each family's contracts, and the bytes a row of each stored kind.
    for (family, v) in subtable(&t, "store.contracts")? {
        put(format!("fin.contracts.{}_rows", slug(family)), real(v, "store.contracts")?);
    }
    for (row, v) in subtable(&t, "bytes")? {
        put(format!("fin.bytes.{row}"), real(v, &format!("bytes.{row}"))?);
    }
    Ok(out)
}

/// The seeded ratchets already in a budget file, by counter.
fn standing(budget: &str) -> Result<BTreeMap<String, f64>, FinError> {
    let file: Table = budget.parse().map_err(|e| FinError(format!("the budget does not parse: {e}")))?;
    let mut out = BTreeMap::new();
    for r in file.get("ratchet").and_then(Value::as_array).into_iter().flatten() {
        if let (Some(c), Some(v)) = (r.get("counter").and_then(Value::as_str), r.get("value")) {
            out.insert(c.to_owned(), real(v, c)?);
        }
    }
    Ok(out)
}

/// A budget file with its seeded section written anew: each key at its seed, or at a measure already below it.
///
/// # Errors
/// A budget that does not parse.
pub fn write(budget: &str, seeded: &[Seeded]) -> Result<String, FinError> {
    let before = standing(budget)?;
    let kept = match (budget.find(BEGIN), budget.find(END)) {
        (Some(b), Some(e)) if b < e => format!("{}{}", &budget[..b], budget[e + END.len()..].trim_start_matches('\n')),
        _ => budget.to_owned(),
    };
    let mut out = kept.trim_end().to_owned();
    let _ = write!(out, "\n\n{BEGIN}\n");
    for s in seeded {
        let value = before.get(&s.counter).copied().filter(|v| *v < s.value).unwrap_or(s.value);
        let _ =
            write!(out, "\n[[ratchet]]\ncounter = {:?}\nvalue = {}\ndirection = \"down\"\n", s.counter, shown(value));
    }
    let _ = writeln!(out, "\n{END}");
    Ok(out)
}

/// A bound as the file shows it: whole where it is whole, else to four places.
fn shown(v: f64) -> String {
    let rounded = format!("{v:.4}");
    let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') { trimmed.to_owned() } else { format!("{trimmed}.0") }
}

/// Every seeded key a budget file holds looser than its seed, or lacks.
///
/// # Errors
/// A budget that does not parse.
pub fn within_design(budget: &str, seeded: &[Seeded]) -> Result<Vec<String>, FinError> {
    let standing = standing(budget)?;
    let tolerance = 1e-9;
    Ok(seeded
        .iter()
        .filter_map(|s| match standing.get(&s.counter) {
            None => Some(format!("`{}` is not in the budget", s.counter)),
            Some(v) if *v > s.value + tolerance => {
                Some(format!("`{}` is {v}, looser than its seed {}", s.counter, s.value))
            }
            Some(_) => None,
        })
        .collect())
}
