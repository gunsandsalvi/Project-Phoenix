use std::path::Path;

use phx_macros::clause;
use serde::{Deserialize, Serialize};

use crate::consts::SAVE_MANIFEST;

/// One store of a save: its name and file, its size compressed and before compression, and its logical hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreEntry {
    pub name: String,
    pub file: String,
    pub bytes: u64,
    pub raw_bytes: u64,
    pub hash: String,
}

/// What a save is: the format it is written in, the build and register that wrote it, the run's seed and settings with
/// its representation's multiplicity and divisor, the day it was written at the close of, each store, and the world
/// hash of that close. It is written last, so a save
/// without one is incomplete.
#[clause("SET.12", "N5")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub build: String,
    pub register: String,
    pub seed: u64,
    pub day: u32,
    pub date: String,
    pub settling_years: u64,
    pub read_trace: bool,
    pub multiplicity: u32,
    pub population_divisor: u32,
    pub stores: Vec<StoreEntry>,
    pub world_hash: String,
}

/// A hash as the manifest writes it.
#[must_use]
pub fn hex(h: u128) -> String {
    format!("{h:032x}")
}

impl Manifest {
    /// # Errors
    /// When the file cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let path = dir.join(SAVE_MANIFEST);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// # Errors
    /// When the file is missing or is not a manifest.
    pub fn read(dir: &Path) -> Result<Manifest, String> {
        let path = dir.join(SAVE_MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The store of a name.
    ///
    /// # Errors
    /// When the save has no such store.
    pub fn store(&self, name: &str) -> Result<&StoreEntry, String> {
        self.stores.iter().find(|s| s.name == name).ok_or_else(|| format!("the save has no store `{name}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::{Manifest, StoreEntry};

    #[test]
    fn manifest_roundtrip() {
        let m = Manifest {
            format: 1,
            build: "b".to_owned(),
            register: "r".to_owned(),
            seed: 7,
            day: 400,
            date: "2027-02-05".to_owned(),
            settling_years: 1,
            read_trace: true,
            multiplicity: 20,
            population_divisor: 1,
            stores: vec![StoreEntry {
                name: "books".to_owned(),
                file: "books.zst".to_owned(),
                bytes: 10,
                raw_bytes: 30,
                hash: "0f".to_owned(),
            }],
            world_hash: "ab".to_owned(),
        };
        let text = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Manifest>(&text).unwrap(), m);
        assert_eq!(m.store("books").unwrap().bytes, 10);
        assert!(m.store("geo").is_err());
    }
}
