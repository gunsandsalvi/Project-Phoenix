//! The design point read from its figure set into typed fields: a key the file lacks, a setting still `Missing` or a
//! count past `u64` is refused by name, never given a default.

use std::collections::BTreeMap;

use toml::{Table, Value};

use crate::{DayType, FinError};

/// The design point's persons and the scale from the steps' figures to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub persons: u64,
    pub seed: u64,
    pub scale: f64,
}

/// The phone model the day's core-ms are read against.
#[derive(Debug, Clone, PartialEq)]
pub struct Phone {
    pub cores: f64,
    pub k_compute: f64,
    pub k_gather: f64,
    pub memory_mb: u64,
    /// The median and the worst turn's budgets, in ms.
    pub turn_ms: [u64; 2],
}

/// The design point's figure set.
#[derive(Debug, Clone)]
pub struct Design {
    pub point: Point,
    pub phone: Phone,
    /// Every store's rows at the design point, by the name the stores are declared under.
    pub store: BTreeMap<String, u64>,
    /// Each stage line's core-ms on a business, a non-business and a heavy day, as the steps declare them.
    pub stage: BTreeMap<String, [f64; 3]>,
    /// The work of the business day after closed days beyond an ordinary one's, in core-ms.
    pub bc_extra_core_ms: f64,
    days: BTreeMap<DayType, BTreeMap<String, u64>>,
    table: Table,
}

fn refuse(key: &str, why: &str) -> FinError {
    FinError(format!("`{key}` {why}"))
}

fn get<'a>(table: &'a Table, path: &str) -> Result<&'a Value, FinError> {
    let mut parts = path.split('.');
    let first = parts.next().unwrap_or_default();
    let mut value = table.get(first).ok_or_else(|| refuse(path, "is missing from the design point"))?;
    for part in parts {
        value = value.get(part).ok_or_else(|| refuse(path, "is missing from the design point"))?;
    }
    Ok(value)
}

fn count(value: &Value, key: &str) -> Result<u64, FinError> {
    match value {
        Value::Integer(i) => u64::try_from(*i).map_err(|_| refuse(key, "is not a count")),
        Value::String(s) if s == "Missing" => Err(refuse(key, "is Missing")),
        _ => Err(refuse(key, "is not a count")),
    }
}

fn real(value: &Value, key: &str) -> Result<f64, FinError> {
    match value {
        Value::Float(f) => Ok(*f),
        Value::Integer(i) => i.to_string().parse().map_err(|_| refuse(key, "is not a number")),
        Value::String(s) if s == "Missing" => Err(refuse(key, "is Missing")),
        _ => Err(refuse(key, "is not a number")),
    }
}

fn counts(table: &Table, path: &str) -> Result<BTreeMap<String, u64>, FinError> {
    let Value::Table(t) = get(table, path)? else {
        return Err(refuse(path, "is not a table"));
    };
    t.iter().filter(|(_, v)| v.is_integer()).map(|(k, v)| Ok((k.clone(), count(v, &format!("{path}.{k}"))?))).collect()
}

impl Design {
    /// # Errors
    /// A text that does not parse, or lacks a key the harness reads.
    pub fn parse(text: &str) -> Result<Design, FinError> {
        let table: Table = text.parse().map_err(|e| FinError(format!("the design point does not parse: {e}")))?;
        let point = Point {
            persons: count(get(&table, "point.persons")?, "point.persons")?,
            seed: count(get(&table, "point.seed")?, "point.seed")?,
            scale: real(get(&table, "point.scale")?, "point.scale")?,
        };
        let turn = get(&table, "phone.turn_ms")?.as_array().ok_or_else(|| refuse("phone.turn_ms", "is not a pair"))?;
        let turn_ms = match turn.as_slice() {
            [median, worst] => [count(median, "phone.turn_ms")?, count(worst, "phone.turn_ms")?],
            _ => return Err(refuse("phone.turn_ms", "is not a pair")),
        };
        let phone = Phone {
            cores: real(get(&table, "phone.cores")?, "phone.cores")?,
            k_compute: real(get(&table, "phone.k_compute")?, "phone.k_compute")?,
            k_gather: real(get(&table, "phone.k_gather")?, "phone.k_gather")?,
            memory_mb: count(get(&table, "phone.memory_mb")?, "phone.memory_mb")?,
            turn_ms,
        };
        let Value::Table(stages) = get(&table, "stage")? else {
            return Err(refuse("stage", "is not a table"));
        };
        let mut stage = BTreeMap::new();
        for (name, line) in stages {
            let key = format!("stage.{name}");
            let figures = line.as_array().ok_or_else(|| refuse(&key, "is not three figures"))?;
            let [b, nb, h] = figures.as_slice() else {
                return Err(refuse(&key, "is not three figures"));
            };
            stage.insert(name.clone(), [real(b, &key)?, real(nb, &key)?, real(h, &key)?]);
        }
        let mut days = BTreeMap::new();
        for day in DayType::ALL {
            days.insert(day, counts(&table, &format!("day.{}", day.key()))?);
        }
        // The business day after closed days is an ordinary one with its own counts written over.
        if let (Some(b), Some(bc)) = (days.get(&DayType::B), days.get(&DayType::Bc)) {
            let mut merged = b.clone();
            merged.extend(bc.iter().map(|(k, v)| (k.clone(), *v)));
            days.insert(DayType::Bc, merged);
        }
        let bc_extra_core_ms = real(get(&table, "day.bc.extra_core_ms")?, "day.bc.extra_core_ms")?;
        let store = counts(&table, "store")?;
        Ok(Design { point, phone, store, stage, bc_extra_core_ms, days, table })
    }

    /// A day type's counts; the business day after closed days holds the ordinary one's with its own written over.
    #[must_use]
    pub fn day(&self, day: DayType) -> Option<&BTreeMap<String, u64>> {
        self.days.get(&day)
    }

    /// A `[resolution]` setting a driver reads, refused where it is absent or still `Missing`.
    ///
    /// # Errors
    /// The setting absent or `Missing`.
    pub fn resolution(&self, key: &str) -> Result<&Value, FinError> {
        let path = format!("resolution.{key}");
        let value = get(&self.table, &path)?;
        if value.as_str() == Some("Missing") { Err(refuse(&path, "is Missing")) } else { Ok(value) }
    }
}
