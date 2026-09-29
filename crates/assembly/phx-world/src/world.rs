//! The assembled world: its calendar, register and streams, each system's own compiled state, and the world on the
//! core; outside it, the run's metrics and findings.

use std::any::Any;

use phx_core::{Calendar, CountryEntry, Findings, Register, Streams};
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
    pub(crate) streams: Streams,
    pub(crate) own: Vec<(&'static str, OwnState)>,
    pub(crate) countries: Vec<CountryEntry>,
    pub(crate) day_zero: Day,
    /// The years the world settles before the play the run measures.
    pub(crate) settling_years: u64,
    pub(crate) today: Day,
    pub(crate) core: crate::core::Core,
    /// The processes acting on the households' persons, in order of kind, then hazard.
    pub(crate) processes: Vec<crate::pop_rules::Bound>,
    pub(crate) labour: Option<if_labour::kind::LabourKind>,
    /// The event kinds the systems declare, by their place.
    pub(crate) event_kinds: Vec<&'static str>,
    /// The country each region lies in, by the region's number.
    pub(crate) regions: Vec<CountryId>,
    pub(crate) game: NewGame,
    pub(crate) metrics: Metrics,
    pub(crate) findings: Findings,
    /// The data's hash, which a save names.
    pub(crate) register_hash: u128,
    pub(crate) seed: u64,
}

/// The map as GEO keeps it, shared.
pub(crate) fn geo_arc<'a>(own: &'a [(&'static str, OwnState)]) -> &'a std::sync::Arc<phx_geo::GeoState> {
    let found = own.iter().find(|(code, _)| *code == <phx_geo::Geo as phx_core::System>::CODE);
    let Some(geo) = found.and_then(|(_, s)| s.downcast_ref::<std::sync::Arc<phx_geo::GeoState>>()) else {
        phx_num::violation!(clause = "GEO.1", "a world whose map GEO does not keep");
    };
    geo
}

impl World {
    /// The map and what GEO compiled from it, which GEO keeps as its own state.
    pub(crate) fn geo(&self) -> &phx_geo::GeoState {
        geo_arc(&self.own)
    }
}
