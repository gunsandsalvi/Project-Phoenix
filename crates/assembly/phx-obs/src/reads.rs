//! The macro reads: series the declarations name, each read day by day from what the run records — the day's work on
//! the cells, the day's settlement and the day's events — and never from a number made for the reading.

use std::collections::BTreeMap;
use std::path::Path;

use phx_id::Day;
use phx_macros::clause;
use phx_world::Inspector;
use serde::Deserialize;

/// A count of the day's work on the population's cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellCount {
    Persons,
    Members,
    Cells,
    Individuals,
    Gone,
    EstatesOpened,
    EstatesSettled,
    Parts,
}

/// A count or sum of the day's settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementCount {
    Payments,
    Settled,
    Failed,
    Gross,
}

/// What a read measures, resolved against the world's declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    Cells(CellCount),
    Settlement(SettlementCount),
    /// The sizes of the day's events of one declared kind, summed in the kind's unit.
    Events(u16),
    /// A measure of a system not yet built, read from the step named on; until then the read records nothing.
    NotYet(&'static str),
}

/// The measures of later steps' systems that reads may be declared over before the systems exist, so every read of a
/// stage is fixed before its code: each measure with the step that brings what it reads.
const PLANNED: &[(&str, &str)] = &[
    ("firms.employment_by_size", "S1.03"),
    ("labour.spells", "S1.08"),
    ("banks.credit", "S1.09"),
    ("banks.defaults", "S1.09"),
    ("households.consumption_by_income", "S1.12"),
    ("households.income_deciles", "S1.12"),
    ("households.decile_transitions", "S1.12"),
    ("households.spending_after_income_change", "S1.12"),
    ("sta.output", "S1.14"),
    ("sta.consumption", "S1.14"),
    ("sta.employment_by_region_age", "S1.14"),
    ("sta.unemployment_by_region_age", "S1.14"),
    ("sta.money", "S1.14"),
    ("idx.prices_by_category", "S1.14"),
];

/// One declared read.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadDecl {
    pub id: String,
    pub title: String,
    pub unit: String,
    pub measure: String,
    /// Why the read may rise on every day of a run, where a mechanism or a finding says so; absent, none is named.
    #[serde(default)]
    pub grows: Option<String>,
    /// The relationship real economies show that the read is read against, from Stage 1's reads on.
    #[serde(default)]
    pub relationship: Option<String>,
    /// Where that relationship is published.
    #[serde(default)]
    pub source: Option<String>,
    /// The fact of N3 the read is, whose definition was registered before any read.
    #[serde(default)]
    pub fact: Option<String>,
}

/// One declared histogram over a population kind's cells, over fixed lower edges: of their weights, each cell
/// counted once (`of = "weight"`), or of a key attribute's values, each cell counted by its members
/// (`of = "key.<attribute>"`). Each is an opening distribution whose distance from the world's own is read.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistogramDecl {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub of: String,
    pub edges: Vec<i64>,
}

/// The observer's declarations: the reads and the histograms.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    #[serde(rename = "read")]
    pub reads: Vec<ReadDecl>,
    #[serde(rename = "histogram")]
    pub histograms: Vec<HistogramDecl>,
}

