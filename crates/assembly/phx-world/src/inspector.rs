use phx_audit::CloseRecord;
use phx_core::{Calendar, CountryEntry, FamilyDecl, Finding, SUB_STEPS, SubStep, SubStepKind};
use phx_id::{CountryId, Date, Day};

use crate::day::AUDIT_AT;
use crate::hash::world_hash;
use crate::metrics::{SubStepRecord, TurnRecord};
use crate::trace::TraceLog;
use crate::world::World;

/// How a sub-step is dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dispatch {
    pub handlers: bool,
    pub kernel_apply: bool,
    pub audit: bool,
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

    /// Whether a sub-step has handlers, whether it is a kernel apply, and whether it is the audit's, which runs every
    /// day.
    #[must_use]
    pub fn dispatches(&self, step: SubStep) -> Dispatch {
        Dispatch {
            handlers: self.world.graph.at(step).next().is_some(),
            kernel_apply: step.info().kind == SubStepKind::KernelApply,
            audit: step == AUDIT_AT,
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
