//! Declared sweeps: the few passes a clause needs over a whole store — the audit's rolling slice, a bank's depositors
//! on its failure, the day's `pending` — each declared with its store, its slot of the day and its cycle or reason,
//! run as a chunk plan over a saved cursor's slice, and counted in the sweep ledger beside every other traversal, so
//! a pass over every row that no declaration names shows as rows nothing explains.

use std::ops::Range;

use phx_num::violation;
use phx_store::StoreStats;

use crate::convert::{to_u32, to_u64, to_usize};
use crate::counters::add;
use crate::partition::for_plan_runs;
use crate::plan::ChunkPlan;
use crate::pool::Pool;
use crate::traverse::for_plan;

/// When a sweep runs: a share of its store each day over a cycle of days, or over the whole store on the days its
/// reason occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Rolling { cycle: u32 },
    OnReason,
}

/// A declared sweep: its name, the store it walks and the slot of the day it runs in, as the ledger's indexes, and
/// when it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SweepDecl {
    pub name: &'static str,
    pub store: usize,
    pub slot: usize,
    pub when: When,
}

/// A rolling sweep's place in its cycle: the slot its next slice reads from and the day its cycle began. Saved, so a restored
/// world continues the cycle at the same slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Cursor {
    from_slot: u32,
    cycle_start_day: u32,
}

impl Cursor {
    /// A cursor whose first cycle begins on `day` at the store's first slot.
    #[must_use]
    pub fn starting(day: u32) -> Cursor {
        Cursor { from_slot: 0, cycle_start_day: day }
    }

    /// The slot the sweep's next slice reads from.
    #[must_use]
    pub fn from_slot(&self) -> u32 {
        self.from_slot
    }

    /// The day the cycle began.
    #[must_use]
    pub fn cycle_start_day(&self) -> u32 {
        self.cycle_start_day
    }

    /// Today's slice of a store whose slots reach `high_water`: the rows left in the cycle shared over its days left,
    /// so a cycle of `cycle` days reads every slot below its last day's high water once, and a store that grows
    /// lengthens only the slices. The day after a cycle's last begins the next at the first slot; rows a store gained
    /// since, and any a day the sweep did not run left unread, are read in that cycle.
    pub fn slice(&mut self, cycle: u32, high_water: u32, today: u32) -> Range<u32> {
        if cycle == 0 {
            violation!(clause = "TIME.6", "a rolling sweep of no days");
        }
        let Some(mut elapsed) = today.checked_sub(self.cycle_start_day) else {
            violation!(clause = "TIME.6", "a sweep run before its cycle began", day = today);
        };
        if elapsed >= cycle {
            (self.from_slot, self.cycle_start_day, elapsed) = (0, today, 0);
        }
        let Some(left) = high_water.checked_sub(self.from_slot) else {
            violation!(clause = "TIME.6", "a store's high water below its sweep's cursor", high_water = high_water);
        };
        let from = self.from_slot;
        self.from_slot += left.div_ceil(cycle - elapsed);
        from..self.from_slot
    }
}

/// How a table is walked: the slots below its high water, its rows a table chunk, and the cost a row declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Walk {
    pub high_water: u32,
    pub rows_per_chunk: u32,
    pub cost_per_row: u64,
}

/// How a traversal came to a row: the agenda's parties due, an index's hits, an apply's targets, a declared sweep,
/// or a pass over a store that none of these explains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visit {
    Agenda,
    Index,
    Apply,
    Sweep,
    Pass,
}

/// A store's rows visited in a slot today, by how the traversals came to them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Visits {
    agenda: u64,
    index: u64,
    apply: u64,
    sweep: u64,
    pass: u64,
}

impl Visits {
    fn of(&self, visit: Visit) -> u64 {
        match visit {
            Visit::Agenda => self.agenda,
            Visit::Index => self.index,
            Visit::Apply => self.apply,
            Visit::Sweep => self.sweep,
            Visit::Pass => self.pass,
        }
    }

    fn of_mut(&mut self, visit: Visit) -> &mut u64 {
        match visit {
            Visit::Agenda => &mut self.agenda,
            Visit::Index => &mut self.index,
            Visit::Apply => &mut self.apply,
            Visit::Sweep => &mut self.sweep,
            Visit::Pass => &mut self.pass,
        }
    }
}

/// A day's rows visited: those following its events (agenda, index and apply), those of its declared sweeps, and
/// those of passes no declaration names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DayVisits {
    pub following: u64,
    pub declared: u64,
    pub undeclared: u64,
}

/// The rows every traversal visited today, by store, slot and how it came to them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SweepLedger {
    slots: usize,
    rows: Vec<Visits>,
}

impl SweepLedger {
    /// A ledger of `stores` stores over a day of `slots` slots, made as the world is assembled.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(stores: usize, slots: usize) -> SweepLedger {
        SweepLedger { slots, rows: vec![Visits::default(); stores * slots] }
    }

    fn cell(&mut self, (store, slot): (usize, usize)) -> &mut Visits {
        if slot >= self.slots {
            violation!(clause = "TIME.6", "a visit in a slot the ledger does not hold", slot = slot);
        }
        let at = store * self.slots + slot;
        let Some(cell) = self.rows.get_mut(at) else {
            violation!(clause = "TIME.6", "a visit to a store the ledger does not hold", store = store);
        };
        cell
    }

    /// `rows` rows of `store` visited in `slot`, come to by `visit`.
    pub fn count(&mut self, (store, slot): (usize, usize), visit: Visit, rows: u64) {
        add(self.cell((store, slot)).of_mut(visit), rows);
    }

    /// The rows of `store` visited in `slot` today, by how they were come to.
    #[must_use]
    pub fn at(&self, (store, slot): (usize, usize), visit: Visit) -> u64 {
        let cell = (slot < self.slots).then(|| self.rows.get(store * self.slots + slot)).flatten();
        match cell {
            Some(c) => c.of(visit),
            None => violation!(clause = "TIME.6", "a read of a store or slot the ledger does not hold", slot = slot),
        }
    }

    /// Today's rows visited over every store and slot.
    #[must_use]
    pub fn day(&self) -> DayVisits {
        let mut day = DayVisits::default();
        for v in &self.rows {
            add(&mut day.following, v.agenda);
            add(&mut day.following, v.index);
            add(&mut day.following, v.apply);
            add(&mut day.declared, v.sweep);
            add(&mut day.undeclared, v.pass);
        }
        day
    }

    /// Today's rows visited, and the ledger emptied for the next day.
    pub fn close_day(&mut self) -> DayVisits {
        let day = self.day();
        self.rows.fill(Visits::default());
        day
    }
}

