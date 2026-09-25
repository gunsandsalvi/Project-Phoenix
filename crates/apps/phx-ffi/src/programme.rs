//! The measurement programme on the phone: the probe and micro-benchmarks, the world's turns and the full-load bench,
//! run one after another in the app's process and written as one device report of the second version.

use std::sync::Arc;

use crate::bench::{BenchHost, BenchLine, DeviceInfo};
use crate::json::Json;

/// The combined report's layout: the first version's fundamentals with the world's and the full-load bench's sections.
const REPORT_VERSION: u64 = 2;
/// The step whose gate the report judges.
const STEP: &str = "S0.26";

/// The three parts run in order, each shown as it goes, and the report written to `report_path` and returned.
///
/// # Errors
/// The first part that stops, or a report that cannot be written.
pub fn run(
    device: &DeviceInfo,
    host: &dyn BenchHost,
    (data, run_dir, turns): (&str, &str, u32),
    (volumes, save_dir): (&str, &str),
    report_path: &str,
) -> Result<String, String> {
    let Json::Object(mut fields) = crate::bench::measure(device, host)? else {
        return Err("the bench's report is not an object".to_owned());
    };
    let world = crate::world::measure(host, data, run_dir, turns)?;
    let load = crate::load::measure(host, volumes, save_dir)?;
    for (key, value) in &mut fields {
        match key.as_str() {
            "report_version" => *value = Json::UInt(REPORT_VERSION),
            "step" => *value = Json::str(STEP),
            _ => {}
        }
    }
    fields.push(("world".to_owned(), world));
    fields.push(("load".to_owned(), load));
    let text = Json::Object(fields).pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    crate::bench::show_written(host, report_path);
    Ok(text)
}

/// The app's entry: runs the whole programme on the calling thread, which must not be the interface's.
#[uniffi::export]
#[expect(clippy::needless_pass_by_value, reason = "the foreign interface hands over owned values")]
#[expect(clippy::too_many_arguments, reason = "the foreign interface takes flat arguments")]
pub fn run_programme(
    device: DeviceInfo,
    host: Arc<dyn BenchHost>,
    data_dir: String,
    run_dir: String,
    turns: u32,
    volumes_path: String,
    save_dir: String,
    report_path: String,
) -> String {
    crate::stopped::watch(&report_path);
    let world = (data_dir.as_str(), run_dir.as_str(), turns);
    match run(&device, host.as_ref(), world, (&volumes_path, &save_dir), &report_path) {
        Ok(report) => report,
        Err(error) => {
            host.on_line(BenchLine {
                section: "error".to_owned(),
                name: "the programme stopped".to_owned(),
                value: error.clone(),
                target: String::new(),
                verdict: String::new(),
                thermal_status: host.thermal_status(),
            });
            error
        }
    }
}
