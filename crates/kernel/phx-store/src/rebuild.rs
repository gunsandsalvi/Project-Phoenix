//! The load's rebuild pass: every index a save left out comes back by the function its field names, store by store in
//! the order the saved tree declares them and each after the rebuilds it reads, so what comes back is the declarations'
//! alone.

use phx_macros::clause;

use crate::convert::to_u64;
use crate::save::Saved;
use crate::stats::StoreStats;

/// A rebuild that failed: the store and field whose index did not come back, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildError {
    pub store: &'static str,
    pub why: String,
}

impl std::fmt::Display for RebuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` did not rebuild: {}", self.store, self.why)
    }
}

/// What a rebuild returns: its rows, or, for one that can fail, its rows or why it failed.
pub trait RebuildRows {
    /// # Errors
    /// Why the rebuild failed.
    fn rows(self) -> Result<u64, String>;
}

impl RebuildRows for u64 {
    fn rows(self) -> Result<u64, String> {
        Ok(self)
    }
}

impl RebuildRows for Result<u64, String> {
    fn rows(self) -> Result<u64, String> {
        self
    }
}

/// What a load's pass rebuilt: each index by its store and field, and its rows, in the order rebuilt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rebuilt {
    stores: Vec<(&'static str, u64)>,
}

impl Rebuilt {
    /// An index rebuilt, and its rows.
    pub fn record(&mut self, store: &'static str, rows: u64) {
        self.stores.push((store, rows));
    }

    /// Each index rebuilt and its rows, in the order the pass rebuilt them.
    #[must_use]
    pub fn stores(&self) -> &[(&'static str, u64)] {
        &self.stores
    }
}

impl StoreStats for Rebuilt {
    fn rows_live(&self) -> u64 {
        self.stores.iter().map(|(_, rows)| rows).sum()
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        to_u64(size_of_val(self.stores.as_slice()))
    }
}

/// The pass over a value read back: every derived index rebuilt in its declared order.
///
/// # Errors
/// The first rebuild that fails, naming its store; the load stops there.
#[clause("SET.15")]
pub fn rebuild<T: Saved>(value: &mut T) -> Result<Rebuilt, RebuildError> {
    let mut out = Rebuilt::default();
    value.rebuild_derived(&mut out)?;
    Ok(out)
}

/// A large rebuild's rows cut into `ranges` slot ranges, each rebuilt apart and joined in range order: what comes back
/// is the same however many workers took the ranges.
pub fn by_ranges<R>(rows: usize, ranges: usize, mut part: impl FnMut(std::ops::Range<usize>, &mut Vec<R>)) -> Vec<R> {
    if ranges == 0 {
        phx_num::violation!(clause = "SET.15", "a rebuild cut into no ranges", rows = rows);
    }
    let step = rows.div_ceil(ranges);
    let mut out = Vec::new();
    let mut start = 0;
    while start < rows {
        let end = if start + step < rows { start + step } else { rows };
        part(start..end, &mut out);
        start = end;
    }
    out
}

#[cfg(test)]
#[path = "rebuild_tests.rs"]
mod tests;
