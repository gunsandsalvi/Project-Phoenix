//! The population's agents in the world: the processes acting on their persons, bound at assembly; each agent's next
//! hit of each process drawn ahead into the agenda; the day's hits gathered at 3b, their outcomes at 3e, and at 10b
//! the agents that changed drawn again.

use phx_core::{
    ActsOn, AgentView, Declarations, Household, NewEvent, PopProcess, Register, StreamDecl, StreamDef, SubStep,
};
use phx_id::{CountryId, Day, LineId, PartyId, Slot, TableId};
use phx_ledger::algebra::Side;
use phx_ledger::apply::ApplyAt;
use phx_ledger::transfer::{LineTransfer, MoveAt};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};
use phx_pop::explicit::{household, write_back};
use phx_pop::hazard::{Booking, any_hit, next_booking, reached};
use phx_pop::kind::PopKindDecl;
use phx_pop::population::Population;
use phx_pop::prims::HouseholdsStream;
use phx_pop::table::AgentList;
use phx_rand::{Draws, Subject, SubjectTag};
use phx_store::SystemBacking;

use crate::rates;
use crate::world::World;

/// A booking drawn as a hit, in the agenda's value beside its day; a redraw holds none.
const HIT: u32 = 1;

/// A process on a kind's persons as the world runs it: its kind, its agenda reason within the kind, the event kind
/// each hit records, the stream it draws from, and the system's process.
pub(crate) struct Bound {
    pub kind: usize,
    pub reason: usize,
    pub event: u16,
    pub stream: StreamDecl,
    pub process: Box<dyn PopProcess>,
}

impl core::fmt::Debug for Bound {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bound")
            .field("kind", &self.kind)
            .field("hazard", &self.process.hazard())
            .finish_non_exhaustive()
    }
}

/// Whether a hazard acts on the persons of a population kind.
fn acts_on(on: ActsOn, kind: &str) -> bool {
    match on {
        ActsOn::Role { kind: k, .. } | ActsOn::Persons { kind: k } => k == kind,
        _ => false,
    }
}

/// One process bound to its kind, hazard, event kind and stream, or why it cannot be.
fn bind_one(
    system: &'static str,
    process: Box<dyn PopProcess>,
    d: &Declarations,
    kinds: &[PopKindDecl],
) -> Result<Bound, String> {
    let name = process.hazard();
    let Some(kind) = kinds.iter().position(|k| k.kind == process.kind()) else {
        return Err(format!("process `{name}` of {system} acts on `{}`, no population kind", process.kind()));
    };
    let Some((owner, hazard)) = d.hazards.iter().find(|(_, h)| h.name == name) else {
        return Err(format!("process `{name}` of {system} answers no declared hazard"));
    };
    if *owner != system {
        return Err(format!("process `{name}` is {system}'s, but its hazard is {owner}'s"));
    }
    if !acts_on(hazard.acts_on, process.kind()) {
        return Err(format!("hazard `{name}` does not act on the persons of `{}`", process.kind()));
    }
    if kinds.get(kind).is_none_or(|k| !k.has_persons()) {
        return Err(format!("process `{name}` acts on `{}`, whose agents hold no persons", process.kind()));
    }
    let Some(event) = d.events.iter().position(|(_, e)| e.name == hazard.outcome) else {
        return Err(format!("hazard `{name}`'s outcome `{}` is no declared event kind", hazard.outcome));
    };
    let Some((_, stream)) = d.streams.iter().find(|(_, s)| s.name == hazard.stream) else {
        return Err(format!("hazard `{name}` draws from `{}`, no declared stream", hazard.stream));
    };
    let event = u16::try_from(event).map_err(|e| e.to_string())?;
    Ok(Bound { kind, reason: 0, event, stream: *stream, process })
}

/// The population kinds, each with the number of processes on its persons, and the processes bound to them.
pub(crate) type Kinds = (Vec<(PopKindDecl, usize)>, Vec<Bound>);

