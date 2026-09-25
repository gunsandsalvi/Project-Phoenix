//! The population's cells in the world: each kind's settings and the processes acting on its members, bound at
//! assembly; then each day's work on them — 3b's screening, 3e's outcomes, and 10b's landing, re-keying, tolerance
//! control, ranks and renumbering.

use phx_core::StreamDef;
use phx_core::{ActsOn, CellView, Declarations, NewEvent, PopProcess, Register, StreamDecl, SubStep};
use phx_id::{Day, PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};
use phx_pop::explicit::{from_named, materialise, named, regroup, take_whole};
use phx_pop::kind::PopKindDecl;
use phx_pop::landing::{Landed, TenB, land};
use phx_pop::measure::Census;
use phx_pop::part::PartId;
use phx_pop::population::{KindSetup, Population};
use phx_pop::prims::RepPrims;
use phx_pop::prims::{HouseholdsStream, PromotionStream, ToleranceStream};
use phx_pop::promote::Ranks;
use phx_pop::promote::{demote, promote, read_ranks};
use phx_pop::rekey::rekey_flagged;
use phx_pop::rekey::{end_cell, set_key};
use phx_pop::renumber;
use phx_pop::screen::{Process, ScreenCounters, screen_due};
use phx_pop::split::{Cells, Parted, SplitSpec, split};
use phx_pop::table::CellTable;
use phx_pop::tolerance::Tolerances;
use phx_pop::tolerance::{choose_narrowing, choose_widening, estimate, sweep, top_levels};
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use crate::consts::{HASH_KEY, PROMOTION_SEQ_BITS, SHARE_WHOLE, SWEEPS_PER_DAY};
use crate::rates;
use crate::world::World;

/// A process on a kind's members as the world runs it: its kind, its agenda reason within the kind, the profile groups
/// its rate is read at, the event kind each hit records, the stream it draws from, and the system's process.
pub(crate) struct Bound {
    pub kind: usize,
    pub reason: usize,
    pub groups: Vec<usize>,
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

/// The positions a kind names, by name: every instance of each, member's and roles', in the kind's order.
fn positions_named(kind: &PopKindDecl, names: &[&str]) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for n in names {
        let found: Vec<usize> =
            kind.positions.iter().enumerate().filter(|(_, p)| p.name == *n).map(|(i, _)| i).collect();
        if found.is_empty() {
            return Err(format!("`{}` names a position `{n}` it does not hold", kind.kind));
        }
        out.extend(found);
    }
    Ok(out)
}

/// A kind's representation settings, read from the primitives its resolution names and the representation's own.
fn settings(kind: &PopKindDecl, register: &Register, rep: &RepPrims) -> Result<(Tolerances, Option<Ranks>), String> {
    let Some(res) = kind.resolution else {
        return Err(format!("population kind `{}` declares no resolution", kind.kind));
    };
    let r = res.item;
    let budget = register.count(r.cell_budget)?;
    let sample = u32::try_from(rep.gap_sample.shared(register).get()).map_err(|e| e.to_string())?;
    let share = u64::try_from(rep.narrow_share.shared(register).raw()).map_err(|e| e.to_string())?;
    let widen_order = positions_named(kind, r.widen_order)?;
    let tolerances = Tolerances { budget, sample, narrow_num: share, narrow_den: SHARE_WHOLE, widen_order };
    let ranks = match r.ranks {
        Some(k) => {
            let measure = positions_named(kind, &[k.measure])?;
            let Some(measure) = measure.first().copied() else {
                return Err(format!("`{}` ranks by no position", kind.kind));
            };
            Some(Ranks { measure, promote: register.count(k.promote)?, demote: register.count(k.demote)? })
        }
        None => None,
    };
    Ok((tolerances, ranks))
}

/// Whether a hazard acts on the members of a population kind.
fn acts_on(on: ActsOn, kind: &str) -> bool {
    match on {
        ActsOn::Role { kind: k, .. } | ActsOn::Party { kind: k } | ActsOn::Persons { kind: k } => k == kind,
        _ => false,
    }
}

