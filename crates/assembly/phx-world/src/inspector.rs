use phx_core::{Calendar, CountryEntry, Finding, SUB_STEPS, SubStep, SubStepKind};
use phx_id::{CountryId, Date, Day};

use crate::hash::world_hash;
use crate::metrics::{SubStepRecord, TurnRecord};
use crate::trace::TraceLog;
use crate::world::World;

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

    /// Whether a sub-step has handlers, and whether it is a kernel apply.
    #[must_use]
    pub fn dispatches(&self, step: SubStep) -> (bool, bool) {
        let info = step.info();
        (self.world.graph.at(step).next().is_some(), info.kind == SubStepKind::KernelApply)
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
