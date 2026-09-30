//! The finished-volume ratchets of `perf/budget.toml`: every `fin.<base>.<measure>` a driver measures, times, bytes,
//! faults and allocations only falling, cores busy only rising.

use serde::Deserialize;

use crate::FinError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Ratchet {
    pub counter: String,
    pub value: f64,
    pub direction: Direction,
}

#[derive(Debug, Deserialize)]
struct File {
    #[serde(default)]
    ratchet: Vec<Ratchet>,
}

/// The file's `fin.` ratchets.
///
/// # Errors
/// A file that does not parse.
pub fn read(text: &str) -> Result<Vec<Ratchet>, FinError> {
    let file: File = toml::from_str(text).map_err(|e| FinError(format!("the budget does not parse: {e}")))?;
    Ok(file.ratchet.into_iter().filter(|r| r.counter.starts_with("fin.")).collect())
}

/// Every ratchet a measure misses, with its key, the measure and the target; a ratchet with no measure is its base's
/// before a driver, and is not read.
#[must_use]
pub fn misses(ratchets: &[Ratchet], measured: &dyn Fn(&str) -> Option<f64>) -> Vec<String> {
    ratchets
        .iter()
        .filter_map(|r| {
            let m = measured(&r.counter)?;
            let missed = match r.direction {
                Direction::Down => m > r.value,
                Direction::Up => m < r.value,
            };
            missed.then(|| format!("`{}` is {m}; its ratchet allows {}", r.counter, r.value))
        })
        .collect()
}
