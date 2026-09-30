//! Saves: the world written whole at a day's close, read back exactly, and checked by reading its files alone.

pub mod inject;
pub mod manifest;
pub mod retention;

use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use phx_core::{ItemDecl, SystemEntry};
use phx_id::Day;
use phx_macros::clause;
use phx_store::{LoadError, Reader, Saved, Writer};

use crate::consts::{HASH_KEY, SAVE_FORMAT};
use crate::core::Core;
use crate::metrics::{Metrics, SaveMeasure};
use crate::registry::WorldConfig;
use crate::world::World;
use manifest::{Manifest, StoreEntry, hex};

/// The world's store: its day and its core, which the world hash reads.
pub const CORE: &str = "core";
/// The run's own record beside the world, outside the world hash: its measures.
pub const RUN: &str = "run";

fn file_of(name: &str) -> String {
    format!("{name}.zst")
}

/// One store as written: its size compressed and before compression, and the root of its frames' hashes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRecord {
    pub name: &'static str,
    pub bytes: u64,
    pub raw_bytes: u64,
    pub hash: u128,
}

/// A save as written: where, at the close of which day, its stores, the run's record, and the world hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRecord {
    pub dir: PathBuf,
    pub day: Day,
    pub stores: Vec<StoreRecord>,
    pub run_bytes: u64,
    pub world_hash: u128,
}

/// Every name the build declares that a core's save holds: its kinds' and its families'.
fn names() -> Vec<&'static str> {
    let mut n: Vec<&'static str> = crate::consts::kinds::KINDS.iter().map(|k| k.name).collect();
    n.extend(crate::consts::families::ALL);
    n
}

fn io_err(path: &Path, e: &dyn core::fmt::Display) -> String {
    format!("{}: {e}", path.display())
}

/// A store written to its file in fixed frames, each wave of frames compressed at once on the pool where there is one.
/// A store written in frames, each compressed and hashed by the worker that holds it: its bytes compressed and before
/// compression, and the root of its frames' hashes.
fn write_store(
    dir: &Path,
    name: &str,
    pool: Option<&phx_exec::Pool>,
    write: &dyn Fn(&mut Writer<'_>),
) -> Result<(u64, u64, u128), String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::create(&path).map_err(|e| io_err(&path, &e))?;
    let mut out = BufWriter::new(file);
    let compress = |frames: &[Vec<u8>]| {
        phx_exec::map_chunks(pool, frames.iter().collect(), |f| phx_store::save::seal_frame(HASH_KEY, f))
    };
    let mut w = Writer::framed(&mut out, &compress);
    write(&mut w);
    let written = w.finish_root(HASH_KEY).map_err(|e| io_err(&path, &e))?;
    out.flush().map_err(|e| io_err(&path, &e))?;
    Ok(written)
}

/// A store's file opened and read by `read`, which must consume it whole, and the root of the frames it read.
fn read_store<T>(
    dir: &Path,
    name: &str,
    read: &mut dyn FnMut(&mut Reader<'_>) -> Result<T, LoadError>,
) -> Result<(T, u128), String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::open(&path).map_err(|e| io_err(&path, &e))?;
    let mut input = BufReader::new(file);
    let mut r = Reader::new(&mut input).map_err(|e| io_err(&path, &e))?;
    r.with_names(&names());
    r.hash_frames(HASH_KEY);
    let value = read(&mut r).map_err(|e| io_err(&path, &e))?;
    if !r.at_end().map_err(|e| io_err(&path, &e))? {
        return Err(format!("{}: bytes past the store's end", path.display()));
    }
    let Some(root) = r.frame_root() else {
        return Err(format!("{}: its frames were not hashed", path.display()));
    };
    Ok((value, root))
}

/// The world's day and core read back, the core holding the address space its stores were reserved in.
fn read_core(r: &mut Reader<'_>) -> Result<(Day, Core), LoadError> {
    let today = Day::load(r)?;
    let mut core = Core::load(r)?;
    core.space = r.take_space();
    Ok((today, core))
}

impl World {
    /// A full save at this day's close into `root`: the world's store and the run's record written, each by a task of
    /// its own whose frames are compressed and hashed on the pool — the world's store's root is the world hash — then
    /// the manifest, then the save made complete and the one before it removed. The world is paused while it writes.
    ///
    /// # Errors
    /// When a file cannot be written, synced or renamed.
    #[clause("SET.12", "SET.13", "N8.10")]
    pub fn save(&self, root: &Path, build: &str) -> Result<SaveRecord, String> {
        let dir = retention::begin(root, self.today)?;
        let pool = self.pool.as_ref();
        // Each store's frames are sealed on the pool as it writes them, so the stores are written one after the other.
        let (bytes, raw_bytes, hash) = write_store(&dir, CORE, pool, &|w| {
            self.today.save(w);
            self.core.save(w);
        })?;
        let (run_bytes, run_raw, _) = write_store(&dir, RUN, pool, &|w| self.metrics.save(w))?;
        let stores = vec![StoreRecord { name: CORE, bytes, raw_bytes, hash }];
        let entry = |name: &str, bytes: u64, raw_bytes: u64, hash: String| StoreEntry {
            name: name.to_owned(),
            file: file_of(name),
            bytes,
            raw_bytes,
            hash,
        };
        let date = self.calendar.date(self.today);
        Manifest {
            format: SAVE_FORMAT,
            build: build.to_owned(),
            register: hex(self.register_hash),
            seed: self.seed,
            day: self.today.get(),
            date: format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
            settling_years: self.settling_years,
            persons: self.persons,
            stores: vec![entry(CORE, bytes, raw_bytes, hex(hash)), entry(RUN, run_bytes, run_raw, String::new())],
            world_hash: hex(hash),
        }
        .write(&dir)?;
        let dir = retention::commit(root, &dir, self.today)?;
        Ok(SaveRecord { dir, day: self.today, stores, run_bytes, world_hash: hash })
    }