/// One process bound to its kind, hazard, group, event kind and stream, or why it cannot be.
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
        return Err(format!("hazard `{name}` does not act on `{}`", process.kind()));
    }
    let Some(decl) = kinds.get(kind) else {
        return Err(format!("process `{name}` of {system} acts on no population kind"));
    };
    let mut groups = Vec::with_capacity(process.groups().len());
    for g in process.groups() {
        let Some(at) = decl.groups.iter().position(|x| x.name == *g) else {
            return Err(format!("process `{name}` reads a group `{g}` its kind does not hold"));
        };
        groups.push(at);
    }
    let components = |g: &usize| decl.groups.get(*g).map(|x| &x.components);
    if groups.is_empty() || groups.iter().any(|g| components(g) != groups.first().and_then(components)) {
        return Err(format!("process `{name}` reads no group, or groups of different components"));
    }
    let Some(event) = d.events.iter().position(|(_, e)| e.name == hazard.outcome) else {
        return Err(format!("hazard `{name}`'s outcome `{}` is no declared event kind", hazard.outcome));
    };
    let Some((_, stream)) = d.streams.iter().find(|(_, s)| s.name == hazard.stream) else {
        return Err(format!("hazard `{name}` draws from `{}`, no declared stream", hazard.stream));
    };
    let event = u16::try_from(event).map_err(|e| e.to_string())?;
    Ok(Bound { kind, reason: 0, groups, event, stream: *stream, process })
}

/// Every population kind's setup and every process bound to its kind, processes in order of kind, then hazard; each
/// is its kind's agenda reason by that order. Every refusal at once.
pub(crate) fn bind(
    d: &mut Declarations,
    register: &Register,
    rep: &RepPrims,
    kinds: Vec<PopKindDecl>,
) -> Result<(Vec<KindSetup>, Vec<Bound>), Vec<String>> {
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
    let mut setups = Vec::with_capacity(kinds.len());
    for (i, decl) in kinds.into_iter().enumerate() {
        let mut reason = 0;
        for b in bound.iter_mut().filter(|b| b.kind == i) {
            b.reason = reason;
            reason += 1;
        }
        match settings(&decl, register, rep) {
            Ok((tolerances, ranks)) => setups.push(KindSetup { decl, processes: reason, tolerances, ranks }),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() { Ok((setups, bound)) } else { Err(errors) }
}

/// Persons of a cell a process hit at 3b, per group and joint value, carried to 3e.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CellHit {
    pub process: usize,
    pub kind: usize,
    pub slot: Slot,
    pub party: PartyId,
    pub by_value: Vec<(usize, u32, u64)>,
}

/// What a day's work on the population's cells did, for the run's counters.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct CellDay {
    pub day: Day,
    pub candidates: u64,
    pub redraws: u64,
    pub hits: u64,
    pub members_hit: u64,
    pub ended: u64,
    pub parts: u64,
    pub landings: u64,
    pub new_cells: u64,
    pub rekeys: u64,
    pub sweeps: u64,
    pub promoted: u64,
    pub demoted: u64,
    pub swaps: u64,
    pub cells: u64,
    pub individuals: u64,
    pub members: u64,
    pub at_weight_one: u64,
    /// The agenda's rows gathered at 3b, and the screens of a process on them.
    pub gathered: u64,
    pub screened: u64,
    /// Persons gone from households at 3e, and the persons the cells hold at the day's end.
    pub gone: u64,
    pub persons: u64,
    /// The cells the kinds' budgets allow.
    pub budget: u64,
    /// The dispersion the day's joins erased, over every position.
    pub erased: f64,
    /// Renumbering slices, and those after which the renumbered table's identity hash was not the one before.
    pub renumbered: u64,
    pub renumber_changed: u64,
}

impl CellDay {
    /// A day on which nothing has yet been done.
    #[must_use]
    pub fn of(day: Day) -> CellDay {
        CellDay {
            day,
            candidates: 0,
            redraws: 0,
            hits: 0,
            members_hit: 0,
            ended: 0,
            parts: 0,
            landings: 0,
            new_cells: 0,
            rekeys: 0,
            sweeps: 0,
            promoted: 0,
            demoted: 0,
            swaps: 0,
            cells: 0,
            individuals: 0,
            members: 0,
            at_weight_one: 0,
            gathered: 0,
            screened: 0,
            gone: 0,
            persons: 0,
            budget: 0,
            erased: 0.0,
            renumbered: 0,
            renumber_changed: 0,
        }
    }
}

fn slot_index(s: Slot) -> usize {
    let Ok(i) = usize::try_from(s.get()) else {
        phx_num::capacity_exceeded!("index width", usize::MAX, s.get());
    };
    i
}

/// The line kinks a landing reads: a facility's limit per member on the lines whose terms give one, found once at
/// assembly or load, since a line's terms never change.
#[derive(Debug, Default)]
pub(crate) struct FacilityKinks(std::collections::BTreeMap<phx_id::LineId, i64>);

