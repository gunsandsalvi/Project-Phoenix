//! Saves: every store written whole at a day's close, read back exactly, and checked by reading the files alone.

pub mod inject;
pub mod manifest;
pub mod retention;

use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use phx_core::{Bindings, EventStore, KernelTable, PlayerQueue, RecordKindDecl, RecordStore};
use phx_id::Day;
use phx_macros::clause;
use phx_store::{LoadError, LogicalHasher, Reader, Saved, Writer};

use crate::consts::{HASH_KEY, SAVE_FORMAT};
use crate::hash::{hash_books, hash_events, hash_geo, hash_records, hash_world_store};
use crate::world::World;
use manifest::{Manifest, StoreEntry, hex};

/// The stores of a save, in the order the world hash reads them.
pub const STORES: [&str; 8] = ["world", "books", "population", "markets", "accounts", "records", "events", "geo"];
/// The run's own record beside the world: its metrics, findings, settlements, the opening's report and the trace.
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

/// The day and what it carries over to the next: the fails the contract process has not yet read, the player's
/// queue, the bindings and the closed fact.
#[derive(Debug)]
pub(crate) struct Carried {
    pub today: Day,
    pub unprocessed: Vec<phx_ledger::fails::Fail>,
    pub queue: PlayerQueue,
    pub bindings: Bindings,
    pub closed: phx_ledger::pending::Closed,
}

/// The run's own record, outside the world hash.
#[derive(Debug)]
pub(crate) struct RunRecord {
    pub metrics: crate::metrics::Metrics,
    pub findings: phx_core::Findings,
    pub settlements: Vec<crate::world::Settled>,
    pub report: phx_core::GenReport,
    pub trace: crate::trace::TraceLog,
    pub traced_first: Vec<crate::graph::HandlerId>,
}

/// What reading a save needs from the build: the names it declares, empty books carrying its books' declarations,
/// its record kinds, and what the accounting standard permits with each kind's legal form.
pub(crate) struct BuildContext {
    pub names: Vec<&'static str>,
    pub declared: phx_ledger::books::Books,
    /// The population kinds with the processes on each, over which a save's agenda is read back, and the
    /// representation the build holds.
    pub pop: Vec<(phx_pop::kind::PopKindDecl, usize)>,
    pub representation: phx_pop::prims::Representation,
    pub record_kinds: Vec<RecordKindDecl>,
    pub permitted: Vec<phx_core::Permitted>,
    pub forms: std::collections::BTreeMap<&'static str, String>,
}

fn io_err(path: &Path, e: &dyn core::fmt::Display) -> String {
    format!("{}: {e}", path.display())
}

