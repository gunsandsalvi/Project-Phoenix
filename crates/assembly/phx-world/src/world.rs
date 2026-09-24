use std::any::Any;

use phx_audit::Audit;
use phx_core::{
    Bindings, Calendar, CountryEntry, DayMessages, EventKindDecl, EventStore, Findings, KernelTable, PlayerQueue,
    RecordStore, Register, RuleTable, Streams,
};
use phx_id::Day;
use phx_num::Count;
use phx_store::AddressSpace;

use crate::graph::{HandlerGraph, HandlerId};
use crate::metrics::Metrics;
use crate::opening::newgame::NewGame;
use crate::trace::TraceLog;

/// What a system compiled at assembly, which only its own handlers are given.
pub type OwnState = Box<dyn Any + Send + Sync>;

/// A day's settlement as published: its measure, what its dated flows came to, and its fails.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Settled {
    pub day: Day,
    pub measure: phx_ledger::apply::Settlement,
    pub dues: phx_ledger::apply_batch::DaySettlement,
    pub fails: Vec<phx_ledger::fails::Fail>,
}

/// The assembled world: its calendar, register, streams and handlers, its stores and its day, the audit that reads
/// it at each close, and, outside it, the run's metrics, findings and trace.
#[derive(Debug)]
pub struct World {
    pub(crate) calendar: Calendar,
    pub(crate) register: Register,
    pub(crate) streams: Streams,
    pub(crate) graph: HandlerGraph,
    pub(crate) rules: RuleTable,
    pub(crate) own: Vec<(&'static str, OwnState)>,
    pub(crate) tables: Vec<KernelTable>,
    pub(crate) event_kinds: Vec<EventKindDecl>,
    pub(crate) bindings: Bindings,
    pub(crate) countries: Vec<CountryEntry>,
    pub(crate) day_zero: Day,
    pub(crate) today: Day,
    pub(crate) settling_years: Count,
    pub(crate) books: phx_ledger::books::Books,
    /// The population kinds: their declarations, keys, landing indexes and step levels; their cells are the books'.
    pub(crate) population: phx_pop::population::Population,
    pub(crate) markets: phx_market::markets::Markets,
    pub(crate) accounts: phx_acct::accounts::Accounts,
    pub(crate) report: phx_core::GenReport,
    pub(crate) unprocessed: Vec<phx_ledger::fails::Fail>,
    pub(crate) due: phx_ledger::due::DueLines,
    pub(crate) closed: phx_ledger::pending::Closed,
    pub(crate) settlements: Vec<Settled>,
    pub(crate) records: RecordStore,
    pub(crate) events: EventStore,
    pub(crate) day_messages: DayMessages,
    pub(crate) queue: PlayerQueue,
    pub(crate) game: NewGame,
    pub(crate) audit: Audit,
    pub(crate) read_trace: bool,
    pub(crate) metrics: Metrics,
    pub(crate) findings: Findings,
    pub(crate) trace: TraceLog,
    pub(crate) traced_first: Vec<HandlerId>,
    pub(crate) space: AddressSpace,
    /// Every name the build declares that a store keeps, which a save's names are read back against.
    pub(crate) names: Vec<&'static str>,
    /// The data's hash, which a save names.
    pub(crate) register_hash: u128,
    pub(crate) seed: u64,
    /// Months between the world's own saves.
    pub(crate) save_every: Count,
    /// Whether the world was read back from a save, the only world an injection may go into.
    pub(crate) loaded: bool,
}

impl World {
    /// The map and what GEO compiled from it, which GEO keeps as its own state.
    pub(crate) fn geo(&self) -> &phx_geo::GeoState {
        let found = self.own.iter().find(|(code, _)| *code == <phx_geo::Geo as phx_core::System>::CODE);
        let Some(geo) = found.and_then(|(_, s)| s.downcast_ref::<std::sync::Arc<phx_geo::GeoState>>()) else {
            phx_num::violation!(clause = "GEO.1", "a world whose map GEO does not keep");
        };
        geo
    }
}