/// Every population kind with the number of processes on it, and every process bound to its kind, in order of kind,
/// then hazard; each is its kind's agenda reason by that order. Every refusal at once.
pub(crate) fn bind(d: &mut Declarations, register: &Register, kinds: Vec<PopKindDecl>) -> Result<Kinds, Vec<String>> {
    let mut errors = Vec::new();
    let mut bound = Vec::new();
    for (system, mut process) in std::mem::take(&mut d.pop_processes) {
        process.bind(register);
        match bind_one(system, process, d, &kinds) {
            Ok(b) => bound.push(b),
            Err(e) => errors.push(e),
        }
    }
    bound.sort_by(|a, b| (a.kind, a.process.hazard()).cmp(&(b.kind, b.process.hazard())));
    let mut out = Vec::with_capacity(kinds.len());
    for (i, decl) in kinds.into_iter().enumerate() {
        let mut reason = 0;
        for b in bound.iter_mut().filter(|b| b.kind == i) {
            b.reason = reason;
            reason += 1;
        }
        out.push((decl, reason));
    }
    if errors.is_empty() { Ok((out, bound)) } else { Err(errors) }
}

/// Persons of an agent a process hit at 3b, by their places in its household, carried to 3e.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AgentHit {
    pub process: usize,
    pub kind: usize,
    pub slot: Slot,
    pub party: PartyId,
    pub reached: Vec<usize>,
}

/// What a day's work on the population's agents did, for the run's counters.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct AgentDay {
    pub day: Day,
    /// The agenda's rows gathered at 3b, and the bookings of a process on them read.
    pub gathered: u64,
    pub read: u64,
    /// Bookings drawn again on the day a rate changed, and hits drawn.
    pub redraws: u64,
    pub hits: u64,
    /// Persons the hits reached, and persons gone from households at 3e, each counted once for every twin.
    pub persons_hit: u64,
    pub gone: u64,
    /// Agents whose households no one was left in, and the estates opened for those that held anything.
    pub ended: u64,
    pub estates: u64,
    /// Agents drawn afresh at 10b, as they changed or began.
    pub booked: u64,
    /// The agents at the day's end, the real parties they stand for, and those parties' persons.
    pub agents: u64,
    pub parties: u64,
    pub persons: u64,
    /// Estates settled and ended, those left waiting on a payment that failed, and what the settled passed on and
    /// their creditors lost.
    pub estates_settled: u64,
    pub estates_waiting: u64,
    pub estates_passed: i128,
    pub estates_written_off: i128,
}

impl AgentDay {
    /// A day on which nothing has yet been done.
    #[must_use]
    pub fn of(day: Day) -> AgentDay {
        AgentDay {
            day,
            gathered: 0,
            read: 0,
            redraws: 0,
            hits: 0,
            persons_hit: 0,
            gone: 0,
            ended: 0,
            estates: 0,
            booked: 0,
            agents: 0,
            parties: 0,
            persons: 0,
            estates_settled: 0,
            estates_waiting: 0,
            estates_passed: 0,
            estates_written_off: 0,
        }
    }
}

/// What reading a process's chances needs: the register, the calendar and the country of each region.
struct Reading<'a> {
    register: &'a Register,
    calendar: &'a phx_core::Calendar,
    country_of: &'a dyn Fn(u32) -> Option<CountryId>,
}

