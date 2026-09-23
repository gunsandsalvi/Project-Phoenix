use std::path::PathBuf;

use phx_num::{CapacityExceeded, Violation};

/// Writes a stopped run's report, `violations/<run>.json`: the clause or limit, the keys, and the site the stopping
/// thread was running.
pub fn install(run: String, dir: PathBuf) {
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let site = phx_exec::site::current().map_or(
            serde_json::Value::String("outside".to_owned()),
            |s| serde_json::json!({ "day": s.day, "substep": s.substep, "handler": s.handler, "chunk": s.chunk }),
        );
        let body = if let Some(v) = payload.downcast_ref::<Violation>() {
            let keys: serde_json::Map<String, serde_json::Value> =
                v.keys().iter().map(|(k, x)| ((*k).to_owned(), serde_json::Value::String(x.to_string()))).collect();
            serde_json::json!({ "violation": v.clause, "message": v.message, "keys": keys, "site": site })
        } else if let Some(c) = payload.downcast_ref::<CapacityExceeded>() {
            serde_json::json!({
                "capacity": c.what, "declared": c.declared.to_string(), "needed": c.needed.to_string(), "site": site
            })
        } else {
            let text = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned());
            serde_json::json!({ "panic": text, "location": info.location().map(ToString::to_string), "site": site })
        };
        let path = dir.join(format!("{run}.json"));
        let written = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, body.to_string()));
        eprintln!("the run stopped: {body}");
        if let Err(e) = written {
            eprintln!("and its report could not be written to {}: {e}", path.display());
        }
    }));
}
