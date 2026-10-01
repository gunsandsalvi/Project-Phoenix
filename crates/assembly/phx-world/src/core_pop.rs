//! Chance on the core's persons: for each process on the households' persons, a wheel of each household's next
//! booking — a hit or a redraw — read from the same rules the books' day reads. A household due is read into its
//! explicit form, its booking followed to today, and its hits' outcomes applied in process order; its gone persons
//! end that day, each one's contracts closing, its newborns are begun and join it, and a household no one is left in
//! ends. A household changed is booked again for every process from the next day.

use phx_core::person_word::PersonWord;
use phx_core::pop_process::{Household, HouseholdState, Person};
use phx_core::slots::DaySlot;
use phx_core::wheel::DueWheel;
use phx_core::{Register, WorldStreams};
use phx_id::{CountryId, Day, PartyKey, PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::hazard::{Booking, any_hit, next_booking};
use phx_pop::kind::PopKindDecl;
use phx_rand::{Subject, SubjectTag};

use crate::consts::HAZARD_FOLLOW_COST;
use crate::core::Core;
use crate::pop_rules::{Bound, Buffers, FollowChunk, Follows, Reading, chances, follow, in_apply_order};
use phx_core::capacity::WHEEL_DAYS;

/// One process's bookings on the core: its place among the world's processes, and each household's next booking on
/// a wheel, with its day and whether it is a hit by slot, so an entry the household was booked past is skipped.
#[derive(Debug, phx_macros::Saved)]
pub struct Hazard {
    pub process: usize,
    pub wheel: DueWheel,
    pub next: Vec<Option<(Day, bool)>>,
}

/// What the core's persons went through on a day: households followed, persons hit, born and gone, households ended,
/// and persons retired, with those of them who claimed the state pension.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct PopDay {
    pub followed: u64,
    pub hits: u64,
    pub born: u64,
    pub gone: u64,
    pub ended: u64,
    pub retired: u64,
    pub claimed: u64,
}

impl PopDay {
    /// The day's chance on persons in counts, for the bench's trace.
    pub(crate) fn note(&self) {
        let n = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
        phx_exec::trace::note(
            "hazards",
            &[
                ("followed", n(self.followed)),
                ("hits", n(self.hits)),
                ("born", n(self.born)),
                ("gone", n(self.gone)),
                ("ended", n(self.ended)),
                ("retired", n(self.retired)),
                ("claimed", n(self.claimed)),
            ],
        );
    }
}

/// Where what a person held and owed went at its death: to its household, which goes on; to the estate its household
/// ended into; or nowhere, its household ending holding nothing, its debts written off their creditors' books.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Destination {
    Household(PartyKey),
    Estate(PartyKey),
    Nothing,
}

/// A death: its day, the person, its household, the event kind of the process that took it, and where what it held and
/// owed went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Death {
    pub day: Day,
    pub person: u64,
    pub household: PartyKey,
    pub cause: u16,
    pub to: Destination,
}

/// A day's events of one kind: how many were recorded and the persons they reached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct EventCount {
    pub kind: u16,
    pub events: u64,
    pub persons: u64,
}

/// A household's hit by a process: its slot, the process, and the places of the persons it reached.
type Hit = (Slot, usize, Vec<usize>);

/// What the core's chance reads of the world.
pub(crate) struct Ctx<'a> {
    pub register: &'a Register,
    pub calendar: &'a phx_core::Calendar,
    pub streams: &'a WorldStreams,
    pub processes: &'a [Bound],
    pub regions: &'a [CountryId],
    /// The world's pool the hazards' wheels are taken on; none at the opening, before the world has one.
    pub pool: Option<&'a phx_exec::Pool>,
}

impl Core {
    /// An event of a kind counted today, with the persons it reached.
    fn count_event(&mut self, kind: u16, persons: u64) {
        match self.events_today.iter_mut().find(|e| e.kind == kind) {
            Some(count) => (count.events, count.persons) = (count.events + 1, count.persons + persons),
            None => self.events_today.push(EventCount { kind, events: 1, persons }),
        }
    }

