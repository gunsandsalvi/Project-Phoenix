use phx_audit::Audit;
use phx_core::{
    Calendar, CountryEntry, DayMessages, Directory, EventStore, Findings, PlayerQueue, RecordStore, Register, Streams,
};
use phx_id::Day;
use phx_store::AddressSpace;

use crate::graph::HandlerGraph;
use crate::metrics::Metrics;
use crate::trace::TraceLog;

/// The assembled world: its calendar, register, streams and handlers, its stores and its day, the audit that reads
/// it at each close, and, outside it, the run's metrics, findings and trace.
#[derive(Debug)]
pub struct World {
    pub(crate) calendar: Calendar,
    pub(crate) register: Register,
    pub(crate) streams: Streams,
    pub(crate) graph: HandlerGraph,
    pub(crate) countries: Vec<CountryEntry>,
    pub(crate) day_zero: Day,
    pub(crate) today: Day,
    pub(crate) directory: Directory,
    pub(crate) records: RecordStore,
    pub(crate) events: EventStore,
    pub(crate) day_messages: DayMessages,
    pub(crate) queue: PlayerQueue,
    pub(crate) audit: Audit,
    pub(crate) read_trace: bool,
    pub(crate) metrics: Metrics,
    pub(crate) findings: Findings,
    pub(crate) trace: TraceLog,
    pub(crate) space: AddressSpace,
}