/// An agent's present persons as a process reads them on a day: their places, each one's daily chance of a hit, and
/// the first day after it on which any of their chances may change.
#[clause("REP.7", "REP.25")]
fn chances(
    reading: &Reading<'_>,
    (kind, party): (&'static str, PartyId),
    bound: &Bound,
    household: &Household,
    day: Day,
) -> (Vec<usize>, Vec<f64>, Missing<Day>) {
    let date = reading.calendar.date(day);
    let attr = |name: &str| household.attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
    let view = AgentView { kind, party, attr: &attr, country_of: reading.country_of, date };
    let (mut places, mut qs, mut change) = (Vec::new(), Vec::new(), None::<Day>);
    for (i, person) in household.present() {
        places.push(i);
        qs.push(bound.process.rate(reading.register, &view, person));
        if let Some(on) = bound.process.changes_after(person, date) {
            let Some(on) = reading.calendar.day(on) else {
                violation!(clause = "TIME.2", "a person's chance changing before the epoch", party = party.get());
            };
            change = Some(match change {
                Some(earlier) if earlier <= on => earlier,
                _ => on,
            });
        }
    }
    (places, qs, change.map_or(Missing::Absent, Missing::Present))
}

/// An agent's next booking for a process drawn from `from` on, written to the agenda: a hit or a redraw on its day,
/// or none.
fn book(
    r: &Reading<'_>,
    who: (&'static str, PartyId),
    (b, table, slot): (&Bound, TableId, Slot),
    h: &Household,
    from: Day,
    (agenda, d): (&mut phx_core::Agenda, &mut Draws),
) {
    let (_, qs, change) = chances(r, who, b, h, from);
    match next_booking(d, any_hit(&qs), from, change) {
        Booking::Hit(day) => agenda.set_next_with(table, slot, b.reason, day, HIT),
        Booking::Redraw(day) => agenda.set_next_with(table, slot, b.reason, day, 0),
        Booking::Never => agenda.clear(table, slot, b.reason),
    }
}

/// An agent's booking for a process come due by `today`, followed on: each hit reaches its persons, each redraw draws
/// afresh from the day the chance changed, until a booking falls after today, which is written to the agenda. Persons a
/// hit reached are passed over by the later draws, as their outcomes have yet to change them.
#[clause("REP.7", "REP.12", "CHN.4")]
fn follow(
    r: &Reading<'_>,
    who: (&'static str, PartyId),
    (b, table, slot): (&Bound, TableId, Slot),
    h: &Household,
    today: Day,
    (agenda, d): (&mut phx_core::Agenda, &mut Draws),
) -> (Vec<usize>, u64) {
    let Some(mut at) = agenda.next(table, slot, b.reason) else {
        violation!(clause = "REP.7", "a booking gathered with no day", slot = slot.get());
    };
    let mut hit = agenda.with(table, slot, b.reason) == HIT;
    let (mut reached_all, mut redraws, mut out) = (Vec::new(), 0_u64, Vec::new());
    // The chances of the persons no hit has yet reached, from a day on.
    let open = |day: Day, reached_all: &[usize]| {
        let (places, qs, change) = chances(r, who, b, h, day);
        let open: Vec<(usize, f64)> = places.into_iter().zip(qs).filter(|(p, _)| !reached_all.contains(p)).collect();
        (open, change)
    };
    loop {
        let from = if hit {
            let (now, _) = open(at, &reached_all);
            let qs: Vec<f64> = now.iter().map(|(_, q)| *q).collect();
            if any_hit(&qs) > 0.0 {
                reached(d, &qs, &mut out);
                reached_all.extend(out.iter().filter_map(|i| now.get(*i).map(|(p, _)| *p)));
            }
            at.succ()
        } else {
            redraws += 1;
            at
        };
        let (next, change) = open(from, &reached_all);
        let qs: Vec<f64> = next.iter().map(|(_, q)| *q).collect();
        match next_booking(d, any_hit(&qs), from, change) {
            Booking::Hit(day) if day <= today => (at, hit) = (day, true),
            Booking::Redraw(day) if day <= today => (at, hit) = (day, false),
            Booking::Hit(day) => {
                agenda.set_next_with(table, slot, b.reason, day, HIT);
                break;
            }
            Booking::Redraw(day) => {
                agenda.set_next_with(table, slot, b.reason, day, 0);
                break;
            }
            Booking::Never => {
                agenda.clear(table, slot, b.reason);
                break;
            }
        }
    }
    reached_all.sort_unstable();
    (reached_all, redraws)
}

impl World {
    /// The country each region lies in.
    fn regions(&self) -> Vec<CountryId> {
        crate::world::geo_in(&self.own).map.regions.iter().map(|r| r.country).collect()
    }

    /// 3b: the agenda's agents for today, each booking come due followed on: its hits reach their persons, record
    /// their events at once and wait for 3e, and its next booking after today is drawn.
    #[clause("REP.7", "REP.12", "CHN.4")]
    pub(crate) fn agents_gather(&mut self, day: Day) {
        self.agent_day = AgentDay::of(day);
        let today = self.population.agenda.gather(day);
        let regions = self.regions();
        let country_of = |r: u32| regions.get(usize::try_from(r).ok()?).copied();
        let r = Reading { register: &self.register, calendar: &self.calendar, country_of: &country_of };
        let Population { kinds, agenda, .. } = &mut self.population;
        let cells = self.books.parties.cells_mut().0;
        for t in &today.per_table {
            let Some(k) = kinds.iter().position(|x| TableId::new(x.place) == t.table) else {
                violation!(clause = "TIME.5", "an agenda table no population kind keeps", table = t.table.get());
            };
            let Some(kd) = kinds.get(k) else { continue };
            let table = Population::table::<SystemBacking>(cells, k);
            for (slot, mask) in t.slots.iter().copied().zip(t.reasons.iter().copied()) {
                if mask == 0 || !table.is_live(slot) {
                    continue;
                }
                self.agent_day.gathered += 1;
                let party = table.party(slot);
                let twins = u64::from(table.multiplicity(slot).get());
                let h = household(&kd.decl, table, slot);
                for (p, b) in self.processes.iter().enumerate().filter(|(_, b)| b.kind == k) {
                    if mask & (1 << b.reason) == 0 {
                        continue;
                    }
                    self.agent_day.read += 1;
                    let subject = Subject::new(SubjectTag::Party, party.get());
                    let mut d = self.streams.open(&b.stream, subject, day, SubStep::S3b.ordinal());
                    let who = (kd.decl.kind, party);
                    let (hit, redraws) = follow(&r, who, (b, t.table, slot), &h, day, (agenda, &mut d));
                    self.agent_day.redraws += redraws;
                    if hit.is_empty() {
                        continue;
                    }
                    if rates::sampled(party) {
                        let year = rates::year_of(self.calendar.date(day));
                        self.metrics.rates.realised((p, year), &h, &hit, twins, self.calendar.date(day));
                    }
                    let details = [(Subject::new(SubjectTag::Party, party.get()), persons_i64(hit.len(), twins))];
                    self.events.record(NewEvent {
                        day,
                        substep: SubStep::S3b,
                        kind: b.event,
                        subjects: &[subject],
                        details: &details,
                        develops_from: Missing::Absent,
                    });
                    self.agent_day.hits += 1;
                    self.agent_day.persons_hit += phx_rand::float::len_u64(hit.len()) * twins;
                    self.agent_hits.push(AgentHit { process: p, kind: k, slot, party, reached: hit });
                }
            }
        }
        // Each event names its agent, which the directory keeps resolvable while the event does, even once it ends.
        let named: Vec<PartyId> = self.agent_hits.iter().map(|h| h.party).collect();
        let directory = self.books.parties.cells_mut().1;
        for party in named {
            directory.retain(party);
        }
        self.measure_rates(day);
    }

    /// 3e: each agent's hits of the day applied together to its household made explicit, by each process's outcome
    /// in order; the household written back, its gone persons' contracts leaving their lines at its multiplicity, and
    /// an agent no one is left in ended, what it held passing to an estate.
    #[clause("REP.26", "REP.23", "REP.16", "PTY.9")]
    pub(crate) fn agents_outcomes(&mut self, day: Day) {
        let hits = std::mem::take(&mut self.agent_hits);
        for agent in hits.chunk_by(|a, b| (a.kind, a.slot) == (b.kind, b.slot)) {
            let Some(first) = agent.first() else { continue };
            self.agent_outcomes(day, (first.kind, first.slot, first.party), agent);
        }
    }

    /// One agent's hits of the day applied.
    fn agent_outcomes(&mut self, day: Day, (kind, slot, party): (usize, Slot, PartyId), hits: &[AgentHit]) {
        let regions = self.regions();
        let country_of = |r: u32| regions.get(usize::try_from(r).ok()?).copied();
        let date = self.calendar.date(day);
        let cells = self.books.parties.cells_mut().0;
        let Some(kd) = self.population.kinds.get(kind) else {
            violation!(clause = "REP.7", "a hit on a kind the world does not keep", kind = kind);
        };
        let table = Population::table_mut::<SystemBacking>(cells, kind);
        if !table.is_live(slot) || table.party(slot) != party {
            violation!(clause = "REP.7", "a hit whose agent left its slot before its outcomes", party = party.get());
        }
        let twins = table.multiplicity(slot).get();
        let mut h = household(&kd.decl, table, slot);
        let attrs = h.attrs.clone();
        let attr = |name: &str| attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        let view = AgentView { kind: kd.decl.kind, party, attr: &attr, country_of: &country_of, date };
        for hit in hits {
            let Some(b) = self.processes.get(hit.process) else {
                violation!(clause = "REP.7", "a hit of a process the world does not hold");
            };
            let places: Vec<usize> =
                hit.reached.iter().copied().filter(|p| h.persons.get(*p).is_some_and(|q| !q.gone)).collect();
            if places.is_empty() {
                continue;
            }
            let mut d =
                self.streams.open(&b.stream, Subject::new(SubjectTag::Party, party.get()), day, SubStep::S3e.ordinal());
            b.process.outcome(&self.register, &view, &mut h, &places, &mut d);
        }
        let gone = phx_rand::float::len_u64(h.persons.iter().filter(|p| p.gone).count());
        let region = match kd.decl.sited_by {
            Missing::Present(i) => kd.decl.attrs.get(i).map(|a| h.attr(a.item.name)),
            Missing::Absent => None,
        };
        let written = write_back(&kd.decl, table, slot, &h);
        let k = u64::from(twins);
        self.agent_day.gone += gone * k;
        self.population.count(kind, (0, 0), (0, gone * k));
        let subject = Subject::new(SubjectTag::Party, party.get());
        let mut draws = self.streams.open(&HouseholdsStream::DECL, subject, day, SubStep::S3e.ordinal());
        self.leave(day, party, (&written.leaving, twins), &mut draws);
        if written.ended {
            self.end_agent(day, (kind, slot, party), region, &mut draws);
        }
    }

    /// The contracts of an agent's gone persons leaving their lines, each once for every twin, with as many of the
    /// lines' other sides.
    #[clause("REP.23", "REP.31")]
    fn leave(&mut self, day: Day, party: PartyId, (leaving, twins): (&[(LineId, Side)], u32), draws: &mut Draws) {
        let m = move_at(&self.register, day, ApplyAt::Day(SubStep::S3e));
        let mut by_side: Vec<((LineId, Side), u32)> = Vec::new();
        for at in leaving {
            match by_side.iter_mut().find(|(x, _)| x == at) {
                Some((_, n)) => *n += 1,
                None => by_side.push((*at, 1)),
            }
        }
        by_side.sort_unstable();
        for ((line, side), n) in by_side {
            let Some(count) = n.checked_mul(twins) else {
                phx_num::capacity_exceeded!("members leaving a line at once", u32::MAX, n);
            };
            if let Err(f) = self.books.members_leave((party, line, side), count, m, draws, self.audit.stream()) {
                violation!(clause = "REP.23", "members leaving a line did not settle", party = f.party.get());
            }
        }
    }

    /// An agent no one is left in ended: what it holds passes to one estate sited in its region, and its row and
    /// identity end.
    #[clause("PTY.9", "POP.15", "REP.16")]
    fn end_agent(&mut self, day: Day, (kind, slot, party): (usize, Slot, PartyId), region: Option<u32>, d: &mut Draws) {
        let m = move_at(&self.register, day, ApplyAt::Day(SubStep::S3e));
        let (rows, twins): (Vec<(LineId, Side, u32)>, u32) = {
            let table = Population::table::<SystemBacking>(self.books.parties.cells(), kind);
            for list in [AgentList::Holdings, AgentList::Lots, AgentList::NamedUnits] {
                if table.list(slot, list).len != 0 {
                    violation!(clause = "POP.15", "a household ended holding what only an estate can take yet");
                }
            }
            let rows = phx_ledger::rows::rows(table, slot).iter().map(|r| (r.row.line, r.side(), r.row.count)).collect();
            (rows, table.multiplicity(slot).get())
        };
        if !rows.is_empty() {
            let Some(region) = region else {
                violation!(
                    clause = "PTY.9",
                    "an estate of a kind that names no region to site it",
                    party = party.get()
                );
            };
            let site = self.estate_site(region, d);
            // An agent's twins leave an estate each, alike: one estate standing for them all.
            let estate = self.books.parties.begin_weighted(phx_core::ESTATE_KIND.name, site, day, twins);
            let succeeded = self.books.dues.succeeded;
            for (line, side, count) in rows {
                let t = LineTransfer { line, side, from: party, to: estate, count, reason: succeeded };
                if let Err(f) = self.books.transfer(t, m, self.audit.stream()) {
                    violation!(clause = "PTY.9", "an estate's succession did not settle", party = f.party.get());
                }
            }
            self.agent_day.estates += 1;
        }
        let (cells, directory, _) = self.books.parties.cells_mut();
        let table = Population::table_mut::<SystemBacking>(cells, kind);
        let twins = u64::from(twins);
        let id = table.id();
        table.remove(slot);
        directory.end(party, day, Missing::Absent);
        if self.population.agenda_table(kind).is_some() {
            self.population.agenda.release(id, slot);
        }
        self.population.count(kind, (0, twins), (0, 0));
        self.agent_day.ended += 1;
    }

    /// Where an estate of a region's households is sited: the centre of one of the region's zones, drawn.
    fn estate_site(&self, region: u32, draws: &mut Draws) -> phx_id::TileId {
        let geo = crate::world::geo_in(&self.own);
        let centres: Vec<phx_id::TileId> =
            geo.map.zones.iter().filter(|z| u32::from(z.region.get()) == region).map(|z| z.centroid).collect();
        let Some(at) = usize::try_from(phx_rand::below_u64(draws, phx_rand::float::len_u64(centres.len())))
            .ok()
            .and_then(|i| centres.get(i))
        else {
            violation!(clause = "PTY.5", "an estate sited in a region with no zone", region = region);
        };
        *at
    }

    /// 10b: every agent that began or changed today drawn afresh for each process on its kind, from tomorrow; and the
    /// day's counts kept.
    #[clause("REP.7", "REP.12", "REP.13")]
    pub(crate) fn agents_settle(&mut self, day: Day) {
        self.book_changed(day, SubStep::S10b.ordinal());
        let mut count = self.agent_day;
        let cells = self.books.parties.cells();
        let pop = &self.population;
        count.agents = (0..pop.kinds.len()).map(|k| Population::table::<SystemBacking>(cells, k).agents()).sum();
        count.parties = pop.members.iter().map(|(_, n)| n).sum();
        count.persons = pop.persons.iter().sum();
        self.agent_day = count;
        self.metrics.agents.push(count);
    }

    /// Every agent begun or changed since the last booking drawn afresh from the day after `day`; a kind no process
    /// acts on keeps no bookings.
    pub(crate) fn book_changed(&mut self, day: Day, ordinal: u8) {
        let regions = self.regions();
        let country_of = |r: u32| regions.get(usize::try_from(r).ok()?).copied();
        let r = Reading { register: &self.register, calendar: &self.calendar, country_of: &country_of };
        let Population { kinds, agenda, .. } = &mut self.population;
        let cells = self.books.parties.cells_mut().0;
        for (k, kd) in kinds.iter().enumerate() {
            let table = Population::table_mut::<SystemBacking>(cells, k);
            let changed = table.take_changed();
            if kd.processes == 0 {
                continue;
            }
            let id = TableId::new(kd.place);
            agenda.grow(id, table.high_water());
            for slot in changed.into_iter().filter(|s| table.is_live(*s)) {
                self.agent_day.booked += 1;
                let party = table.party(slot);
                let h = household(&kd.decl, table, slot);
                for b in self.processes.iter().filter(|b| b.kind == k) {
                    let subject = Subject::new(SubjectTag::Party, party.get());
                    let mut d = self.streams.open(&b.stream, subject, day, ordinal);
                    book(&r, (kd.decl.kind, party), (b, id, slot), &h, day.succ(), (agenda, &mut d));
                }
            }
        }
    }
}

/// What each of the day's instructions on agents needs besides its legs.
fn move_at(register: &Register, day: Day, at: ApplyAt) -> MoveAt {
    MoveAt { contracts: phx_ledger::opening::contract_unit(register), rounding: Round::HalfEven, day, at }
}

/// Persons a hit reached, each once for every twin, as an event's size.
fn persons_i64(reached: usize, twins: u64) -> i64 {
    let n = phx_rand::float::len_u64(reached) * twins;
    let Ok(n) = i64::try_from(n) else { phx_num::capacity_exceeded!("persons hit", i64::MAX, n) };
    n
}

impl World {
    /// The player's household, drawn at the opening from the households of the country the setup names, each real
    /// household equally likely: one twin of the agent drawn is seated as an agent of its own, of multiplicity one,
    /// with its persons, attributes and one twin's share of every contract, the agent left with one twin fewer; an
    /// agent of one twin is the player's as it stands.
    ///
    /// # Errors
    /// A world that keeps no household kind, or whose player's country holds no household.
    #[clause("OBS.4", "REP.1")]
    pub(crate) fn open_player(&mut self) -> Result<(), String> {
        let choice = self.game.setup.player;
        let country = choice
            .country
            .checked_sub(1)
            .and_then(|c| u8::try_from(c).ok())
            .map(CountryId::new)
            .ok_or_else(|| format!("the player lives in country {}, which is none", choice.country))?;
        let Some(k) = self.population.kinds.iter().position(|kd| kd.decl.kind == crate::consts::PLAYER_KIND) else {
            return Err(format!("the world keeps no `{}` kind for the player", crate::consts::PLAYER_KIND));
        };
        let regions = self.regions();
        let day = self.today;
        let Some(kd) = self.population.kinds.get(k) else {
            return Err("the player's kind has no table".to_owned());
        };
        let Missing::Present(sited) = kd.decl.sited_by else {
            return Err(format!("the `{}` kind names no region its agents are sited by", kd.decl.kind));
        };
        let table = Population::table::<SystemBacking>(self.books.parties.cells(), k);
        let in_country = |slot: Slot| {
            let region = usize::try_from(table.attr(slot, sited)).ok();
            region.and_then(|r| regions.get(r)) == Some(&country)
        };
        let eligible: Vec<(Slot, u64)> =
            table.slots().filter(|s| in_country(*s)).map(|s| (s, u64::from(table.multiplicity(s).get()))).collect();
        let households: u64 = eligible.iter().map(|(_, w)| w).sum();
        if households == 0 {
            return Err(format!("the player's country {} holds no household", choice.country));
        }
        let subject = Subject::new(SubjectTag::Country, u64::from(country.get()));
        let mut d = self.streams.open(&crate::opening::prims::PLAYER_STREAM, subject, day, 0);
        let mut at = phx_rand::below_u64(&mut d, households);
        let Some((slot, twins)) = eligible.iter().find_map(|(s, w)| {
            if at < *w {
                Some((*s, *w))
            } else {
                at -= w;
                None
            }
        }) else {
            violation!(clause = "OBS.4", "a household drawn past the country's households");
        };
        let donor = table.party(slot);
        let player = if twins == 1 { donor } else { self.seat_twin(day, k, (slot, donor)) };
        self.queue.seat(phx_core::Player { party: player, delegate: choice.delegate });
        Ok(())
    }

    /// One twin of an agent seated as an agent of its own, of multiplicity one: its household copied, and one twin's
    /// share of each of the agent's contracts moved to it.
    #[clause("REP.1", "REP.9", "REP.17")]
    fn seat_twin(&mut self, day: Day, kind: usize, (slot, donor): (Slot, PartyId)) -> PartyId {
        let (cells, directory, space) = self.books.parties.cells_mut();
        let table = Population::table_mut::<SystemBacking>(cells, kind);
        let attrs = table.attrs(slot);
        let per_twin = phx_pop::explicit::per_twin(table, slot);
        let (persons, attachments) = (table.persons(slot).to_vec(), table.attachments(slot).to_vec());
        let (seat, player) = phx_pop::table::begin(table, directory, space, (day, phx_core::Weight::new(1), &attrs));
        table.set_persons(seat, &persons);
        table.set_attachments(seat, &attachments);
        table.take_twin(slot);
        let m = move_at(&self.register, day, ApplyAt::Opening);
        let seated = self.books.dues.seated;
        for ((line, side), count) in per_twin {
            let t = LineTransfer { line, side, from: donor, to: player, count, reason: seated };
            if let Err(f) = self.books.transfer(t, m, self.audit.stream()) {
                violation!(clause = "REP.9", "a twin's contracts did not move to its seat", party = f.party.get());
            }
        }
        player
    }
}

#[cfg(test)]
mod tests {
    use phx_core::streams::Purpose;
    use phx_core::{
        ActsOn, AgentView, Declarations, DrawScheme, EventKindDecl, HandlerTable, HazardDecl, PopEntry, PopItem,
        PopProcess, RateFn, Register, RoleDecl, StreamDecl, System, declare_system,
    };
    use phx_pop::kind::PopKindDecl;

    use super::bind_one;

    /// A process as a system would declare it, of a hazard by name.
    struct Proc(&'static str);

    impl PopProcess for Proc {
        fn bind(&mut self, _: &Register) {}
        fn hazard(&self) -> &'static str {
            self.0
        }
        fn kind(&self) -> &'static str {
            "household"
        }
        fn rate(&self, _: &Register, _: &AgentView<'_>, _: &phx_core::Person) -> f64 {
            0.0
        }
        fn changes_after(&self, _: &phx_core::Person, _: phx_id::Date) -> Option<phx_id::Date> {
            None
        }
        fn outcome(
            &self,
            _: &Register,
            _: &AgentView<'_>,
            _: &mut phx_core::Household,
            _: &[usize],
            _: &mut phx_rand::Draws,
        ) {
        }
    }

    struct Dem;
    impl System for Dem {
        const CODE: &'static str = "DEM";
        fn declare(d: &mut Declarations) {
            d.stream(StreamDecl { name: "DEM.mortality", purpose: Purpose::Mortality, keyed: false, clause: "CHN.3" });
            d.event(EventKindDecl { name: "DEM.dies", size_unit: "persons", clause: "POP.3" });
            d.hazard(HazardDecl {
                name: "DEM.death",
                acts_on: ActsOn::Persons { kind: "household" },
                rate: RateFn { table: "DEM.life_table", axes: &["age"], changes: &[] },
                outcome: "DEM.dies",
                scheme: DrawScheme::Scheduled,
                stream: "DEM.mortality",
                clause: "POP.3",
                source: "life tables",
            });
        }
        fn handlers(_: &mut HandlerTable) {}
    }

    fn kinds() -> Vec<PopKindDecl> {
        let entries = [PopEntry {
            system: "DEM",
            kind: "household",
            item: PopItem::Role(RoleDecl { name: "head", clause: "x" }),
        }];
        vec![PopKindDecl::compile("household", &entries).unwrap()]
    }

    #[test]
    fn processes_bind_to_their_owners_hazards() {
        let (mut d, mut h) = (Declarations::new(), HandlerTable::default());
        declare_system::<Dem>(&mut d, &mut h);
        let kinds = kinds();
        let bound = bind_one("DEM", Box::new(Proc("DEM.death")), &d, &kinds).unwrap();
        assert_eq!((bound.kind, bound.event, bound.stream.name), (0, 0, "DEM.mortality"));
        let refused = |system, p: Proc| bind_one(system, Box::new(p), &d, &kinds).map(|_| ()).unwrap_err();
        assert!(refused("DEM", Proc("DEM.birth")).contains("no declared hazard"));
        assert!(refused("HH", Proc("DEM.death")).contains("its hazard is DEM's"));
    }
}
