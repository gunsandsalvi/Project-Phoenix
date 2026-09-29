//! The world assembled: every system's declarations compiled against the data, the new game's map generated, each
//! system's own state compiled, and the world on the core drawn by its own opening.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use phx_core::{
    CountryEntry, DataFile, Declarations, Findings, HandlerTable, ItemDecl, OpeningCtx, SystemEntry, declare_entry,
};
use phx_geo::state::MAP_PHASE;
use phx_geo::{Allotment, GeoState};
use phx_id::{CountryId, SystemCode};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::Seed;

use crate::compile::{Compiled, KernelPrims, compile};
use crate::metrics::Metrics;
use crate::opening::newgame::{NewGame, instantiate, new_game};
use crate::refusals::{AssemblyErrors, refusals};
use crate::world::{OwnState, World};

/// How a run is set up: its one seed, where its data and its new game's setup lie, the run's own directory, where
/// the new game's countries are instantiated, and the representation the owner's switch names in place of the
/// register's.
#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    pub data: PathBuf,
    pub setup: PathBuf,
    pub run_dir: PathBuf,
    pub representation: Missing<phx_pop::prims::Representation>,
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
        let tables = toml_files(&dir.join("gen"))?.into_iter().chain(toml_files(&dir.join("economy"))?);
        for path in toml_files(dir)?.into_iter().chain(tables) {
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

/// Decisions the register and the systems do not declare alike, or taken in an office no legal form has.
fn undeclared_decisions(d: &Declarations, kernel: &KernelPrims, register: &phx_core::Register) -> Vec<String> {
    let points: Vec<&str> = d.decisions.iter().map(|m| m.name).collect();
    match kernel.decisions.shared(register).check(&points, kernel.legal_forms.shared(register)) {
        Ok(()) => Vec::new(),
        Err(refused) => refused,
    }
}

/// The setup's countries as the opening reads them, and the world's persons they were split from. Everything the
/// opening derives follows from the countries' persons; a world larger than the setup's population is refused.
fn opening_countries(
    kernel: &KernelPrims,
    c: &crate::compile::Compiled,
    game: &NewGame,
    geo: &phx_geo::GeoState,
    representation: phx_pop::prims::Representation,
) -> (Vec<phx_core::OpeningCountry>, u64) {
    let total = kernel.opening.population.shared(&c.register).get();
    let persons = representation.persons;
    if persons > total {
        phx_num::violation!(
            clause = "REP.40",
            "a world of more persons than the setup's population",
            persons = persons,
            population = total
        );
    }
    let units: Vec<u64> = (0_u8..)
        .take(game.countries.len())
        .map(|i| kernel.opening.units_per_dollar.get(&c.register, CountryId::new(i)).get())
        .collect();
    (crate::opening::countries::countries(game, geo, persons, &units), persons)
}

/// What the build supplies before any state: the new game, every system's declarations and handlers, and the
/// register, calendar, streams and handler graph compiled against the data, with the data's hash.
struct Prepared {
    game: NewGame,
    levels: Vec<CountryEntry>,
    d: Declarations,
    /// Each population kind compiled from the systems' items with the number of processes on its persons; the
    /// processes; and the representation in force.
    pop: Vec<(phx_pop::kind::PopKindDecl, usize)>,
    processes: Vec<crate::pop_rules::Bound>,
    representation: phx_pop::prims::Representation,
    kernel: KernelPrims,
    c: Compiled,
    entries: Vec<SystemEntry>,
    register_hash: u128,
}

/// The population kinds the systems declare, each compiled from every system's items for it, with the processes on
/// its persons.
fn population_kinds(
    d: &mut Declarations,
    register: &phx_core::Register,
) -> Result<crate::pop_rules::Kinds, Vec<String>> {
    let kinds: Vec<&'static str> =
        d.kinds.iter().filter(|(_, k)| k.table == phx_core::KindTableRef::Agents).map(|(_, k)| k.name).collect();
    let decls = phx_pop::population::Population::compile(&kinds, &d.pop)?;
    crate::pop_rules::bind(d, register, decls)
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
    errors.extend(undeclared_decisions(&d, &kernel, &c.register));
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
        kernel,
        c,
        entries,
        register_hash: data_hash(&files),
    })
}

