// The declaration macros name this crate by its path, here as everywhere else.
extern crate self as phx_core;

pub mod agenda;
pub mod calendar;
pub mod consts;
pub mod directory;
pub mod facts;
pub mod findings;
pub mod kind_tables;
pub mod kinds;
pub mod map;
pub mod policy;
pub mod register;
pub mod schedule;
pub mod schema;
pub mod weight;

pub use agenda::{Agenda, AgendaCounters, AgendaTableSpec, TableToday, TodayAgenda};
pub use calendar::bizday::BusinessDayConvention;
pub use calendar::daycount::{DayCount, day_fraction};
pub use calendar::period::{EndOfMonth, Period, ScheduleDates, advance};
pub use calendar::prims::{CALENDAR, EPOCH};
pub use calendar::rules::{CountryRules, HolidayRule, WeekendRule, easter_sunday};
pub use calendar::{Calendar, CountryCalendar};
pub use directory::{Directory, PartyState, Resolved};
pub use facts::{Audience, Claim, FactDecl, FactType, ItemDecl, ItemKind, Lag, ReprClass, Writer, check_claims};
pub use findings::{Finding, FindingOwner, Findings, Unit};
pub use kind_tables::{FacetDecl, KindTable, ListKind, NewIndividual};
pub use kinds::{Feature, KindDecl, KindId, KindTableRef, LegalForm};
pub use map::{KernelMap, MapKey};
pub use phx_macros::{declare_facet, declare_fact, declare_kind, declare_prim};
pub use policy::{AnnounceRefused, Announcement, PolicyValue};
pub use register::limit::{Binding, Bindings, Bound, DeclaredLimit, Limited, PhysicalToken, TermsToken};
pub use register::values::{
    Discretisation, Distribution, Family, Outside, OutsideAxes, PrimType, PrimValue, Table1, Table2, TypeId, TypeSet,
    TypeShare, ValueType, draw_type,
};
pub use register::{
    DataFile, Prim, PrimDecl, PrimKind, PrimPeriod, Register, RegisterBuilder, RoleId, Scope, ShapeInfo, Source,
    read_data,
};
pub use schedule::{DecisionSchedule, Phase, RunsOn, WakeKind, next_due};
pub use schema::{FactColumn, TableSchema};
pub use weight::Weight;
