use std::fmt::Write;

/// A JSON value, enough for the device report; objects keep their keys in the order they were added.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(pairs: impl IntoIterator<Item = (&'static str, Json)>) -> Json {
        Json::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// A value that may be missing: null when it is.
    pub fn opt<T>(v: Option<T>, f: impl FnOnce(T) -> Json) -> Json {
        v.map_or(Json::Null, f)
    }

    fn write(&self, out: &mut String, indent: usize) {
        let pad = |out: &mut String, n: usize| out.extend(std::iter::repeat_n(' ', n));
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(v) => {
                let _ = write!(out, "{v}");
            }
            Json::UInt(v) => {
                let _ = write!(out, "{v}");
            }
            // JSON has no infinities or NaN; a value that is not a number is written as missing.
            Json::Float(v) => {
                if v.is_finite() {
                    let _ = write!(out, "{v}");
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => escape(out, s),
            Json::Array(items) if items.is_empty() => out.push_str("[]"),
            Json::Array(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    pad(out, indent + 2);
                    item.write(out, indent + 2);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                pad(out, indent);
                out.push(']');
            }
            Json::Object(pairs) if pairs.is_empty() => out.push_str("{}"),
            Json::Object(pairs) => {
                out.push_str("{\n");
                for (i, (k, v)) in pairs.iter().enumerate() {
                    pad(out, indent + 2);
                    escape(out, k);
                    out.push_str(": ");
                    v.write(out, indent + 2);
                    out.push_str(if i + 1 < pairs.len() { ",\n" } else { "\n" });
                }
                pad(out, indent);
                out.push('}');
            }
        }
    }

    #[must_use]
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }
}

fn escape(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn writes_valid_json() {
        let v = Json::obj([
            ("a", Json::UInt(3)),
            ("b", Json::Array(vec![Json::str("x\"y\n"), Json::Null, Json::Float(1.5), Json::Float(f64::NAN)])),
            ("c", Json::obj([])),
        ]);
        assert_eq!(
            v.pretty(),
            "{\n  \"a\": 3,\n  \"b\": [\n    \"x\\\"y\\n\",\n    null,\n    1.5,\n    null\n  ],\n  \"c\": {}\n}\n"
        );
    }
}
