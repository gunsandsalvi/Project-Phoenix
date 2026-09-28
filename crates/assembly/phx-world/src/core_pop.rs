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

use crate::agents::{Bound, Buffers, Reading, chances, follow};
use crate::consts::CORE_WHEEL_DAYS;
use crate::core::Core;

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
    /// The household kind's place on the core and among the population's kinds.
    fn household(&self) -> Option<(usize, usize)> {
        let place = self.names.iter().position(|n| *n == "household")?;
        Some((place, place.checked_sub(usize::from(self.first_agents))?))
    }

    /// A household read into its explicit form: its attributes by name from its record, its persons unpacked.
    fn read_household(&self, (place, decl): (usize, &PopKindDecl), slot: Slot, h: &mut Household) {
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
        if let Some(Some(p)) = self.persons.get(place) {
            h.persons.extend(p.of(slot).iter().map(|w| unpack(decl, *w)));
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
        // The gone leave from the last, so each earlier place stays where the contracts name it.
        for at in (0..before).rev() {
            if h.persons.get(at).is_some_and(|p| p.gone) {
                self.person_left(key, at);
                if let Some(Some(p)) = self.persons.get_mut(place) {
                    p.remove(&mut self.space, slot, at);
                }
                record.gone += 1;
            }
        }
        for p in h.persons.iter().skip(before).filter(|p| !p.gone) {
            if let Some(Some(persons)) = self.persons.get_mut(place) {
                persons.push(&mut self.space, slot, pack(decl, p));
            }
            record.born += 1;
        }
        let left: Vec<Person> = h.persons.into_iter().filter(|p| !p.gone).collect();
        if left.is_empty() {
            self.end_household(key);
            record.ended += 1;
            return;
        }
        let rest = Household { attrs, persons: left, positions: Vec::new() };
        let mut buffers = Buffers::default();
        self.book_all(ctx, (place, decl), slot, (&rest, &mut buffers), day.succ());
    }

    /// A person gone from its household: every contract naming it closes, and those naming a later place move up.
    #[clause("REP.3", "REP.26")]
    fn person_left(&mut self, household: PartyKey, at: usize) {
        let Ok(at) = u32::try_from(at) else { return };
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == household.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, household.slot()).collect();
            for edge in mine {
                let Some(row) = family.store.edges.row(edge) else { continue };
                if row.person == at {
                    family.store.close(edge);
                } else if row.person > at
                    && let Some(r) = family.store.edges.rows_mut().get_mut(index(edge))
                {
                    r.person -= 1;
                }
            }
        }
    }

    /// A household no one is left in ends: its contracts close and its slot is released after the day.
    #[clause("PTY.9")]
    fn end_household(&mut self, key: PartyKey) {
        for family in &mut self.families {
            let Some(side) = family.store.kinds.iter().position(|k| *k == key.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, key.slot()).collect();
            for edge in mine {
                family.store.close(edge);
            }
        }
        let place = usize::from(key.kind());
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
