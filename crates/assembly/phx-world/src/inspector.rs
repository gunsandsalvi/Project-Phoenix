//! The read-only surface of the world, which the observer, the checks and the app read.

use phx_core::{Calendar, CountryEntry, Finding, Register};
use phx_id::{CountryId, Date, Day};

use crate::metrics::TurnRecord;
use crate::opening::newgame::NewGame;
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

    /// The years the world settles before the play the run measures.
    #[must_use]
    pub fn settling_years(&self) -> u64 {
        self.world.settling_years
    }

    /// A day's date.
    pub fn date(&self, day: Day) -> Date {
        self.world.calendar.date(day)
    }

    /// Whether a day is a business day in a country.
    #[must_use]
    pub fn is_business(&self, country: CountryId, day: Day) -> bool {
        self.world.calendar.is_business(country, day)
    }

    /// Whether a day is a business day in some country.
    #[must_use]
    pub fn any_business(&self, day: Day) -> bool {
        self.world.calendar.any_business(day)
    }

    #[must_use]
    pub fn calendar(&self) -> &'a Calendar {
        &self.world.calendar
    }

    #[must_use]
    pub fn register(&self) -> &'a Register {
        &self.world.register
    }

    /// The world on the core.
    #[must_use]
    pub fn core(&self) -> &'a crate::core::Core {
        &self.world.core
    }

    /// The map and what GEO compiled from it.
    #[must_use]
    pub fn geo(&self) -> &'a phx_geo::GeoState {
        self.world.geo()
    }

    /// The country each region lies in.
    pub fn regions(&self) -> &'a [CountryId] {
        &self.world.regions
    }

    #[must_use]
    pub fn countries(&self) -> &'a [CountryEntry] {
        &self.world.countries
    }

    /// The event kinds the systems declare, by their place.
    #[must_use]
    pub fn event_kinds(&self) -> &'a [&'static str] {
        &self.world.event_kinds
    }

    /// The firms' management, as their system compiled it.
    #[must_use]
    pub fn management(&self) -> Option<&'a sys_frm::decide::Management> {
        let code = <sys_frm::Frm as phx_core::System>::CODE;
        let own = self.world.own.iter().find(|(c, _)| *c == code)?;
        own.1.downcast_ref::<sys_frm::Own>().map(sys_frm::Own::management)
    }

    #[must_use]
    pub fn game(&self) -> &'a NewGame {
        &self.world.game
    }

    #[must_use]
    pub fn findings(&self) -> &'a [Finding] {
        self.world.findings.all()
    }

    #[must_use]
    pub fn turns(&self) -> &'a [TurnRecord] {
        &self.world.metrics.turns
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.world.seed
    }

    /// The data's hash.
    #[must_use]
    pub fn register_hash(&self) -> u128 {
        self.world.register_hash
    }

    /// The processes on persons, in the world's order: each one's hazard, kind and event.
    #[must_use]
    pub fn processes(&self) -> Vec<(&'static str, usize, u16)> {
        self.world.processes.iter().map(|b| (b.process.hazard(), b.kind, b.event)).collect()
    }
}
