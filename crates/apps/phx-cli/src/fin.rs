//! `phx fin`: the finished-volume measure's arguments read and its report written; the harness is `phx-fin`'s.

use std::fs;

use phx_fin::{Args, DayType, REGISTRY};

use crate::FinArgs;

/// Runs the measure; whether every capacity and ratchet held.
///
/// # Errors
/// A file that cannot be read or written, or the harness's refusal.
pub fn run(a: &FinArgs) -> Result<bool, String> {
    let read = |p: &std::path::Path| fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    if a.seed_budget {
        let seeded = phx_fin::seed::seed(&read(&a.design)?).map_err(|e| e.0)?;
        let written = phx_fin::seed::write(&read(&a.budget)?, &seeded).map_err(|e| e.0)?;
        fs::write(&a.budget, written).map_err(|e| format!("{}: {e}", a.budget.display()))?;
        println!("{}: {} seeded keys", a.budget.display(), seeded.len());
        return Ok(true);
    }
    let days = if a.days == "turn" {
        DayType::ALL.to_vec()
    } else {
        a.days.split(',').map(|d| DayType::parse(d.trim())).collect::<Result<_, _>>().map_err(|e| e.0)?
    };
    let args = Args {
        design: read(&a.design)?,
        budget: read(&a.budget)?,
        bases: Some(a.bases.split(',').map(|b| b.trim().to_owned()).collect()),
        days,
    };
    let report = phx_fin::run(&args, REGISTRY, &crate::clock::WallClock::new()).map_err(|e| e.0)?;
    print!("{}", report.summary());
    if let Some(path) = &a.report {
        let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(report.clean())
}
