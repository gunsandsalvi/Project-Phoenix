//! The macro reads: series the declarations name, each read day by day from what the core records — its households
//! and persons, its day of chance and its day's settlement — and never from a number made for the reading.

use std::path::Path;

use phx_id::Day;
use phx_macros::clause;
use phx_world::Inspector;
use serde::Deserialize;

/// A count of the households and persons at the day's close, or of the day's chance on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentCount {
    Persons,
    Agents,
    Gone,
    Ended,
    EstatesOpened,
    EstatesSettled,
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
    Agents(AgentCount),
    Settlement(SettlementCount),
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
    /// The stylised fact the read is, whose definition was registered before any read.
    #[serde(default)]
    pub fact: Option<String>,
}

/// One declared histogram over a kind's parties, over fixed lower edges: of their persons (`of = "persons"`), of an
/// attribute's values (`of = "attr.<attribute>"`), or of their contracts in a family (`of = "contracts.<family>"`). Each is
/// an opening distribution whose distance from the world's own is read.
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

/// The event measures, read again once the core records its events.
const EVENTS_STEP: &str = "S1.24";

/// A measure named in the declarations.
///
/// # Errors
/// A name that is no measure.
pub fn measure(name: &str) -> Result<Measure, String> {
    use AgentCount as A;
    use SettlementCount as S;
    let m = match name {
        "agents.persons" => Measure::Agents(A::Persons),
        "agents.agents" => Measure::Agents(A::Agents),
        "agents.gone" => Measure::Agents(A::Gone),
        "agents.ended" => Measure::Agents(A::Ended),
        "agents.estates_opened" => Measure::Agents(A::EstatesOpened),
        "agents.estates_settled" => Measure::Agents(A::EstatesSettled),
        "settlement.payments" => Measure::Settlement(S::Payments),
        "settlement.settled" => Measure::Settlement(S::Settled),
        "settlement.failed" => Measure::Settlement(S::Failed),
        "settlement.gross" => Measure::Settlement(S::Gross),
        other => {
            if let Some((_, step)) = PLANNED.iter().find(|(m, _)| *m == other) {
                return Ok(Measure::NotYet(step));
            }
            if other.strip_prefix("events.").is_none() {
                return Err(format!("`{other}` is no measure"));
            }
            Measure::NotYet(EVENTS_STEP)
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

/// The reads taken so far: each series and the last day read.
#[clause("OBS.1", "OBS.6")]
#[derive(Clone, Debug)]
pub struct Recorder {
    measures: Vec<Measure>,
    series: Vec<Series>,
    read_to: Option<Day>,
}

impl Recorder {
    /// A recorder of the declared reads.
    ///
    /// # Errors
    /// Every read whose measure the world cannot give, and a read declared twice.
    pub fn new(decls: &[ReadDecl]) -> Result<Recorder, String> {
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
            match measure(&d.measure) {
                Ok(m) => measures.push(m),
                Err(e) => errors.push(format!("read `{}`: {e}", d.id)),
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        let series =
            decls.iter().map(|d| Series { id: d.id.clone(), unit: d.unit.clone(), values: Vec::new() }).collect();
        Ok(Recorder { measures, series, read_to: None })
    }

    /// Reads the day the core closed last, where it is past the last reading: the households and persons as it
    /// closed, its day of chance and its day's settlement.
    pub fn read(&mut self, w: Inspector<'_>) {
        let core = w.core();
        let Some(settled) = core.days.last() else { return };
        if self.read_to.is_some_and(|r| settled.day <= r) {
            return;
        }
        let day = settled.day;
        let chance = core.pop_days.iter().rev().find(|(d, _)| *d == day).map(|(_, p)| p);
        for (m, s) in self.measures.iter().zip(&mut self.series) {
            let value = match m {
                Measure::Agents(c) => chance.map(|p| agent_count(core, p, *c)),
                Measure::Settlement(c) => Some(settlement_count(settled, *c)),
                Measure::NotYet(_) => None,
            };
            if let Some(v) = value {
                s.values.push((day, v));
            }
        }
        self.read_to = Some(day);
    }

    #[must_use]
    pub fn series(&self) -> &[Series] {
        &self.series
    }
}

fn agent_count(core: &phx_world::core::Core, d: &phx_world::core_pop::PopDay, c: AgentCount) -> i128 {
    let n = match c {
        AgentCount::Persons => core.persons_held(),
        AgentCount::Agents => core.count("household"),
        AgentCount::Gone => d.gone,
        // A household ended leaves an estate.
        AgentCount::Ended | AgentCount::EstatesOpened => d.ended,
        AgentCount::EstatesSettled => core.days.last().map_or(0, |s| s.estates),
    };
    i128::from(n)
}

/// The observer the host runs beside the world: the reads taken each day.
#[derive(Debug)]
pub struct Watch {
    pub recorder: Recorder,
}

impl phx_world::Observer for Watch {
    fn day_closed(&mut self, w: Inspector<'_>) {
        self.recorder.read(w);
    }
}

fn settlement_count(s: &phx_world::core_day::CoreDay, c: SettlementCount) -> i128 {
    match c {
        SettlementCount::Payments => i128::from(s.flows),
        SettlementCount::Settled => i128::from(s.settled),
        SettlementCount::Failed => i128::from(s.failed),
        SettlementCount::Gross => s.gross,
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentCount, Measure, measure};

    #[test]
    fn measures_are_named_or_refused() {
        assert_eq!(measure("agents.persons"), Ok(Measure::Agents(AgentCount::Persons)));
        assert_eq!(measure("events.DEM.died"), Ok(Measure::NotYet("S1.24")));
        assert!(measure("agents.happiness").is_err());
        assert_eq!(measure("sta.output"), Ok(Measure::NotYet("S1.14")));
    }
}