impl FacilityKinks {
    #[must_use]
    pub(crate) fn of(ledger: &phx_ledger::apply::Ledger) -> FacilityKinks {
        let mut map = std::collections::BTreeMap::new();
        for line in ledger.lines.ids() {
            if let Missing::Present(f) = ledger.terms.get(ledger.lines.terms(line)).facility {
                map.insert(line, -f.limit.amt());
            }
        }
        FacilityKinks(map)
    }
}

impl phx_pop::check::LineKinks for FacilityKinks {
    fn points(&self, line: phx_id::LineId, side: phx_ledger::algebra::Side) -> Vec<i64> {
        match (side, self.0.get(&line)) {
            (phx_ledger::algebra::Side::Asset, Some(p)) => vec![*p],
            _ => Vec::new(),
        }
    }
}

impl World {
    /// 3b: the agenda's cells for today, each screened for the processes it is booked for: a candidate thinned and its
    /// hits drawn, a redraw booked afresh, and its next booking kept with the rung it was drawn at. Each hit records its
    /// event at once and waits for 3e.
    #[clause("REP.7", "REP.12", "CHN.4")]
    pub(crate) fn cells_screen(&mut self, day: Day) {
        for l in &mut self.population.landed {
            *l = phx_pop::landing::Landed::default();
        }
        self.cell_day = CellDay::of(day);
        let today = self.population.agenda.gather(day);
        let geo = crate::world::geo_in(&self.own);
        let country_of = |r: u32| geo.map.regions.get(usize::try_from(r).ok()?).map(|x| x.country);
        let calendar = &self.calendar;
        let register = &self.register;
        let Population { kinds, agenda, .. } = &mut self.population;
        let cells = self.books.parties.cells_mut().0;
        let mut counters = ScreenCounters::default();
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
                self.cell_day.gathered += 1;
                let party = table.party(slot);
                let record = kd.keys.record(table.hot(slot).key_id);
                let key = |name: &str| {
                    kd.decl.key_attrs.iter().position(|a| a.item.name == name).map(|i| kd.decl.key.get(&record, i))
                };
                for (p, b) in self.processes.iter().enumerate().filter(|(_, b)| b.kind == k) {
                    if mask & (1 << b.reason) == 0 {
                        continue;
                    }
                    let view = |d: Day| CellView {
                        kind: kd.decl.kind,
                        party,
                        key: &key,
                        country_of: &country_of,
                        date: calendar.date(d),
                    };
                    let rate =
                        |g: usize, v: u32, d: Day| b.process.rate(register, &view(d), group_name(&kd.decl, g), v);
                    // A rate never turns within its window, so its greatest there is at the window's first or last day.
                    let envelope = |values: &[(usize, u32)], d: Day| {
                        let change = b.process.changes_after(calendar.date(d)).and_then(|c| calendar.day(c));
                        let last = change.and_then(|c| c.get().checked_sub(1)).map(Day::new);
                        let mut bar = 0.0_f64;
                        for (g, v) in values {
                            for r in [Some(rate(*g, *v, d)), last.map(|l| rate(*g, *v, l))].into_iter().flatten() {
                                if r > bar {
                                    bar = r;
                                }
                            }
                        }
                        (bar, change.map_or(Missing::Absent, Missing::Present))
                    };
                    let layout = table.profile_layout();
                    let persons = b.groups.iter().map(|g| layout.persons(*g, &record, 1)).sum();
                    let process = Process { groups: &b.groups, persons, rate: &rate, envelope: &envelope };
                    let subject = Subject::new(SubjectTag::Party, party.get());
                    let mut d = self.streams.open(&b.stream, subject, day, SubStep::S3b.ordinal());
                    let hit =
                        screen_due(table, slot, &process, (t.table, b.reason, day), agenda, &mut d, &mut counters);
                    self.cell_day.screened += 1;
                    if let Some(h) = hit {
                        if rates::sampled(party) {
                            let year = rates::year_of(calendar.date(day));
                            self.metrics.rates.realised((p, year), &kd.decl, &h.by_value);
                        }
                        let details = hit_details(&h.by_value);
                        self.events.record(NewEvent {
                            day,
                            substep: SubStep::S3b,
                            kind: b.event,
                            subjects: &[subject],
                            details: &details,
                            public: false,
                            develops_from: Missing::Absent,
                        });
                        self.cell_day.hits += 1;
                        self.cell_day.members_hit += h.by_value.iter().map(|(_, _, n)| n).sum::<u64>();
                        self.cell_hits.push(CellHit { process: p, kind: k, slot, party, by_value: h.by_value });
                    }
                }
            }
        }
        // Each event names its cell, which the directory keeps resolvable while the event does, even once it ends.
        let named: Vec<PartyId> = self.cell_hits.iter().map(|h| h.party).collect();
        let directory = self.books.parties.cells_mut().1;
        for party in named {
            directory.retain(party);
        }
        self.cell_day.candidates = counters.candidates;
        self.cell_day.redraws = counters.redraws;
        self.measure_rates(day);
    }
}