/// Each system's state its handlers are given: the map for GEO's, what each system compiles for its own, nothing for
/// the rest.
fn own_states(p: &mut Prepared, geo: &Arc<GeoState>) -> Result<Vec<(&'static str, OwnState)>, AssemblyErrors> {
    let nothing = || -> OwnState { Box::new(()) };
    let mut own: Vec<(&'static str, OwnState)> = p.entries.iter().map(|e| (e.code, nothing())).collect();
    for (code, state) in &mut own {
        if *code == <phx_geo::Geo as phx_core::System>::CODE {
            *state = Box::new(Arc::clone(geo));
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
    if refused.is_empty() { Ok(own) } else { Err(AssemblyErrors(refused)) }
}

/// Labour's kind as the systems declare it, none where none does; more than one is refused.
fn labour_kind(d: &Declarations) -> Result<Option<if_labour::kind::LabourKind>, AssemblyErrors> {
    let kinds: Vec<if_labour::kind::LabourKind> =
        d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<if_labour::kind::LabourKind>()).copied().collect();
    match kinds.as_slice() {
        [] => Ok(None),
        [kind] => Ok(Some(*kind)),
        _ => Err(AssemblyErrors(vec!["more than one labour kind, where the employment family is one".to_owned()])),
    }
}

/// The state's kinds bound with each country's law.
fn state_of(p: &Prepared, geo: &phx_geo::GeoState) -> Result<crate::state::State, AssemblyErrors> {
    let (opening, _) = opening_countries(&p.kernel, &p.c, &p.game, geo, p.representation);
    crate::state::bind(&p.d, &p.c.register, &opening).map_err(AssemblyErrors)
}

/// Each country's statistics agency opened on the core: its law, and its indices' base, from the kinds the systems
/// declare.
fn open_stats(
    p: &Prepared,
    countries: &[phx_core::OpeningCountry],
    core: &mut crate::core::Core,
) -> Result<(), AssemblyErrors> {
    let sta = p.d.markets.iter().find_map(|(_, k)| k.downcast_ref::<if_state::stats::StaKind>()).copied();
    let index = p.d.markets.iter().find_map(|(_, k)| k.downcast_ref::<if_state::stats::IndexKind>()).copied();
    let Some(sta) = sta else { return Ok(()) };
    let one = |e: String| AssemblyErrors(vec![e]);
    let laws =
        countries.iter().map(|c| (sta.law)(&p.c.register, c)).collect::<Result<Vec<_>, String>>().map_err(one)?;
    let bases = match index {
        Some(i) => {
            countries.iter().map(|c| (i.base)(&p.c.register, c)).collect::<Result<Vec<_>, String>>().map_err(one)?
        }
        None => Vec::new(),
    };
    let classes = p.c.register.partition(sta.classes).map_err(one)?.bounds.to_vec();
    core.open_stats((laws, Some(sta.rate), classes), (index, bases));
    Ok(())
}

/// The core's own opening over the population's household kind.
fn open_core(
    p: &Prepared,
    (countries, sheets): (&[phx_core::OpeningCountry], &[crate::opening::sheet::Sheet]),
    calendar: &phx_core::calendar::Calendar,
    today: phx_id::Day,
) -> Result<crate::core::Core, AssemblyErrors> {
    let found = p.pop.iter().enumerate().find(|(_, (d, _))| d.kind == if_pop::HOUSEHOLD);
    let Some((household_pop, (household, _))) = found else {
        return Err(AssemblyErrors(vec!["no household kind among the population's".to_owned()]));
    };
    crate::core::Core::open(&crate::core_open::CoreOpening {
        register: &p.c.register,
        countries,
        sheets,
        streams: &p.c.streams,
        calendar,
        today,
        household: (household, household_pop),
    })
    .map_err(|e| AssemblyErrors(vec![e]))
}

/// The decisions the world takes opened on the core, each office found among its kinds' legal forms.
fn open_decisions(p: &Prepared, core: &mut crate::core::Core) {
    let forms = p.kernel.legal_forms.shared(&p.c.register);
    let form_of = |name: &str| {
        let kind = p.d.kinds.iter().find(|(_, k)| k.name == name)?;
        forms.iter().find(|f| f.name == kind.1.legal_form)
    };
    let by_kind: Vec<Option<&phx_core::LegalForm>> = core.names.iter().map(|n| form_of(n)).collect();
    core.open_decisions(p.kernel.decisions.shared(&p.c.register), &by_kind);
}

/// The world on the core, drawn by its own opening: its parties and their accounts, then its firms, jobs, loans,
/// labour, state and goods.
fn core_of(
    p: &Prepared,
    geo: &phx_geo::GeoState,
    (state, calendar, today): (&crate::state::State, &phx_core::calendar::Calendar, phx_id::Day),
    (own, labour): (&[(&'static str, OwnState)], Option<&if_labour::kind::LabourKind>),
) -> Result<crate::core::Core, AssemblyErrors> {
    let (opening, _) = opening_countries(&p.kernel, &p.c, &p.game, geo, p.representation);
    let sheets = opening
        .iter()
        .map(|c| crate::opening::sheet::country_sheet(&p.c.register, c))
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| AssemblyErrors(vec![e]))?;
    let mut core = open_core(p, (&opening, &sheets), calendar, today)?;
    open_decisions(p, &mut core);
    let regions: Vec<phx_id::CountryId> = geo.map.regions.iter().map(|r| r.country).collect();
    let ctx = crate::core_pop::Ctx {
        register: &p.c.register,
        calendar,
        streams: &p.c.streams,
        processes: &p.processes,
        regions: &regions,
    };
    core.open_hazards(&ctx, &p.pop, today.succ());
    let Some(frm) = own
        .iter()
        .find(|(c, _)| *c == <sys_frm::Frm as phx_core::System>::CODE)
        .and_then(|(_, s)| s.downcast_ref::<sys_frm::Own>())
    else {
        return Err(AssemblyErrors(vec!["the firms' management not compiled for their opening".to_owned()]));
    };
    core.open_firms(&crate::core_firms::FirmsOpening {
        register: &p.c.register,
        countries: &opening,
        sheets: &sheets,
        streams: &p.c.streams,
        stream: &<sys_frm::OpeningStream as phx_core::StreamDef>::DECL,
        management: frm.management(),
        today,
    })
    .map_err(|e| AssemblyErrors(vec![e]))?;
    let _ = core
        .open_jobs(&crate::core_jobs::JobsOpening {
            register: &p.c.register,
            countries: &opening,
            calendar,
            today,
            streams: &p.c.streams,
            stream: &<sys_frm::OpeningStream as phx_core::StreamDef>::DECL,
        })
        .map_err(|e| AssemblyErrors(vec![e]))?;
    core.open_loans(&crate::core_credit::CreditOpening {
        register: &p.c.register,
        countries: &opening,
        sheets: &sheets,
        calendar,
        today,
        streams: &p.c.streams,
        stream: &<sys_frm::OpeningStream as phx_core::StreamDef>::DECL,
    })
    .map_err(|e| AssemblyErrors(vec![e]))?;
    if let Some(kind) = labour {
        let lctx = crate::core_labour::LabourCtx {
            register: &p.c.register,
            calendar,
            streams: &p.c.streams,
            kind,
            regions: &regions,
        };
        core.open_labour(&lctx, &opening, today).map_err(|e| AssemblyErrors(vec![e]))?;
    }
    core.open_agencies();
    core.open_central(&p.c.register, (&opening, &sheets), (calendar, today)).map_err(|e| AssemblyErrors(vec![e]))?;
    core.open_bills(state, (&opening, &sheets), (calendar, today)).map_err(|e| AssemblyErrors(vec![e]))?;
    core.open_state(state, today);
    core.open_taxes(today);
    core.open_insolvency(&p.c.register).map_err(|e| AssemblyErrors(vec![e]))?;
    core.open_loan_books();
    open_core_goods(p, (&opening, &regions), (calendar, today), (own, frm), &mut core)?;
    core.open_freight((geo, &p.c.register, &p.c.streams), &regions, today).map_err(|e| AssemblyErrors(vec![e]))?;
    core.open_rights(geo, &p.c.register).map_err(|e| AssemblyErrors(vec![e]))?;
    open_stats(p, &opening, &mut core)?;
    core.open_accounts(today);
    core.open_credit(&p.c.register, &opening, today).map_err(|e| AssemblyErrors(vec![e]))?;

    Ok(core)
}

/// The goods on the core opened: the households' spending rule, the firms' stocks and prices.
fn open_core_goods(
    p: &Prepared,
    (opening, regions): (&[phx_core::OpeningCountry], &[phx_id::CountryId]),
    (calendar, today): (&phx_core::calendar::Calendar, phx_id::Day),
    (own, frm): (&[(&'static str, OwnState)], &sys_frm::Own),
    core: &mut crate::core::Core,
) -> Result<(), AssemblyErrors> {
    if let Some(rule) = own
        .iter()
        .find(|(c, _)| *c == <sys_hh::Hh as phx_core::System>::CODE)
        .and_then(|(_, s)| s.downcast_ref::<sys_hh::Own>())
    {
        let gctx = crate::core_goods::GoodsCtx {
            register: &p.c.register,
            calendar,
            streams: &p.c.streams,
            rule,
            management: frm.management(),
            regions,
            weights: retail_weights(&p.c.register).map_err(|e| AssemblyErrors(vec![e]))?,
            pool: None,
        };
        let cover = own
            .iter()
            .find(|(c, _)| *c == <sys_frm::Frm as phx_core::System>::CODE)
            .and_then(|(_, s)| s.downcast_ref::<sys_frm::Own>())
            .map(|f| (f.management().cover_days, f.management().adjustment_days));
        let Some(cover) = cover else {
            return Err(AssemblyErrors(vec!["the firms' management not compiled for the goods' opening".to_owned()]));
        };
        core.open_goods(&gctx, (opening, cover), today).map_err(|e| AssemblyErrors(vec![e]))?;
        let cap = own
            .iter()
            .find(|(c, _)| *c == <sys_cap::Cap as phx_core::System>::CODE)
            .and_then(|(_, s)| s.downcast_ref::<sys_cap::CapOwn>());
        let Some(cap) = cap else {
            return Err(AssemblyErrors(vec!["the plant's kinds not compiled for the plant's opening".to_owned()]));
        };
        core.open_plant((cap, &p.c.register), (opening, regions), today).map_err(|e| AssemblyErrors(vec![e]))?;
    }
    Ok(())
}

/// The retail logit's weights: of a seller's price's log and of its distance.
pub(crate) fn retail_weights(register: &phx_core::Register) -> Result<phx_market::retail::Weights, String> {
    Ok(phx_market::retail::Weights {
        price: register.fixed(sys_srv::PRICE_WEIGHT.id)?,
        distance: register.fixed(sys_srv::DISTANCE_WEIGHT.id)?,
    })
}

/// The device's pool, where it has more than one worker to give.
fn pool_of(spec: &phx_exec::PoolSpec) -> Option<phx_exec::Pool> {
    if spec.workers() > 1 { phx_exec::Pool::new(spec).ok() } else { None }
}

/// What a world is built from before its core: the prepared declarations and data, the map, each system's own state,
/// labour's kind and the state's laws.
struct Parts {
    p: Prepared,
    geo: Arc<GeoState>,
    own: Vec<(&'static str, OwnState)>,
    labour: Option<if_labour::kind::LabourKind>,
    state: crate::state::State,
    news: phx_core::EventsRule,
}

fn parts(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
) -> Result<Parts, AssemblyErrors> {
    let mut p = prepare(systems, interfaces, config)?;
    let geo = Arc::new(open_map(&p.kernel, &p.c, &p.game, &p.d)?);
    let own = own_states(&mut p, &geo)?;
    let labour = labour_kind(&p.d)?;
    let state = state_of(&p, &geo)?;
    let kinds: Vec<phx_core::EventKindDecl> = p.d.events.iter().map(|(_, e)| *e).collect();
    let news = phx_core::EventsRule::new(p.kernel.public_events.shared(&p.c.register), &kinds)
        .map_err(|e| AssemblyErrors(vec![e]))?;
    Ok(Parts { p, geo, own, labour, state, news })
}

/// The world of its parts and its core, at the day given.
fn world_of(parts: Parts, core: crate::core::Core, (today, seed): (phx_id::Day, u64)) -> World {
    let Parts { p, geo, own, labour, news, .. } = parts;
    let regions: Vec<CountryId> = geo.map.regions.iter().map(|r| r.country).collect();
    let event_kinds = p.d.events.iter().map(|(_, e)| e.name).collect();
    let settling_years = p.kernel.opening.settling_years.shared(&p.c.register).get();
    let save_every = p.kernel.save_every.shared(&p.c.register).get();
    World {
        settling_years,
        save_every,
        calendar: p.c.calendar,
        register: p.c.register,
        streams: p.c.streams,
        own,
        countries: p.levels,
        day_zero: p.c.day_zero,
        today,
        core,
        processes: p.processes,
        labour,
        event_kinds,
        news,
        regions,
        game: p.game,
        metrics: Metrics::default(),
        findings: Findings::default(),
        register_hash: p.register_hash,
        seed,
        loaded: false,
        persons: p.representation.persons,
        pool: pool_of(&phx_exec::PoolSpec::detect()),
    }
}

/// Assembles the world: every system's declarations, then compilation against the data, every refusal reported at
/// once; the map generated by the new game, and the world on the core drawn by its own opening.
///
/// # Errors
/// Every refusal of the declarations, the handlers, the items and the data.
pub fn assemble(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
) -> Result<World, AssemblyErrors> {
    let parts = parts(systems, interfaces, config)?;
    let today = parts.p.c.day_zero;
    let calendar = parts.p.c.calendar.clone();
    let mut core =
        core_of(&parts.p, &parts.geo, (&parts.state, &calendar, today), (&parts.own, parts.labour.as_ref()))?;
    let player = parts.p.game.setup.player;
    let regions: Vec<CountryId> = parts.geo.map.regions.iter().map(|r| r.country).collect();
    let country = player
        .country
        .checked_sub(1)
        .and_then(|c| u8::try_from(c).ok())
        .ok_or_else(|| AssemblyErrors(vec![format!("the player's country {}", player.country)]))?;
    core.seat_player((&parts.p.c.streams, today), (country, player.delegate), &regions)
        .map_err(|e| AssemblyErrors(vec![e]))?;
    let opened = opened(&core, &parts.p.c.register);
    let mut world = world_of(parts, core, (today, config.seed));
    world.metrics.opened = opened;
    Ok(world)
}

/// The opening's report: each kind's parties and the money written to them, and each distribution with its source.
#[clause("GEN.2", "GEN.4", "NUM.3")]
fn opened(core: &crate::core::Core, register: &phx_core::Register) -> crate::metrics::Opened {
    let kinds = core
        .names
        .iter()
        .zip(&core.kinds)
        .map(|(name, store)| {
            let money = store.accounts.as_ref().map_or(0, |a| {
                a.balance.slice().iter().zip(a.pending.slice()).map(|(m, p)| i128::from(*m) + i128::from(*p)).sum()
            });
            let parties = phx_rand::float::len_u64(store.parties.live_slots().count());
            crate::metrics::OpenedKind { kind: (*name).to_owned(), parties, money }
        })
        .collect();
    let distributions = register
        .source_refs()
        .into_iter()
        .filter(|(id, _)| register.distribution(id).is_ok())
        .map(|(id, source)| (id.to_owned(), source.to_owned()))
        .collect();
    crate::metrics::Opened { kinds, distributions }
}

/// A world read back from a save: assembled from the build and the data as the save's was, without its opening, its
/// core and its day read from the save by `read`, which is handed the register's hash to hold the save to, and the
/// build's declarations its core holds bound again.
///
/// # Errors
/// Every refusal of assembly, and the save's refusal.
pub(crate) fn assemble_loaded(
    systems: &[fn() -> SystemEntry],
    interfaces: &[&[ItemDecl]],
    config: &WorldConfig,
    read: &mut dyn FnMut(u128) -> Result<(phx_id::Day, crate::core::Core), String>,
) -> Result<World, String> {
    let parts = parts(systems, interfaces, config).map_err(|e| format!("assembly refused:\n{e}"))?;
    let (today, mut core) = read(parts.p.register_hash)?;
    let household = parts.p.pop.iter().find(|(d, _)| d.kind == if_pop::HOUSEHOLD).map(|(d, _)| d.clone());
    let sta = parts.p.d.markets.iter().find_map(|(_, k)| k.downcast_ref::<if_state::stats::StaKind>()).copied();
    let index = parts.p.d.markets.iter().find_map(|(_, k)| k.downcast_ref::<if_state::stats::IndexKind>()).copied();
    core.rebind(household, &parts.state, (sta.map(|s| s.rate), index));
    let mut world = world_of(parts, core, (today, config.seed));
    let (start, now) = (world.calendar.date(world.day_zero).year(), world.calendar.date(today).year());
    if now > start {
        world.calendar.move_window(now);
    }
    world.loaded = true;
    Ok(world)
}
