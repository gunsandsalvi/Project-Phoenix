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

/// A day's settlement as published: its measure, what its dated flows came to, its fails counted — all, those of no
/// contract's due and those on each line kind — and the fails themselves until the next business day's contract process
/// has taken them; then how many of its fails on a row a party still held were not in arrears, and the fails are let
/// go, so the run's record of them does not grow with the run.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Settled {
    pub day: Day,
    pub measure: phx_ledger::apply::Settlement,
    pub dues: phx_ledger::apply_batch::DaySettlement,
    pub recorded: u64,
    pub rowless: u64,
    pub by_kind: Vec<(u16, u64)>,
    pub fails: Vec<phx_ledger::fails::Fail>,
    pub unrecorded: phx_num::Missing<u64>,
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
    /// The declared rule of which recorded events become public at the close.
    pub(crate) news: phx_core::EventsRule,
    pub(crate) bindings: Bindings,
    pub(crate) countries: Vec<CountryEntry>,
    pub(crate) day_zero: Day,
    pub(crate) today: Day,
    pub(crate) settling_years: Count,
    pub(crate) books: phx_ledger::books::Books,
    /// The population kinds, the representation, the parties counted and the agenda; their agents are the books'.
    pub(crate) population: phx_pop::population::Population,
    /// The processes acting on the agents' persons, in order of kind, then hazard.
    pub(crate) processes: Vec<crate::agents::Bound>,
    /// The day's hits, from 3b's gathering to 3e's outcomes.
    pub(crate) agent_hits: Vec<crate::agents::AgentHit>,
    /// The agents the rates are measured over, by kind and slot, found once when first read.
    pub(crate) rate_sample: Option<Vec<(usize, phx_id::Slot, phx_id::PartyId)>>,
    /// What the day's work on agents did.
    pub(crate) agent_day: crate::agents::AgentDay,
    /// The decisions taken on the rows of kinds as they come due, the rows due today by visit, the day they were
    /// gathered, and the undeclared reads their handlers made since the last close.
    pub(crate) visits: Vec<crate::visits::Bound>,
    pub(crate) visit_due: Vec<(usize, phx_id::Slot)>,
    pub(crate) visit_day: Day,
    pub(crate) visit_reads: phx_core::ReadTrace,
    /// What today's visits did, kept for the metrics at the day's close.
    pub(crate) visit_today: crate::visits::VisitDay,
    /// The kinds under an insolvency law, and the days the grace of each contract in arrears ends.
    pub(crate) laws: Vec<crate::defaults::Law>,
    /// The declared wear of chains of classes, each realised at its visit.
    pub(crate) wears: Vec<crate::wear::WearBound>,
    /// The declared spoilage of goods in stock, each realised at its visit.
    pub(crate) spoils: Vec<crate::spoil::SpoilBound>,
    pub(crate) defaults: std::collections::BTreeSet<crate::defaults::Due>,
    pub(crate) markets: phx_market::markets::Markets,
    /// The market kinds the systems declare, the templates of the instances the markets keep.
    pub(crate) market_kinds: phx_market::instances::Kinds,
    /// The retail and carriage kinds and the one set of counterparties a search reaches.
    pub(crate) trade: crate::retail::TradeKinds,
    /// Labour's kind and laws, its book, and what its rounds rebuild from the world.
    pub(crate) labour: crate::labour::Labour,
    pub(crate) credit: crate::credit::Credit,
    /// What the kernel reads to key goods.
    pub(crate) goods_frame: crate::goods::Frame,
    /// The day's orders, matches and trades, from their admission to their settlement.
    pub(crate) market_day: crate::goods::MarketDay,
    /// Each good's latest mark where it stands, and the public outlook of its price there, for the handlers' reads.
    pub(crate) marks: crate::goods::Marks,
    pub(crate) outlooks: crate::goods::Outlooks,
    /// The methods public series are forecast by, each with its memory type's parameters, by the method's index.
    pub(crate) val_methods: Vec<(phx_val::method::Method, phx_val::heuristic::Params)>,
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

/// The map and what GEO compiled from it, among the systems' own states.
pub(crate) fn geo_in<'a>(own: &'a [(&'static str, OwnState)]) -> &'a phx_geo::GeoState {
    let found = own.iter().find(|(code, _)| *code == <phx_geo::Geo as phx_core::System>::CODE);
    let Some(geo) = found.and_then(|(_, s)| s.downcast_ref::<std::sync::Arc<phx_geo::GeoState>>()) else {
        phx_num::violation!(clause = "GEO.1", "a world whose map GEO does not keep");
    };
    geo
}

impl World {
    /// Workers for the day's sharded kernel work; the world's results are the same with or without them.
    pub fn use_pool(&mut self, pool: std::sync::Arc<phx_exec::Pool>) {
        self.books.use_pool(pool);
    }

    /// The map and what GEO compiled from it, which GEO keeps as its own state.
    pub(crate) fn geo(&self) -> &phx_geo::GeoState {
        geo_in(&self.own)
    }
}
