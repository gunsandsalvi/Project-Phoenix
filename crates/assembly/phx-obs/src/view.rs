//! A view: what the observer shows at a turn's close — each read's latest value and the declared histograms of the
//! state then — built whole and swapped in, so a page is always read from one close.

use std::sync::Arc;

use phx_id::Day;
use phx_macros::clause;
use phx_num::Missing;
use phx_world::Inspector;

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
}

/// What a histogram reads of each agent, which it counts once for every twin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Of {
    /// Its persons.
    Persons,
    /// The value of the attribute at this place among its kind's.
    Attr(usize),
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
        let kinds = &w.population().kinds;
        let mut histograms = Vec::with_capacity(decls.len());
        for d in decls {
            let Some(kind) = kinds.iter().position(|k| k.decl.kind == d.kind) else {
                return Err(format!("histogram `{}` is of `{}`, a kind the world does not keep", d.id, d.kind));
            };
            let of = match (d.of.as_str(), d.of.strip_prefix("attr.")) {
                ("persons", _) => Of::Persons,
                (_, Some(attr)) => {
                    let Some(i) = kinds.get(kind).and_then(|k| k.decl.attr(attr)) else {
                        return Err(format!("histogram `{}` reads `{attr}`, no attribute of `{}`", d.id, d.kind));
                    };
                    Of::Attr(i)
                }
                (other, None) => {
                    return Err(format!("histogram `{}` is of `{other}`, neither persons nor an attribute", d.id));
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
        for r in &self.histograms {
            let Ok(mut h) = Histogram::new(r.edges.clone()) else { continue };
            let table = w.agent_table(r.kind);
            for slot in table.slots() {
                let twins = u64::from(table.multiplicity(slot).get());
                let value = match r.of {
                    Of::Persons => i64::try_from(table.persons(slot).len()).unwrap_or(i64::MAX),
                    Of::Attr(attr) => i64::from(table.attr(slot, attr)),
                };
                h.add(value, twins);
            }
            histograms.push((r.id.clone(), h));
        }
        let view = Arc::new(View { day: w.today(), reads: latest, histograms });
        self.current = Some(Arc::clone(&view));
        view
    }

    /// The view of the last close, if one was built.
    #[must_use]
    pub fn current(&self) -> Option<Arc<View>> {
        self.current.clone()
    }
}