    /// A hit recorded as an event: its household the subject, each person it reached a detail of one person.
    #[clause("OBS.3", "CHN.4")]
    fn record_event(&mut self, kind: u16, (slot, household): (Slot, phx_id::PartyRef), reached: &[usize], day: Day) {
        // The persons reached, read in the household's order, which the places a hit reached follow.
        let details: Vec<(Subject, i64)> = (self.members(slot).enumerate())
            .filter(|(at, _)| reached.contains(at))
            .map(|(_, (person, _))| (Subject::new(SubjectTag::Person, person.word()), 1))
            .collect();
        self.happened.record(phx_core::NewEvent {
            day,
            slot: DaySlot::S3b,
            kind,
            subjects: &[Subject::from(household)],
            details: &details,
            develops_from: phx_num::Missing::Absent,
        });
    }

    /// A household's rows at a slot, gathered once for a visit's reads; none before the households open.
    #[inline]
    pub fn household_view(&self, slot: Slot) -> Option<crate::household_store::HouseholdView<'_>> {
        self.households.as_ref()?.view(slot)
    }

    /// The rows of the party a key names, where its kind is the households'; none for a party of another kind.
    pub fn household_of(&self, party: PartyKey) -> Option<crate::household_store::HouseholdView<'_>> {
        let hs = self.households.as_ref()?;
        if party.kind() == hs.kind() { hs.view(party.slot()) } else { None }
    }

    /// A household's words written through its store; nothing before the households open.
    pub(crate) fn household_write(&mut self, write: impl FnOnce(&mut crate::household_store::HouseholdStore)) {
        if let Some(hs) = self.households.as_mut() {
            write(hs);
        }
    }

    /// The region the household at a slot lives in.
    pub fn household_region(&self, slot: Slot) -> Option<u32> {
        self.household_view(slot)?.region()
    }

    /// The region a party placed by its zone lies in, read from the store that holds its kind.
    pub fn zoned_region(&self, party: PartyKey) -> Option<u32> {
        match self.firm_of(party) {
            Some(f) => f.region(),
            None => self.household_of(party)?.region(),
        }
    }

    /// The household kind's place on the core and among the population's kinds.
    fn household(&self) -> Option<(usize, usize)> {
        let place = self.bound.kinds.household?;
        Some((place, self.household_pop))
    }

    /// A household's own state as its processes read it: its region, whether it tries for a child, its ideal and its
    /// outlook of its income.
    pub(crate) fn household_state(&self, slot: Slot) -> Option<HouseholdState> {
        let v = self.household_view(slot)?;
        Some(HouseholdState {
            region: v.region()?,
            trying: v.trying()?,
            ideal: v.ideal(),
            income: v.income().map_or(Missing::Absent, Missing::Present),
        })
    }

    /// A household read into its explicit form in a buffer kept across reads: its own state from its store, its
    /// persons' words in the order they joined it. A live household its store does not hold stops the run.
    #[phx_macros::absent_is_zero(reason = "a buffer before its first household holds no persons")]
    pub(crate) fn read_household<'h>(&self, slot: Slot, buffer: &'h mut Option<Household>) -> &'h mut Household {
        let Some(state) = self.household_state(slot) else {
            violation!(clause = "REP.41", "a household its store does not hold", slot = slot.get());
        };
        let mut persons = buffer.take().map(|h| h.persons).unwrap_or_default();
        persons.clear();
        persons.extend(self.members(slot).map(|(_, w)| Person::of(w)));
        buffer.insert(Household { state, persons })
    }

    /// Every household booked for every process on its persons, from `from` on.
    #[clause("REP.7")]
    pub(crate) fn open_hazards(&mut self, ctx: &Ctx<'_>, pop: &[(PopKindDecl, usize)], from: Day) {
        let Some((place, pop_at)) = self.household() else { return };
        let Some((decl, _)) = pop.get(pop_at) else { return };
        self.declared.household = Some(decl.clone());
        let high = self.directory.high_water(crate::core::kind_number(place));
        self.hazards = ctx
            .processes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == pop_at)
            .map(|(process, _)| Hazard {
                process,
                wheel: DueWheel::new(from, WHEEL_DAYS),
                next: vec![None; usize::try_from(high).unwrap_or(0)],
            })
            .collect();
        let slots: Vec<Slot> = self.directory.live_slots(crate::core::kind_number(place)).collect();
        let mut h = None;
        let mut buffers = Buffers::default();
        for slot in slots {
            let h = self.read_household(slot, &mut h);
            self.book_all(ctx, (place, decl), slot, (h, &mut buffers), from);
        }
    }

    /// A household's next booking for every process drawn from `from` on.
    fn book_all(
        &mut self,
        ctx: &Ctx<'_>,
        (place, decl): (usize, &PopKindDecl),
        slot: Slot,
        (h, buffers): (&Household, &mut Buffers),
        from: Day,
    ) {
        let Some(party) = self.directory.reference(crate::core::kind_number(place), slot) else { return };
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let reading = Reading { register: ctx.register, calendar: ctx.calendar, country_of: &country_of };
        for hz in &mut self.hazards {
            let Some(b) = ctx.processes.get(hz.process) else { continue };
            let change = chances(&reading, (decl.kind, party, decl.blank), b, h, from, buffers);
            let subject = Subject::from(party);
            let mut d = ctx.streams.open_at(&b.stream, subject, from, DaySlot::S10b.ordinal());
            let booking = next_booking(&mut d, any_hit(&buffers.qs), from, change);
            book(hz, slot, booking);
        }
    }

    /// The day's chance on the core's households: every booking due followed to today, and each household hit
    /// changed by its outcomes in process order and booked again.
    #[clause("REP.7", "REP.12", "REP.26", "PTY.9")]
    pub(crate) fn run_hazards(&mut self, ctx: &Ctx<'_>, day: Day) -> PopDay {
        let mut record = PopDay::default();
        let (Some((place, _)), Some(decl)) = (self.household(), self.declared.household.take()) else { return record };
        let decl = &decl;
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let reading = Reading { register: ctx.register, calendar: ctx.calendar, country_of: &country_of };
        let mut hits: Vec<Hit> = Vec::new();
        let mut due = Vec::new();
        let mut h = None;
        let mut buffers = Buffers::default();
        // A household whose persons labour changed since the last draws has its chances read again from today.
        for s in std::mem::take(&mut self.touched) {
            let slot = Slot::new(s);
            if self.directory.at(crate::core::kind_number(place), slot).is_none() {
                continue;
            }
            let h = self.read_household(slot, &mut h);
            self.book_all(ctx, (place, decl), slot, (h, &mut buffers), day);
        }
        // Every process's bookings due today, taken in process order.
        let mut follows = std::mem::take(&mut self.work.follows);
        follows.todo.clear();
        for at in 0..self.hazards.len() {
            let Some(hz) = self.hazards.get_mut(at) else { continue };
            hz.wheel.take(day, &mut due, ctx.pool);
            for slot in due.iter().copied().map(Slot::new) {
                let booked = self.hazards.get(at).and_then(|hz| hz.next.get(index(slot)).copied().flatten());
                let Some(start) = booked.filter(|(d, _)| *d == day) else { continue };
                if self.directory.at(crate::core::kind_number(place), slot).is_none() {
                    continue;
                }
                follows.todo.push((slot, at, start));
            }
        }
        // Each followed on the pool by runs of households in slot order, reading only; what it came to is applied
        // after, household by household and each household's processes in order.
        in_apply_order(&mut follows.todo);
        follows.plan.cut_rows(follows.todo.len(), HAZARD_FOLLOW_COST);
        follows.chunks.resize_with(follows.plan.len(), FollowChunk::default);
        let Follows { todo, plan, chunks } = &mut follows;
        let Some(chunks) = chunks.get_mut(..plan.len()) else {
            violation!(clause = "REP.7", "a plan of follows past its chunks' buffers", chunks = plan.len());
        };
        let this = &*self;
        phx_exec::for_plan(ctx.pool, plan, chunks, |rows, chunk| {
            chunk.out.clear();
            let Some(rows) = todo.get(rows) else {
                violation!(clause = "REP.7", "a chunk of follows past the day's bookings");
            };
            for &(slot, at, start) in rows {
                let process = this.hazards.get(at).map(|hz| hz.process);
                let (Some(id), Some(b)) = (
                    this.directory.reference(crate::core::kind_number(place), slot),
                    process.and_then(|p| ctx.processes.get(p)),
                ) else {
                    violation!(clause = "REP.7", "a booking due for no household or process", slot = slot.get());
                };
                let h = this.read_household(slot, &mut chunk.h);
                let subject = Subject::from(id);
                let mut d = ctx.streams.open_at(&b.stream, subject, day, DaySlot::S3b.ordinal());
                let f = follow(&reading, (decl.kind, id, decl.blank), b, (h, &mut chunk.buffers), (day, start), &mut d);
                chunk.out.push(f);
            }
        });
        let followed = chunks.iter_mut().flat_map(|c| c.out.drain(..));
        for ((slot, at, _), f) in todo.iter().copied().zip(followed) {
            let (Some(process), Some(id)) = (
                self.hazards.get(at).map(|hz| hz.process),
                self.directory.reference(crate::core::kind_number(place), slot),
            ) else {
                continue;
            };
            let Some(b) = ctx.processes.get(process) else { continue };
            record.followed += 1;
            if let Some(hz) = self.hazards.get_mut(at) {
                book(hz, slot, f.next);
            }
            if !f.reached.is_empty() {
                record.hits += 1;
                self.count_event(b.event, phx_rand::float::len_u64(f.reached.len()));
                self.record_event(b.event, (slot, id), &f.reached, day);
                if crate::core_rates::sampled(id) {
                    let h = self.read_household(slot, &mut h);
                    self.rates.realised(process, h, &f.reached, ctx.calendar.date(day));
                }
                hits.push((slot, process, f.reached));
            }
        }
        self.work.follows = follows;
        hits.sort_by_key(|(slot, process, _)| (slot.get(), *process));
        let mut i = 0;
        while let Some((slot, _, _)) = hits.get(i) {
            let slot = *slot;
            let end = hits.iter().skip(i).take_while(|(s, _, _)| *s == slot).count() + i;
            self.outcomes(ctx, (place, decl), slot, (hits.get(i..end).unwrap_or(&[]), day), &mut record);
            i = end;
        }
        self.declared.household = Some(decl.clone());
        record
    }

    /// One household's hits of the day applied, written back, and booked again from tomorrow.
    fn outcomes(
        &mut self,
        ctx: &Ctx<'_>,
        (place, decl): (usize, &PopKindDecl),
        slot: Slot,
        (hits, day): (&[Hit], Day),
        record: &mut PopDay,
    ) {
        let Some(id) = self.directory.reference(crate::core::kind_number(place), slot) else { return };
        // The household's read, its persons as read and the hits' places are kept across households.
        let mut w = std::mem::take(&mut self.work.outcome);
        let h = self.read_household(slot, &mut w.h);
        let before = h.persons.len();
        w.held.clear();
        w.held.extend(self.members(slot));
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let was = h.state;
        let decider = |name: &str| self.decided_in_process(name, (crate::core::kind_number(place), slot));
        let view = phx_core::pop_process::AgentView {
            kind: decl.kind,
            party: id,
            state: was,
            country_of: &country_of,
            date: ctx.calendar.date(day),
            decider: &decider,
            blank: decl.blank,
        };
        w.causes.clear();
        for (_, process, reached) in hits {
            let Some(b) = ctx.processes.get(*process) else { continue };
            w.places.clear();
            w.places.extend(reached.iter().copied().filter(|p| h.persons.get(*p).is_some_and(|q| !q.gone)));
            if w.places.is_empty() {
                continue;
            }
            let mut d = ctx.streams.open_at(&b.stream, Subject::from(id), day, DaySlot::S3b.ordinal());
            b.process.outcome(ctx.register, &view, h, &w.places, &mut d);
            // Each person gone first by this process takes its event as its cause.
            for (at, _) in h.persons.iter().enumerate().take(before).filter(|(_, p)| p.gone) {
                if !w.causes.iter().any(|(i, _)| *i == at) {
                    w.causes.push((at, b.event));
                }
            }
        }
        if h.persons.len() < before {
            violation!(
                clause = "REP.26",
                "an outcome took persons out rather than marking them gone",
                party = id.word()
            );
        }
        let key = PartyKey::new(u8::try_from(place).unwrap_or(u8::MAX), slot);
        self.record_vitals(h, &w.held, (ctx, day));
        self.write_changed(key, (&h.persons, &w.held), (ctx, day), record);
        // What an outcome changed of the household's own, as its decision to try for a child, is its store's.
        let now = h.state;
        if now.trying != was.trying {
            self.household_write(|hs| hs.set_trying(slot, now.trying));
        }
        if now.ideal != was.ideal {
            self.household_write(|hs| hs.set_ideal(slot, now.ideal));
        }
        // The gone end the same day, their household succeeding to what they held; the born join it.
        let died = self.deaths.len();
        w.causes.sort_unstable();
        for (at, (person, _)) in w.held.iter().enumerate().filter(|(at, _)| h.persons.get(*at).is_some_and(|p| p.gone))
        {
            if let (Some(ps), Some(hs)) = (self.persons.as_mut(), self.households.as_mut()) {
                ps.end_person(&mut self.directory, &mut hs.heads(), *person, (day, Missing::Present(id)));
            }
            self.person_left(key, person.word());
            record.gone += 1;
            let Some(cause) = w.causes.iter().find(|(i, _)| *i == at).map(|(_, e)| *e) else {
                violation!(clause = "POP.15", "a person gone by no process", party = id.word());
            };
            let death = Death { day, person: person.word(), household: key, cause, to: Destination::Household(key) };
            self.deaths.push(death);
        }
        for p in h.persons.iter().skip(before).filter(|p| !p.gone) {
            if let (Some(ps), Some(hs)) = (self.persons.as_mut(), self.households.as_mut()) {
                let _ = ps.begin_person(&mut self.directory, &mut hs.heads(), id, (p.word, &[]));
            }
            record.born += 1;
        }
        h.persons.retain(|p| !p.gone);
        if h.persons.is_empty() {
            let country = ctx.regions.get(usize::try_from(h.state.region).unwrap_or(usize::MAX)).copied();
            let Some(country) = country else {
                violation!(clause = "PTY.5", "an ended household sited in no country", party = id.word());
            };
            let to = self.end_household(key, (country, day)).map_or(Destination::Nothing, Destination::Estate);
            for d in self.deaths.iter_mut().skip(died) {
                d.to = to;
            }
            record.ended += 1;
            self.work.outcome = w;
            return;
        }
        self.book_all(ctx, (place, decl), slot, (h, &mut w.buffers), day.succ());
        self.work.outcome = w;
    }

    /// The deaths an outcome made and the onsets of disability, each counted in its household's country's month by the
    /// person's age class and health before it.
    fn record_vitals(&mut self, h: &Household, held: &[(PartyRef, PersonWord)], (ctx, day): (&Ctx<'_>, Day)) {
        let Some(country) = ctx.regions.get(usize::try_from(h.state.region).unwrap_or(usize::MAX)).map(|c| c.get())
        else {
            return;
        };
        let date = ctx.calendar.date(day);
        for (p, was) in h.persons.iter().zip(held) {
            let before = was.1;
            let health = before.get(if_pop::HEALTH.field);
            let age = before.age_on(date);
            if p.gone {
                self.record_vital(country, (age, health), crate::core_stats::DEATH);
            } else if health == if_pop::ABLE && p.get(if_pop::HEALTH.field) == if_pop::DISABLED {
                self.record_vital(country, (age, health), crate::core_stats::ONSET);
            }
        }
    }

    /// The persons an outcome changed and kept, written back; one who retired leaves its jobs and the searchers, and
    /// claims its state pension.
    #[clause("REP.26", "LAB.6")]
    fn write_changed(
        &mut self,
        key: PartyKey,
        (persons, held): (&[Person], &[(PartyRef, PersonWord)]),
        (ctx, day): (&Ctx<'_>, Day),
        record: &mut PopDay,
    ) {
        let state = sys_lab::STATE.field;
        for (p, (r, word)) in persons.iter().zip(held) {
            if p.gone || p.word == *word {
                continue;
            }
            if let Some(ps) = self.persons.as_mut() {
                ps.set_word(&self.directory, *r, p.word);
            }
            if p.get(state) == if_labour::class::RETIRED && word.get(state) != if_labour::class::RETIRED {
                self.leave_jobs(key, r.word());
                record.retired += 1;
                if let Some(country) = self.country_of_household(ctx, key)
                    && self.claim_pension((ctx.calendar, ctx.streams, day), (key, p), (r.word(), country))
                {
                    record.claimed += 1;
                }
            }
        }
    }

    /// The country a household is sited in, by the region its zone lies in.
    fn country_of_household(&self, ctx: &Ctx<'_>, key: PartyKey) -> Option<u8> {
        let region = self.household_region(key.slot())?;
        ctx.regions.get(usize::try_from(region).ok()?).map(|c| c.get())
    }

    /// A person gone from its household: every contract naming it closes, and it no longer works in a firm it owns.
    #[clause("REP.3", "REP.26", "REP.31")]
    fn person_left(&mut self, household: PartyKey, person: u64) {
        self.stop_working(household, person);
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == household.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, household.slot()).collect();
            for edge in mine {
                if family.store.edges.row(edge).is_some_and(|r| r.person == person) {
                    family.close_contract(edge);
                }
            }
        }
    }

    /// A household no one is left in ends: what its account holds and the shares it owns pass to an estate that opens at
    /// the same bank, owing what the household owed, its contracts close, and its slot is released after the day.
    #[clause("PTY.9", "SET.7")]
    fn end_household(&mut self, key: PartyKey, (country, day): (CountryId, Day)) -> Option<PartyKey> {
        let place = usize::from(key.kind());
        let account = self.kinds.get(place).and_then(|k| k.accounts.as_ref()).and_then(|a| {
            let (bank, balance) = (a.bank.get(key.slot())?, a.balance.get(key.slot())?);
            let pending = a.pending.get(key.slot())?;
            Some((bank, balance + pending))
        });
        let debts = self.debts_of(key);
        let mut estate = None;
        let owns = self.owners.holds.contains_key(&key);
        if let Some((bank, money)) = account.filter(|(_, m)| *m != 0 || owns) {
            let e = self.open_estate((bank, money), (country, day));
            self.pass_holdings(key, e);
            self.insolvency.claims.insert(e, debts);
            if let Some(a) = self.kinds.get_mut(place).and_then(|k| k.accounts.as_mut()) {
                a.balance.set(key.slot(), 0);
                a.pending.set(key.slot(), 0);
            }
            estate = Some(e);
        }
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == key.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, key.slot()).collect();
            for edge in mine {
                family.close_contract(edge);
            }
        }
        self.end_party(key, day, estate);
        estate
    }
}

fn index(slot: Slot) -> usize {
    usize::try_from(slot.get()).unwrap_or(usize::MAX)
}

/// What applying one household's outcomes reads and writes, kept across households so applying them allocates nothing.
#[derive(Debug, Default)]
pub(crate) struct OutcomeWork {
    h: Option<Household>,
    held: Vec<(PartyRef, PersonWord)>,
    /// Each person gone, by its place, and the event of the process that first marked it.
    causes: Vec<(usize, u16)>,
    places: Vec<usize>,
    buffers: Buffers,
}

/// A household's next booking written to its process's wheel: a hit or a redraw on its day, or none.
fn book(hz: &mut Hazard, slot: Slot, booking: Booking) {
    let at = index(slot);
    if hz.next.len() <= at {
        hz.next.resize(at + 1, None);
    }
    let next = match booking {
        Booking::Hit(d) => Some((d, true)),
        Booking::Redraw(d) => Some((d, false)),
        Booking::Never => None,
    };
    if let Some(n) = hz.next.get_mut(at) {
        *n = next;
    }
    if let Some((day, _)) = next {
        hz.wheel.schedule(slot.get(), day);
    }
}