impl World {
    /// Each kind's day lists, sized to its kinds.
    fn cell_lists(&mut self) {
        let n = self.population.kinds.len();
        if self.cell_parts.len() != n {
            self.cell_parts = (0..n).map(|_| Vec::new()).collect();
        }
        if self.cell_flagged.len() != n {
            self.cell_flagged = (0..n).map(|_| Vec::new()).collect();
        }
    }

    /// 3e: each cell's hits of the day applied together to its households made explicit: drawn out of its counts,
    /// changed by each process's outcome in order, and returned in place or split out under the keys they have
    /// become, each part to land at 10b; households no one is left in end. When every household of the cell moves
    /// together the cell itself takes their profile and key, keeping its identity, and re-keys at 10b.
    #[clause("REP.26", "REP.23", "REP.8", "PTY.9")]
    pub(crate) fn cells_outcomes(&mut self, day: Day) {
        self.cell_lists();
        let hits = std::mem::take(&mut self.cell_hits);
        let mut ended = vec![0_u64; self.population.kinds.len()];
        for cell in hits.chunk_by(|a, b| (a.kind, a.slot) == (b.kind, b.slot)) {
            let Some(first) = cell.first() else { continue };
            let n = self.cell_outcomes(day, first.kind, (first.slot, first.party), cell);
            if let Some(e) = ended.get_mut(first.kind) {
                *e += n;
            }
        }
        for (k, n) in ended.into_iter().enumerate() {
            self.cell_day.ended += n;
            self.population.count(k, 0, n);
        }
    }

