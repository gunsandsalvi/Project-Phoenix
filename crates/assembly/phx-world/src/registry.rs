use std::path::{Path, PathBuf};
use std::sync::Arc;

use phx_audit::{Audit, kernel_families};
use phx_core::{
    Bindings, CountryEntry, DataFile, DayMessages, Declarations, EventStore, Findings, HandlerTable, ItemDecl,
    KernelTable, OpeningCtx, PlayerQueue, RecordStore, System, SystemEntry, declare_entry,
};
use phx_geo::state::MAP_PHASE;
use phx_geo::{Allotment, GeoState};
use phx_id::{CountryId, SystemCode};
use phx_num::Missing;
use phx_rand::Seed;
use phx_store::AddressSpace;

use crate::compile::{Compiled, KernelPrims, compile};
use crate::consts::{EVENT_ROWS, RECORD_ROWS, STORE_ARENA_WORDS};
use crate::metrics::Metrics;
use crate::opening::newgame::{NewGame, instantiate, new_game};
use crate::refusals::{AssemblyErrors, refusals};
use crate::trace::TraceLog;
use crate::world::{OwnState, World};

/// How a run is set up: its one seed, where its data and its new game's setup lie, the run's own directory, where
/// the new game's countries are instantiated, and whether reads are traced.
#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    pub data: PathBuf,
    pub setup: PathBuf,
    pub run_dir: PathBuf,
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

/// The data a world reads: its constants, the shared primitives, and each country's own primitives, instantiated in
/// the run's directory by its new game.
fn data_files(root: &Path, countries: &[PathBuf]) -> Result<Vec<DataFile>, String> {
    let mut files = vec![read(&root.join("world.toml"), Missing::Absent)?];
    for path in toml_files(&root.join("shared"))? {
        files.push(read(&path, Missing::Absent)?);
    }
    for (i, dir) in countries.iter().enumerate() {
        let id = CountryId::new(u8::try_from(i).map_err(|_| format!("{} countries", countries.len()))?);
        for path in toml_files(dir)? {
            files.push(read(&path, Missing::Present(id))?);
        }
    }
    Ok(files)
}

/// The map, generated in the opening's map phase from the new game's allotment, and everything GEO reads from it.
fn open_map(kernel: &KernelPrims, c: &Compiled, game: &NewGame, d: &Declarations) -> Result<GeoState, AssemblyErrors> {
    let allotment = Allotment {
        land: game.countries.iter().map(|g| g.land_tiles).collect(),
        regions: game.countries.iter().map(|g| g.regions).collect(),
    };
    let names: Vec<&str> = d.events.iter().map(|(_, e)| e.name).collect();
    let kind = |name: &str| names.iter().position(|n| *n == name).and_then(|i| u16::try_from(i).ok());
    let ctx = OpeningCtx::new(&c.streams, MAP_PHASE);
    GeoState::build(&kernel.geo, &c.register, &allotment, &ctx, &kind).map_err(AssemblyErrors)
}

/// Kinds whose legal form the law does not declare.
fn unlawful_kinds(d: &Declarations, kernel: &KernelPrims, register: &phx_core::Register) -> Vec<String> {
    let forms = kernel.legal_forms.shared(register);
    d.kinds
        .iter()
        .filter(|(_, kind)| !forms.iter().any(|f| f.name == kind.legal_form))
        .map(|(_, kind)| {
            format!("kind `{}` takes the legal form `{}`, which the law does not declare", kind.name, kind.legal_form)
        })
        .collect()
}