impl StoreStats for SweepLedger {
    fn rows_live(&self) -> u64 {
        to_u64(self.rows.len())
    }

    fn rows_ever(&self) -> u64 {
        to_u64(self.rows.len())
    }

    fn bytes(&self) -> u64 {
        to_u64(self.rows.capacity() * size_of::<Visits>())
    }
}

/// A sweep's kept buffers: its slice's plan and a result a chunk, so a day sweeps without allocating once its
/// largest slice has sized them.
#[derive(Debug, Default)]
pub struct SweepRun<T> {
    plan: ChunkPlan,
    out: Vec<T>,
}

impl<T: Send + Default> SweepRun<T> {
    /// A rolling sweep's slice today, run as a chunk plan: `f` is handed each chunk's slots and its result, and the
    /// results come back in chunk order, the same for any workers. Its rows are counted in the ledger as the sweep's.
    pub fn rolling(
        &mut self,
        pool: Option<&Pool>,
        (decl, cursor, walk): (&SweepDecl, &mut Cursor, Walk),
        today: u32,
        ledger: &mut SweepLedger,
        f: impl Fn(Range<u32>, &mut T) + Sync,
    ) -> &[T] {
        let When::Rolling { cycle } = decl.when else {
            violation!(
                clause = "TIME.6",
                "a sweep run on a cycle it does not declare",
                store = decl.store,
                slot = decl.slot
            );
        };
        let span = cursor.slice(cycle, walk.high_water, today);
        self.run(pool, (decl, span, walk), ledger, f)
    }

    /// A rolling sweep that writes its store: as `rolling`, each chunk handed besides its result its own run of
    /// `column`, the store's rows from its first slot, so chunks write disjoint rows and need no lock.
    pub fn rolling_mut<C: Send>(
        &mut self,
        pool: Option<&Pool>,
        (decl, cursor, walk): (&SweepDecl, &mut Cursor, Walk),
        today: u32,
        (ledger, column): (&mut SweepLedger, &mut [C]),
        f: impl Fn(Range<u32>, &mut [C], &mut T) + Sync,
    ) -> &[T] {
        let When::Rolling { cycle } = decl.when else {
            violation!(
                clause = "TIME.6",
                "a sweep run on a cycle it does not declare",
                store = decl.store,
                slot = decl.slot
            );
        };
        let span = cursor.slice(cycle, walk.high_water, today);
        self.plan.cut_span(span.start..span.end, walk.cost_per_row, walk.rows_per_chunk);
        self.out.clear();
        self.out.resize_with(self.plan.len(), T::default);
        let Some(rows) = column.get_mut(to_usize(span.start)..to_usize(span.end)) else {
            violation!(clause = "TIME.6", "a sweep's slice past its column", high_water = walk.high_water);
        };
        let from = to_usize(span.start);
        for_plan_runs(pool, &self.plan, (rows, &mut self.out), |places, run, out| {
            f(to_u32(from + places.start)..to_u32(from + places.end), run, out);
        });
        ledger.count((decl.store, decl.slot), Visit::Sweep, u64::from(span.end - span.start));
        &self.out
    }

    /// A reason sweep over the whole store, run only on a day its reason occurred; on any other day it reads nothing.
    pub fn on_reason(
        &mut self,
        pool: Option<&Pool>,
        (decl, walk): (&SweepDecl, Walk),
        occurred: bool,
        ledger: &mut SweepLedger,
        f: impl Fn(Range<u32>, &mut T) + Sync,
    ) -> &[T] {
        if decl.when != When::OnReason {
            violation!(
                clause = "TIME.6",
                "a sweep run on a reason it does not declare",
                store = decl.store,
                slot = decl.slot
            );
        }
        let span = if occurred { 0..walk.high_water } else { 0..0 };
        self.run(pool, (decl, span, walk), ledger, f)
    }

    fn run(
        &mut self,
        pool: Option<&Pool>,
        (decl, span, walk): (&SweepDecl, Range<u32>, Walk),
        ledger: &mut SweepLedger,
        f: impl Fn(Range<u32>, &mut T) + Sync,
    ) -> &[T] {
        self.plan.cut_span(span.start..span.end, walk.cost_per_row, walk.rows_per_chunk);
        self.out.clear();
        self.out.resize_with(self.plan.len(), T::default);
        let from = to_usize(span.start);
        for_plan(pool, &self.plan, &mut self.out, |places, out| {
            f(to_u32(from + places.start)..to_u32(from + places.end), out);
        });
        ledger.count((decl.store, decl.slot), Visit::Sweep, u64::from(span.end - span.start));
        &self.out
    }
}

#[cfg(test)]
#[path = "sweep_tests.rs"]
mod tests;