    /// One cell's hits of the day applied; returns the households that ended.
    fn cell_outcomes(&mut self, day: Day, kind: usize, (slot, party): (Slot, PartyId), hits: &[CellHit]) -> u64 {
        let geo = crate::world::geo_in(&self.own);
        let country_of = |r: u32| geo.map.regions.get(usize::try_from(r).ok()?).map(|x| x.country);
        let date = self.calendar.date(day);
        let phx_ledger::books::Books { ledger, parties, .. } = &mut self.books;
        let (cells, directory, space) = parties.cells_mut();
        let Some(kd) = self.population.kinds.get_mut(kind) else {
            violation!(clause = "REP.7", "a hit on a kind the world does not keep", kind = kind);
        };
        let table = Population::table_mut::<SystemBacking>(cells, kind);
        if !table.is_live(slot) || table.party(slot) != party {
            violation!(clause = "REP.7", "a hit whose cell left its slot before its outcomes", party = party.get());
        }
        let record = kd.keys.record(table.hot(slot).key_id);
        let layout = table.profile_layout().clone();
        let subject = Subject::new(SubjectTag::Part, party.get());
        let mut draws = self.streams.open(&HouseholdsStream::DECL, subject, day, SubStep::S3e.ordinal());
        let by_hit: Vec<Vec<(usize, u32, u64)>> = hits.iter().map(|h| h.by_value.clone()).collect();
        let weight = u64::from(table.weight(slot).get());
        let touched = materialise(&kd.decl, &record, weight, &table.profile(slot), &by_hit, &mut draws);
        let mut households: Vec<phx_core::Household> =
            touched.households.iter().map(|e| named(&kd.decl, &record, e)).collect();
        let key = |name: &str| {
            kd.decl.key_attrs.iter().position(|a| a.item.name == name).map(|i| kd.decl.key.get(&record, i))
        };
        let view = CellView { kind: kd.decl.kind, party, key: &key, country_of: &country_of, date };
        let mut draws_of = |b: &Bound| {
            self.streams.open(&b.stream, Subject::new(SubjectTag::Party, party.get()), day, SubStep::S3e.ordinal())
        };
        let hits_reached = (hits, touched.reached.as_slice());
        apply_outcomes(&self.processes, &self.register, &view, hits_reached, &mut households, &mut draws_of);
        self.cell_day.gone += households.iter().flat_map(|h| &h.persons).filter(|p| p.gone).map(|_| 1).sum::<u64>();
        let after: Vec<_> = households.iter().map(|h| from_named(&kd.decl, h)).collect();
        let regrouped = regroup(&kd.decl, &layout, &record, &touched.households, &after);
        if !regrouped.in_place.is_empty() {
            table.shift_profile(slot, &regrouped.in_place);
        }
        let mut ended = 0_u64;
        let groups = regrouped.parts.into_iter().map(|m| (m, false)).chain(regrouped.ended.map(|m| (m, true)));
        for (seq, (moved, end)) in (0_u32..).zip(groups) {
            let given: Vec<Vec<(u32, u64)>> = (0..layout.groups.len())
                .map(|g| moved.before.held(g).iter().map(|(v, n)| (*v, u64::from(*n))).collect())
                .collect();
            let given: Vec<(usize, &[(u32, u64)])> = given.iter().enumerate().map(|(g, v)| (g, &v[..])).collect();
            let spec = SplitSpec {
                count: moved.households,
                given: &given,
                rows: &[],
                own: &[],
                reviewed: Missing::Absent,
                rounding: Round::HalfEven,
            };
            let id = PartId { origin: party, seq };
            let mut at = Cells { ledger, table, place: kd.place, keys: &kd.keys };
            let parted = split(&mut at, slot, id, &spec, &mut draws);
            let phx_pop::population::PopKind { decl, keys, index, levels, place, .. } = &mut *kd;
            let mut ctx = TenB {
                ledger,
                table,
                place: *place,
                keys,
                directory,
                space,
                kind: decl,
                levels,
                kinks: &self.kinks,
                today: day,
            };
            match (parted, end) {
                (Parted::Part(part), true) => {
                    if !part.rows.is_empty() || !part.holdings.is_empty() {
                        violation!(clause = "POP.15", "households ended holding what only an estate can take");
                    }
                    ended += u64::from(moved.households);
                }
                (Parted::Whole, true) => {
                    if holds_anything(ctx.table, slot) {
                        violation!(clause = "POP.15", "a cell ended holding what only an estate can take");
                    }
                    end_cell(&mut ctx, index, slot);
                    ended += u64::from(moved.households);
                }
                (Parted::Part(mut part), false) => {
                    (part.profile, part.key) = (moved.after, moved.key);
                    if let Some(parts) = self.cell_parts.get_mut(kind) {
                        parts.push(*part);
                    }
                    self.cell_day.parts += 1;
                }
                (Parted::Whole, false) => {
                    take_whole(ctx.table, slot, &moved);
                    set_key(&mut ctx, slot, moved.key);
                    if let Some(f) = self.cell_flagged.get_mut(kind) {
                        f.push(slot);
                    }
                    self.cell_day.parts += 1;
                }
            }
        }
        ended
    }
}

/// A hit's event details: the persons it reached at each value, the value named with its group.
fn hit_details(by_value: &[(usize, u32, u64)]) -> Vec<(Subject, i64)> {
    by_value
        .iter()
        .map(|(g, v, n)| {
            let Ok(n) = i64::try_from(*n) else {
                phx_num::capacity_exceeded!("members hit", i64::MAX, *n);
            };
            let at = (phx_rand::float::len_u64(*g) << u32::BITS) | u64::from(*v);
            (Subject::new(SubjectTag::ProfileValue, at), n)
        })
        .collect()
}

/// The persons one hit reached, each by its household and its place there.
type Reached = Vec<(usize, usize)>;

/// Each hit's process's outcome on each household the hit reached, in the order of the hits, a person gone before a
/// later hit reaches it passed over; each process draws from its own stream for the cell at 3e.
fn apply_outcomes(
    processes: &[Bound],
    register: &Register,
    view: &CellView<'_>,
    (hits, reached): (&[CellHit], &[Reached]),
    households: &mut [phx_core::Household],
    draws_of: &mut dyn FnMut(&Bound) -> phx_rand::Draws,
) {
    for (hit, reached) in hits.iter().zip(reached) {
        let Some(bound) = processes.get(hit.process) else {
            violation!(clause = "REP.7", "a hit of a process the world does not hold");
        };
        let mut draws = draws_of(bound);
        for (h, household) in households.iter_mut().enumerate() {
            let places: Vec<usize> = reached
                .iter()
                .filter(|(x, p)| *x == h && household.persons.get(*p).is_some_and(|q| !q.gone))
                .map(|(_, p)| *p)
                .collect();
            if !places.is_empty() {
                bound.process.outcome(register, view, household, &places, &mut draws);
            }
        }
    }
}

pub(crate) fn group_name(kind: &PopKindDecl, g: usize) -> &'static str {
    let Some(x) = kind.groups.get(g) else {
        violation!(clause = "REP.32", "a process's group beyond its kind's", group = g);
    };
    x.name
}

