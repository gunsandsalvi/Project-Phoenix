use std::any::Any;

use phx_audit::Audit;
use phx_core::{
    Bindings, Calendar, CountryEntry, DayMessages, Directory, EventKindDecl, EventStore, Findings, KernelTable,
    PlayerQueue, RecordStore, Register, RuleTable, Streams,
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
    pub(crate) directory: Directory,
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
}
