use phx_audit::CloseRecord;
use phx_core::{Calendar, CountryEntry, FamilyDecl, Finding, SUB_STEPS, SubStep, SubStepKind};
use phx_id::{CountryId, Date, Day};

use crate::day::{AUDIT_AT, KERNEL_WORK};
use crate::hash::world_hash;
use crate::metrics::{SubStepRecord, TurnRecord};
use crate::opening::newgame::NewGame;
use crate::trace::TraceLog;
use crate::world::World;

/// What runs at a sub-step: the audit; a kernel apply; the kernel's own work though no handler runs there; its
/// handlers; or nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dispatch {
    Audit,
    KernelApply,
    KernelWork,
    Handlers,
    Idle,
}

/// The read-only surface of the world: only `&self` methods and no public fields, so looking changes nothing.
#[derive(Clone, Copy, Debug)]
pub struct Inspector<'a> {
    world: &'a World,
}

impl<'a> Inspector<'a> {
    #[must_use]
    pub fn new(world: &'a World) -> Inspector<'a> {
        Inspector { world }
    }

    pub fn today(&self) -> Day {
        self.world.today
    }

    pub fn day_zero(&self) -> Day {
        self.world.day_zero
    }

    /// How many years the world settles before play, the owner's setting.
    #[must_use]
    pub fn settling_years(&self) -> u64 {
        self.world.settling_years.get()
    }

    #[must_use]
    pub fn calendar(&self) -> &Calendar {
        &self.world.calendar
    }

    pub fn date(&self, day: Day) -> Date {
        self.world.calendar.date(day)
    }

    #[must_use]
    pub fn countries(&self) -> &[CountryEntry] {
        &self.world.countries
    }

    /// GEO's compiled state: the accepted map and what was read from it.
    #[must_use]
    pub fn geo(&self) -> &phx_geo::GeoState {
        &self.world.geo
    }

    /// The world's books: the ledger and the parties whose rows it moves.
    #[must_use]
    pub fn books(&self) -> &phx_ledger::books::Books {
        &self.world.books
    }

    /// The markets: their public tape, the linked calls' bases, and the measures of every day a market met.
    #[must_use]
    pub fn markets(&self) -> &phx_market::markets::Markets {
        &self.world.markets
    }

    /// What the opening wrote, drew, apportioned and adjusted, and each party's opening equity.
    #[must_use]
    pub fn opening(&self) -> &phx_core::GenReport {
        &self.world.report
    }

    /// Each day's settlement as published, in day order.
    #[must_use]
    pub fn settlements(&self) -> &[crate::world::Settled] {
        &self.world.settlements
    }

    /// A kernel table, by name.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&phx_core::FactColumns> {
        self.world.tables.iter().find(|t| t.name == name).map(|t| &t.columns)
    }

    /// The events recorded, each by its identity from one.
    #[must_use]
    pub fn events(&self) -> &phx_core::EventStore {
        &self.world.events
    }

    /// The declared event kinds, each at the place an event's kind names.
    #[must_use]
    pub fn event_kinds(&self) -> &[phx_core::EventKindDecl] {
        &self.world.event_kinds
    }

    /// The register the world compiled.
    #[must_use]
    pub fn register(&self) -> &phx_core::Register {
        &self.world.register
    }

    /// The new game the world opened from: its setup and each country's name, regions, land and derived values.
    #[must_use]
    pub fn game(&self) -> &NewGame {
        &self.world.game
    }

    #[must_use]
    pub fn is_business(&self, country: CountryId, day: Day) -> bool {
        self.world.calendar.is_business(country, day)
    }

    #[must_use]
    pub fn any_business(&self, day: Day) -> bool {
        self.world.calendar.any_business(day)
    }

    #[must_use]
    pub fn substep_records(&self) -> &[SubStepRecord] {
        &self.world.metrics.substeps
    }

    #[must_use]
    pub fn turn_records(&self) -> &[TurnRecord] {
        &self.world.metrics.turns
    }

    #[must_use]
    pub fn trace(&self) -> &TraceLog {
        &self.world.trace
    }

    #[must_use]
    pub fn read_traced(&self) -> bool {
        self.world.read_trace
    }

    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        self.world.findings.all()
    }

    /// What runs at a sub-step.
    #[must_use]
    pub fn dispatches(&self, step: SubStep) -> Dispatch {
        if step == AUDIT_AT {
            Dispatch::Audit
        } else if step.info().kind == SubStepKind::KernelApply {
            Dispatch::KernelApply
        } else if KERNEL_WORK.contains(&step) {
            Dispatch::KernelWork
        } else if self.world.graph.at(step).next().is_some() {
            Dispatch::Handlers
        } else {
            Dispatch::Idle
        }
    }

    /// Each close's audit: its day, the families run, the rows checked and the findings.
    #[must_use]
    pub fn closes(&self) -> &[CloseRecord] {
        &self.world.metrics.closes
    }

    /// Every family the audit runs, the kernel's and the systems'.
    #[must_use]
    pub fn families(&self) -> Vec<FamilyDecl> {
        self.world.audit.families().collect()
    }

    /// The (day, sub-step) every record is dated with.
    #[must_use]
    pub fn record_dates(&self) -> Vec<(Day, u8)> {
        self.world.records.dates()
    }

    #[must_use]
    pub fn substep_count(&self) -> usize {
        SUB_STEPS.len()
    }

    #[must_use]
    pub fn world_hash(&self) -> u128 {
        world_hash(self.world)
    }

    /// Address space the world's stores reserve.
    #[must_use]
    pub fn bytes_reserved(&self) -> usize {
        self.world.space.reserved()
    }

    /// The placeholder SHAPEs, each with the system that retires it.
    #[must_use]
    pub fn placeholders(&self) -> Vec<(&'static str, &'static str)> {
        self.world.register.placeholders()
    }

    /// The standing SHAPEs with their reasons.
    #[must_use]
    pub fn standing_shapes(&self) -> Vec<(&'static str, &'static str)> {
        self.world.register.standing_shapes()
    }

    #[must_use]
    pub fn stream_count(&self) -> usize {
        self.world.streams.len()
    }
}