/// Whether a cell still holds rows or holdings, which only an estate can take when it ends.
fn holds_anything(table: &CellTable<SystemBacking>, slot: Slot) -> bool {
    phx_ledger::rows::iter(table, slot).next().is_some() || !phx_ledger::part::cell_holdings(table, slot).is_empty()
}

/// What 10b's work on every kind shares: the ledger, the directory, the address space, the line kinks, the streams
/// and the day.
struct TenBShared<'a> {
    ledger: &'a mut phx_ledger::apply::Ledger,
    directory: &'a mut phx_core::Directory,
    space: &'a mut phx_store::AddressSpace,
    kinks: &'a FacilityKinks,
    streams: &'a phx_core::Streams,
    day: Day,
}

impl TenBShared<'_> {
    /// 10b's context for one kind at the levels given.
    fn ctx<'b>(
        &'b mut self,
        table: &'b mut CellTable<SystemBacking>,
        kd: &'b mut phx_pop::population::PopKind,
        levels: &'b [u8],
    ) -> (TenB<'b, SystemBacking, SystemBacking>, &'b mut phx_pop::index::Index) {
        let ctx = TenB {
            ledger: &mut *self.ledger,
            table,
            place: kd.place,
            keys: &mut kd.keys,
            directory: &mut *self.directory,
            space: &mut *self.space,
            kind: &kd.decl,
            levels,
            kinks: self.kinks,
            today: self.day,
        };
        (ctx, &mut kd.index)
    }
}

/// A kind's parts landed, its ranks read on the rank day with members promoted and individuals demoted, and its
/// flagged cells re-keyed.
fn land_rank_rekey(
    sh: &mut TenBShared<'_>,
    table: &mut CellTable<SystemBacking>,
    kd: &mut phx_pop::population::PopKind,
    work: (Vec<phx_pop::part::Part>, Vec<Slot>, bool, Subject),
    count: &mut CellDay,
) -> Landed {
    let (parts, mut flagged, rank_day, kind_subject) = work;
    let (day, streams, ordinal) = (sh.day, sh.streams, SubStep::S10b.ordinal());
    let levels = kd.levels.clone();
    let ranks = kd.ranks;
    let (mut ctx, index) = sh.ctx(table, kd, &levels);
    let mut landed = Landed::default();
    if !parts.is_empty() {
        landed = land(&mut ctx, index, parts);
    }
    if rank_day && let Some(ranks) = ranks {
        let mut draws = streams.open(&PromotionStream::DECL, kind_subject, day, ordinal);
        let read = read_ranks(&*ctx.table, ranks, &|_| false, &mut draws);
        for (slot, members) in read.promote {
            let origin = ctx.table.party(slot).get();
            let mut per: Vec<phx_rand::Draws> = (0..members)
                .map(|i| {
                    if u64::from(i) >> PROMOTION_SEQ_BITS != 0 {
                        phx_num::capacity_exceeded!("members promoted from one cell", 1 << PROMOTION_SEQ_BITS, i);
                    }
                    let id = (origin << PROMOTION_SEQ_BITS) | u64::from(i);
                    streams.open(&PromotionStream::DECL, Subject::new(SubjectTag::Part, id), day, ordinal)
                })
                .collect();
            let _ = promote(&mut ctx, index, slot, members, &mut per);
            flagged.push(slot);
            count.promoted += u64::from(members);
        }
        if !read.demote.is_empty() {
            count.demoted += phx_rand::float::len_u64(read.demote.len());
            merge(&mut landed, demote(&mut ctx, index, &read.demote));
        }
    }
    flagged.retain(|s| ctx.table.is_live(*s));
    count.rekeys += rekey_flagged(&mut ctx, index, &flagged, &mut landed);
    landed
}

