use phx_audit::CloseRecord;
use phx_core::{Calendar, CountryEntry, FamilyDecl, Finding, SUB_STEPS, SubStep, SubStepKind};
use phx_id::{CountryId, Date, Day};
use phx_macros::clause;

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

    /// The population: its kinds, members counted by the events that began and ended them, and the agenda.
    #[must_use]
    pub fn population(&self) -> &phx_pop::population::Population {
        &self.world.population
    }

    /// A population kind's agent table.
    #[must_use]
    pub fn agent_table(&self, kind: usize) -> &phx_pop::table::AgentTable<phx_store::SystemBacking> {
        phx_pop::population::Population::table(self.world.books.parties.cells(), kind)
    }

    /// What each day's work on the agents did, day by day.
    #[must_use]
    pub fn agent_days(&self) -> &[crate::agents::AgentDay] {
        &self.world.metrics.agents
    }

    /// What each day's visits did: the rows each handler visited and the facts they moved.
    #[must_use]
    /// Each day's stances: the public surprises, the reviews they woke, the methods held and the firms on each
    /// heuristic.
    pub fn stance_days(&self) -> &[crate::stances::StanceDay] {
        &self.world.stance_days
    }

    #[must_use]
    pub fn visit_days(&self) -> &[crate::visits::VisitDay] {
        &self.world.metrics.visits
    }

    /// What each day's goods did: calls met, extractions, orders admitted and refused, and failures.
    pub fn goods_days(&self) -> &[(Day, crate::goods::GoodsDay)] {
        &self.world.metrics.goods
    }

    /// What each day's labour did: searchers, vacancies seen, applications, offers, matches, hires, layoffs,
    /// separations and retirements.
    #[clause("LAB.14")]
    pub fn labour_days(&self) -> &[(Day, crate::labour::LabourDay)] {
        &self.world.metrics.labour
    }

    /// What each day's credit did: applications, quotes, declines by bank, choices, loans written, reviews.
    pub fn credit_days(&self) -> &[(Day, crate::credit::CreditDay)] {
        &self.world.metrics.credit
    }

    /// What each day's fund stage did: the facilities' uses and quantities, their interest and the remittances.
    pub fn central_days(&self) -> &[(Day, crate::central::CentralDay)] {
        &self.world.metrics.central
    }

    /// Every statistic the agencies published, each vintage with its day.
    #[must_use]
    pub fn releases(&self) -> &[if_state::stats::Release] {
        &self.world.state.book.stats.releases
    }

    /// The day a period's release of a vintage falls on in a country by its agency's calendar.
    #[must_use]
    pub fn release_day(&self, country: u8, series: usize, period: u32, vintage: u8) -> Option<Day> {
        self.world.release_day(country, series, (period, vintage))
    }

    /// The month a day falls in, as the agencies number periods.
    #[must_use]
    pub fn month_of(&self, day: Day) -> u32 {
        crate::stats::month_of(self.world.calendar.date(day))
    }

    /// What each day's state did: the consumption tax charged, the claims, the auctions, the statistics.
    pub fn state_days(&self) -> &[(Day, crate::state::StateDay)] {
        &self.world.metrics.state
    }

    /// Each country's bills: the face issued and redeemed, and what the bill lines hold outstanding.
    #[must_use]
    pub fn bills(&self) -> Vec<(u8, i64, i64, i64)> {
        let w = self.world;
        let Some(kind) = w.state.bills else { return Vec::new() };
        let lines = &w.books.ledger.lines;
        let k = lines.kind_index(kind.line);
        (0..w.state.countries.len())
            .filter_map(|i| u8::try_from(i).ok())
            .map(|c| {
                let ccy = phx_ledger::opening::currency(CountryId::new(c));
                let outstanding: i64 = w
                    .state
                    .book
                    .bills
                    .keys()
                    .filter(|l| lines.kind_of(**l) == k && w.books.ledger.terms.get(lines.terms(**l)).ccy == ccy)
                    .map(|l| {
                        lines.sole_holder(*l, phx_ledger::algebra::Side::Liability).map_or(0, |key| {
                            let t = w.books.party_of_key(key);
                            let (place, slot) = w.books.parties.row(t);
                            match phx_ledger::rows::find(
                                w.books.parties.holder(place),
                                slot,
                                *l,
                                phx_ledger::algebra::Side::Liability,
                            )
                            .map(|r| r.optional.balance)
                            {
                                Some(phx_num::Missing::Present(b)) => -b,
                                _ => 0,
                            }
                        })
                    })
                    .sum();
                let get = |m: &std::collections::BTreeMap<u8, i64>| m.get(&c).copied().unwrap_or(0);
                (c, get(&w.state.book.issued), get(&w.state.book.redeemed), outstanding)
            })
            .collect()
    }

    /// Employment as the books hold it: each employment line's two sides' members, and the people each employed
    /// person's household holds, for the checks.
    #[must_use]
    pub fn employment_lines(&self) -> Vec<(u32, u64, u64)> {
        let Some(kind) = self.world.labour.kind else { return Vec::new() };
        let lines = &self.world.books.ledger.lines;
        let k = lines.kind_index(kind.line);
        (0..lines.len())
            .filter_map(|i| u32::try_from(i).ok())
            .filter(|i| lines.kind_of(phx_id::LineId::new(*i)) == k)
            .map(|i| {
                let l = phx_id::LineId::new(i);
                let side = |s| u64::from(lines.side_count(l, s));
                (i, side(phx_ledger::algebra::Side::Asset), side(phx_ledger::algebra::Side::Liability))
            })
            .collect()
    }

    /// The parties that issue an instrument and are held in a population: none, since every issuer is an individual.
    #[clause("REP.2")]
    #[must_use]
    pub fn agent_issuers(&self) -> u64 {
        let books = &self.world.books;
        let first = books.parties.first_cell_place();
        let mut issuers: std::collections::BTreeSet<phx_id::PartyId> = std::collections::BTreeSet::new();
        for i in 0..books.ledger.instruments.len() {
            let Ok(i) = u32::try_from(i) else { break };
            if let phx_num::Missing::Present(issuer) = books.ledger.instruments.get(phx_id::InstrumentId::new(i)).issuer
            {
                issuers.insert(issuer);
            }
        }
        let live = |p: phx_id::PartyId| matches!(books.parties.directory().lookup(p), phx_core::PartyState::Live(_));
        phx_rand::float::len_u64(issuers.into_iter().filter(|p| live(*p) && books.parties.row(*p).0 >= first).count())
    }

    /// The persons of the household agents whose jobs buy more than `most` hours a week between them.
    #[must_use]
    pub fn persons_over_hours(&self, most: u64) -> u64 {
        let w = self.world;
        let Some(kind) = w.labour.kind else { return 0 };
        let Some(place) = w.labour_kind_place(&kind) else { return 0 };
        let lines = &w.books.ledger.lines;
        let k = lines.kind_index(kind.line);
        let hours_of = |l: phx_id::LineId| {
            let terms = w.books.ledger.terms.get(lines.terms(l));
            terms.class.get(if_labour::class::HOURS).copied().map_or(0, u64::from)
        };
        let table = phx_pop::population::Population::table::<phx_store::SystemBacking>(w.books.parties.cells(), place);
        let mut over = 0_u64;
        for slot in table.slots() {
            let mut by_person: std::collections::BTreeMap<usize, u64> = std::collections::BTreeMap::new();
            for word in table.attachments(slot) {
                let a = phx_pop::person::Attachment::unpack(*word);
                if let phx_pop::person::Holder::Person(i) = a.holder
                    && lines.kind_of(a.line) == k
                {
                    *by_person.entry(i).or_default() += hours_of(a.line);
                }
            }
            over += phx_rand::float::len_u64(by_person.values().filter(|h| **h > most).count());
        }
        over
    }

    /// The processes' realised and expected hits over the sampled agents.
    #[must_use]
    pub fn rates(&self) -> &crate::rates::Rates {
        &self.world.metrics.rates
    }

    /// Each process bound, in order: the hazard it answers, its kind's place among the population's kinds and the
    /// event kind its hits record.
    #[must_use]
    pub fn processes(&self) -> Vec<(&'static str, usize, u16)> {
        self.world.processes.iter().map(|b| (b.process.hazard(), b.kind, b.event)).collect()
    }

    /// The saves taken over the run, with their sizes, times and checks.
    #[must_use]
    pub fn saves(&self) -> &[crate::metrics::SaveMeasure] {
        &self.world.metrics.saves
    }

    /// The injections into the run's injection save, one per family.
    #[must_use]
    pub fn injections(&self) -> &[crate::metrics::InjectionRecord] {
        &self.world.metrics.injections
    }

    /// Months between the world's own saves, the owner's interval.
    #[must_use]
    pub fn save_every_months(&self) -> u64 {
        self.world.save_every.get()
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

    /// A system's state compiled at assembly, as its own type.
    #[must_use]
    pub fn own<T: 'static>(&self, system: &str) -> Option<&T> {
        self.world.own.iter().find(|(code, _)| *code == system).and_then(|(_, s)| s.downcast_ref::<T>())
    }

    /// The country a live party is in: an individual's by its site, an agent's by its region.
    pub fn country_of_party(&self, party: phx_id::PartyId) -> phx_num::Missing<phx_id::CountryId> {
        self.world.country_of_party(party)
    }

    /// GEO's compiled state: the accepted map and what was read from it.
    #[must_use]
    pub fn geo(&self) -> &phx_geo::GeoState {
        self.world.geo()
    }

    /// A market kind's place among the declared kinds, by its name.
    pub fn market_kind(&self, name: &str) -> phx_num::Missing<u16> {
        self.world.market_kinds.kind(phx_ledger::instruction::name_code(name))
    }

    /// The world's parties on the core, mirrored from the books while the port runs beside them.
    #[must_use]
    pub fn core(&self) -> &crate::core::Core {
        &self.world.core
    }

    /// The world's books: the ledger and the parties whose rows it moves.
    #[must_use]
    pub fn books(&self) -> &phx_ledger::books::Books {
        &self.world.books
    }

    /// The parties' accounts: their equity accounts, recognised claims and the period's tallies.
    #[must_use]
    pub fn accounts(&self) -> &phx_acct::accounts::Accounts {
        &self.world.accounts
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

    /// The player's queue: its party and the intents still queued.
    #[must_use]
    pub fn player_queue(&self) -> &phx_core::PlayerQueue {
        &self.world.queue
    }

    /// The observer's draws: its own streams only.
    #[must_use]
    pub fn observer_draws(&self) -> phx_core::ObserverDraws<'_> {
        phx_core::ObserverDraws::new(&self.world.streams)
    }

    /// The declared rule of which recorded events become public.
    #[must_use]
    pub fn news(&self) -> &phx_core::EventsRule {
        &self.world.news
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
        &self.world.metrics.closes.0
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
