use std::path::{Path, PathBuf};

use phx_id::Day;
use phx_macros::clause;

use crate::consts::{SAVE_MANIFEST, SAVE_PARTIAL};

/// A complete save's directory name for the day it was written at the close of.
#[must_use]
pub fn name(day: Day) -> String {
    format!("day-{:010}", day.get())
}

/// What goes when a save is written: nothing until it is complete, then every other save in the directory, complete
/// or left partial by a save that never finished.
#[clause("SET.12", "N8.4")]
#[must_use]
pub fn to_delete(entries: &[String], new: &str, complete: bool) -> Vec<String> {
    if !complete {
        return Vec::new();
    }
    entries.iter().filter(|e| e.as_str() != new).cloned().collect()
}

fn io(path: &Path, e: &std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// A save's directory begun beside the complete one: named partial until everything in it is written and synced.
///
/// # Errors
/// When the directory cannot be made.
pub fn begin(root: &Path, day: Day) -> Result<PathBuf, String> {
    let partial = root.join(format!("{}{SAVE_PARTIAL}", name(day)));
    if partial.exists() {
        std::fs::remove_dir_all(&partial).map_err(|e| io(&partial, &e))?;
    }
    std::fs::create_dir_all(&partial).map_err(|e| io(&partial, &e))?;
    Ok(partial)
}

fn sync(path: &Path) -> Result<(), String> {
    std::fs::File::open(path).and_then(|f| f.sync_all()).map_err(|e| io(path, &e))
}

/// A save made complete: every file and the directory synced, the directory given its complete name and the root
/// synced, and only then every other save removed.
///
/// # Errors
/// When a file cannot be synced, renamed or removed.
pub fn commit(root: &Path, partial: &Path, day: Day) -> Result<PathBuf, String> {
    for entry in std::fs::read_dir(partial).map_err(|e| io(partial, &e))? {
        sync(&entry.map_err(|e| io(partial, &e))?.path())?;
    }
    sync(partial)?;
    let done = root.join(name(day));
    if done.exists() {
        std::fs::remove_dir_all(&done).map_err(|e| io(&done, &e))?;
    }
    std::fs::rename(partial, &done).map_err(|e| io(partial, &e))?;
    sync(root)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| io(root, &e))? {
        entries.push(entry.map_err(|e| io(root, &e))?.file_name().to_string_lossy().into_owned());
    }
    entries.sort();
    for old in to_delete(&entries, &name(day), done.join(SAVE_MANIFEST).exists()) {
        let path = root.join(old);
        std::fs::remove_dir_all(&path).map_err(|e| io(&path, &e))?;
    }
    Ok(done)
}

/// The latest complete save in a directory, if any.
#[must_use]
pub fn latest(root: &Path) -> Option<PathBuf> {
    let mut complete: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join(SAVE_MANIFEST).exists() && !p.to_string_lossy().ends_with(SAVE_PARTIAL))
        .collect();
    complete.sort();
    complete.pop()
}

#[cfg(test)]
mod tests {
    use super::to_delete;

    #[test]
    fn retention_deletes_only_after_the_new_save_syncs() {
        let entries = ["day-0000000090".to_owned(), "day-0000000181.partial".to_owned()];
        assert!(to_delete(&entries, "day-0000000181", false).is_empty(), "nothing goes while the new save is partial");
        let entries = ["day-0000000090".to_owned(), "day-0000000120.partial".to_owned(), "day-0000000181".to_owned()];
        assert_eq!(
            to_delete(&entries, "day-0000000181", true),
            ["day-0000000090", "day-0000000120.partial"],
            "once complete, the older save and a save that never finished go"
        );
    }
}