/// Tolerance control for a kind: while its cells exceed its budget, the widenings of least gap swept in, at most
/// twice a day; on a light day with room, one narrowing of greatest gap swept in.
fn tolerance(
    sh: &mut TenBShared<'_>,
    table: &mut CellTable<SystemBacking>,
    kd: &mut phx_pop::population::PopKind,
    at: (bool, Subject),
    landed: &mut Landed,
    count: &mut CellDay,
) {
    let (light, kind_subject) = at;
    let mut levels = kd.levels.clone();
    let mut sweeps = 0;
    loop {
        let carried = Census::of(table).cells;
        let over = carried > kd.tolerances.budget;
        let narrowing = light && sweeps == 0;
        if !(over || narrowing) || sweeps >= SWEEPS_PER_DAY {
            break;
        }
        let mut draws = sh.streams.open(&ToleranceStream::DECL, kind_subject, sh.day, SubStep::S10b.ordinal());
        let est = estimate(table, &kd.index, &kd.decl, &levels, &[], kd.tolerances.sample, &mut draws);
        let changed = if over {
            let top = top_levels(&kd.decl);
            let chosen = choose_widening(&est, carried, &kd.tolerances, &levels, &top, &mut draws);
            for p in &chosen {
                if let Some(l) = levels.get_mut(*p) {
                    *l += 1;
                }
            }
            !chosen.is_empty()
        } else if let Missing::Present(p) = choose_narrowing(&est, carried, &kd.tolerances, &levels) {
            if let Some(l) = levels.get_mut(p) {
                *l -= 1;
            }
            true
        } else {
            false
        };
        if !changed {
            break;
        }
        let at_levels = levels.clone();
        let (mut ctx, index) = sh.ctx(table, kd, &at_levels);
        count.rekeys += sweep(&mut ctx, index, landed);
        sweeps += 1;
        count.sweeps += 1;
        if !over {
            break;
        }
    }
    kd.levels = levels;
}

/// On a light day, one chunk of a kind's table put back in order, the chunk turning with the day.
fn renumber_chunk(
    sh: &mut TenBShared<'_>,
    table: &mut CellTable<SystemBacking>,
    kd: &mut phx_pop::population::PopKind,
    agenda: &mut phx_core::Agenda,
    count: &mut CellDay,
) {
    let chunks = table.chunks();
    if chunks == 0 {
        return;
    }
    let chunk = slot_index(Slot::new(sh.day.get())) % chunks;
    let before = phx_pop::measure::identity_hash(table, &kd.keys, chunk..chunk + 1, HASH_KEY);
    let booked = kd.processes > 0;
    let levels = kd.levels.clone();
    let (mut ctx, index) = sh.ctx(table, kd, &levels);
    let swaps = renumber::plan(&ctx, chunk..chunk + 1);
    renumber::apply(&mut ctx, index, booked.then_some(agenda), &swaps);
    count.swaps += phx_rand::float::len_u64(swaps.len());
    count.renumbered += 1;
    if phx_pop::measure::identity_hash(table, &kd.keys, chunk..chunk + 1, HASH_KEY) != before {
        count.renumber_changed += 1;
    }
}

impl World {
    /// 10b: each kind's day ends for its cells — the day's parts land, the cells flagged re-key, and on the rank day
    /// ranks are read and members promoted or demoted; tolerance control widens where the cells exceed the budget and,
    /// on a light day, narrows where there is room and renumbers one chunk; every row added, removed or grown is booked
    /// afresh for tomorrow; and the day's counts are kept.
    #[clause("REP.8", "REP.28", "REP.29", "REP.13", "REP.15")]
    pub(crate) fn cells_settle(&mut self, day: Day) {
        self.cell_lists();
        let date = self.calendar.date(day);
        let light = !self.calendar.any_business(day);
        let rank_day = u64::from(date.day()) == self.rank_day;
        let phx_ledger::books::Books { ledger, parties, .. } = &mut self.books;
        let (cells, directory, space) = parties.cells_mut();
        let mut sh = TenBShared { ledger, directory, space, kinks: &self.kinks, streams: &self.streams, day };
        let Population { kinds, landed: day_landed, agenda, .. } = &mut self.population;
        let mut count = self.cell_day;
        for (k, kd) in kinds.iter_mut().enumerate() {
            let table = Population::table_mut::<SystemBacking>(cells, k);
            let parts = self.cell_parts.get_mut(k).map(std::mem::take).unwrap_or_default();
            let flagged = self.cell_flagged.get_mut(k).map(std::mem::take).unwrap_or_default();
            let kind_subject = Subject::new(SubjectTag::World, phx_rand::float::len_u64(k));
            let mut landed = land_rank_rekey(&mut sh, table, kd, (parts, flagged, rank_day, kind_subject), &mut count);
            tolerance(&mut sh, table, kd, (light, kind_subject), &mut landed, &mut count);
            // Rows are booked before any is renumbered, since a swap carries each row's bookings to its new slot.
            phx_pop::population::book_changed(kd, table, agenda, day.succ());
            if light {
                renumber_chunk(&mut sh, table, kd, agenda, &mut count);
            }
            count.persons += phx_pop::measure::persons(table, &kd.decl, &kd.keys);
            count.budget += kd.tolerances.budget;
            count.landings += landed.landings;
            count.new_cells += landed.new_cells;
            count.erased += landed.erased.iter().sum::<f64>();
            let census = Census::of(table);
            count.cells += census.cells;
            count.individuals += census.individuals;
            count.members += census.members;
            count.at_weight_one += census.at_weight_one;
            if let Some(l) = day_landed.get_mut(k) {
                merge(l, landed);
            }
        }
        self.cell_day = count;
        self.metrics.cells.push(count);
    }
}

