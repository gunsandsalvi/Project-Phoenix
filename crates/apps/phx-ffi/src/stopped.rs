//! A stop on the phone: the run's contract violations abort the process, which on Android closes the app with no word,
//! so the stop is written beside the report before the abort, for the app to show when it next opens.

use phx_num::{CapacityExceeded, Violation};

/// The file a stop is written to: beside the report the run would have written.
#[must_use]
pub fn stop_path(report_path: &str) -> String {
    format!("{report_path}.stopped.txt")
}

/// From here until the process ends, a panic writes what stopped the run to `stop_path(report_path)`.
pub fn watch(report_path: &str) {
    let path = stop_path(report_path);
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let site = phx_exec::site::current()
            .map_or_else(|| "outside a day".to_owned(), |s| format!("day {}, sub-step {}", s.day, s.substep));
        let what = if let Some(v) = payload.downcast_ref::<Violation>() {
            let keys: Vec<String> = v.keys().iter().map(|(k, x)| format!("{k} = {x}")).collect();
            format!("{}: {} ({})", v.clause, v.message, keys.join(", "))
        } else if let Some(c) = payload.downcast_ref::<CapacityExceeded>() {
            format!("capacity: {} (declared {}, needed {})", c.what, c.declared, c.needed)
        } else {
            let text = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            format!("{text} at {}", info.location().map_or_else(String::new, ToString::to_string))
        };
        // Nothing can be done if the file cannot be written: the process is about to abort.
        let _ = std::fs::write(&path, format!("{what}; {site}"));
    }));
}