fn write_store(dir: &Path, name: &str, write: &dyn Fn(&mut Writer<'_>)) -> Result<(u64, u64), String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::create(&path).map_err(|e| io_err(&path, &e))?;
    let mut out = BufWriter::new(file);
    let mut w = Writer::new(&mut out).map_err(|e| io_err(&path, &e))?;
    write(&mut w);
    let sizes = w.finish().map_err(|e| io_err(&path, &e))?;
    out.flush().map_err(|e| io_err(&path, &e))?;
    Ok(sizes)
}

/// A store's file opened and read by `read`, which must consume it whole.
pub(crate) fn read_store<T>(
    dir: &Path,
    name: &str,
    names: &[&'static str],
    read: &mut dyn FnMut(&mut Reader<'_>) -> Result<T, LoadError>,
) -> Result<T, String> {
    let path = dir.join(file_of(name));
    let file = std::fs::File::open(&path).map_err(|e| io_err(&path, &e))?;
    let mut input = BufReader::new(file);
    let mut r = Reader::new(&mut input).map_err(|e| io_err(&path, &e))?;
    r.with_names(names);
    let value = read(&mut r).map_err(|e| io_err(&path, &e))?;
    if !r.at_end().map_err(|e| io_err(&path, &e))? {
        return Err(format!("{}: bytes past the store's end", path.display()));
    }
    Ok(value)
}

impl World {
    fn write_one(&self, name: &str, w: &mut Writer<'_>) {
        match name {
            "world" => {
                self.today.save(w);
                self.unprocessed.save(w);
                self.queue.save(w);
                self.bindings.save(w);
                self.closed.save(w);
            }
            "books" => self.books.save_to(w),
            "population" => self.population.save_to(w),
            "markets" => self.markets.save(w),
            "accounts" => self.accounts.save_to(w),
            "records" => self.records.save_to(w),
            "events" => self.events.save(w),
            "geo" => {
                self.geo().save(w);
                self.tables.save(w);
            }
            _ => {
                self.metrics.save(w);
                self.findings.save(w);
                self.settlements.save(w);
                self.report.save(w);
                self.trace.save(w);
                self.traced_first.save(w);
            }
        }
    }

    fn hash_one(&self, name: &str, h: &mut LogicalHasher) {
        match name {
            "world" => hash_world_store(h, self.today, &self.unprocessed, &self.queue, &self.bindings, &self.closed),
            "books" => hash_books(h, &self.books),
            "population" => self.population.hash_into(h),
            "markets" => self.markets.hash_into(h),
            "accounts" => self.accounts.hash_into(h),
            "records" => hash_records(h, &self.records),
            "events" => hash_events(h, &self.events),
            _ => hash_geo(h, self.geo(), &self.tables),
        }
    }

    /// A full save at this day's close into `root`: every store written whole, then the manifest, then the save made
    /// complete and the one before it removed. The world is paused while it writes.
    ///
    /// # Errors
    /// When a file cannot be written, synced or renamed.
    #[clause("SET.12", "SET.13", "N8.10")]
    pub fn save(&self, root: &Path, build: &str) -> Result<SaveRecord, String> {
        let dir = retention::begin(root, self.today)?;
        let mut stores = Vec::new();
        let mut whole = LogicalHasher::new(HASH_KEY);
        for name in STORES {
            let (bytes, raw_bytes) = write_store(&dir, name, &|w| self.write_one(name, w))?;
            let mut h = LogicalHasher::new(HASH_KEY);
            self.hash_one(name, &mut h);
            self.hash_one(name, &mut whole);
            stores.push(StoreRecord { name, bytes, raw_bytes, hash: h.finish() });
        }
        let (run_bytes, run_raw) = write_store(&dir, RUN, &|w| self.write_one(RUN, w))?;
        let world_hash = whole.finish();
        let mut entries: Vec<StoreEntry> = stores
            .iter()
            .map(|s| StoreEntry {
                name: s.name.to_owned(),
                file: file_of(s.name),
                bytes: s.bytes,
                raw_bytes: s.raw_bytes,
                hash: hex(s.hash),
            })
            .collect();
        entries.push(StoreEntry {
            name: RUN.to_owned(),
            file: file_of(RUN),
            bytes: run_bytes,
            raw_bytes: run_raw,
            hash: String::new(),
        });
        let date = self.calendar.date(self.today);
        Manifest {
            format: SAVE_FORMAT,
            build: build.to_owned(),
            register: hex(self.register_hash),
            seed: self.seed,
            day: self.today.get(),
            date: format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
            settling_years: self.settling_years.get(),
            read_trace: self.read_trace,
            multiplicity: self.population.representation.multiplicity,
            population_divisor: self.population.representation.population_divisor,
            stores: entries,
            world_hash: hex(world_hash),
        }
        .write(&dir)?;
        let dir = retention::commit(root, &dir, self.today)?;
        Ok(SaveRecord { dir, day: self.today, stores, run_bytes, world_hash })
    }

    /// The save check: the save's stores read from their files one at a time, each hashed as it is decoded and then
    /// dropped, so no second world is held. The hash they give must be the one written in the manifest, the world
    /// hash of the close the save was taken at.
    ///
    /// # Errors
    /// When a store cannot be read, or the hash its files give is not the manifest's.
    #[clause("SET.15")]
    pub fn check_save(&self, dir: &Path) -> Result<u128, String> {
        let manifest = Manifest::read(dir)?;
        let mut ctx = self.build_context();
        let mut h = LogicalHasher::new(HASH_KEY);
        for name in STORES {
            let mut one = LogicalHasher::new(HASH_KEY);
            read_and_hash(dir, name, &mut ctx, &mut [&mut h, &mut one])?;
            if hex(one.finish()) != manifest.store(name)?.hash {
                return Err(format!("store `{name}` reads back with another hash than was written"));
            }
        }
        let got = h.finish();
        if hex(got) != manifest.world_hash {
            return Err(format!("the save reads back as {} where the close hashed {}", hex(got), manifest.world_hash));
        }
        Ok(got)
    }

    /// A save's measures kept with the run's, outside the world.
    pub fn record_save(&mut self, m: crate::metrics::SaveMeasure) {
        self.metrics.saves.push(m);
    }

    /// An injection's outcome kept with the run's measures, outside the world.
    pub fn record_injection(&mut self, r: crate::metrics::InjectionRecord) {
        self.metrics.injections.push(r);
    }

    /// What reading this world's saves needs from its build.
    pub(crate) fn build_context(&self) -> BuildContext {
        let pop: Vec<(phx_pop::kind::PopKindDecl, usize)> =
            self.population.kinds.iter().map(|k| (k.decl.clone(), k.processes)).collect();
        let decls: Vec<phx_pop::kind::PopKindDecl> = pop.iter().map(|(d, _)| d.clone()).collect();
        BuildContext {
            names: self.names.clone(),
            declared: self.books.declared(crate::opening::books::size(), crate::opening::books::agent_tables(&decls)),
            pop,
            representation: self.population.representation,
            record_kinds: self.records.kinds().to_vec(),
            permitted: self.accounts.permitted().to_vec(),
            forms: self.accounts.forms().clone(),
        }
    }
}

/// One store read from its file and fed to each hasher, then dropped.
fn read_and_hash(dir: &Path, name: &str, ctx: &mut BuildContext, hs: &mut [&mut LogicalHasher]) -> Result<(), String> {
    let names = ctx.names.clone();
    match name {
        "world" => {
            let c = read_store(dir, name, &names, &mut read_carried)?;
            for h in hs.iter_mut() {
                hash_world_store(h, c.today, &c.unprocessed, &c.queue, &c.bindings, &c.closed);
            }
        }
        "books" => {
            let decls: Vec<phx_pop::kind::PopKindDecl> = ctx.pop.iter().map(|(d, _)| d.clone()).collect();
            let tables = crate::opening::books::agent_tables(&decls);
            let mut declared = Some(ctx.declared.declared(crate::opening::books::size(), tables));
            let books = read_store(dir, name, &names, &mut |r| {
                let d = declared.take().ok_or_else(|| LoadError::Invalid("the books read twice".to_owned()))?;
                phx_ledger::books::Books::load_from(r, d)
            })?;
            for h in hs.iter_mut() {
                hash_books(h, &books);
            }
        }
        "population" => {
            let first = ctx.declared.parties.first_cell_place();
            let mut space = phx_store::AddressSpace::empty();
            let (pop, rep) = (ctx.pop.clone(), ctx.representation);
            let mut population = phx_pop::population::Population::new(pop, rep, first, phx_id::Day::new(0), &mut space);
            read_store(dir, name, &names, &mut |r| population.load_from(r, &mut space))?;
            for h in hs.iter_mut() {
                population.hash_into(h);
            }
        }
        "markets" => {
            let m: phx_market::markets::Markets = read_store(dir, name, &names, &mut |r| Saved::load(r))?;
            for h in hs.iter_mut() {
                m.hash_into(h);
            }
        }
        "accounts" => {
            let (permitted, forms) = (ctx.permitted.clone(), ctx.forms.clone());
            let a = read_store(dir, name, &names, &mut |r| {
                phx_acct::accounts::Accounts::load_from(r, permitted.clone(), forms.clone())
            })?;
            for h in hs.iter_mut() {
                a.hash_into(h);
            }
        }
        "records" => {
            let kinds = ctx.record_kinds.clone();
            let rec: RecordStore = read_store(dir, name, &names, &mut |r| RecordStore::load_from(r, kinds.clone()))?;
            for h in hs.iter_mut() {
                hash_records(h, &rec);
            }
        }
        "events" => {
            let ev: EventStore = read_store(dir, name, &names, &mut |r| Saved::load(r))?;
            for h in hs.iter_mut() {
                hash_events(h, &ev);
            }
        }
        _ => {
            let (geo, tables) = read_store(dir, name, &names, &mut read_geo)?;
            for h in hs.iter_mut() {
                hash_geo(h, &geo, &tables);
            }
        }
    }
    Ok(())
}

pub(crate) fn read_carried(r: &mut Reader<'_>) -> Result<Carried, LoadError> {
    Ok(Carried {
        today: Day::load(r)?,
        unprocessed: Saved::load(r)?,
        queue: Saved::load(r)?,
        bindings: Saved::load(r)?,
        closed: Saved::load(r)?,
    })
}

pub(crate) fn read_geo(r: &mut Reader<'_>) -> Result<(phx_geo::GeoState, Vec<KernelTable>), LoadError> {
    let geo = phx_geo::GeoState::load(r)?;
    let tables = Saved::load(r)?;
    Ok((geo, tables))
}

pub(crate) fn read_run(r: &mut Reader<'_>) -> Result<RunRecord, LoadError> {
    Ok(RunRecord {
        metrics: Saved::load(r)?,
        findings: Saved::load(r)?,
        settlements: Saved::load(r)?,
        report: Saved::load(r)?,
        trace: Saved::load(r)?,
        traced_first: Saved::load(r)?,
    })
}