/// One set of landings added to another.
fn merge(into: &mut Landed, from: Landed) {
    into.parts += from.parts;
    into.landings += from.landings;
    into.new_cells += from.new_cells;
    into.rows += from.rows;
    into.holder_list_changes += from.holder_list_changes;
    for (i, e) in from.erased.iter().enumerate() {
        match into.erased.get_mut(i) {
            Some(t) => *t += e,
            None => into.erased.push(*e),
        }
    }
    for (i, m) in from.moved.iter().enumerate() {
        match into.moved.get_mut(i) {
            Some(t) => *t += m,
            None => into.moved.push(*m),
        }
    }
    into.resolved.extend(from.resolved);
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::streams::Purpose;
    use phx_core::{
        ActsOn, CellView, Declarations, DrawScheme, EventKindDecl, GroupDecl, HandlerTable, HazardDecl, KinkRegistry,
        PopEntry, PopItem, PopProcess, ProfileComponent, RateFn, Register, RoleDecl, StreamDecl, System,
        declare_system,
    };
    use phx_pop::kind::PopKindDecl;
    use phx_pop::steps::StepTable;

    use super::bind_one;

    const AGE: &[ProfileComponent] = &[ProfileComponent { name: "age_band", values: 3 }];

    /// A process as a system would declare it, of a hazard and groups by name.
    struct Proc(&'static str, &'static [&'static str]);

    impl PopProcess for Proc {
        fn bind(&mut self, _: &Register) {}
        fn hazard(&self) -> &'static str {
            self.0
        }
        fn kind(&self) -> &'static str {
            "household"
        }
        fn groups(&self) -> &'static [&'static str] {
            self.1
        }
        fn rate(&self, _: &Register, _: &CellView<'_>, _: &'static str, _: u32) -> f64 {
            0.0
        }
        fn changes_after(&self, _: phx_id::Date) -> Option<phx_id::Date> {
            None
        }
        fn outcome(
            &self,
            _: &Register,
            _: &CellView<'_>,
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
                acts_on: ActsOn::Role { kind: "household", role: "person" },
                rate: RateFn { table: "DEM.life_table", axes: &["age"], changes: &[] },
                outcome: "DEM.dies",
                scheme: DrawScheme::Scheduled { envelope: phx_core::EnvelopeRule::MaxOverProfile },
                stream: "DEM.mortality",
                clause: "POP.3",
                source: "life tables",
            });
        }
        fn handlers(_: &mut HandlerTable) {}
    }

    fn kinds() -> Vec<PopKindDecl> {
        let entry = |item| PopEntry { system: "DEM", kind: "household", item };
        let entries = [
            entry(PopItem::Role(RoleDecl { name: "person", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
            entry(PopItem::ProfileGroup(GroupDecl { name: "age", role: "person", components: AGE, clause: "REP.32" })),
        ];
        let steps = |_: &'static str| StepTable::new(&Partition { exp: 0, bounds: [1].into() });
        vec![PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap()]
    }

    #[test]
    fn processes_bind_to_their_owners_hazards_and_groups() {
        let (mut d, mut h) = (Declarations::new(), HandlerTable::default());
        declare_system::<Dem>(&mut d, &mut h);
        let kinds = kinds();
        let bound = bind_one("DEM", Box::new(Proc("DEM.death", &["age"])), &d, &kinds).unwrap();
        assert_eq!((bound.kind, bound.groups, bound.event, bound.stream.name), (0, vec![0], 0, "DEM.mortality"));
        let refused = |system, p: Proc| bind_one(system, Box::new(p), &d, &kinds).map(|_| ()).unwrap_err();
        assert!(refused("DEM", Proc("DEM.birth", &["age"])).contains("no declared hazard"));
        assert!(refused("HH", Proc("DEM.death", &["age"])).contains("its hazard is DEM's"));
        assert!(refused("DEM", Proc("DEM.death", &["health"])).contains("does not hold"));
        assert!(refused("DEM", Proc("DEM.death", &[])).contains("reads no group"));
    }
}
