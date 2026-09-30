//! The population's processes: each process on a kind's persons bound at assembly to its kind, hazard, event kind
//! and stream; a household's persons' chances read on a day; and a booking come due followed on to its hits.

use phx_core::{ActsOn, AgentView, Declarations, Household, PopProcess, Register, StreamDecl};
use phx_id::{CountryId, Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::hazard::{Booking, any_hit, next_booking, reached};
use phx_pop::kind::PopKindDecl;
use phx_rand::Draws;

/// A process on a kind's persons as the world runs it: its kind, its place among its kind's processes, the event kind
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
    let Some((_, stream)) = d.streams.iter().find(|(_, s)| s.name == hazard.stream) else {
        return Err(format!("hazard `{name}` draws from `{}`, no declared stream", hazard.stream));
    };
    let Some(event) = d.events.iter().position(|(_, e)| e.name == hazard.outcome) else {
        return Err(format!("hazard `{name}`'s outcome `{}` is no declared event kind", hazard.outcome));
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

/// What reading a process's chances needs: the register, the calendar and the country of each region.
pub(crate) struct Reading<'a> {
    pub register: &'a Register,
    pub calendar: &'a phx_core::Calendar,
    pub country_of: &'a (dyn Fn(u32) -> Option<CountryId> + Sync),
}

/// An agent's present persons as a process reads them on a day, written to `s`: their places and each one's daily
/// chance of a hit; returned, the first day after it on which any of their chances may change.
#[clause("REP.7", "REP.25")]
pub(crate) fn chances(
    reading: &Reading<'_>,
    (kind, party): (&'static str, PartyId),
    bound: &Bound,
    household: &Household,
    day: Day,
    s: &mut Buffers,
) -> Missing<Day> {
    let date = reading.calendar.date(day);
    let attr = |name: &str| household.attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
    let decider = |_: &str| violation!(clause = "MND.20", "a decision taken while a chance is read");
    let view = AgentView { kind, party, attr: &attr, country_of: reading.country_of, date, decider: &decider };
    let mut change = None::<Day>;
    s.places.clear();
    s.qs.clear();
    for (i, person) in household.present() {
        s.places.push(i);
        s.qs.push(bound.process.rate(reading.register, &view, person));
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
    change.map_or(Missing::Absent, Missing::Present)
}

/// The buffers reading chances fills, held by a pass over many agents and reused for each, so the pass allocates only
/// for its largest household.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    places: Vec<usize>,
    pub qs: Vec<f64>,
    open_places: Vec<usize>,
    open_qs: Vec<f64>,
    out: Vec<usize>,
}

/// A chunk of the day's follows: its household read, its chance buffers and what its follows came to.
#[derive(Debug, Default)]
pub(crate) struct FollowChunk {
    pub h: Household,
    pub buffers: Buffers,
    pub out: Vec<Followed>,
}

/// The day's follows — each booking due, the plan that chunks them, each chunk's own buffers — kept from day to day,
/// so a day allocates nothing once the heaviest has sized them.
#[derive(Debug, Default)]
pub(crate) struct Follows {
    pub todo: Vec<(phx_id::Slot, usize, (Day, bool))>,
    pub plan: phx_exec::ChunkPlan,
    pub chunks: Vec<FollowChunk>,
}

/// The day's follows put in the order their outcomes are applied: household by household in slot order, each
/// household's processes in their order, whatever order the processes' wheels gave them in.
pub(crate) fn in_apply_order<T>(todo: &mut [(phx_id::Slot, usize, T)]) {
    todo.sort_by_key(|(slot, process, _)| (slot.get(), *process));
}

/// What following a household's booking came to: the persons its hits reached, and its next booking after today.
#[derive(Debug)]
pub(crate) struct Followed {
    pub reached: Vec<usize>,
    pub next: Booking,
}

/// An agent's booking for a process come due by `today`, from the day and kind the agenda holds for it, followed on:
/// each hit reaches its persons, each redraw draws afresh from the day the chance changed, until a booking falls after
/// today. Persons a hit reached are passed over by the later draws, as their outcomes have yet to change them. It
/// reads only, so agents are followed on the pool; the next booking is written to the agenda after.
#[clause("REP.7", "REP.12", "CHN.4")]
pub(crate) fn follow(
    r: &Reading<'_>,
    who: (&'static str, PartyId),
    b: &Bound,
    (h, buffers): (&Household, &mut Buffers),
    (today, start): (Day, (Day, bool)),
    d: &mut Draws,
) -> Followed {
    let (mut at, mut hit) = start;
    let mut reached_all = Vec::new();
    let next = loop {
        let from = if hit {
            // The persons reached are drawn from the day's chances; when they change is the next draw's to read.
            let _ = open(r, (who, b, h), at, &reached_all, buffers);
            if any_hit(&buffers.open_qs) > 0.0 {
                reached(d, &buffers.open_qs, &mut buffers.out);
                reached_all.extend(buffers.out.iter().filter_map(|i| buffers.open_places.get(*i).copied()));
            }
            at.succ()
        } else {
            at
        };
        let change = open(r, (who, b, h), from, &reached_all, buffers);
        match next_booking(d, any_hit(&buffers.open_qs), from, change) {
            Booking::Hit(day) if day <= today => (at, hit) = (day, true),
            Booking::Redraw(day) if day <= today => (at, hit) = (day, false),
            later => break later,
        }
    };
    reached_all.sort_unstable();
    Followed { reached: reached_all, next }
}

/// The chances of the persons no hit has yet reached, from a day on, written to the buffers' open places and chances;
/// returned, the day any may change.
fn open(
    r: &Reading<'_>,
    (who, b, h): ((&'static str, PartyId), &Bound, &Household),
    day: Day,
    reached_all: &[usize],
    s: &mut Buffers,
) -> Missing<Day> {
    let change = chances(r, who, b, h, day, s);
    s.open_places.clear();
    s.open_qs.clear();
    for (p, q) in s.places.iter().zip(&s.qs) {
        if !reached_all.contains(p) {
            s.open_places.push(*p);
            s.open_qs.push(*q);
        }
    }
    change
}

#[cfg(test)]
mod tests {
    use phx_core::streams::Purpose;
    use phx_core::{
        ActsOn, AgentView, Declarations, DrawScheme, EventKindDecl, HazardDecl, PopEntry, PopItem, PopProcess, RateFn,
        Register, RoleDecl, StreamDecl, System, declare_system,
    };
    use phx_pop::kind::PopKindDecl;

    use phx_core::Household;
    use phx_id::{Day, PartyId};
    use phx_rand::{Draws, Subject, SubjectTag};

    use super::{Buffers, Reading, bind_one, follow};

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
            d.stream(StreamDecl {
                name: "DEM.mortality",
                family: phx_core::StreamFamily::World,
                purpose: Purpose::Mortality,
                keyed: false,
                clause: "CHN.3",
            });
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
    }

    fn kinds() -> Vec<PopKindDecl> {
        let entries = [PopEntry {
            system: "DEM",
            kind: "household",
            item: PopItem::Role(RoleDecl { name: "head", clause: "x" }),
        }];
        vec![PopKindDecl::compile("household", &entries).unwrap()]
    }

    /// A process whose persons each face a fixed daily chance.
    struct Rated(f64);

    impl PopProcess for Rated {
        fn bind(&mut self, _: &Register) {}
        fn hazard(&self) -> &'static str {
            "DEM.death"
        }
        fn kind(&self) -> &'static str {
            "household"
        }
        fn rate(&self, _: &Register, _: &AgentView<'_>, _: &phx_core::Person) -> f64 {
            self.0
        }
        fn changes_after(&self, _: &phx_core::Person, _: phx_id::Date) -> Option<phx_id::Date> {
            None
        }
        fn outcome(&self, _: &Register, _: &AgentView<'_>, _: &mut phx_core::Household, _: &[usize], _: &mut Draws) {}
    }

    #[test]
    fn follow_same_for_any_chunking() {
        let mut d = Declarations::new();
        declare_system::<Dem>(&mut d);
        let bound = bind_one("DEM", Box::new(Rated(0.02)), &d, &kinds()).unwrap();
        let register = phx_core::register::RegisterBuilder::new().build(&[], 1).unwrap();
        let calendar = phx_core::Calendar::new(phx_id::Date::new(2000, 1, 1).unwrap(), vec![], 2000).unwrap();
        let country_of = |_: u32| None;
        let reading = Reading { register: &register, calendar: &calendar, country_of: &country_of };
        let born = phx_id::Date::new(1960, 1, 1).unwrap();
        let person = || phx_core::Person { role: "head", born, attrs: Vec::new(), gone: false };
        let households: Vec<Household> = (0..40)
            .map(|i| Household {
                attrs: Vec::new(),
                persons: (0..=i % 5).map(|_| person()).collect(),
                positions: Vec::new(),
            })
            .collect();
        let today = Day::new(400);
        let key = phx_rand::key::stream_key(phx_rand::key::Seed::new(7), "DEM.mortality");
        // Each household booked some days back, as a hit or a redraw, and followed to today.
        let follow_in_runs = |run: usize| {
            let mut out = Vec::new();
            for chunk in households.iter().enumerate().collect::<Vec<_>>().chunks(run) {
                let mut buffers = Buffers::default();
                for (k, h) in chunk {
                    let k = u32::try_from(*k).unwrap();
                    let start = (Day::new(400 - k * 7), k % 2 == 0);
                    let id = PartyId::new(u64::from(k) + 1);
                    let mut draws = Draws::new(key, Subject::new(SubjectTag::Party, id.get()), today.get(), 0);
                    let f = follow(&reading, ("household", id), &bound, (h, &mut buffers), (today, start), &mut draws);
                    out.push((f.reached, f.next));
                }
            }
            out
        };
        let whole = follow_in_runs(households.len());
        assert!(whole.iter().any(|(r, _)| !r.is_empty()), "some hits reach persons");
        for run in [1, 3, 7] {
            assert_eq!(follow_in_runs(run), whole, "buffers reused over runs of {run}");
        }
    }

    #[test]
    fn apply_order_is_slot_then_process() {
        let slot = phx_id::Slot::new;
        let mut todo =
            vec![(slot(9), 0, 'a'), (slot(3), 1, 'b'), (slot(9), 1, 'c'), (slot(3), 0, 'd'), (slot(5), 2, 'e')];
        super::in_apply_order(&mut todo);
        assert_eq!(todo.iter().map(|t| t.2).collect::<String>(), "dbeac");
    }

    #[test]
    fn processes_bind_to_their_owners_hazards() {
        let mut d = Declarations::new();
        declare_system::<Dem>(&mut d);
        let kinds = kinds();
        let bound = bind_one("DEM", Box::new(Proc("DEM.death")), &d, &kinds).unwrap();
        assert_eq!((bound.kind, bound.event, bound.stream.name), (0, 0, "DEM.mortality"));
        let refused = |system, p: Proc| bind_one(system, Box::new(p), &d, &kinds).map(|_| ()).unwrap_err();
        assert!(refused("DEM", Proc("DEM.birth")).contains("no declared hazard"));
        assert!(refused("HH", Proc("DEM.death")).contains("its hazard is DEM's"));
    }
}
