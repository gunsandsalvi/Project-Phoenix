//! The decisions taken on the rows of kinds as they come due: each bound at assembly to its handler, its kind's table
//! and its stream; every row booked on the visits' agenda once the opening is done; the rows due gathered on the day's
//! first decision sub-step, the handler run on them, and each booked again after it.

use phx_core::{
    AgendaTableSpec, BusinessDayConvention, Cadence, ColumnTrace, CtxParts, DecisionSchedule, Declarations, FactStore,
    HandlerTable, Intents, KindTableRef, Period, Phase, ReadTrace, Register, StreamDecl, SubStep, VisitDecl, next_due,
};
use phx_id::{CountryId, Day, PartyId, Slot, TableId};
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag, below_u64};
use phx_store::SystemBacking;

use crate::consts::{AGENT_ROWS, BILLION, KIND_ROWS};
use crate::world::World;

/// What a day's visits did: the rows each handler visited, and the facts they moved to a new value, each by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisitDay {
    pub day: Day,
    pub visits: Vec<(&'static str, u64)>,
    pub moved: Vec<(&'static str, u64)>,
}

impl VisitDay {
    /// A day before any visit.
    #[must_use]
    pub fn of(day: Day) -> VisitDay {
        VisitDay { day, visits: Vec::new(), moved: Vec::new() }
    }

    /// The rows a handler visited on the day.
    #[must_use]
    pub fn visits_of(&self, handler: &str) -> u64 {
        self.visits.iter().filter(|(h, _)| *h == handler).map(|(_, n)| n).sum()
    }

    /// The times a fact was moved to a new value on the day.
    #[must_use]
    pub fn moved_of(&self, fact: &str) -> u64 {
        self.moved.iter().filter(|(f, _)| *f == fact).map(|(_, n)| n).sum()
    }
}

/// A visit as the world runs it: its declaration and the schedule its period's count gives it, its kind's table,
/// whether that table is a kind table of individuals, its reason among the table's visits, and its stream.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bound {
    pub decl: VisitDecl,
    pub schedule: Option<DecisionSchedule>,
    pub table: TableId,
    pub individuals: bool,
    pub reason: usize,
    pub stream: StreamDecl,
}

