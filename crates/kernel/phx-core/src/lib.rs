// The declaration macros name this crate by its path, here as everywhere else.
extern crate self as phx_core;

pub mod accounting;
pub mod calendar;
pub mod capacity;
pub mod catalogue;
pub mod consts;
pub mod contribution;
pub mod decisions;
pub mod events;
pub mod events_rule;
pub mod extensions;
pub mod facts;
pub mod findings;
pub mod flows;
pub mod goods;
pub mod hazards;
pub mod kinds;
pub mod policy;
pub mod pop;
pub mod pop_process;
pub mod products;
pub mod register;
pub mod schedule;
pub mod settle;
pub mod slots;
pub mod spoilage;
pub mod stages;
pub mod store;
pub mod streams;
pub mod system;
pub mod unit_registry;
pub mod units;
pub mod wear;
pub mod wheel;

pub use accounting::{CarryingBasis, HeldFor, Permitted};
pub use calendar::bizday::BusinessDayConvention;
pub use calendar::daycount::{DayCount, day_fraction};
pub use calendar::period::{EndOfMonth, Period, ScheduleDates, advance};
pub use calendar::prims::{CALENDAR, DAY_ZERO, EPOCH};
pub use calendar::rules::{CountryRules, HolidayRule, WeekendRule, easter_sunday};
pub use calendar::{Calendar, CountryCalendar};
pub use contribution::{
    Adjustment, Apportioned, BALANCES, CONTRACTS, Contribution, DECLARATIONS, GenReport, Opening, OpeningCountry,
    OpeningCtx, PARTIES, PHASES, PHYSICAL_STOCK, PRESENT_VALUES, ReportSink, WriteRecord, apportion, opening_subject,
};
pub use decisions::{
    DECISIONS, DecisionKind, DecisionKinds, DecisionPointDecl, Mode, Prefs, QueuedIntent, QueuedPayload, Say, Standing,
    TakenIn,
};
pub use events::{Event, EventIntent, EventKindDecl, EventStore, NewEvent};
pub use events_rule::{EventsRule, NewsEntry, Notice, PUBLIC_EVENTS};
pub use extensions::PublicEventRule;
pub use facts::{
    Audience, Claim, FactDecl, FactDef, FactType, ItemDecl, ItemKind, Lag, ReprClass, Writer, check_claims,
};
pub use findings::{Finding, FindingOwner, Findings, Unit};
pub use hazards::{ActsOn, DrawScheme, HazardDecl, RateChange, RateFn, annual_to_daily};
pub use kinds::{ESTATE_KIND, Feature, KindDecl, KindId, LEGAL_FORMS, LegalForm, Place};
pub use phx_macros::{declare_fact, declare_hazard, declare_kind, declare_prim, declare_stream};
pub use policy::{AnnounceRefused, PolicyBook, PolicyEntry, PolicyH};
pub use pop::{PersonAttrDecl, PopEntry, PopItem, PopKindBuilder, RoleDecl};
pub use pop_process::{AgentView, Household, HouseholdState, Person, PopProcess};
pub use products::ProductEntry;
pub use register::limit::{Binding, Bindings, Bound, DeclaredLimit, Limited, PhysicalToken, TermsToken};
pub use register::profile::{JointProfile, Pinned, ProfileValue, Transform, draw_profile};
pub use register::values::{
    Discretisation, Distribution, Family, Outside, OutsideAxes, PrimType, PrimValue, Table1, Table2, TypeId, TypeSet,
    TypeShare, ValueType, draw_type,
};
pub use register::{
    CountryEntry, DataFile, Level, Prim, PrimDecl, PrimKind, PrimPeriod, Register, RegisterBuilder, RoleId, Scope,
    ShapeInfo, Source, read_data,
};
pub use schedule::{DecisionSchedule, Phase, RunsOn, WakeKind, next_due};
pub use spoilage::SpoilageDecl;
pub use streams::{
    AdviceDraws, NotObserver, ObserverDraws, OpeningPhase, Purpose, StreamDecl, StreamDef, StreamFamily, WorldStreams,
};
pub use system::{DecisionMeta, Declarations, SetupValue, System, SystemEntry, declare_entry, declare_system};
pub use wear::{WearDecl, WearSpec};
