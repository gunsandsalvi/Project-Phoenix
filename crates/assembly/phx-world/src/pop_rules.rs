//! The population's processes: each process on a kind's persons bound at assembly to its kind, hazard, event kind
//! and stream; a household's persons' chances read on a day; and a booking come due followed on to its hits.

use phx_core::{ActsOn, AgentView, Declarations, Household, PopProcess, Register, StreamDecl};
use phx_id::{CountryId, Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::hazard::{Booking, any_hit, next_booking, reached};
use phx_pop::kind::PopKindDecl;
use phx_rand::Draws;

/// A process on a kind's persons as the world runs it: its kind, its place among its kind's processes, the stream it
/// draws from, and the system's process.
pub(crate) struct Bound {
    pub kind: usize,
    pub reason: usize,
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
    if !d.events.iter().any(|(_, e)| e.name == hazard.outcome) {
        return Err(format!("hazard `{name}`'s outcome `{}` is no declared event kind", hazard.outcome));
    }
    Ok(Bound { kind, reason: 0, stream: *stream, process })
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
    let view = AgentView { kind, party, attr: &attr, country_of: reading.country_of, date };
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

/// What following a household's booking came to: the persons its hits reached, and its next booking after today.
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
        assert_eq!((bound.kind, bound.stream.name), (0, "DEM.mortality"));
        let refused = |system, p: Proc| bind_one(system, Box::new(p), &d, &kinds).map(|_| ()).unwrap_err();
        assert!(refused("DEM", Proc("DEM.birth")).contains("no declared hazard"));
        assert!(refused("HH", Proc("DEM.death")).contains("its hazard is DEM's"));
    }
}