/// The world's books opened from the setup's countries, their people and their currencies' units.
fn open(
    d: &mut Declarations,
    kernel: &KernelPrims,
    c: &crate::compile::Compiled,
    game: &NewGame,
    geo: &phx_geo::GeoState,
) -> (phx_ledger::books::Books, phx_core::GenReport) {
    let population = kernel.opening.population.shared(&c.register).get();
    let units: Vec<u64> = (0_u8..)
        .take(game.countries.len())
        .map(|i| kernel.opening.units_per_dollar.get(&c.register, CountryId::new(i)).get())
        .collect();
    let countries = crate::opening::books::countries(game, geo, population, &units);
    crate::opening::books::open_books(
        d,
        &c.register,
        &c.streams,
        &countries,
        (c.day_zero, kernel.day_zero.shared(&c.register)),
        &c.calendar,
    )
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
    let game = new_game(&config.data, &config.setup, Seed::new(config.seed)).map_err(AssemblyErrors)?;
    let dirs = instantiate(&game, &config.data, &config.run_dir).map_err(one)?;
    let levels: Vec<CountryEntry> = game.levels.iter().map(|level| CountryEntry { level: *level }).collect();
    let files = data_files(&config.data, &dirs).map_err(one)?;
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
    let Some(c) = compiled else {
        return Err(AssemblyErrors(errors));
    };
    errors.extend(unlawful_kinds(&d, &kernel, &c.register));
    if !errors.is_empty() {
        return Err(AssemblyErrors(errors));
    }
    let geo = Arc::new(open_map(&kernel, &c, &game, &d)?);
    let countries = u32::try_from(game.countries.len()).map_err(|e| one(e.to_string()))?;
    let (books, report) = open(&mut d, &kernel, &c, &game, &geo);
    let mut families = kernel_families();
    families.extend(std::mem::take(&mut d.families).into_iter().map(|(_, f)| f));
    families.push(Box::new(phx_geo::audit::Places { geo: Arc::clone(&geo), countries: game.countries.len() }));
    families.push(Box::new(phx_geo::audit::Deposits));
    families.extend(phx_ledger::audit::families());
    let audit = Audit::new(families).map_err(AssemblyErrors)?;
    let settling_years = kernel.opening.settling_years.shared(&c.register);
    let nothing = || -> OwnState { Box::new(()) };
    let mut own: Vec<(&'static str, OwnState)> = entries.iter().map(|e| (e.code, nothing())).collect();
    for (code, state) in &mut own {
        if *code == phx_geo::Geo::CODE {
            *state = Box::new(Arc::clone(&geo));
        }
    }
    let tables: Vec<KernelTable> = phx_geo::tables(&geo, countries);
    let kept = |name: &str| tables.iter().any(|t| t.name == name);
    let unkept: Vec<String> = h
        .entries
        .iter()
        .filter(|e| !kept(e.table))
        .map(|e| format!("handler `{}` runs on `{}`, a table the world does not keep", e.name, e.table))
        .collect();
    if !unkept.is_empty() {
        return Err(AssemblyErrors(unkept));
    }
    let mut space = AddressSpace::empty();
    let record_kinds = d.records.iter().map(|(_, r)| *r).collect();
    Ok(World {
        records: RecordStore::new(&mut space, record_kinds, RECORD_ROWS, STORE_ARENA_WORDS),
        events: EventStore::new(&mut space, EVENT_ROWS, phx_store::consts::DEFAULT_ROWS_PER_CHUNK, STORE_ARENA_WORDS),
        calendar: c.calendar,
        register: c.register,
        streams: c.streams,
        graph: c.graph,
        rules: std::mem::take(&mut d.rules),
        own,
        tables,
        event_kinds: d.events.iter().map(|(_, e)| *e).collect(),
        bindings: Bindings::default(),
        countries: levels,
        day_zero: c.day_zero,
        today: c.day_zero,
        settling_years,
        books,
        report,
        unprocessed: Vec::new(),
        due: phx_ledger::due::DueLines::default(),
        closed: phx_ledger::pending::Closed::default(),
        settlements: Vec::new(),
        day_messages: DayMessages::default(),
        queue: PlayerQueue::default(),
        game,
        audit,
        read_trace: config.read_trace,
        metrics: Metrics::default(),
        findings: Findings::default(),
        trace: TraceLog::default(),
        traced_first: Vec::new(),
        geo,
        space,
    })
}