    /// The save check: the save's world read from its files alone, its frames hashed as they are read, then dropped.
    /// The root they give must be the one written in the manifest, the world hash of the close the save was taken at.
    ///
    /// # Errors
    /// When the store cannot be read, or the hash its files give is not the manifest's.
    #[clause("SET.15")]
    pub fn check_save(&self, dir: &Path) -> Result<u128, String> {
        let manifest = Manifest::read(dir)?;
        let (_, got) = read_store(dir, CORE, &mut read_core)?;
        if hex(got) != manifest.world_hash {
            return Err(format!("the save reads back as {} where the close hashed {}", hex(got), manifest.world_hash));
        }
        Ok(got)
    }

    /// A save's measures kept with the run's, outside the world.
    pub fn record_save(&mut self, m: SaveMeasure) {
        self.metrics.saves.push(m);
    }

    /// An injection's outcome kept with the run's measures, outside the world.
    pub fn record_injection(&mut self, r: crate::metrics::InjectionRecord) {
        self.metrics.injections.push(r);
    }
}

/// Whether this build reads a save of a format: only its own, since a format names what each store holds and how.
///
/// # Errors
/// A save of another format.
fn readable(format: u32) -> Result<(), String> {
    if format == SAVE_FORMAT {
        Ok(())
    } else {
        Err(format!("a save of format {format} where this build reads {SAVE_FORMAT}"))
    }
}

/// Whether this build and run read a save by its manifest: its format, its build and its seed must be theirs.
///
/// # Errors
/// A save of another format, build or seed, with its reason.
#[clause("SET.15")]
fn admitted(manifest: &Manifest, build: &str, seed: u64) -> Result<(), String> {
    readable(manifest.format)?;
    if manifest.build != build {
        return Err("a save another build wrote".to_owned());
    }
    if manifest.seed != seed {
        return Err(format!("a save of seed {} where the run's is {seed}", manifest.seed));
    }
    Ok(())
}

/// A world read back from a save to continue from its close: the build and data assembled as the save's were, its
/// opening not drawn, its core and day read from the save and held to the manifest's world hash, and the run's
/// measures read back beside it. The save's format, build, register and seed must be this build's and run's.
///
/// # Errors
/// A save that is incomplete, damaged, of another build, format, register or seed, or whose world does not hash to
/// its close's.
#[clause("SET.15", "N5")]
pub fn load(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
    dir: &Path,
    build: &str,
) -> Result<World, String> {
    let manifest = Manifest::read(dir)?;
    admitted(&manifest, build, config.seed)?;
    let mut world = crate::registry::assemble_loaded(systems, interfaces, config, &mut |register| {
        if hex(register) != manifest.register {
            return Err("a save of other data than this run's".to_owned());
        }
        let ((today, core), root) = read_store(dir, CORE, &mut read_core)?;
        if hex(root) != manifest.world_hash {
            return Err("the save's world does not hash to its close's".to_owned());
        }
        Ok((today, core))
    })?;
    if world.persons != manifest.persons {
        return Err(format!("a save of {} persons where the run opens {}", manifest.persons, world.persons));
    }
    // Every index left out of the save and naming its rebuild is rebuilt now, the world rebound around it.
    phx_store::rebuild(&mut world.core).map_err(|e| e.to_string())?;
    world.metrics = read_store(dir, RUN, &mut |r| Metrics::load(r))?.0;
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::{admitted, readable};
    use crate::consts::SAVE_FORMAT;
    use crate::save::manifest::Manifest;

    fn manifest(build: &str, seed: u64) -> Manifest {
        Manifest {
            format: SAVE_FORMAT,
            build: build.to_owned(),
            register: String::new(),
            seed,
            day: 0,
            date: String::new(),
            settling_years: 0,
            persons: 0,
            stores: Vec::new(),
            world_hash: String::new(),
        }
    }

    #[test]
    fn other_build_refused() {
        assert!(admitted(&manifest("b1", 7), "b1", 7).is_ok());
        assert_eq!(admitted(&manifest("b0", 7), "b1", 7).unwrap_err(), "a save another build wrote");
        assert!(admitted(&manifest("b1", 8), "b1", 7).unwrap_err().contains("seed 8"), "a save of another seed");
        let old = Manifest { format: SAVE_FORMAT - 1, ..manifest("b1", 7) };
        assert!(admitted(&old, "b1", 7).is_err(), "a save of another format");
    }

    #[test]
    fn save_format_rises_with_the_wheel() {
        let before_the_wheel = SAVE_FORMAT - 1;
        assert!(readable(before_the_wheel).is_err(), "a save whose wheels hold another horizon is refused");
        assert!(readable(SAVE_FORMAT).is_ok());
    }
}
