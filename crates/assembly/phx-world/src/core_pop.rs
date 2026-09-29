//! Chance on the core's persons: for each process on the households' persons, a wheel of each household's next
//! booking — a hit or a redraw — read from the same rules the books' day reads. A household due is read into its
//! explicit form, its booking followed to today, and its hits' outcomes applied in process order; its gone persons
//! leave it, each one's contracts closing and the places after it moving up, its newborns join it, and a household no
//! one is left in ends. A household changed is booked again for every process from the next day.

use phx_core::pop_process::{Household, Person};
use phx_core::wheel::DueWheel;
use phx_core::{Register, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_pop::hazard::{Booking, any_hit, next_booking};
use phx_pop::kind::PopKindDecl;
use phx_pop::person::{pack, unpack};
use phx_rand::{Subject, SubjectTag};

use crate::consts::CORE_WHEEL_DAYS;
use crate::core::Core;
use crate::pop_rules::{Bound, Buffers, Reading, chances, follow};

/// One process's bookings on the core: its place among the world's processes, and each household's next booking on
/// a wheel, with its day and whether it is a hit by slot, so an entry the household was booked past is skipped.
#[derive(Debug)]
pub struct Hazard {
    pub process: usize,
    pub wheel: DueWheel,
    pub next: Vec<Option<(Day, bool)>>,
}

/// What the core's persons went through on a day: households followed, persons hit, born and gone, households ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PopDay {
    pub followed: u64,
    pub hits: u64,
    pub born: u64,
    pub gone: u64,
    pub ended: u64,
}

/// A day's events of one kind: how many were recorded and the persons they reached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    pub streams: &'a Streams,
    pub processes: &'a [Bound],
    pub regions: &'a [CountryId],
}

impl Core {
    /// An event of a kind counted today, with the persons it reached.
    fn count_event(&mut self, kind: u16, persons: u64) {
        match self.events_today.iter_mut().find(|e| e.kind == kind) {
            Some(count) => (count.events, count.persons) = (count.events + 1, count.persons + persons),
            None => self.events_today.push(EventCount { kind, events: 1, persons }),
        }
    }

    /// The household kind's place on the core and among the population's kinds.
    fn household(&self) -> Option<(usize, usize)> {
        let place = self.names.iter().position(|n| *n == "household")?;
        Some((place, self.household_pop))
    }

    /// A household read into its explicit form: its attributes and positions by name from its record, its persons
    /// unpacked.
    pub(crate) fn read_household(&self, (place, decl): (usize, &PopKindDecl), slot: Slot, h: &mut Household) {
        h.attrs.clear();
        h.persons.clear();
        h.positions.clear();
        let Some(store) = self.kinds.get(place) else { return };
        let record = store.record(slot);
        for (a, w) in decl.attrs.iter().zip(record) {
            let v = match w.get() {
                phx_num::Missing::Present(v) => u32::try_from(v).unwrap_or_else(|_| {
                    violation!(clause = "REP.41", "an attribute beyond its width", slot = slot.get())
                }),
                phx_num::Missing::Absent => {
                    violation!(clause = "REP.41", "a household missing an attribute", slot = slot.get())
                }
            };
            h.attrs.push((a.item.name, v));
        }
        for (i, p) in decl.positions.iter().enumerate() {
            let v = record.get(decl.attrs.len() + i).map_or(phx_num::Missing::Absent, |w| w.get());
            h.positions.push((p.item.name, v));
        }
        if let Some(Some(p)) = self.persons.get(place) {
            h.persons.extend(p.of(slot).map(|x| unpack(decl, x.word)));
        }
    }

