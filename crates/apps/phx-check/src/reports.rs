use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::rules::Breach;

const CHECK: &str = "device-reports";
pub const SCHEMA: &str = "perf/schema/device-report.json";
pub const REPORTS: &str = "perf/device";

/// The kind of a JSON value by the schema's names; an integer is also a number.
fn is_type(v: &Value, name: &str) -> bool {
    match name {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    }
}

/// Where a value breaks the schema, by the keywords the report's schema uses: `type`, `enum`, `minimum`,
/// `required`, `properties`, `additionalProperties: false` and `items`. A keyword outside that set is itself refused,
/// so the schema cannot silently ask for more than is checked.
pub fn validate(schema: &Value, v: &Value, at: &str, errors: &mut Vec<String>) {
    let Some(rules) = schema.as_object() else {
        errors.push(format!("{at}: the schema here is not an object"));
        return;
    };
    for key in rules.keys() {
        let known =
            ["$schema", "title", "type", "enum", "minimum", "required", "properties", "additionalProperties", "items"];
        if !known.contains(&key.as_str()) {
            errors.push(format!("{at}: schema keyword `{key}` is not checked"));
        }
    }
    if let Some(t) = rules.get("type") {
        let names: Vec<&str> = t
            .as_str()
            .map_or_else(|| t.as_array().into_iter().flatten().filter_map(Value::as_str).collect(), |s| vec![s]);
        if !names.iter().any(|n| is_type(v, n)) {
            errors.push(format!("{at}: expected {}, found {v}", names.join(" or ")));
            return;
        }
    }
    if let Some(allowed) = rules.get("enum").and_then(Value::as_array)
        && !allowed.contains(v)
    {
        errors.push(format!("{at}: {v} is not one of {}", Value::Array(allowed.clone())));
    }
    if let (Some(min), Some(n)) = (rules.get("minimum").and_then(Value::as_f64), v.as_f64())
        && n < min
    {
        errors.push(format!("{at}: {n} is below {min}"));
    }
    if let Some(obj) = v.as_object() {
        for field in rules.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            if !obj.contains_key(field) {
                errors.push(format!("{at}: missing `{field}`"));
            }
        }
        let props = rules.get("properties").and_then(Value::as_object);
        for (k, child) in obj {
            match props.and_then(|p| p.get(k)) {
                Some(s) => validate(s, child, &format!("{at}.{k}"), errors),
                None if rules.get("additionalProperties") == Some(&Value::Bool(false)) => {
                    errors.push(format!("{at}: `{k}` is not in the schema"));
                }
                None => {}
            }
        }
    }
    if let (Some(items), Some(arr)) = (rules.get("items"), v.as_array()) {
        for (i, item) in arr.iter().enumerate() {
            validate(items, item, &format!("{at}[{i}]"), errors);
        }
    }
}

/// Every committed device report, validated against the schema.
pub fn check(root: &Path) -> Vec<Breach> {
    let schema = match fs::read_to_string(root.join(SCHEMA))
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string()))
    {
        Ok(s) => s,
        Err(error) => return vec![Breach::new(CHECK, SCHEMA, 1, error)],
    };
    let Ok(entries) = fs::read_dir(root.join(REPORTS)) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut breaches = Vec::new();
    for path in paths {
        let name =
            format!("{REPORTS}/{}", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        let parsed = fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string()));
        match parsed {
            Ok(report) => {
                let mut errors = Vec::new();
                validate(&schema, &report, "$", &mut errors);
                breaches.extend(errors.into_iter().map(|e| Breach::new(CHECK, &name, 1, e)));
            }
            Err(error) => breaches.push(Breach::new(CHECK, &name, 1, format!("not JSON: {error}"))),
        }
    }
    breaches
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate;

    #[test]
    fn reports_are_held_to_their_schema() {
        let schema = json!({
            "type": "object",
            "required": ["v", "xs"],
            "additionalProperties": false,
            "properties": {
                "v": { "type": "integer", "enum": [1] },
                "xs": { "type": "array", "items": { "type": ["integer", "null"], "minimum": 0 } }
            }
        });
        let errors = |v| {
            let mut e = Vec::new();
            validate(&schema, &v, "$", &mut e);
            e.len()
        };
        assert_eq!(errors(json!({ "v": 1, "xs": [0, null, 7] })), 0);
        assert_eq!(errors(json!({ "v": 2, "xs": [-1, "a"] })), 3);
        assert_eq!(errors(json!({ "v": 1, "extra": true })), 2);
        let mut e = Vec::new();
        validate(&json!({ "pattern": "x" }), &json!("y"), "$", &mut e);
        assert_eq!(e.len(), 1, "a keyword the validator does not check is refused");
    }

    #[test]
    fn the_committed_schema_accepts_a_report_of_its_shape() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(super::SCHEMA)).unwrap()).unwrap();
        let report = json!({
            "report_version": 1, "step": "S0.07", "commit": "abc",
            "device": { "manufacturer": "m", "model": "p", "soc": "s", "android_sdk": 37, "total_ram_bytes": 1, "app_version": "1", "started_at": "t" },
            "pool": { "cores": [4, 5], "pinned": true, "unpinned": 0 },
            "core_rates": [{ "at": "start", "thermal_status": 0, "pool_fast_core_seconds_per_s": 2.5, "cores": [
                { "core": 4, "capacity": null, "in_pool": true, "alone_units_per_s": 10, "loaded_units_per_s": 9 }] }],
            "barrier_ns": 30000,
            "gathers": [{ "bytes": 1, "prefetch": false, "rows": 1, "ns_per_row": 1, "core_ns_per_row": 1, "page_size": 16384 }],
            "sweep_bytes_per_s": null,
            "micro": [{ "name": "phx_rand.philox", "ns_per_op": 20, "ops": 1000 }],
            "targets": [{ "section": "targets", "name": "n", "value": "1 ms", "target": "≤ 2", "verdict": "met", "thermal_status": 0 }]
        });
        let mut e = Vec::new();
        validate(&schema, &report, "$", &mut e);
        assert!(e.is_empty(), "{e:?}");
    }
}
