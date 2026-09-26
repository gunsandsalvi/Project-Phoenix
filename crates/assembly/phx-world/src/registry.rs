use std::path::{Path, PathBuf};
use std::sync::Arc;

use phx_audit::{Audit, kernel_families};
use phx_core::{
    Bindings, CountryEntry, DataFile, DayMessages, Declarations, EventStore, Findings, HandlerTable, ItemDecl,
    KernelTable, OpeningCtx, PlayerQueue, RecordStore, SubStep, System, SystemEntry, declare_entry,
};
use phx_geo::state::MAP_PHASE;
use phx_geo::{Allotment, GeoState};
use phx_id::{CountryId, SystemCode};
use phx_macros::clause;
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
/// the new game's countries are instantiated, whether reads are traced, and the representation the owner's switch
/// names in place of the register's, and the workers the world runs on, the opening among its work.
#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    pub data: PathBuf,
    pub setup: PathBuf,
    pub run_dir: PathBuf,
    pub read_trace: bool,
    pub representation: Missing<phx_pop::prims::Representation>,
    pub pool: Option<Arc<phx_exec::Pool>>,
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

/// The data a world reads: its constants, the shared primitives, and each country's own primitives and opening
/// tables, instantiated in the run's directory by its new game.
fn data_files(root: &Path, countries: &[PathBuf]) -> Result<Vec<DataFile>, String> {
    let mut files = vec![read(&root.join("world.toml"), Missing::Absent)?];
    for path in toml_files(&root.join("shared"))? {
        files.push(read(&path, Missing::Absent)?);
    }
    for (i, dir) in countries.iter().enumerate() {
        let id = CountryId::new(u8::try_from(i).map_err(|_| format!("{} countries", countries.len()))?);
        for path in toml_files(dir)?.into_iter().chain(toml_files(&dir.join("gen"))?) {
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
    (d, facets, visits): (&mut Declarations, &[Facet], &[crate::visits::Bound]),
    pop: (&[(phx_pop::kind::PopKindDecl, usize)], phx_pop::prims::Representation),
    kernel: &KernelPrims,
    c: &crate::compile::Compiled,
    game: &NewGame,
    geo: &phx_geo::GeoState,
    (phases, pool): (&[phx_core::OpeningPhase], Option<&Arc<phx_exec::Pool>>),
) -> (phx_ledger::books::Books, phx_pop::population::Population, phx_core::GenReport) {
    // A small world's countries hold one factor-th of the population, and everything the opening derives follows.
    let total = kernel.opening.population.shared(&c.register).get();
    let divisor = u64::from(pop.1.population_divisor);
    let population = total / divisor;
    let units: Vec<u64> = (0_u8..)
        .take(game.countries.len())
        .map(|i| kernel.opening.units_per_dollar.get(&c.register, CountryId::new(i)).get())
        .collect();
    let countries = crate::opening::books::countries(game, geo, population, &units);
    let (books, people, mut report) = crate::opening::books::open_books(
        (d, facets, &crate::visits::specs(visits)),
        pop,
        c,
        &countries,
        kernel.day_zero.shared(&c.register),
        (phases, pool),
    );
    if population * divisor != total {
        report.adjustments.push(phx_core::Adjustment {
            what: format!("the population, one {divisor}-th of the setup's in whole persons"),
            drawn: i128::from(total),
            set: i128::from(population * divisor),
        });
    }
    (books, people, report)
}

/// The month a day falls in, counted from the calendar's year nought: the period the accounts close by.
pub(crate) fn period_of(calendar: &phx_core::Calendar, day: phx_id::Day) -> u32 {
    let date = calendar.date(day);
    let months = i64::from(date.year()) * i64::from(phx_core::consts::MONTHS_PER_YEAR) + i64::from(date.month());
    let Ok(period) = u32::try_from(months) else {
        phx_num::violation!(clause = "ACC.11", "a period before the calendar's year nought", year = date.year());
    };
    period
}

/// Every party of a kind whose legal form has owners keeps an equity account, opened on the opening's books, each in
/// the currency of the country its site lies in.
/// What the accounting standard permits, and each kind's legal form, as the build declares them.
fn standard(
    d: &Declarations,
    kernel: &KernelPrims,
    c: &Compiled,
) -> (Vec<phx_core::Permitted>, std::collections::BTreeMap<&'static str, String>) {
    let forms = d.kinds.iter().map(|(_, k)| (k.name, k.legal_form.to_owned())).collect();
    (kernel.acct.carrying_bases.shared(&c.register).clone(), forms)
}

fn open_accounts(
    d: &Declarations,
    kernel: &KernelPrims,
    c: &Compiled,
    books: &phx_ledger::books::Books,
    geo: &GeoState,
) -> phx_acct::accounts::Accounts {
    let law = kernel.legal_forms.shared(&c.register);
    let owned = |form: &str| law.iter().any(|f| f.name == form && !f.owners.is_empty());
    let (permitted, forms) = standard(d, kernel, c);
    let ccy_of = |party: phx_id::PartyId| {
        let Missing::Present(country) = geo.country_of(books.parties.site(party)) else {
            phx_num::violation!(clause = "PTY.5", "a party sited on no country's land", party = party.get());
        };
        phx_ledger::opening::currency(country)
    };
    phx_acct::accounts::Accounts::open(permitted, forms, &owned, books, &ccy_of, period_of(&c.calendar, c.day_zero))
}

/// The families of the kernel crates that keep the world's map, books, markets and accounts, over what they read, and
/// the agents' where the world keeps a population kind, since only then can the family's injection reach an agent.
fn crate_families(countries: usize, agents: bool) -> Vec<Box<dyn phx_core::AuditFamily>> {
    let mut families: Vec<Box<dyn phx_core::AuditFamily>> =
        vec![Box::new(phx_geo::audit::Places { countries }), Box::new(phx_geo::audit::Deposits)];
    families.extend(phx_ledger::audit::families());
    families.push(Box::new(phx_market::audit::Prices));
    families.extend(phx_acct::audit::families());
    if agents {
        families.push(Box::new(phx_pop::audit::Agents));
    }
    families
}

/// What the build supplies before any state: the new game, every system's declarations and handlers, and the
/// register, calendar, streams and handler graph compiled against the data, with the data's hash.
struct Prepared {
    game: NewGame,
    levels: Vec<CountryEntry>,
    d: Declarations,
    /// Each population kind compiled from the systems' items with the number of processes on its persons, in the
    /// order their tables follow the kind tables; the processes; and the representation in force.
    pop: Vec<(phx_pop::kind::PopKindDecl, usize)>,
    processes: Vec<crate::agents::Bound>,
    representation: phx_pop::prims::Representation,
    h: HandlerTable,
    kernel: KernelPrims,
    c: Compiled,
    entries: Vec<SystemEntry>,
    /// Each fact a system keeps on a kind of individual: the kind, the fact's name and its declaration.
    facets: Vec<Facet>,
    /// The decisions taken on kinds' rows as they come due.
    visits: Vec<crate::visits::Bound>,
    /// The kinds under an insolvency law.
    laws: Vec<crate::defaults::Law>,
    register_hash: u128,
}

/// A fact kept as a column of a kind's table of individuals: the kind, the fact's name and its declaration.
pub(crate) type Facet = (&'static str, &'static str, phx_core::FactDecl);

/// Each fact the systems keep on kinds of individuals, found among the interfaces' facts; a fact no interface
/// exports, or not of the kind it is kept on, is refused.
fn facets(d: &Declarations, items: &[ItemDecl]) -> (Vec<Facet>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let individual =
        |kind: &str| d.kinds.iter().any(|(_, k)| k.name == kind && k.table == phx_core::KindTableRef::Individuals);
    for (system, facet) in &d.facets {
        if !individual(facet.kind) {
            errors.push(format!("{system} keeps `{}` on `{}`, which is no kind of individual", facet.fact, facet.kind));
            continue;
        }
        if out.iter().any(|(k, n, _)| *k == facet.kind && *n == facet.fact) {
            errors.push(format!("`{}` kept twice on `{}`", facet.fact, facet.kind));
            continue;
        }
        let fact = items.iter().find_map(|i| match i.kind {
            phx_core::ItemKind::Fact(f) if i.name == facet.fact => Some(f),
            _ => None,
        });
        match fact {
            Some(f) if f.kinds.contains(&facet.kind) => out.push((facet.kind, facet.fact, f)),
            Some(_) => {
                errors.push(format!("{system} keeps `{}` on `{}`, which is not its kind", facet.fact, facet.kind));
            }
            None => errors.push(format!("{system} keeps `{}`, which no interface exports as a fact", facet.fact)),
        }
    }
    (out, errors)
}

/// The world's state, however it came to be: opened by a new game, or read back from a save.
struct State {
    geo: Arc<GeoState>,
    tables: Vec<KernelTable>,
    books: phx_ledger::books::Books,
    population: phx_pop::population::Population,
    markets: phx_market::markets::Markets,
    accounts: phx_acct::accounts::Accounts,
    records: RecordStore,
    events: EventStore,
    carried: crate::save::Carried,
    run: crate::save::RunRecord,
    space: AddressSpace,
}

/// The population kinds the systems declare, each compiled from every system's items for it, with the processes on
/// its persons.
fn population_kinds(d: &mut Declarations, register: &phx_core::Register) -> Result<crate::agents::Kinds, Vec<String>> {
    let kinds: Vec<&'static str> =
        d.kinds.iter().filter(|(_, k)| k.table == phx_core::KindTableRef::Cells).map(|(_, k)| k.name).collect();
    let decls = phx_pop::population::Population::compile(&kinds, &d.pop)?;
    crate::agents::bind(d, register, decls)
}

/// The data's content, which a save names so that a load over other data is refused.
fn data_hash(files: &[DataFile]) -> u128 {
    let mut h = phx_store::LogicalHasher::new(crate::consts::HASH_KEY);
    for f in files {
        h.u64(phx_rand::float::len_u64(f.text.len()));
        h.bytes(f.text.as_bytes());
    }
    h.finish()
}

fn prepare(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
) -> Result<Prepared, AssemblyErrors> {
    let one = |e: String| AssemblyErrors(vec![e]);
    let game = new_game(&config.data, &config.setup, Seed::new(config.seed)).map_err(AssemblyErrors)?;
    let (mut d, mut h) = (Declarations::new(), HandlerTable::default());
    let kernel = KernelPrims::declare(&mut d);
    let entries: Vec<SystemEntry> = systems.iter().map(|s| s()).collect();
    for entry in &entries {
        declare_entry(entry, &mut d, &mut h);
    }
    let codes: Vec<&str> = entries.iter().map(|e| e.code).collect();
    let setup: Vec<phx_core::SetupValue> = d.setup_values.iter().map(|(_, v)| *v).collect();
    let dirs = instantiate(&game, &config.data, &config.run_dir, &codes, &setup).map_err(one)?;
    let levels: Vec<CountryEntry> = game.levels.iter().map(|level| CountryEntry { level: *level }).collect();
    let files = data_files(&config.data, &dirs).map_err(one)?;
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
    let (facets, unkept) = facets(&d, &items);
    errors.extend(unkept);

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
    let visits = crate::visits::bind(&d, &h, &c.register).unwrap_or_else(|e| {
        errors.extend(e);
        Vec::new()
    });
    let laws = crate::defaults::bind(&d, &c.register).unwrap_or_else(|e| {
        errors.extend(e);
        Vec::new()
    });
    errors.extend(unlawful_kinds(&d, &kernel, &c.register));
    let (pop, processes) = match population_kinds(&mut d, &c.register) {
        Ok(bound) => bound,
        Err(e) => {
            errors.extend(e);
            (Vec::new(), Vec::new())
        }
    };
    if !errors.is_empty() {
        return Err(AssemblyErrors(errors));
    }
    let representation = match config.representation {
        Missing::Present(r) => r,
        Missing::Absent => phx_pop::prims::Representation::of(&kernel.rep, &c.register),
    };
    Ok(Prepared {
        game,
        levels,
        d,
        pop,
        processes,
        representation,
        h,
        kernel,
        c,
        entries,
        facets,
        visits,
        laws,
        register_hash: data_hash(&files),
    })
}

/// Every name the build declares that a store keeps: kinds, record kinds, streams, decision points, the audit's
/// families and their clauses, the kernel tables and their facts, and the books' line kinds and reasons.
fn names(
    d: &Declarations,
    families: &[phx_core::FamilyDecl],
    tables: &[KernelTable],
    books: &phx_ledger::books::Books,
) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend(d.kinds.iter().map(|(_, k)| k.name));
    out.extend(d.records.iter().map(|(_, r)| r.name));
    out.extend(d.streams.iter().map(|(_, s)| s.name));
    out.extend(d.decisions.iter().map(|m| m.name));
    out.extend(d.facets.iter().map(|(_, f)| f.fact));
    out.extend(families.iter().flat_map(|f| [f.name, f.clause]));
    for t in tables {
        out.push(t.name);
        out.extend(t.columns.facts());
    }
    out.extend(books.ledger.lines.kind_names());
    out.extend(books.ledger.reasons.names());
    out.sort_unstable();
    out.dedup();
    out
}

/// The world built from what the build supplies and a state: the audit over it, each system's own state, and the
/// handlers checked against the tables it keeps.
fn finish(mut p: Prepared, s: State, config: &WorldConfig) -> Result<World, AssemblyErrors> {
    let State { geo, tables, books, population, markets, accounts, records, events, carried, run, space } = s;
    let mut families = kernel_families();
    families.extend(std::mem::take(&mut p.d.families).into_iter().map(|(_, f)| f));
    families.extend(crate_families(p.game.countries.len(), !p.pop.is_empty()));
    let mut audit = Audit::new(families).map_err(AssemblyErrors)?;
    audit.resume(records.len(), events.len());
    let decls: Vec<phx_core::FamilyDecl> = audit.families().collect();
    let names = names(&p.d, &decls, &tables, &books);
    let settling_years = p.kernel.opening.settling_years.shared(&p.c.register);
    let save_every = p.kernel.save_every.shared(&p.c.register);
    let nothing = || -> OwnState { Box::new(()) };
    let mut own: Vec<(&'static str, OwnState)> = p.entries.iter().map(|e| (e.code, nothing())).collect();
    for (code, state) in &mut own {
        if *code == phx_geo::Geo::CODE {
            *state = Box::new(Arc::clone(&geo));
        }
    }
    let mut refused = Vec::new();
    for (code, compile) in std::mem::take(&mut p.d.compiled) {
        match (compile(&p.c.register, p.game.countries.len()), own.iter_mut().find(|(c, _)| *c == code)) {
            (Ok(compiled), Some((_, state))) => *state = compiled,
            (Ok(_), None) => refused.push(format!("`{code}` compiles state but is no system of the world")),
            (Err(e), _) => refused.push(format!("{code}: {e}")),
        }
    }
    if !refused.is_empty() {
        return Err(AssemblyErrors(refused));
    }
    let kept = |name: &str| tables.iter().any(|t| t.name == name) || p.visits.iter().any(|v| v.decl.kind == name);
    let unkept: Vec<String> =
        p.h.entries
            .iter()
            .filter(|e| !kept(e.table))
            .map(|e| format!("handler `{}` runs on `{}`, a table the world does not keep", e.name, e.table))
            .collect();
    if !unkept.is_empty() {
        return Err(AssemblyErrors(unkept));
    }
    let event_kinds: Vec<phx_core::EventKindDecl> = p.d.events.iter().map(|(_, e)| *e).collect();
    let news = phx_core::EventsRule::new(p.kernel.public_events.shared(&p.c.register), &event_kinds)
        .map_err(|e| AssemblyErrors(vec![e]))?;
    let mut calendar = p.c.calendar;
    calendar.move_window(calendar.date(carried.today).year());
    Ok(World {
        records,
        events,
        calendar,
        register: p.c.register,
        streams: p.c.streams,
        graph: p.c.graph,
        rules: std::mem::take(&mut p.d.rules),
        own,
        tables,
        event_kinds,
        news,
        bindings: carried.bindings,
        countries: p.levels,
        day_zero: p.c.day_zero,
        today: carried.today,
        settling_years,
        books,
        population,
        processes: std::mem::take(&mut p.processes),
        agent_hits: Vec::new(),
        rate_sample: None,
        agent_day: crate::agents::AgentDay::of(carried.today),
        visits: std::mem::take(&mut p.visits),
        visit_due: Vec::new(),
        visit_day: carried.today,
        visit_reads: phx_core::ReadTrace::default(),
        visit_today: crate::visits::VisitDay::of(carried.today),
        laws: std::mem::take(&mut p.laws),
        defaults: std::collections::BTreeSet::new(),
        markets,
        accounts,
        report: run.report,
        unprocessed: carried.unprocessed,
        due: phx_ledger::due::DueLines::default(),
        closed: carried.closed,
        settlements: run.settlements,
        day_messages: DayMessages::default(),
        queue: carried.queue,
        game: p.game,
        audit,
        read_trace: config.read_trace,
        metrics: run.metrics,
        findings: run.findings,
        trace: run.trace,
        traced_first: run.traced_first,
        space,
        names,
        register_hash: p.register_hash,
        seed: config.seed,
        save_every,
        loaded: false,
    })
}

/// Assembles the world: every system's declarations, then every system's handlers, then compilation against the
/// data; every refusal is reported at once. The map is generated and the books opened by the new game.
///
/// # Errors
/// Every refusal of the declarations, the handlers, the items and the data.
pub fn assemble(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
) -> Result<World, AssemblyErrors> {
    let one = |e: String| AssemblyErrors(vec![e]);
    let mut p = prepare(systems, interfaces, config)?;
    let geo = Arc::new(open_map(&p.kernel, &p.c, &p.game, &p.d)?);
    let countries = u32::try_from(p.game.countries.len()).map_err(|e| one(e.to_string()))?;
    let pop = (p.pop.as_slice(), p.representation);
    let (books, population, report) = open(
        (&mut p.d, &p.facets, &p.visits),
        pop,
        &p.kernel,
        &p.c,
        &p.game,
        &geo,
        (&phx_core::PHASES, config.pool.as_ref()),
    );
    let accounts = open_accounts(&p.d, &p.kernel, &p.c, &books, &geo);
    let tables = phx_geo::tables(&geo, countries);
    let mut space = AddressSpace::empty();
    let record_kinds = p.d.records.iter().map(|(_, r)| *r).collect();
    let state = State {
        population,
        records: RecordStore::new(&mut space, record_kinds, RECORD_ROWS, STORE_ARENA_WORDS),
        events: EventStore::new(&mut space, EVENT_ROWS, phx_store::consts::DEFAULT_ROWS_PER_CHUNK, STORE_ARENA_WORDS),
        geo,
        tables,
        books,
        markets: phx_market::markets::Markets::default(),
        accounts,
        carried: crate::save::Carried {
            today: p.c.day_zero,
            unprocessed: Vec::new(),
            queue: PlayerQueue::unseated(),
            bindings: Bindings::default(),
            closed: phx_ledger::pending::Closed::default(),
        },
        run: crate::save::RunRecord {
            metrics: Metrics::default(),
            findings: Findings::default(),
            settlements: Vec::new(),
            report,
            trace: TraceLog::default(),
            traced_first: Vec::new(),
        },
        space,
    };
    let mut world = finish(p, state, config)?;
    world.open_player().map_err(one)?;
    // Every agent the opening began, the player's among them, is drawn its first bookings from the day after it.
    world.book_changed(world.today, SubStep::S10b.ordinal());
    world.visits_book_all(world.today);
    world.books.ledger.opened();
    Ok(world)
}

/// A world read back from a save to continue: the build's declarations and data compiled as for a new game, the
/// save's format, build and data checked against them, then every store read and the indexes rebuilt. The map and
/// the books are read, never generated or opened again.
///
/// # Errors
/// Every refusal of the build, a save of another format, build or data, or a store that does not read back.
#[clause("SET.12", "SET.15", "N5")]
pub fn load(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
    dir: &Path,
    build: &str,
) -> Result<World, AssemblyErrors> {
    use crate::save::{read_carried, read_run, read_store as read_file};
    let one = |e: String| AssemblyErrors(vec![e]);
    let manifest = crate::save::manifest::Manifest::read(dir).map_err(one)?;
    let mut p = prepare(systems, interfaces, config)?;
    let refusal = if manifest.format != crate::consts::SAVE_FORMAT {
        Some(format!("a save of format {} where this build reads {}", manifest.format, crate::consts::SAVE_FORMAT))
    } else if manifest.build != build {
        Some(format!("a save written by build {} where this is {build}", manifest.build))
    } else if manifest.register != crate::save::manifest::hex(p.register_hash) {
        Some("a save written over other data".to_owned())
    } else if manifest.seed != config.seed {
        Some(format!("a save of seed {} loaded with seed {}", manifest.seed, config.seed))
    } else {
        None
    };
    if let Some(why) = refusal {
        return Err(one(why));
    }
    let countries = u32::try_from(p.game.countries.len()).map_err(|e| one(e.to_string()))?;
    // The kernel tables' names are known once the map they are built over is read, so the map comes first.
    let (geo, tables) = read_file(dir, "geo", &[], &mut |r| {
        let geo = <GeoState as phx_store::Saved>::load(r)?;
        let fresh = phx_geo::tables(&geo, countries);
        let known: Vec<&'static str> =
            fresh.iter().flat_map(|t| core::iter::once(t.name).chain(t.columns.facts())).collect();
        r.with_names(&known);
        let tables: Vec<KernelTable> = phx_store::Saved::load(r)?;
        Ok((geo, tables))
    })
    .map_err(one)?;
    let geo = Arc::new(geo);
    let pop = (p.pop.as_slice(), p.representation);
    let (declared, _, _) =
        open((&mut p.d, &p.facets, &p.visits), pop, &p.kernel, &p.c, &p.game, &geo, (&[phx_core::DECLARATIONS], None));
    let families: Vec<phx_core::FamilyDecl> = kernel_families()
        .iter()
        .map(|f| f.decl())
        .chain(p.d.families.iter().map(|(_, f)| f.decl()))
        .chain(crate_families(p.game.countries.len(), !p.pop.is_empty()).iter().map(|f| f.decl()))
        .collect();
    let names = names(&p.d, &families, &tables, &declared);
    let mut declared = Some(declared);
    let books = read_file(dir, "books", &names, &mut |r| {
        let d = declared.take().ok_or_else(|| phx_store::LoadError::Invalid("the books read twice".to_owned()))?;
        phx_ledger::books::Books::load_from(r, d)
    })
    .map_err(one)?;
    let first = books.parties.first_cell_place();
    let mut space_pop = AddressSpace::empty();
    let mut population = phx_pop::population::Population::new(
        p.pop.clone(),
        p.representation,
        first,
        p.c.day_zero,
        &mut space_pop,
        crate::visits::specs(&p.visits),
    );
    read_file(dir, "population", &names, &mut |r| population.load_from(r, &mut space_pop)).map_err(one)?;
    let markets = read_file(dir, "markets", &names, &mut |r| phx_store::Saved::load(r)).map_err(one)?;
    let (permitted, forms) = standard(&p.d, &p.kernel, &p.c);
    let accounts = read_file(dir, "accounts", &names, &mut |r| {
        phx_acct::accounts::Accounts::load_from(r, permitted.clone(), forms.clone())
    })
    .map_err(one)?;
    let mut space = space_pop;
    let record_kinds: Vec<phx_core::RecordKindDecl> = p.d.records.iter().map(|(_, r)| *r).collect();
    let records = read_file(dir, "records", &names, &mut |r| {
        let store = RecordStore::load_from(r, record_kinds.clone())?;
        space.join(&r.take_space());
        Ok(store)
    })
    .map_err(one)?;
    let events = read_file(dir, "events", &names, &mut |r| {
        let store: EventStore = phx_store::Saved::load(r)?;
        space.join(&r.take_space());
        Ok(store)
    })
    .map_err(one)?;
    let carried = read_file(dir, "world", &names, &mut read_carried).map_err(one)?;
    let run = read_file(dir, crate::save::RUN, &names, &mut read_run).map_err(one)?;
    let state = State { geo, tables, books, population, markets, accounts, records, events, carried, run, space };
    let mut world = finish(p, state, config)?;
    world.defaults_rebuild();
    world.loaded = true;
    let rebuilt = crate::save::manifest::hex(crate::hash::world_hash(&world));
    if rebuilt != manifest.world_hash {
        return Err(one(format!("the save loads as {rebuilt} where its close hashed {}", manifest.world_hash)));
    }
    Ok(world)
}
