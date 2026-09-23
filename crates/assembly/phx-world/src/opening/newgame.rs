use std::path::{Path, PathBuf};

use phx_core::{DataFile, Declarations, JointProfile, Level, OpeningCtx, OpeningPhase, Pinned, Streams, draw_profile};
use phx_id::CountryId;
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64};
use serde::Serialize;

use crate::opening::names::{CountryName, GeneratedTable, LabelTemplates, RealTable, name};
use crate::opening::prims::{GenPrims, NAMES_STREAM, PROFILE, SETUP_STREAM};
use crate::opening::regions::allot;
use crate::opening::setup::{CountrySetup, DEGREES, Degree, Guardrails, ResolvedSetup, read, refusals, resolve};

/// The setup's derivation runs before every other part of the opening.
pub const SETUP_PHASE: OpeningPhase = OpeningPhase(0);

/// A new game's country: its level, name, regions and land, and the derived values drawn for it, with the ones its
/// group's profile lacks named as not derived.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NewCountry {
    pub level: &'static str,
    #[serde(flatten)]
    pub name: NamesRecord,
    pub regions: u64,
    pub land_tiles: u64,
    pub derived: Vec<(String, f64)>,
    pub not_derived: Vec<String>,
}

/// A country's name as the run records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamesRecord {
    pub name: String,
    pub real: Option<String>,
    pub currency: String,
    pub central_bank: String,
    pub treasury: String,
    pub statistics: String,
    pub parliament: String,
    pub exchange: String,
}

impl From<CountryName> for NamesRecord {
    fn from(n: CountryName) -> NamesRecord {
        NamesRecord {
            name: n.name,
            real: n.real,
            currency: n.currency,
            central_bank: n.central_bank,
            treasury: n.treasury,
            statistics: n.statistics,
            parliament: n.parliament,
            exchange: n.exchange,
        }
    }
}

/// A new game: its setup with every choice made, and each country's derivation.
#[derive(Clone, Debug, PartialEq)]
pub struct NewGame {
    pub setup: ResolvedSetup,
    pub levels: Vec<Level>,
    pub countries: Vec<NewCountry>,
}

/// A choice and the derived values its level pins to their part of the profile.
struct Pin {
    level: fn(&CountrySetup) -> Degree,
    values: &'static [&'static str],
}

/// What each choice pins; risk appetite moves a PREFERENCE, and pins no derived value.
const PINS: &[Pin] = &[
    Pin { level: public_debt, values: &["GEN.public_debt"] },
    Pin { level: private_debt, values: &["GEN.household_debt", "GEN.firm_debt"] },
    Pin { level: inequality, values: &["GEN.income_gini", "GEN.top10_wealth_share"] },
    Pin { level: openness, values: &["GEN.trade"] },
];

fn public_debt(c: &CountrySetup) -> Degree {
    c.public_debt
}

fn private_debt(c: &CountrySetup) -> Degree {
    c.private_debt
}

fn inequality(c: &CountrySetup) -> Degree {
    c.inequality
}

fn openness(c: &CountrySetup) -> Degree {
    c.openness
}

fn one(e: String) -> Vec<String> {
    vec![e]
}

fn text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn toml_of<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Vec<String>> {
    toml::from_str(&text(path).map_err(one)?).map_err(|e| vec![format!("{}: {e}", path.display())])
}

/// A development level's profile, read alone through the register from its template.
fn level_profile(data: &Path, level: Level) -> Result<JointProfile, Vec<String>> {
    let path = data.join("profiles").join(level.dir()).join("GEN.toml");
    let file = DataFile {
        path: path.display().to_string(),
        country: Missing::Present(CountryId::new(0)),
        text: text(&path).map_err(one)?,
    };
    let mut d = Declarations::new();
    let profile: phx_core::Prim<JointProfile> = d.prim(&PROFILE);
    let register = std::mem::take(&mut d.prims).build(&[file], 1)?;
    Ok(profile.get(&register, CountryId::new(0)).clone())
}

/// The countries in a drawn order, which breaks the allotments' ties by lot.
fn lot(d: &mut Draws, n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let bound = u64::try_from(i + 1).unwrap_or(u64::MAX);
        if let Ok(j) = usize::try_from(below_u64(d, bound)) {
            order.swap(i, j);
        }
    }
    order
}