impl Definitions {
    /// The declarations in `observer/READS.toml` under the data's root.
    ///
    /// # Errors
    /// A file that cannot be read or that declares something unknown.
    pub fn read(data: &Path) -> Result<Definitions, String> {
        let path = data.join("observer").join("READS.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// A measure named in the declarations, with the event kinds the world declares.
///
/// # Errors
/// A name that is no measure, or an event kind the world does not declare.
pub fn measure(name: &str, event_kinds: &[&str]) -> Result<Measure, String> {
    use CellCount as C;
    use SettlementCount as S;
    let m = match name {
        "cells.persons" => Measure::Cells(C::Persons),
        "cells.members" => Measure::Cells(C::Members),
        "cells.cells" => Measure::Cells(C::Cells),
        "cells.individuals" => Measure::Cells(C::Individuals),
        "cells.gone" => Measure::Cells(C::Gone),
        "cells.estates_opened" => Measure::Cells(C::EstatesOpened),
        "cells.estates_settled" => Measure::Cells(C::EstatesSettled),
        "cells.parts" => Measure::Cells(C::Parts),
        "settlement.payments" => Measure::Settlement(S::Payments),
        "settlement.settled" => Measure::Settlement(S::Settled),
        "settlement.failed" => Measure::Settlement(S::Failed),
        "settlement.gross" => Measure::Settlement(S::Gross),
        other => {
            if let Some((_, step)) = PLANNED.iter().find(|(m, _)| *m == other) {
                return Ok(Measure::NotYet(step));
            }
            let Some(kind) = other.strip_prefix("events.") else {
                return Err(format!("`{other}` is no measure"));
            };
            let Some(i) = event_kinds.iter().position(|k| *k == kind) else {
                return Err(format!("`{kind}` is no event kind the world declares"));
            };
            Measure::Events(u16::try_from(i).map_err(|e| e.to_string())?)
        }
    };
    Ok(m)
}

/// One read's values, one a day for each day the run recorded what it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Series {
    pub id: String,
    pub unit: String,
    pub values: Vec<(Day, i128)>,
}

/// The reads taken so far: each series, the last day read, and the events already summed.
#[clause("OBS.1", "OBS.6")]
#[derive(Clone, Debug)]
pub struct Recorder {
    measures: Vec<Measure>,
    series: Vec<Series>,
    read_to: Option<Day>,
    events_read: usize,
}

impl Recorder {
    /// A recorder of the declared reads, resolved against the world's event kinds.
    ///
    /// # Errors
    /// Every read whose measure the world cannot give, and a read declared twice.
    pub fn new(decls: &[ReadDecl], w: Inspector<'_>) -> Result<Recorder, String> {
        let kinds: Vec<&str> = w.event_kinds().iter().map(|k| k.name).collect();
        let mut errors = Vec::new();
        let mut measures = Vec::with_capacity(decls.len());
        for (i, d) in decls.iter().enumerate() {
            if decls.iter().take(i).any(|e| e.id == d.id) {
                errors.push(format!("read `{}` is declared twice", d.id));
            }
            if d.relationship.is_some() != d.source.is_some() {
                errors
                    .push(format!("read `{}` names a relationship without its source, or a source without one", d.id));
            }
            match measure(&d.measure, &kinds) {
                Ok(m) => measures.push(m),
                Err(e) => errors.push(format!("read `{}`: {e}", d.id)),
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        let series =
            decls.iter().map(|d| Series { id: d.id.clone(), unit: d.unit.clone(), values: Vec::new() }).collect();
        Ok(Recorder { measures, series, read_to: None, events_read: 0 })
    }

    /// Reads every day the run closed since the last reading.
    pub fn read(&mut self, w: Inspector<'_>) {
        let after = |d: Day| self.read_to.is_none_or(|r| d > r);
        let cells: BTreeMap<Day, &phx_world::cells::CellDay> =
            w.cell_days().iter().filter(|c| after(c.day)).map(|c| (c.day, c)).collect();
        let settled: BTreeMap<Day, &phx_world::world::Settled> =
            w.settlements().iter().filter(|s| after(s.day)).map(|s| (s.day, s)).collect();
        let mut events: BTreeMap<(Day, u16), i128> = BTreeMap::new();
        let store = w.events();
        for id in (self.events_read + 1)..=store.len() {
            let Ok(id) = u64::try_from(id) else { break };
            let e = store.get(id);
            let size: i128 = e.details.iter().map(|(_, s)| i128::from(*s)).sum();
            *events.entry((e.day, e.kind)).or_insert(0) += size;
        }
        self.events_read = store.len();
        let mut days: Vec<Day> = cells.keys().chain(settled.keys()).copied().collect();
        days.sort_unstable();
        days.dedup();
        for day in &days {
            for (m, s) in self.measures.iter().zip(&mut self.series) {
                let value = match m {
                    Measure::Cells(c) => cells.get(day).map(|d| cell_count(d, *c)),
                    Measure::Settlement(c) => settled.get(day).map(|d| settlement_count(d, *c)),
                    // A day the run closed with no event of the kind had none: its sum is nought, not unknown.
                    Measure::Events(k) => Some(events.get(&(*day, *k)).copied().unwrap_or(0)),
                    Measure::NotYet(_) => None,
                };
                if let Some(v) = value {
                    s.values.push((*day, v));
                }
            }
        }
        if let Some(last) = days.last() {
            self.read_to = Some(*last);
        }
    }

    #[must_use]
    pub fn series(&self) -> &[Series] {
        &self.series
    }
}

fn cell_count(d: &phx_world::cells::CellDay, c: CellCount) -> i128 {
    let n = match c {
        CellCount::Persons => d.persons,
        CellCount::Members => d.members,
        CellCount::Cells => d.cells,
        CellCount::Individuals => d.individuals,
        CellCount::Gone => d.gone,
        CellCount::EstatesOpened => d.estates,
        CellCount::EstatesSettled => d.estates_settled,
        CellCount::Parts => d.parts,
    };
    i128::from(n)
}

fn settlement_count(s: &phx_world::world::Settled, c: SettlementCount) -> i128 {
    match c {
        SettlementCount::Payments => i128::from(s.dues.payments),
        SettlementCount::Settled => i128::from(s.dues.settled),
        SettlementCount::Failed => i128::from(s.dues.failed),
        SettlementCount::Gross => s.dues.gross,
    }
}

#[cfg(test)]
mod tests {
    use super::{CellCount, Measure, measure};

    #[test]
    fn measures_are_named_or_refused() {
        let kinds = ["GEO.rain", "DEM.died"];
        assert_eq!(measure("cells.persons", &kinds), Ok(Measure::Cells(CellCount::Persons)));
        assert_eq!(measure("events.DEM.died", &kinds), Ok(Measure::Events(1)));
        assert!(measure("events.DEM.born", &kinds).is_err());
        assert!(measure("cells.happiness", &kinds).is_err());
        assert_eq!(measure("sta.output", &kinds), Ok(Measure::NotYet("S1.14")));
    }
}
