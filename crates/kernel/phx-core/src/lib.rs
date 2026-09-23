// The declaration macros name this crate by its path, here as everywhere else.
extern crate self as phx_core;

pub mod agenda;
pub mod calendar;
pub mod consts;
pub mod contribution;
pub mod decisions;
pub mod directory;
pub mod events;
pub mod extensions;
pub mod facts;
pub mod family;
pub mod findings;
pub mod handler;
pub mod hazards;
pub mod kind_tables;
pub mod kinds;
pub mod kinks;
pub mod map;
pub mod messages;
pub mod occasions;
pub mod policy;
pub mod records;
pub mod register;
pub mod rules;
pub mod schedule;
pub mod schema;
pub mod streams;
pub mod substep;
pub mod system;
pub mod touched;
pub mod weight;

pub use agenda::{Agenda, AgendaCounters, AgendaTableSpec, TableToday, TodayAgenda};
pub use calendar::bizday::BusinessDayConvention;
pub use calendar::daycount::{DayCount, day_fraction};
pub use calendar::period::{EndOfMonth, Period, ScheduleDates, advance};
pub use calendar::prims::{CALENDAR, DAY_ZERO, EPOCH};
pub use calendar::rules::{CountryRules, HolidayRule, WeekendRule, easter_sunday};
pub use calendar::{Calendar, CountryCalendar};
pub use contribution::{Contribution, OpeningCtx};
pub use decisions::{Decider, DecisionPointDecl, PlayerQueue, QueuedIntent, QueuedPayload, dispatch};
pub use directory::{Directory, PartyState, Resolved};
pub use events::{Event, EventKindDecl, EventStore, NewEvent};
pub use extensions::{GroupDemand, PublicEventRule, TracedCells};
pub use facts::{
    Audience, Claim, FactDecl, FactDef, FactType, ItemDecl, ItemKind, Lag, ReprClass, Writer, check_claims,
};
pub use family::{
    AUDIT_SUBSTEP, AuditFamily, AuditInputs, AuditStream, FamilyCtx, FamilyDecl, FamilyMode, InjectTarget, ReadTrace,
    Span, rolling_slice,
};
pub use findings::{Finding, FindingOwner, Findings, Unit};
pub use handler::{Ctx, CtxParts, DrawsFrom, Emits, FactStore, HandlerDecl, IntentDef, Intents, Reads, Writes};
pub use hazards::{ActsOn, DrawScheme, EnvelopeRule, HazardDecl, RateFn, annual_to_daily};
pub use kind_tables::{FacetDecl, KindTable, ListKind, NewIndividual};
pub use kinds::{Feature, KindDecl, KindId, KindTableRef, LegalForm};
pub use kinks::{KinkDecl, KinkOn, KinkRegistry, KinkSource};
pub use map::{KernelMap, MapKey};
pub use messages::{
    Address, Answering, Concerns, DayMessages, Message, MessageDef, MessageKindDecl, MessageState, MessageStore,
};
pub use occasions::{OccasionDecl, OccasionKind};
pub use phx_macros::{
    declare_decision, declare_facet, declare_fact, declare_family, declare_handler, declare_hazard, declare_kind,
    declare_message, declare_prim, declare_record, declare_rule, declare_stream,
};
pub use policy::{AnnounceRefused, Announcement, PolicyValue};
pub use records::{Reader, RecordEntry, RecordKindDecl, RecordStamp, RecordStore};
pub use register::limit::{Binding, Bindings, Bound, DeclaredLimit, Limited, PhysicalToken, TermsToken};
pub use register::values::{
    Discretisation, Distribution, Family, Outside, OutsideAxes, PrimType, PrimValue, Table1, Table2, TypeId, TypeSet,
    TypeShare, ValueType, draw_type,
};
pub use register::{
    CountryEntry, DataFile, Level, Prim, PrimDecl, PrimKind, PrimPeriod, Register, RegisterBuilder, RoleId, Scope,
    ShapeInfo, Source, countries, read_data,
};
pub use rules::{RuleSig, RuleTable};
pub use schedule::{DecisionSchedule, Phase, RunsOn, WakeKind, next_due};
pub use schema::{FactColumn, TableSchema};
pub use streams::{NotObserver, ObserverDraws, OpeningPhase, Purpose, StreamDecl, StreamDef, Streams};
pub use substep::{SUB_STEPS, SubStep, SubStepInfo, SubStepKind};
pub use system::{
    DecisionMeta, Declarations, HandlerEntry, HandlerTable, System, SystemEntry, declare_entry, declare_system,
    handler_refusals,
};
pub use touched::TouchedRows;
pub use weight::Weight;