/// Every visit bound to its handler, table and stream, each its table's agenda reason in the order declared; every
/// refusal at once.
pub(crate) fn bind(d: &Declarations, h: &HandlerTable, register: &Register) -> Result<Vec<Bound>, Vec<String>> {
    let individuals: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Individuals).map(|(_, k)| k.name).collect();
    let agents: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Cells).map(|(_, k)| k.name).collect();
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, v) in &d.visits {
        let place = individuals
            .iter()
            .position(|k| *k == v.kind)
            .map(|i| (i, true))
            .or_else(|| agents.iter().position(|k| *k == v.kind).map(|i| (individuals.len() + i, false)));
        let Some((place, is_individual)) = place else {
            errors.push(format!("{system} visits `{}`, a kind the world does not keep", v.kind));
            continue;
        };
        match h.entries.iter().find(|e| e.name == v.handler) {
            None => errors.push(format!("visit of `{}` runs `{}`, no registered handler", v.kind, v.handler)),
            Some(e) if e.system != *system => {
                errors.push(format!("{system} visits with `{}`, {}'s handler", v.handler, e.system));
            }
            Some(e) if e.table != v.kind => {
                errors.push(format!("`{}` runs on `{}`, not on the visited `{}`", v.handler, e.table, v.kind));
            }
            Some(e) if !matches!(e.substep, SubStep::S5b | SubStep::S5c) => {
                errors.push(format!("`{}` visits outside the decisions' sub-steps", v.handler));
            }
            Some(_) => {}
        }
        if !v.wakes.is_empty() {
            errors.push(format!("`{}` answers wakes, which surprises bring once firms sell (S1.05)", v.handler));
        }
        let mut schedule = None;
        if let Cadence::Schedule { days, runs_on } = v.cadence {
            match register.count(days).map(u16::try_from) {
                Ok(Ok(n)) => match Period::days(n) {
                    Some(period) => {
                        schedule =
                            Some(DecisionSchedule { period, convention: BusinessDayConvention::Following, runs_on });
                    }
                    None => errors.push(format!("`{}`'s schedule of `{days}` is no period", v.handler)),
                },
                Ok(Err(_)) => errors.push(format!("`{}`'s schedule of `{days}` is beyond a period", v.handler)),
                Err(e) => errors.push(format!("`{}`'s schedule: {e}", v.handler)),
            }
        }
        if let Cadence::Attention { position } = v.cadence {
            let held = if is_individual {
                d.facets.iter().any(|(_, f)| f.kind == v.kind && f.fact == position)
            } else {
                d.pop
                    .iter()
                    .any(|e| e.kind == v.kind && matches!(e.item, phx_core::PopItem::Position(p) if p.name == position))
            };
            if !held {
                errors.push(format!("`{}`'s reviews read `{position}`, which `{}` does not hold", v.handler, v.kind));
            }
        }
        let Some((_, stream)) = d.streams.iter().find(|(_, s)| s.name == v.stream) else {
            errors.push(format!("visit `{}` draws from `{}`, no declared stream", v.handler, v.stream));
            continue;
        };
        let Ok(place) = u16::try_from(place) else {
            errors.push(format!("`{}`'s table beyond a place", v.kind));
            continue;
        };
        let table = TableId::new(place);
        let reason = out.iter().filter(|b: &&Bound| b.table == table).count();
        out.push(Bound { decl: *v, schedule, table, individuals: is_individual, reason, stream: *stream });
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The visits' agenda tables: one for each visited kind, in the order of their tables, each reason one of its visits.
pub(crate) fn specs(bound: &[Bound]) -> Vec<AgendaTableSpec> {
    let mut out: Vec<AgendaTableSpec> = Vec::new();
    for b in bound {
        let max_rows = if b.individuals { KIND_ROWS } else { AGENT_ROWS };
        match out.iter_mut().find(|s| s.table == b.table) {
            Some(s) => s.reasons += 1,
            None => out.push(AgendaTableSpec { table: b.table, max_rows, reasons: 1 }),
        }
    }
    out.sort_by_key(|s| s.table);
    out
}

/// A schedule's shortest instance in days, within which each row's phase lies.
fn shortest_days(period: phx_core::Period) -> u64 {
    if period.month_count() > 0 {
        u64::from(period.month_count()) * u64::from(phx_core::consts::SHORTEST_MONTH_DAYS)
    } else {
        u64::from(period.day_count())
    }
}

/// A population kind's index from its table's place among the books' holders, the first agent table's at `first`.
fn agent_kind(table: TableId, first: u16) -> usize {
    let Some(k) = table.get().checked_sub(first) else {
        violation!(clause = "REP.1", "a visited agent table before the first", table = table.get());
    };
    usize::from(k)
}

/// The runs of consecutive slots among sorted slots, as the row ranges a handler's body is given.
fn runs(slots: &[Slot]) -> Vec<core::ops::Range<u32>> {
    let mut out: Vec<core::ops::Range<u32>> = Vec::new();
    for s in slots {
        let at = s.get();
        match out.last_mut() {
            Some(r) if r.end == at => r.end = at + 1,
            _ => out.push(at..at + 1),
        }
    }
    out
}

impl World {
    /// A row's party and the country it lies in.
    fn visited_row(&self, b: &Bound, slot: Slot) -> Option<(PartyId, CountryId)> {
        let place = b.table.get();
        if b.individuals {
            let t = self.books.parties.table(place);
            if !t.is_live(slot) {
                return None;
            }
            let party = t.party(slot);
            let Missing::Present(country) = self.geo().country_of(t.site(slot)) else {
                violation!(clause = "PTY.5", "a visited party sited on no country's land", party = party.get());
            };
            return Some((party, country));
        }
        let first = self.books.parties.first_cell_place();
        let k = agent_kind(b.table, first);
        let t = Population::table::<SystemBacking>(self.books.parties.cells(), k);
        if !t.is_live(slot) {
            return None;
        }
        let party = t.party(slot);
        let kd = self.population.kinds.get(k)?;
        let Missing::Present(at) = kd.decl.sited_by else {
            violation!(clause = "REP.41", "a visited agent of a kind that is not sited", party = party.get());
        };
        let region = t.attr(slot, at);
        let map = &self.geo().map;
        let Some(country) = usize::try_from(region).ok().and_then(|r| map.regions.get(r)).map(|r| r.country) else {
            violation!(clause = "REP.41", "a visited agent in no region of the map", party = party.get());
        };
        Some((party, country))
    }

    /// A row's review chance a day, in billionths, as its position holds it.
    fn attention(&self, b: &Bound, position: &str, slot: Slot) -> Missing<i64> {
        let place = b.table.get();
        if b.individuals {
            let t = self.books.parties.table(place);
            return t.facet_named(position).map_or(Missing::Absent, |c| t.fact(slot, c));
        }
        let first = self.books.parties.first_cell_place();
        let k = agent_kind(b.table, first);
        let t = Population::table::<SystemBacking>(self.books.parties.cells(), k);
        t.position(position).map_or(Missing::Absent, |c| t.fact(slot, c))
    }

    /// Books a row's next visit after `after`: its schedule's next instance at its phase, drawn the first time and
    /// kept beside its booking; or its next review, drawn at the chance its attention gives it, or none while it holds
    /// none.
    fn book_visit(&mut self, i: usize, slot: Slot, after: Day, ordinal: u8, first: bool) {
        let Some(b) = self.visits.get(i).copied() else { return };
        let Some((party, country)) = self.visited_row(&b, slot) else { return };
        let subject = Subject::new(SubjectTag::Party, party.get());
        let mut d = self.streams.open(&b.stream, subject, after, ordinal);
        let agenda = &mut self.population.visits;
        agenda.grow(b.table, slot.get() + 1);
        match b.decl.cadence {
            Cadence::Schedule { .. } => {
                let Some(s) = b.schedule else {
                    violation!(clause = "TIME.5", "a scheduled visit bound without its schedule");
                };
                let offset = if first {
                    below_u64(&mut d, shortest_days(s.period))
                } else {
                    u64::from(agenda.with(b.table, slot, b.reason))
                };
                let (Ok(days), Ok(with)) = (u16::try_from(offset), u32::try_from(offset)) else {
                    violation!(clause = "TIME.5", "a phase beyond a period's days", offset = offset);
                };
                let Some(phase) = Phase::within(s.period, days) else {
                    violation!(clause = "TIME.5", "a phase beyond its period", offset = offset);
                };
                let due = next_due(&self.calendar, country, s, phase, after);
                self.population.visits.set_next_with(b.table, slot, b.reason, due, with);
            }
            Cadence::Attention { position } => {
                let chance = match self.attention(&b, position, slot) {
                    Missing::Present(g) if g > 0 => phx_rand::float::from_i64(g) / BILLION,
                    _ => {
                        self.population.visits.clear(b.table, slot, b.reason);
                        return;
                    }
                };
                match phx_rand::geometric(&mut d, chance) {
                    Missing::Present(wait) => {
                        let Ok(wait) = u32::try_from(wait) else {
                            self.population.visits.clear(b.table, slot, b.reason);
                            return;
                        };
                        match after.get().checked_add(wait).and_then(|d| d.checked_add(1)) {
                            Some(due) => self.population.visits.set_next(b.table, slot, b.reason, Day::new(due)),
                            None => self.population.visits.clear(b.table, slot, b.reason),
                        }
                    }
                    Missing::Absent => self.population.visits.clear(b.table, slot, b.reason),
                }
            }
        }
    }

    /// Every row of every visited kind booked its first visit after `day`, as the opening leaves them.
    pub(crate) fn visits_book_all(&mut self, day: Day) {
        for i in 0..self.visits.len() {
            let Some(b) = self.visits.get(i).copied() else { continue };
            let slots: Vec<Slot> = if b.individuals {
                self.books.parties.table(b.table.get()).slots().collect()
            } else {
                let first = self.books.parties.first_cell_place();
                let k = agent_kind(b.table, first);
                Population::table::<SystemBacking>(self.books.parties.cells(), k).slots().collect()
            };
            for slot in slots {
                self.book_visit(i, slot, day, SubStep::S10b.ordinal(), true);
            }
        }
    }

    /// The rows due today, gathered once on the day's first decision sub-step.
    fn visits_gather(&mut self, day: Day) {
        if self.visit_day == day {
            return;
        }
        self.visit_day = day;
        self.visit_due.clear();
        if self.visits.is_empty() {
            return;
        }
        let today = self.population.visits.gather(day);
        for t in &today.per_table {
            for (slot, mask) in t.slots.iter().zip(&t.reasons) {
                for (i, b) in self.visits.iter().enumerate().filter(|(_, b)| b.table == t.table) {
                    if mask & (1 << b.reason) != 0 {
                        self.visit_due.push((i, *slot));
                    }
                }
            }
        }
        self.visit_due.sort_unstable_by_key(|(i, s)| (*i, *s));
    }

    /// Runs each visit's handler at `step` on its rows due today, in runs of consecutive slots, reading and writing
    /// their table's facts, and books each row again after it. Returns the rows visited.
    pub(crate) fn visits_run(&mut self, day: Day, step: SubStep, pending: &mut Vec<crate::goods::Gathered>) -> u64 {
        if self.visits.is_empty() {
            return 0;
        }
        self.visits_gather(day);
        let date = self.calendar.date(day);
        let mut visited = 0_u64;
        for i in 0..self.visits.len() {
            let Some(b) = self.visits.get(i).copied() else { continue };
            let Some((id, h)) = self.graph.at(step).find(|(_, h)| h.name == b.decl.handler).map(|(id, h)| (*id, *h))
            else {
                continue;
            };
            let Missing::Present(run) = h.run else {
                violation!(clause = "TIME.6", "a visit's handler with no body", handler = id.0);
            };
            let slots: Vec<Slot> = self
                .visit_due
                .iter()
                .filter(|(v, s)| *v == i && self.visited_row(&b, *s).is_some())
                .map(|(_, s)| *s)
                .collect();
            if slots.is_empty() {
                continue;
            }
            visited += phx_rand::float::len_u64(slots.len());
            match self.visit_today.visits.iter_mut().find(|(n, _)| *n == h.name) {
                Some((_, n)) => *n += phx_rand::float::len_u64(slots.len()),
                None => self.visit_today.visits.push((h.name, phx_rand::float::len_u64(slots.len()))),
            }
            // Declared reads are checked only on a run that traces them, as the day's other handlers are.
            let trace = self.read_trace.then_some(ColumnTrace {
                substep: step.ordinal(),
                handler: id.0,
                reads: h.reads,
                writes: h.writes,
            });
            let rows = crate::goods::Rows { place: b.table.get(), individuals: b.individuals };
            let views: Vec<crate::goods::RunGoods> =
                runs(&slots).into_iter().map(|r| self.run_goods(rows, r)).collect();
            let World { books, own, streams, register, bindings, rules, queue, .. } = self;
            let Some((_, own)) = own.iter().find(|(system, _)| *system == h.system) else {
                violation!(clause = "TIME.6", "a handler whose system compiled no state", handler = id.0);
            };
            let first = books.parties.first_cell_place();
            for (range, goods) in runs(&slots).into_iter().zip(&views) {
                let mut intents = Intents::default();
                let store: &mut dyn FactStore = if b.individuals {
                    let t = books.parties.table_mut(b.table.get());
                    t.trace(trace);
                    t
                } else {
                    let k = agent_kind(b.table, first);
                    let t = Population::table_mut::<SystemBacking>(books.parties.cells_mut().0, k);
                    t.trace(trace);
                    t
                };
                run(
                    CtxParts {
                        day,
                        date,
                        streams,
                        register,
                        own: own.as_ref(),
                        facts: store,
                        goods,
                        intents: &mut intents,
                        bindings,
                        rules,
                        queue,
                        opens: None,
                    },
                    range,
                );
                pending.push(crate::goods::Gathered { step, rows: Missing::Present(rows), intents });
            }
            self.visits_taken(&b);
            self.labour_after(b.decl.handler, rows, &slots);
            self.wear_after(i, &slots, (day, step));
            self.spoil_after(i, &slots, (day, step));
            self.visits_rebook(i, h.writes, &slots, (day, step));
        }
        visited
    }

    /// What a visit's handler did to its table, taken once it has run: its undeclared reads and the facts it moved.
    fn visits_taken(&mut self, b: &Bound) {
        let first = self.books.parties.first_cell_place();
        let (undeclared, moved) = if b.individuals {
            let t = self.books.parties.table_mut(b.table.get());
            t.trace(None);
            (t.take_undeclared(), t.take_moved())
        } else {
            let k = agent_kind(b.table, first);
            let t = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, k);
            t.trace(None);
            (t.take_undeclared(), t.take_moved())
        };
        self.visit_reads.undeclared_reads += undeclared;
        for (fact, n) in moved {
            match self.visit_today.moved.iter_mut().find(|(f, _)| *f == fact) {
                Some((_, m)) => *m += n,
                None => self.visit_today.moved.push((fact, n)),
            }
        }
    }

    /// Each row visited booked again; and the visits whose reviews run at an attention the handler writes drawn
    /// afresh for its rows, so no review is drawn at an attention the row no longer holds.
    fn visits_rebook(&mut self, i: usize, writes: &[&str], slots: &[Slot], (day, step): (Day, SubStep)) {
        let Some(table) = self.visits.get(i).map(|b| b.table) else { return };
        let redrawn: Vec<usize> = self
            .visits
            .iter()
            .enumerate()
            .filter(|(j, v)| {
                *j != i
                    && v.table == table
                    && matches!(v.decl.cadence, Cadence::Attention { position } if writes.contains(&position))
            })
            .map(|(j, _)| j)
            .collect();
        for slot in slots {
            self.book_visit(i, *slot, day, step.ordinal(), false);
            for j in &redrawn {
                self.book_visit(*j, *slot, day, step.ordinal(), false);
            }
        }
    }

    /// A row that ends leaves the visits' agenda, if its table is visited.
    pub(crate) fn release_visits(&mut self, place: u16, slot: Slot) {
        let table = TableId::new(place);
        if self.visits.iter().any(|b| b.table == table) {
            self.population.visits.grow(table, slot.get() + 1);
            self.population.visits.release(table, slot);
        }
    }

    /// The reads the visits' handlers made that they do not declare, since the last close.
    pub(crate) fn take_visit_reads(&mut self) -> ReadTrace {
        std::mem::take(&mut self.visit_reads)
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;

    use super::runs;

    #[test]
    fn slots_run_in_ranges() {
        let slots: Vec<Slot> = [1, 2, 3, 7, 9, 10].into_iter().map(Slot::new).collect();
        assert_eq!(runs(&slots), vec![1..4, 7..8, 9..11]);
        assert!(runs(&[]).is_empty());
    }
}
