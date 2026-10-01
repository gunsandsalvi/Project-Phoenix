//! A view: what the observer shows at a turn's close — each read's latest value and the declared histograms of the
//! state then — built whole and swapped in, so a page is always read from one close.

use std::sync::Arc;

use phx_id::{Day, PartyRef};
use phx_macros::clause;
use phx_num::Missing;
use phx_world::Inspector;

use crate::consts::MOBILITY_SAMPLE;
use crate::histogram::Histogram;
use crate::reads::{HistogramDecl, Recorder};

/// One close's view.
#[clause("OBS.6", "OBS.7")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub day: Day,
    /// Each read's latest value, absent where the run has not yet recorded one.
    pub reads: Vec<(String, Missing<i128>)>,
    pub histograms: Vec<(String, Histogram)>,
    /// For each histogram, the bin of each agent of a fixed sample, so its moves within it are read between two views.
    pub sampled: Vec<Vec<(PartyRef, Option<usize>)>>,
}

/// What a histogram reads of each agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Of {
    /// Its persons.
    Persons,
    /// The region it lies in.
    Region,
    /// The bank it banks with, by the bank's slot.
    Bank,
    /// Its contracts in the family at this place among the core's, on the side its kind is.
    Contracts { family: usize, side: usize },
}

/// The declared histograms resolved against the world's population kinds.
#[derive(Clone, Debug)]
struct Resolved {
    id: String,
    kind: usize,
    of: Of,
    edges: Vec<i64>,
}

/// The current view, swapped whole at each close.
#[derive(Debug)]
pub struct Views {
    histograms: Vec<Resolved>,
    current: Option<Arc<View>>,
}

impl Views {
    /// Views of the declared histograms.
    ///
    /// # Errors
    /// A histogram of a kind the world does not keep, or edges a histogram cannot have.
    pub fn new(decls: &[HistogramDecl], w: Inspector<'_>) -> Result<Views, String> {
        let core = w.core();
        let mut histograms = Vec::with_capacity(decls.len());
        for d in decls {
            let Some(kind) = core.names.iter().position(|n| *n == d.kind) else {
                return Err(format!("histogram `{}` is of `{}`, a kind the world does not keep", d.id, d.kind));
            };
            let of = match d.of.as_str() {
                "persons" => Of::Persons,
                "region" => Of::Region,
                "bank" => Of::Bank,
                of if of.starts_with("contracts.") => {
                    let name = of.trim_start_matches("contracts.");
                    let number = u8::try_from(kind).ok();
                    let found = core.families.iter().enumerate().find_map(|(i, f)| {
                        let side = f.store.kinds.iter().position(|k| Some(*k) == number)?;
                        (f.name == name && f.store.heads.get(side).is_some_and(Option::is_some)).then_some((i, side))
                    });
                    let Some((family, side)) = found else {
                        return Err(format!("histogram `{}` counts `{name}`, no family listing `{}`", d.id, d.kind));
                    };
                    Of::Contracts { family, side }
                }
                other => {
                    return Err(format!(
                        "histogram `{}` is of `{other}`, not persons, a region, a bank or contracts",
                        d.id
                    ));
                }
            };
            Histogram::new(d.edges.clone()).map_err(|e| format!("histogram `{}`: {e}", d.id))?;
            histograms.push(Resolved { id: d.id.clone(), kind, of, edges: d.edges.clone() });
        }
        Ok(Views { histograms, current: None })
    }

    /// Builds the view of the world as it closed, from the reads taken, and swaps it in.
    pub fn close(&mut self, w: Inspector<'_>, reads: &Recorder) -> Arc<View> {
        let latest = reads
            .series()
            .iter()
            .map(|s| (s.id.clone(), s.values.last().map_or(Missing::Absent, |(_, v)| Missing::Present(*v))))
            .collect();
        let mut histograms = Vec::with_capacity(self.histograms.len());
        let mut sampled = Vec::with_capacity(self.histograms.len());
        for r in &self.histograms {
            let Ok(mut h) = Histogram::new(r.edges.clone()) else { continue };
            let core = w.core();
            let persons = core.persons.get(r.kind).and_then(Option::as_ref);
            let Ok(kind) = u8::try_from(r.kind) else { continue };
            let mut sample = Vec::new();
            for slot in core.directory.live_slots(kind) {
                let value = match r.of {
                    Of::Persons => i64::try_from(persons.map_or(0, |p| p.count(slot))).unwrap_or(i64::MAX),
                    Of::Region => match core.zoned_region(phx_id::PartyKey::new(kind, slot)) {
                        Some(r) => i64::from(r),
                        None => continue,
                    },
                    Of::Bank => match core.bank_of(phx_id::PartyKey::new(kind, slot)) {
                        Some(b) => i64::from(b.slot().get()),
                        None => continue,
                    },
                    Of::Contracts { family, side } => core
                        .families
                        .get(family)
                        .map_or(0, |f| i64::try_from(f.store.of(side, slot).count()).unwrap_or(i64::MAX)),
                };
                h.add(value, 1);
                let Some(party) = core.directory.at(kind, slot) else { continue };
                if u64::from(slot.get()) % MOBILITY_SAMPLE == 0 {
                    sample.push((party, h.bin_of(value)));
                }
            }
            sample.sort_unstable_by_key(|(p, _)| *p);
            histograms.push((r.id.clone(), h));
            sampled.push(sample);
        }
        let view = Arc::new(View { day: w.today(), reads: latest, histograms, sampled });
        self.current = Some(Arc::clone(&view));
        view
    }

    /// The view of the last close, if one was built.
    #[must_use]
    pub fn current(&self) -> Option<Arc<View>> {
        self.current.clone()
    }
}
