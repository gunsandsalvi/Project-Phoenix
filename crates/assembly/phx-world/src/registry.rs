use std::path::{Path, PathBuf};

use phx_core::{
    DataFile, DayMessages, Declarations, Directory, EventStore, Findings, HandlerTable, ItemDecl, PlayerQueue,
    RecordStore, SystemEntry, countries, declare_entry,
};
use phx_id::{CountryId, SystemCode};
use phx_num::Missing;
use phx_rand::Seed;
use phx_store::AddressSpace;

use crate::compile::{KernelPrims, compile};
use crate::consts::{EVENT_ROWS, RECORD_ROWS, STORE_ARENA_WORDS};
use crate::metrics::Metrics;
use crate::refusals::{AssemblyErrors, refusals};
use crate::trace::TraceLog;
use crate::world::World;

/// How a run is set up: its one seed, where its data lies, and whether reads are traced.
#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    pub data: PathBuf,
    pub read_trace: bool,
}

fn read(path: &Path, country: Missing<CountryId>) -> Result<DataFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(DataFile { path: path.display().to_string(), country, text })
}

fn toml_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// The data a world reads: its constants, the shared primitives, and each country's primitives, which until a new
/// game instantiates them are read from the country's development level's templates.
fn data_files(root: &Path, levels: &[phx_core::CountryEntry]) -> Result<Vec<DataFile>, String> {
    let mut files = vec![read(&root.join("world.toml"), Missing::Absent)?];
    for path in toml_files(&root.join("shared"))? {
        files.push(read(&path, Missing::Absent)?);
    }
    for (i, country) in levels.iter().enumerate() {
        let id = CountryId::new(u8::try_from(i).map_err(|_| format!("{} countries", levels.len()))?);
        for path in toml_files(&root.join("profiles").join(country.level.dir()))? {
            files.push(read(&path, Missing::Present(id))?);
        }
    }
    Ok(files)
}

/// Assembles the world: every system's declarations, then every system's handlers, then compilation against the
/// data; every refusal is reported at once.
///
/// # Errors
/// Every refusal of the declarations, the handlers, the items and the data.
pub fn assemble(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
) -> Result<World, AssemblyErrors> {
    let one = |e: String| AssemblyErrors(vec![e]);
    let world_text = std::fs::read_to_string(config.data.join("world.toml")).map_err(|e| one(e.to_string()))?;
    let levels = countries(&world_text).map_err(one)?;
    let files = data_files(&config.data, &levels).map_err(one)?;
    let (mut d, mut h) = (Declarations::new(), HandlerTable::default());
    let kernel = KernelPrims::declare(&mut d);
    let entries: Vec<SystemEntry> = systems.iter().map(|s| s()).collect();
    for entry in &entries {
        declare_entry(entry, &mut d, &mut h);
    }
    let mut registered: Vec<SystemCode> = Vec::new();
    let mut errors = Vec::new();
    for entry in &entries {
        match SystemCode::new(entry.code) {
            Some(code) => registered.push(code),
            None => errors.push(format!("system code `{}` is not two to four capital letters", entry.code)),
        }
    }
    let items: Vec<ItemDecl> = interfaces.iter().flat_map(|i| i.iter().copied()).collect();
    errors.extend(refusals(&d, &h, &items, &registered));
    let compiled = match compile(&mut d, &kernel, &h.entries, &files, &levels, Seed::new(config.seed)) {
        Ok(c) => Some(c),
        Err(e) => {
            errors.extend(e);
            None
        }
    };
    let Some(c) = compiled.filter(|_| errors.is_empty()) else {
        return Err(AssemblyErrors(errors));
    };
    let mut space = AddressSpace::empty();
    let record_kinds = d.records.iter().map(|(_, r)| *r).collect();
    Ok(World {
        records: RecordStore::new(&mut space, record_kinds, RECORD_ROWS, STORE_ARENA_WORDS),
        events: EventStore::new(&mut space, EVENT_ROWS, phx_store::consts::DEFAULT_ROWS_PER_CHUNK, STORE_ARENA_WORDS),
        calendar: c.calendar,
        register: c.register,
        streams: c.streams,
        graph: c.graph,
        countries: levels,
        day_zero: c.day_zero,
        today: c.day_zero,
        directory: Directory::new(),
        day_messages: DayMessages::default(),
        queue: PlayerQueue::default(),
        read_trace: config.read_trace,
        metrics: Metrics::default(),
        findings: Findings::default(),
        trace: TraceLog::default(),
        space,
    })
}
