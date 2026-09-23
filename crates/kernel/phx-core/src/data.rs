use phx_id::{Date, Weekday};
use serde::Deserialize;
use toml::Value;

use crate::calendar::rules::{CountryRules, HolidayRule, WeekendRule};

/// One entry of a data file, as every declared number is written.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub unit: Option<String>,
    pub period: Option<String>,
    pub owner: String,
    pub decided_by: Option<String>,
    pub source: String,
    pub source_ref: String,
    pub shape: Option<String>,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    primitive: Vec<Entry>,
}

const KINDS: [&str; 6] = ["TECHNOLOGY", "PREFERENCE", "POLICY", "ENDOWMENT", "RESOLUTION", "SHAPE"];
const SOURCES: [&str; 4] = ["measured", "estimated", "assumed", "placeholder"];

/// A data file's entries, each checked for the form every entry takes: a known kind and source, and a reason.
///
/// # Errors
/// When the file does not parse or an entry is malformed.
pub fn entries(text: &str) -> Result<Vec<Entry>, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    for e in &file.primitive {
        if !KINDS.contains(&e.kind.as_str()) {
            return Err(format!("`{}`: kind `{}` is not a kind of primitive", e.id, e.kind));
        }
        if !SOURCES.contains(&e.source.as_str()) {
            return Err(format!("`{}`: source `{}` is not a kind of source", e.id, e.source));
        }
        if e.source_ref.trim().is_empty() {
            return Err(format!("`{}`: no source_ref", e.id));
        }
    }
    Ok(file.primitive)
}

fn find<'a>(entries: &'a [Entry], id: &str) -> Result<&'a Value, String> {
    entries.iter().find(|e| e.id == id).map(|e| &e.value).ok_or_else(|| format!("no `{id}`"))
}

/// The epoch from the world's constants.
///
/// # Errors
/// When the entry is missing or is not a date.
pub fn epoch(world: &str) -> Result<Date, String> {
    let es = entries(world)?;
    let value = find(&es, "TIME.epoch")?;
    let date = value.as_datetime().and_then(|d| d.date).ok_or("`TIME.epoch` is not a date")?;
    Date::new(i32::from(date.year), date.month, date.day).ok_or_else(|| format!("`TIME.epoch` {date} is no date"))
}

fn weekday(v: &Value) -> Result<Weekday, String> {
    let names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let s = v.as_str().ok_or("a weekday is not a string")?;
    let i = names.iter().position(|n| *n == s).ok_or_else(|| format!("`{s}` is not a weekday"))?;
    crate::calendar::rules::WEEK.get(i).copied().ok_or_else(|| format!("`{s}` is not a weekday"))
}

fn weekdays(v: Option<&Value>) -> Result<Vec<Weekday>, String> {
    v.and_then(Value::as_array).ok_or("weekdays are not a list")?.iter().map(weekday).collect()
}

fn field<'a>(t: &'a toml::Table, key: &str, name: &str) -> Result<&'a Value, String> {
    t.get(key).ok_or_else(|| format!("`{name}` has no `{key}`"))
}

fn small<T: TryFrom<i64>>(t: &toml::Table, key: &str, name: &str) -> Result<T, String> {
    let v = field(t, key, name)?.as_integer().ok_or_else(|| format!("`{name}`: `{key}` is not a whole number"))?;
    T::try_from(v).map_err(|_| format!("`{name}`: `{key}` = {v} is out of range"))
}

fn holiday(v: &Value) -> Result<HolidayRule, String> {
    let t = v.as_table().ok_or("a holiday rule is not a table")?;
    let name = field(t, "name", "a holiday")?.as_str().ok_or("a holiday's name is not a string")?.to_owned();
    let rule = field(t, "rule", &name)?.as_str().ok_or_else(|| format!("`{name}`: `rule` is not a string"))?;
    match rule {
        "fixed" => Ok(HolidayRule::Fixed { month: small(t, "month", &name)?, day: small(t, "day", &name)?, name }),
        "nth_weekday" => Ok(HolidayRule::NthWeekday {
            month: small(t, "month", &name)?,
            weekday: weekday(field(t, "weekday", &name)?)?,
            n: small(t, "n", &name)?,
            name,
        }),
        "easter" => Ok(HolidayRule::EasterOffset { days: small(t, "offset_days", &name)?, name }),
        "substitute" => Ok(HolidayRule::Substitute {
            of: field(t, "of", &name)?.as_str().ok_or_else(|| format!("`{name}`: `of` is not a string"))?.to_owned(),
            when_on: weekdays(t.get("when_on"))?,
            name,
        }),
        other => Err(format!("`{name}`: `{other}` is not a kind of holiday rule")),
    }
}

/// A country's calendar rules from its TIME file.
///
/// # Errors
/// When an entry is missing or malformed, or the rules cannot describe a calendar.
pub fn country_rules(text: &str) -> Result<CountryRules, String> {
    let es = entries(text)?;
    let weekend = WeekendRule { days: weekdays(Some(find(&es, "TIME.weekend")?))? };
    let holidays = find(&es, "TIME.holidays")?
        .as_array()
        .ok_or("`TIME.holidays` is not a list")?
        .iter()
        .map(holiday)
        .collect::<Result<_, _>>()?;
    let rules = CountryRules { weekend, holidays };
    rules.validate()?;
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use phx_id::Date;

    use super::{country_rules, epoch};

    fn read(path: &str) -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join(path)).unwrap()
    }

    #[test]
    fn the_committed_data_loads() {
        assert_eq!(epoch(&read("data/world.toml")), Ok(Date::new(1950, 1, 1).unwrap()));
        for level in ["developed", "emerging", "developing"] {
            let rules = country_rules(&read(&format!("data/profiles/{level}/TIME.toml"))).unwrap();
            assert!(!rules.holidays.is_empty(), "{level}");
        }
        let developed = country_rules(&read("data/profiles/developed/TIME.toml")).unwrap();
        let h = developed.holidays_in(2021);
        // 2021 in England: 1 Jan, 2 Apr, 5 Apr, 3 May, 31 May, 30 Aug, and Christmas and Boxing Day on the weekend
        // with their substitutes on 27 and 28 Dec.
        let expected: Vec<Date> =
            [(1, 1), (4, 2), (4, 5), (5, 3), (5, 31), (8, 30), (12, 25), (12, 26), (12, 27), (12, 28)]
                .iter()
                .map(|(m, d)| Date::new(2021, *m, *d).unwrap())
                .collect();
        assert_eq!(h, expected);
        let emerging = country_rules(&read("data/profiles/emerging/TIME.toml")).unwrap();
        // Carnival 2025 fell on 3 and 4 March, Corpus Christi on 19 June.
        let h = emerging.holidays_in(2025);
        for (m, d) in [(3, 3), (3, 4), (6, 19), (11, 20)] {
            assert!(h.contains(&Date::new(2025, m, d).unwrap()), "{m}-{d}");
        }
    }

    #[test]
    fn malformed_data_is_refused() {
        let bad_kind = "[[primitive]]\nid = \"TIME.epoch\"\nkind = \"GUESS\"\nowner = \"TIME\"\nsource = \"assumed\"\nsource_ref = \"x\"\nvalue = 1950-01-01\n";
        assert!(epoch(bad_kind).is_err());
        let no_ref = bad_kind.replace("GUESS", "ENDOWMENT").replace("\"x\"", "\"\"");
        assert!(epoch(&no_ref).is_err());
    }
}