    /// Every household booked for every process on its persons, from `from` on.
    #[clause("REP.7")]
    pub(crate) fn open_hazards(&mut self, ctx: &Ctx<'_>, pop: &[(PopKindDecl, usize)], from: Day) {
        let Some((place, pop_at)) = self.household() else { return };
        let Some((decl, _)) = pop.get(pop_at) else { return };
        self.household_decl = Some(decl.clone());
        let high = self.kinds.get(place).map_or(0, |k| k.parties.high_water());
        self.hazards = ctx
            .processes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == pop_at)
            .map(|(process, _)| Hazard {
                process,
                wheel: DueWheel::new(from, CORE_WHEEL_DAYS),
                next: vec![None; usize::try_from(high).unwrap_or(0)],
            })
            .collect();
        let slots: Vec<Slot> = self.kinds.get(place).map(|k| k.parties.live_slots().collect()).unwrap_or_default();
        let mut h = Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() };
        let mut buffers = Buffers::default();
        for slot in slots {
            self.read_household((place, decl), slot, &mut h);
            self.book_all(ctx, (place, decl), slot, (&h, &mut buffers), from);
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
        let Some(party) = self.kinds.get(place).and_then(|k| k.parties.id(slot)) else { return };
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let reading = Reading { register: ctx.register, calendar: ctx.calendar, country_of: &country_of };
        for hz in &mut self.hazards {
            let Some(b) = ctx.processes.get(hz.process) else { continue };
            let change = chances(&reading, (decl.kind, party), b, h, from, buffers);
            let subject = Subject::new(SubjectTag::Party, party.get());
            let mut d = ctx.streams.open(&b.stream, subject, from, SubStep::S10b.ordinal());
            let booking = next_booking(&mut d, any_hit(&buffers.qs), from, change);
            book(hz, slot, booking);
        }
    }

    /// The day's chance on the core's households: every booking due followed to today, and each household hit
    /// changed by its outcomes in process order and booked again.
    #[clause("REP.7", "REP.12", "REP.26", "PTY.9")]
    pub(crate) fn run_hazards(&mut self, ctx: &Ctx<'_>, day: Day) -> PopDay {
        let mut record = PopDay::default();
        let (Some((place, _)), Some(decl)) = (self.household(), self.household_decl.take()) else { return record };
        let decl = &decl;
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let reading = Reading { register: ctx.register, calendar: ctx.calendar, country_of: &country_of };
        let mut hits: Vec<Hit> = Vec::new();
        let mut due = Vec::new();
        let mut h = Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() };
        let mut buffers = Buffers::default();
        // A household whose persons labour changed since the last draws has its chances read again from today.
        for s in std::mem::take(&mut self.touched) {
            let slot = Slot::new(s);
            if self.kinds.get(place).and_then(|k| k.parties.at(slot)).is_none() {
                continue;
            }
            self.read_household((place, decl), slot, &mut h);
            self.book_all(ctx, (place, decl), slot, (&h, &mut buffers), day);
        }
        for at in 0..self.hazards.len() {
            let Some(hz) = self.hazards.get_mut(at) else { continue };
            hz.wheel.take(day, &mut due, None);
            let process = hz.process;
            for slot in due.iter().copied().map(Slot::new) {
                let booked = self.hazards.get(at).and_then(|hz| hz.next.get(index(slot)).copied().flatten());
                let Some((booked_day, hit)) = booked.filter(|(d, _)| *d == day) else { continue };
                if self.kinds.get(place).and_then(|k| k.parties.at(slot)).is_none() {
                    continue;
                }
                let Some(id) = self.kinds.get(place).and_then(|k| k.parties.id(slot)) else { continue };
                let Some(b) = ctx.processes.get(process) else { continue };
                self.read_household((place, decl), slot, &mut h);
                let subject = Subject::new(SubjectTag::Party, id.get());
                let mut d = ctx.streams.open(&b.stream, subject, day, SubStep::S3b.ordinal());
                let f = follow(&reading, (decl.kind, id), b, (&h, &mut buffers), (day, (booked_day, hit)), &mut d);
                record.followed += 1;
                if let Some(hz) = self.hazards.get_mut(at) {
                    book(hz, slot, f.next);
                }
                if !f.reached.is_empty() {
                    record.hits += 1;
                    self.count_event(b.event, phx_rand::float::len_u64(f.reached.len()));
                    if crate::core_rates::sampled(id) {
                        self.rates.realised(process, &h, &f.reached, ctx.calendar.date(day));
                    }
                    hits.push((slot, process, f.reached));
                }
            }
        }
        hits.sort_by_key(|(slot, process, _)| (slot.get(), *process));
        let mut i = 0;
        while let Some((slot, _, _)) = hits.get(i) {
            let slot = *slot;
            let end = hits.iter().skip(i).take_while(|(s, _, _)| *s == slot).count() + i;
            self.outcomes(ctx, (place, decl), slot, (hits.get(i..end).unwrap_or(&[]), day), &mut record);
            i = end;
        }
        self.household_decl = Some(decl.clone());
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
        let Some(id) = self.kinds.get(place).and_then(|k| k.parties.id(slot)) else { return };
        let mut h = Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() };
        self.read_household((place, decl), slot, &mut h);
        let before = h.persons.len();
        let held: Vec<phx_pop::persons::Held> =
            self.persons.get(place).and_then(Option::as_ref).map(|p| p.of(slot).collect()).unwrap_or_default();
        let country_of = |r: u32| ctx.regions.get(usize::try_from(r).ok()?).copied();
        let attrs = h.attrs.clone();
        let attr = |name: &str| attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        let view = phx_core::pop_process::AgentView {
            kind: decl.kind,
            party: id,
            attr: &attr,
            country_of: &country_of,
            date: ctx.calendar.date(day),
        };
        for (_, process, reached) in hits {
            let Some(b) = ctx.processes.get(*process) else { continue };
            let places: Vec<usize> =
                reached.iter().copied().filter(|p| h.persons.get(*p).is_some_and(|q| !q.gone)).collect();
            if places.is_empty() {
                continue;
            }
            let mut d =
                ctx.streams.open(&b.stream, Subject::new(SubjectTag::Party, id.get()), day, SubStep::S3e.ordinal());
            b.process.outcome(ctx.register, &view, &mut h, &places, &mut d);
        }
        if h.persons.len() < before {
            violation!(
                clause = "REP.26",
                "an outcome took persons out rather than marking them gone",
                party = id.get()
            );
        }
        let key = PartyKey::new(u8::try_from(place).unwrap_or(u8::MAX), slot);
        self.record_vitals((decl, &h), &held, (ctx, day));
        self.write_changed((place, decl), key, &h.persons, &held);
        // The household's own attributes an outcome changed, as its decision to try for a child, are its record's.
        if h.attrs != attrs
            && let Some(store) = self.kinds.get_mut(place)
        {
            let record = store.record_mut(slot);
            for ((_, v), w) in h.attrs.iter().zip(record.iter_mut()) {
                *w = phx_num::MaybeI64::present(i64::from(*v));
            }
        }
        // The gone leave from the last, so each earlier place still reads the person the outcome marked.
        for at in (0..before).rev() {
            if h.persons.get(at).is_some_and(|p| p.gone) {
                let Some(Some(persons)) = self.persons.get_mut(place) else { continue };
                let Some(person) = persons.of(slot).nth(at).map(|x| x.id) else {
                    violation!(clause = "REP.26", "a person gone that its household does not hold", party = id.get());
                };
                persons.remove(&mut self.space, slot, at);
                self.person_left(key, person);
                record.gone += 1;
            }
        }
        for p in h.persons.iter().skip(before).filter(|p| !p.gone) {
            let person = self.next_id;
            self.next_id += 1;
            if let Some(Some(persons)) = self.persons.get_mut(place) {
                persons.push(&mut self.space, slot, phx_pop::persons::Held { word: pack(decl, p), id: person });
            }
            record.born += 1;
        }
        let left: Vec<Person> = h.persons.into_iter().filter(|p| !p.gone).collect();
        if left.is_empty() {
            let sited = match decl.sited_by {
                phx_num::Missing::Present(i) => decl.attrs.get(i),
                phx_num::Missing::Absent => None,
            };
            let region = sited.and_then(|a| h.attrs.iter().find(|(n, _)| *n == a.item.name).map(|(_, v)| *v));
            let country = region.and_then(|r| ctx.regions.get(usize::try_from(r).ok()?).copied());
            let Some(country) = country else {
                violation!(clause = "PTY.5", "an ended household sited in no country", party = id.get());
            };
            self.end_household(key, (country, day));
            record.ended += 1;
            return;
        }
        let rest = Household { attrs: h.attrs, persons: left, positions: h.positions };
        let mut buffers = Buffers::default();
        self.book_all(ctx, (place, decl), slot, (&rest, &mut buffers), day.succ());
    }

    /// The deaths an outcome made and the onsets of disability, each counted in its household's country's month by the
    /// person's age class and health before it.
    fn record_vitals(
        &mut self,
        (decl, h): (&PopKindDecl, &Household),
        held: &[phx_pop::persons::Held],
        (ctx, day): (&Ctx<'_>, Day),
    ) {
        let phx_num::Missing::Present(region_at) = decl.sited_by else { return };
        let Some(region) = decl.attrs.get(region_at).and_then(|a| h.attrs.iter().find(|(n, _)| *n == a.item.name))
        else {
            return;
        };
        let Some(country) = ctx.regions.get(usize::try_from(region.1).unwrap_or(usize::MAX)).map(|c| c.get()) else {
            return;
        };
        let date = ctx.calendar.date(day);
        for (p, was) in h.persons.iter().zip(held) {
            let before = unpack(decl, was.word);
            let health = before.attr(if_pop::HEALTH.name).unwrap_or(if_pop::ABLE);
            let age = before.age_on(date);
            if p.gone {
                self.record_vital(country, (age, health), crate::core_stats::DEATH);
            } else if health == if_pop::ABLE && p.attr(if_pop::HEALTH.name) == Some(if_pop::DISABLED) {
                self.record_vital(country, (age, health), crate::core_stats::ONSET);
            }
        }
    }

    /// The persons an outcome changed and kept, written back; one who retired leaves its jobs and the searchers.
    #[clause("REP.26", "LAB.6")]
    fn write_changed(
        &mut self,
        (place, decl): (usize, &PopKindDecl),
        key: PartyKey,
        persons: &[Person],
        held: &[phx_pop::persons::Held],
    ) {
        let state = sys_lab::STATE.name;
        for (at, (p, h)) in persons.iter().zip(held).enumerate() {
            if p.gone {
                continue;
            }
            let word = pack(decl, p);
            if word == h.word {
                continue;
            }
            if let Some(Some(ps)) = self.persons.get_mut(place) {
                ps.set_word(&mut self.space, key.slot(), at, word);
            }
            let was = unpack(decl, h.word).attr(state);
            if p.attr(state) == Some(if_labour::class::RETIRED) && was != Some(if_labour::class::RETIRED) {
                self.leave_jobs(key, h.id);
            }
        }
    }

    /// A person gone from its household: every contract naming it closes.
    #[clause("REP.3", "REP.26")]
    fn person_left(&mut self, household: PartyKey, person: u64) {
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == household.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, household.slot()).collect();
            for edge in mine {
                if family.store.edges.row(edge).is_some_and(|r| r.person == person) {
                    family.store.close(edge);
                }
            }
        }
    }

    /// A household no one is left in ends: its contracts close, what its account holds passes to an estate that
    /// opens at the same bank, and its slot is released after the day.
    #[clause("PTY.9")]
    fn end_household(&mut self, key: PartyKey, (country, day): (CountryId, Day)) {
        let place = usize::from(key.kind());
        let account = self.kinds.get(place).and_then(|k| k.accounts.as_ref()).and_then(|a| {
            let (bank, balance) = (a.bank.get(key.slot())?, a.balance.get(key.slot())?);
            let pending = a.pending.get(key.slot())?;
            Some((bank, balance + pending))
        });
        if let Some((bank, money)) = account.filter(|(_, m)| *m != 0) {
            let _ = self.open_estate((bank, money), (country, day));
            if let Some(a) = self.kinds.get_mut(place).and_then(|k| k.accounts.as_mut()) {
                a.balance.set(key.slot(), 0);
                a.pending.set(key.slot(), 0);
            }
        }
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == key.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, key.slot()).collect();
            for edge in mine {
                family.store.close(edge);
            }
        }
        if let Some(Some(p)) = self.persons.get_mut(place) {
            p.clear(&mut self.space, key.slot());
        }
        if let Some(k) = self.kinds.get_mut(place)
            && let Some(r) = k.parties.at(key.slot())
        {
            k.parties.end(r);
        }
    }
}

fn index(slot: Slot) -> usize {
    usize::try_from(slot.get()).unwrap_or(usize::MAX)
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
