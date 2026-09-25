//! `phx inject`: a family's injection into a save loaded apart, the audit run over it without a day stepped, and the
//! load discarded.

use std::path::Path;

use phx_exec::Clock;
use phx_world::systems::{INTERFACES, SYSTEMS};
use phx_world::{InjectionRecord, Inspector, WorldConfig, load};
use serde_json::json;

use crate::clock::WallClock;

/// Each family's injection, or the one named, each into its own load of the save: the families it lit.
///
/// # Errors
/// A save that does not load.
pub fn injections(
    from: &Path,
    data: &Path,
    setup: &Path,
    run_dir: &Path,
    family: Option<&str>,
) -> Result<Vec<InjectionRecord>, String> {
    let manifest = phx_world::save::manifest::Manifest::read(from)?;
    let config = WorldConfig {
        seed: manifest.seed,
        data: data.to_path_buf(),
        setup: setup.to_path_buf(),
        run_dir: run_dir.to_path_buf(),
        read_trace: manifest.read_trace,
        representation: phx_num::Missing::Present(phx_pop::prims::Representation {
            multiplicity: manifest.multiplicity,
            population_divisor: manifest.population_divisor,
        }),
    };
    let build = crate::run::build_id()?;
    let clock = WallClock::new();
    let loaded = || {
        let t0 = clock.now_ns();
        let world = load(SYSTEMS, INTERFACES, &config, from, &build).map_err(|e| format!("the save refused:\n{e}"))?;
        Ok::<_, String>((world, clock.now_ns().checked_sub(t0)))
    };
    let (mut world, mut load_ns) = loaded()?;
    let families: Vec<&'static str> = match family {
        Some(f) => vec![
            Inspector::new(&world)
                .families()
                .into_iter()
                .map(|d| d.name)
                .find(|n| *n == f)
                .ok_or_else(|| format!("the audit runs no family `{f}`"))?,
        ],
        None => Inspector::new(&world).families().into_iter().map(|d| d.name).collect(),
    };
    let mut out = Vec::new();
    for (i, name) in families.iter().enumerate() {
        if i > 0 {
            (world, load_ns) = loaded()?;
        }
        let family = (*name).to_owned();
        let record = match world.inject(name) {
            Ok(lit) => {
                InjectionRecord { family, load_ns, lit: lit.iter().map(|l| (*l).to_owned()).collect(), refused: None }
            }
            Err(why) => InjectionRecord { family, load_ns, lit: Vec::new(), refused: Some(why) },
        };
        out.push(record);
    }
    Ok(out)
}

/// Whether an injection lit its own family and no other.
#[must_use]
pub fn alone(r: &InjectionRecord) -> bool {
    r.refused.is_none() && r.lit.len() == 1 && r.lit.first().is_some_and(|l| *l == r.family)
}

/// The injections as the report writes them.
#[must_use]
pub fn report(records: &[InjectionRecord]) -> serde_json::Value {
    json!(records.iter().map(|r| json!({ "family": r.family, "load_ns": r.load_ns, "lit": r.lit, "refused": r.refused, "alone": alone(r) })).collect::<Vec<_>>())
}

/// The injections read back from a report.
///
/// # Errors
/// A report that is not the injections'.
pub fn parse(text: &str) -> Result<Vec<InjectionRecord>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let rows = v.as_array().ok_or("an injection report that is not a list")?;
    let mut out = Vec::new();
    for r in rows {
        let family = r["family"].as_str().ok_or("an injection with no family")?.to_owned();
        let lit = r["lit"]
            .as_array()
            .ok_or("an injection with no families lit")?
            .iter()
            .map(|l| l.as_str().map(str::to_owned).ok_or("a family lit that is not a name"))
            .collect::<Result<Vec<_>, _>>()?;
        let refused = r["refused"].as_str().map(str::to_owned);
        out.push(InjectionRecord { family, load_ns: r["load_ns"].as_u64(), lit, refused });
    }
    Ok(out)
}

/// `phx inject`: the injections into the save, printed and written to the report; true when each lit its family alone.
///
/// # Errors
/// A save that does not load, or a report that cannot be written.
pub fn run(
    from: &Path,
    data: &Path,
    setup: &Path,
    run_dir: &Path,
    family: Option<&str>,
    out: Option<&Path>,
) -> Result<bool, String> {
    let records = injections(from, data, setup, run_dir, family)?;
    for r in &records {
        match &r.refused {
            Some(why) => println!("{} refused: {why}", r.family),
            None => println!("{} lit {:?}", r.family, r.lit),
        }
    }
    if let Some(path) = out {
        let text = serde_json::to_string_pretty(&report(&records)).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(records.iter().all(alone))
}
