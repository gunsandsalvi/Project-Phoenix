//! The assembled world: its calendar, register and streams, each system's own compiled state, and the world on the
//! core; outside it, the run's metrics and findings.

use std::any::Any;

use phx_core::{Calendar, CountryEntry, Findings, Register, WorldStreams};
use phx_id::{CountryId, Day};

use crate::metrics::Metrics;
use crate::opening::newgame::NewGame;

/// What a system compiled at assembly, which only its own rules are given.
pub type OwnState = Box<dyn Any + Send + Sync>;

/// The assembled world.
#[derive(Debug)]
pub struct World {
    pub(crate) calendar: Calendar,
    pub(crate) register: Register,
    pub(crate) streams: WorldStreams,
    pub(crate) own: Vec<(&'static str, OwnState)>,
    /// Where the day finds the own states it reads, and the retail meeting's weights, bound at assembly.
    pub(crate) own_at: OwnAt,
    pub(crate) weights: Option<phx_market::retail::Weights>,
    pub(crate) countries: Vec<CountryEntry>,
    pub(crate) day_zero: Day,
    /// The years the world settles before the play the run measures.
    pub(crate) settling_years: u64,
    /// The months between the world's own saves.
    pub(crate) save_every: u64,
    pub(crate) today: Day,
    pub(crate) core: crate::core::Core,
    /// The processes acting on the households' persons, in order of kind, then hazard.
    pub(crate) processes: Vec<crate::pop_rules::Bound>,
    pub(crate) labour: Option<if_labour::kind::LabourKind>,
    /// The event kinds the systems declare, by their place.
    pub(crate) event_kinds: Vec<&'static str>,
    /// The declared rule of which events become public.
    pub(crate) news: phx_core::EventsRule,
    /// The country each region lies in, by the region's number.
    pub(crate) regions: Vec<CountryId>,
    /// The day's stage table, compiled at assembly.
    pub(crate) stages: phx_core::stages::StageTable,
    pub(crate) game: NewGame,
    pub(crate) metrics: Metrics,
    pub(crate) findings: Findings,
    /// The data's hash, which a save names.
    pub(crate) register_hash: u128,
    pub(crate) seed: u64,
    /// Whether the world was read back from a save, which an injection goes into and the run never is.
    pub(crate) loaded: bool,
    /// The persons the world was opened with.
    pub(crate) persons: u64,
    /// The device's workers, one a fast core, which the day's meetings run on; none where it has one core.
    pub(crate) pool: Option<phx_exec::Pool>,
}

/// Where the day finds the systems' own states it reads, each found by its system's code once, at assembly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct OwnAt {
    pub(crate) geo: Option<usize>,
    pub(crate) hh: Option<usize>,
    pub(crate) frm: Option<usize>,
}

impl OwnAt {
    #[phx_macros::opening]
    pub(crate) fn of(own: &[(&'static str, OwnState)]) -> OwnAt {
        let at = |code: &str| own.iter().position(|(c, _)| *c == code);
        OwnAt {
            geo: at(<phx_geo::Geo as phx_core::System>::CODE),
            hh: at(<sys_hh::Hh as phx_core::System>::CODE),
            frm: at(<sys_frm::Frm as phx_core::System>::CODE),
        }
    }
}

/// A system's own state at its bound place, as its type; none where the world keeps none.
pub(crate) fn own_at<'a, T: 'static>(own: &'a [(&'static str, OwnState)], at: Option<usize>) -> Option<&'a T> {
    own.get(at?).and_then(|(_, s)| s.downcast_ref::<T>())
}

/// The map as GEO keeps it, shared.
pub(crate) fn geo_arc<'a>(own: &'a [(&'static str, OwnState)], at: OwnAt) -> &'a std::sync::Arc<phx_geo::GeoState> {
    let Some(geo) = own_at::<std::sync::Arc<phx_geo::GeoState>>(own, at.geo) else {
        phx_num::violation!(clause = "GEO.1", "a world whose map GEO does not keep");
    };
    geo
}

impl World {
    /// The map and what GEO compiled from it, which GEO keeps as its own state.
    pub(crate) fn geo(&self) -> &phx_geo::GeoState {
        geo_arc(&self.own, self.own_at)
    }
}