/// Derives a new game from the data and the setup: the setup checked against the guardrails and its open choices
/// drawn; regions and land allotted by the split; each country's derived values drawn from its level's profile, its
/// pinned values within their parts; and each country named.
///
/// # Errors
/// Every rule the setup breaks, and any datum the derivation cannot read.
#[clause("GEN.14", "GEN.15")]
pub fn new_game(data: &Path, setup_path: &Path, seed: Seed) -> Result<NewGame, Vec<String>> {
    let shared = data.join("shared").join("GEN.toml");
    let mut d = Declarations::new();
    let g = GenPrims::declare(&mut d);
    let file =
        DataFile { path: shared.display().to_string(), country: Missing::Absent, text: text(&shared).map_err(one)? };
    let register = std::mem::take(&mut d.prims).build(&[file], 0)?;
    let guard = Guardrails {
        countries: g.countries.shared(&register).get(),
        floor: u64::try_from(g.share_floor.shared(&register).raw()).map_err(|e| vec![e.to_string()])?,
        ceiling: u64::try_from(g.share_ceiling.shared(&register).raw()).map_err(|e| vec![e.to_string()])?,
    };
    let setup = read(&text(setup_path).map_err(one)?).map_err(one)?;
    let refused = refusals(&setup, guard);
    if !refused.is_empty() {
        return Err(refused);
    }
    let streams = Streams::new(seed, &[SETUP_STREAM, NAMES_STREAM])?;
    let ctx = OpeningCtx::new(&streams, SETUP_PHASE);
    let mut world = ctx.draws(&SETUP_STREAM, Subject::new(SubjectTag::World, 0));
    let mut per_country: Vec<Draws> = (0..setup.country.len())
        .map(|i| ctx.draws(&SETUP_STREAM, Subject::new(SubjectTag::Country, u64::try_from(i).unwrap_or(u64::MAX))))
        .collect();
    let resolved = resolve(&setup, guard, &mut world, &mut per_country);
    let order = lot(&mut world, resolved.countries.len());
    let regions =
        allot(&resolved.split, g.regions.shared(&register).get(), g.regions_floor.shared(&register).get(), &order)
            .map_err(one)?;
    let land = allot(&resolved.split, g.land_tiles.shared(&register).get(), 0, &order).map_err(one)?;
    let real: RealTable = toml_of(&data.join("names").join("real.toml"))?;
    let generated: GeneratedTable = toml_of(&data.join("names").join("generated.toml"))?;
    let labels: LabelTemplates = toml_of(&data.join("names").join("labels.toml"))?;
    let levels_count = u8::try_from(DEGREES.len()).map_err(|e| vec![e.to_string()])?;
    let all_values: Vec<String> = {
        let mut names = Vec::new();
        for level in [Level::Developed, Level::Emerging, Level::Developing] {
            for v in level_profile(data, level)?.values {
                if !names.contains(&v.name) {
                    names.push(v.name);
                }
            }
        }
        names
    };
    let mut countries = Vec::with_capacity(resolved.countries.len());
    for (i, (c, draws)) in resolved.countries.iter().zip(per_country.iter_mut()).enumerate() {
        let profile = level_profile(data, c.development)?;
        let pinned: Vec<Pinned> = PINS
            .iter()
            .flat_map(|pin| {
                let part = (pin.level)(c).third();
                pin.values.iter().filter_map(|v| profile.index(v)).map(move |index| Pinned { index, part })
            })
            .collect();
        let drawn = draw_profile(&profile, &pinned, levels_count, draws).map_err(one)?;
        let derived: Vec<(String, f64)> = profile.values.iter().map(|v| v.name.clone()).zip(drawn).collect();
        let not_derived = all_values.iter().filter(|n| profile.index(n).is_none()).cloned().collect();
        let subject = Subject::new(SubjectTag::Country, u64::try_from(i).unwrap_or(u64::MAX));
        let mut name_draws = ctx.draws(&NAMES_STREAM, subject);
        let named = name(&c.name, &real, &generated, &labels, &mut name_draws).map_err(one)?;
        countries.push(NewCountry {
            level: c.development.dir(),
            name: named.into(),
            regions: *regions.get(i).ok_or_else(|| vec![format!("no regions allotted to country {i}")])?,
            land_tiles: *land.get(i).ok_or_else(|| vec![format!("no land allotted to country {i}")])?,
            derived,
            not_derived,
        });
    }
    let levels = resolved.countries.iter().map(|c| c.development).collect();
    Ok(NewGame { setup: resolved, levels, countries })
}

fn slug(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase()
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes a new game into the run's directory: the record of each country's derivation, and each country's data,
/// instantiated from its level's templates. Returns each country's data directory, in the countries' order.
///
/// # Errors
/// When a file cannot be read or written.
#[clause("GEN.15")]
pub fn instantiate(game: &NewGame, data: &Path, run_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs = Vec::with_capacity(game.countries.len());
    let countries_dir = run_dir.join("data");
    for old in [countries_dir.clone(), run_dir.join("countries")] {
        if old.exists() {
            std::fs::remove_dir_all(&old).map_err(|e| format!("{}: {e}", old.display()))?;
        }
    }
    for (i, c) in game.countries.iter().enumerate() {
        let id = format!("{i}-{}", slug(&c.name.name));
        let dir = countries_dir.join(&id);
        let templates = data.join("profiles").join(c.level);
        let mut files: Vec<PathBuf> = std::fs::read_dir(&templates)
            .map_err(|e| format!("{}: {e}", templates.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for f in files {
            let Some(file_name) = f.file_name() else { continue };
            write(&dir.join(file_name), &text(&f)?)?;
        }
        let record = toml::to_string(c).map_err(|e| e.to_string())?;
        write(&run_dir.join("countries").join(format!("{id}.toml")), &record)?;
        dirs.push(dir);
    }
    let setup = format!(
        "split = {:?}\nlevels = {:?}\n",
        game.setup.split,
        game.countries.iter().map(|c| c.level).collect::<Vec<_>>()
    );
    write(&run_dir.join("setup.toml"), &setup)?;
    Ok(dirs)
}
