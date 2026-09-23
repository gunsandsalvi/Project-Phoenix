use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::ratchets::Ratchets;
use crate::rules::Breach;
use crate::workspace::RATCHETS;

const RULE: &str = "bench-ratchets";

/// A kernel micro-benchmark's instruction count, named `<package>.<benchmark function>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measured {
    pub counter: String,
    pub instructions: u64,
}

/// Every `summary.json` under `dir`, as gungraun writes them with `GUNGRAUN_SAVE_SUMMARY=json`.
pub fn read(dir: &Path) -> Result<Vec<Measured>, String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = fs::read_dir(&next).map_err(|e| format!("{}: {e}", next.display()))?;
        for entry in entries {
            let path: PathBuf = entry.map_err(|e| format!("{}: {e}", next.display()))?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().is_some_and(|n| n == "summary.json") {
                let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                found.push(measured(&text).map_err(|e| format!("{}: {e}", path.display()))?);
            }
        }
    }
    found.sort_by(|a, b| a.counter.cmp(&b.counter));
    Ok(found)
}

fn measured(summary: &str) -> Result<Measured, String> {
    let v: Value = serde_json::from_str(summary).map_err(|e| e.to_string())?;
    let package = v
        .get("package_dir")
        .and_then(Value::as_str)
        .and_then(|d| Path::new(d).file_name())
        .map(|n| n.to_string_lossy().replace('-', "_"))
        .ok_or("no package_dir")?;
    let function = v.get("function_name").and_then(Value::as_str).ok_or("no function_name")?;
    let instructions = v
        .pointer("/profiles/0/summaries/total/summary/Callgrind/Ir/metrics/Left/Int")
        .and_then(Value::as_u64)
        .ok_or("no total instruction count")?;
    Ok(Measured { counter: format!("{package}.{function}"), instructions })
}

/// Breaches for counts with no entry or moved the wrong way, and notes for counts that could tighten their entry.
pub fn compare(measured: &[Measured], ratchets: &Ratchets) -> (Vec<Breach>, Vec<String>) {
    let mut breaches = Vec::new();
    let mut notes = Vec::new();
    for m in measured {
        match ratchets.find(&m.counter) {
            None => {
                let message = format!("`{}` measured {} instructions and has no ratchet", m.counter, m.instructions);
                breaches.push(Breach::new(RULE, RATCHETS, 1, message));
            }
            Some(r) if r.breached_by(m.instructions) => {
                let message = format!("`{}` measured {}; the ratchet holds {}", m.counter, m.instructions, r.value);
                breaches.push(Breach::new(RULE, RATCHETS, 1, message));
            }
            Some(r) if r.value != m.instructions => {
                notes.push(format!(
                    "`{}` measured {}; its ratchet may move to it from {}",
                    m.counter, m.instructions, r.value
                ));
            }
            Some(_) => {}
        }
    }
    (breaches, notes)
}

#[cfg(test)]
mod tests {
    use super::{Measured, compare, measured};
    use crate::ratchets::parse;

    #[test]
    fn bench_counts_held_to_their_ratchets() {
        let ratchets = parse(
            "[[ratchet]]\ncounter = \"phx_num.ir_a\"\nvalue = 100\ndirection = \"down\"\n\
             [[ratchet]]\ncounter = \"phx_num.ir_b\"\nvalue = 100\ndirection = \"down\"\n",
        )
        .unwrap();
        let m = |counter: &str, instructions| Measured { counter: counter.to_owned(), instructions };
        let (breaches, notes) =
            compare(&[m("phx_num.ir_a", 101), m("phx_num.ir_b", 90), m("phx_num.ir_c", 5)], &ratchets);
        assert_eq!(breaches.len(), 2);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn summary_names_package_and_function() {
        let summary = r#"{"package_dir": "/w/crates/foundation/phx-num", "function_name": "ir_accrue",
            "profiles": [{"summaries": {"total": {"summary": {"Callgrind": {"Ir": {"metrics": {"Left": {"Int": 239}}}}}}}}]}"#;
        assert_eq!(measured(summary), Ok(Measured { counter: "phx_num.ir_accrue".to_owned(), instructions: 239 }));
    }
}
