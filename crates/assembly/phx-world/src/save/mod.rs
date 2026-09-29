//! Saves: the world written whole at a day's close, read back exactly, and checked by reading its files alone.

pub mod manifest;
pub mod retention;

use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use phx_core::{ItemDecl, SystemEntry};
use phx_id::Day;
use phx_macros::clause;
use phx_store::{LoadError, LogicalHasher, Reader, Saved, Writer, hash_saved};

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

/// One store as written: its size compressed and before compression, and its logical hash.
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
    let mut n = crate::consts::kinds::KINDS.to_vec();
    n.extend(crate::consts::families::ALL);
    n
}

fn io_err(path: &Path, e: &dyn core::fmt::Display) -> String {
    format!("{}: {e}", path.display())
}

/// A store written to its file in fixed frames, each wave of frames compressed at once on the pool where there is one.
fn write_store(
    dir: &Path,
    name: &str,
    pool: Option<&phx_exec::Pool>,
    write: &dyn Fn(&mut Writer<'_>),
) -> Result<(u64, u64), String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::create(&path).map_err(|e| io_err(&path, &e))?;
    let mut out = BufWriter::new(file);
    let compress = |frames: &[Vec<u8>]| {
        phx_exec::pool::map(pool, frames.len(), |i| {
            frames.get(i).map_or_else(|| Ok(Vec::new()), |f| phx_store::save::compress_frame(f))
        })
    };
    let mut w = Writer::framed(&mut out, &compress);
    write(&mut w);
    let sizes = w.finish().map_err(|e| io_err(&path, &e))?;
    out.flush().map_err(|e| io_err(&path, &e))?;
    Ok(sizes)
}

/// A store's file opened and read by `read`, which must consume it whole.
fn read_store<T>(
    dir: &Path,
    name: &str,
    read: &mut dyn FnMut(&mut Reader<'_>) -> Result<T, LoadError>,
) -> Result<T, String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::open(&path).map_err(|e| io_err(&path, &e))?;
    let mut input = BufReader::new(file);
    let mut r = Reader::new(&mut input).map_err(|e| io_err(&path, &e))?;
    r.with_names(&names());
    let value = read(&mut r).map_err(|e| io_err(&path, &e))?;
    if !r.at_end().map_err(|e| io_err(&path, &e))? {
        return Err(format!("{}: bytes past the store's end", path.display()));
    }
    Ok(value)
}

/// The world's day and core read back, the core holding the address space its stores were reserved in.
fn read_core(r: &mut Reader<'_>) -> Result<(Day, Core), LoadError> {
    let today = Day::load(r)?;
    let mut core = Core::load(r)?;
    core.space = r.take_space();
    Ok((today, core))
}

/// The world hash of a day and its core: their logical content, in the order the store holds them.
fn world_hash(today: Day, core: &Core) -> u128 {
    let mut h = LogicalHasher::new(HASH_KEY);
    hash_saved(&today, &mut h);
    hash_saved(core, &mut h);
    h.finish()
}

impl World {
    /// A full save at this day's close into `root`: the world's store and the run's record written, each by a task of
    /// its own beside one hashing the world, then the manifest, then the save made complete and the one before it
    /// removed. The world is paused while it writes.
    ///
    /// # Errors
    /// When a file cannot be written, synced or renamed.
    #[clause("SET.12", "SET.13", "N8.10")]
    pub fn save(&self, root: &Path, build: &str) -> Result<SaveRecord, String> {
        let dir = retention::begin(root, self.today)?;
        let pool = self.pool.as_ref();
        let done = phx_exec::pool::map(pool, crate::consts::SAVE_TASKS, |i| match i {
            0 => write_store(&dir, CORE, pool, &|w| {
                self.today.save(w);
                self.core.save(w);
            })
            .map(|s| (s, 0)),
            1 => write_store(&dir, RUN, pool, &|w| self.metrics.save(w)).map(|s| (s, 0)),
            _ => Ok(((0, 0), world_hash(self.today, &self.core))),
        });
        let mut done = done.into_iter().collect::<Result<Vec<_>, String>>()?.into_iter();
        let (Some(((bytes, raw_bytes), _)), Some(((run_bytes, run_raw), _)), Some((_, hash))) =
            (done.next(), done.next(), done.next())
        else {
            return Err("a save task that never ran".to_owned());
        };
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

    /// The save check: the save's world read from its files alone and hashed, then dropped. The hash it gives must be
    /// the one written in the manifest, the world hash of the close the save was taken at.
    ///
    /// # Errors
    /// When the store cannot be read, or the hash its files give is not the manifest's.
    #[clause("SET.15")]
    pub fn check_save(&self, dir: &Path) -> Result<u128, String> {
        let manifest = Manifest::read(dir)?;
        let (today, core) = read_store(dir, CORE, &mut read_core)?;
        let got = world_hash(today, &core);
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
    if manifest.format != SAVE_FORMAT {
        return Err(format!("a save of format {} where this build reads {SAVE_FORMAT}", manifest.format));
    }
    if manifest.build != build {
        return Err("a save another build wrote".to_owned());
    }
    if manifest.seed != config.seed {
        return Err(format!("a save of seed {} where the run's is {}", manifest.seed, config.seed));
    }
    let mut world = crate::registry::assemble_loaded(systems, interfaces, config, &mut |register| {
        if hex(register) != manifest.register {
            return Err("a save of other data than this run's".to_owned());
        }
        let (today, core) = read_store(dir, CORE, &mut read_core)?;
        if hex(world_hash(today, &core)) != manifest.world_hash {
            return Err("the save's world does not hash to its close's".to_owned());
        }
        Ok((today, core))
    })?;
    if world.persons != manifest.persons {
        return Err(format!("a save of {} persons where the run opens {}", manifest.persons, world.persons));
    }
    world.metrics = read_store(dir, RUN, &mut |r| Metrics::load(r))?;
    Ok(world)
}
