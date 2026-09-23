use phx_id::{Day, Slot, TableId};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, BlockBag, BlockPool, Region, SystemBacking};

use crate::consts::{MAX_REASONS, WHEEL_BITS, WHEEL_BUCKETS};

/// A reason with no next day, and a row with no entry.
const NEVER: u32 = u32::MAX;
const LOW: u32 = (1 << WHEEL_BITS) - 1;

fn index(n: u32) -> usize {
    let Ok(i) = usize::try_from(n) else {
        capacity_exceeded!("index width", usize::MAX, n);
    };
    i
}

fn count(n: usize) -> u64 {
    let Ok(c) = u64::try_from(n) else {
        capacity_exceeded!("agenda entries", u64::MAX, n);
    };
    c
}

/// Where an entry for a day lies, seen from `today`: a day bucket of the current block, or the bucket of its block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bucket {
    Day(usize),
    Block(usize),
}

fn bucket(today: u32, day: u32) -> Bucket {
    if day >> WHEEL_BITS == today >> WHEEL_BITS {
        Bucket::Day(index(day & LOW))
    } else {
        Bucket::Block(index((day >> WHEEL_BITS) & LOW))
    }
}

/// A table whose rows the agenda books, with the number of reasons they may be booked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgendaTableSpec {
    pub table: TableId,
    pub max_rows: u32,
    pub reasons: usize,
}

/// One table's part: each row's next day per reason, its booked day, and the two levels of the wheel.
#[derive(Debug)]
struct TableAgenda<B: Backing> {
    spec: AgendaTableSpec,
    rows: usize,
    next: Region<u32, B>,
    booked: Region<u32, B>,
    days: Vec<BlockBag>,
    blocks: Vec<BlockBag>,
    live: u64,
    stale: u64,
}

impl<B: Backing> TableAgenda<B> {
    fn row(&self, slot: Slot) -> usize {
        let at = index(slot.get());
        if at >= self.rows {
            violation!(clause = "TIME.5", "a row the agenda has not grown to", slot = slot.get(), rows = self.rows);
        }
        at
    }

    fn booked(&self, slot: Slot) -> u32 {
        let at = self.row(slot);
        let Some(&day) = self.booked.slice(self.rows).get(at) else {
            violation!(clause = "TIME.5", "a row the agenda has not grown to", slot = slot.get(), rows = self.rows);
        };
        day
    }

    fn set_booked(&mut self, slot: Slot, day: u32) {
        let at = self.row(slot);
        let Some(cell) = self.booked.slice_mut(self.rows).get_mut(at) else {
            violation!(clause = "TIME.5", "a row the agenda has not grown to", slot = slot.get(), rows = self.rows);
        };
        *cell = day;
    }

    fn reasons(&self, slot: Slot) -> &[u32] {
        let (at, width) = (self.row(slot) * self.spec.reasons, self.spec.reasons);
        let Some(cells) = self.next.slice(self.rows * width).get(at..at + width) else {
            violation!(clause = "TIME.5", "a row the agenda has not grown to", slot = slot.get(), rows = self.rows);
        };
        cells
    }

    fn reasons_mut(&mut self, slot: Slot) -> &mut [u32] {
        let (at, width, rows) = (self.row(slot) * self.spec.reasons, self.spec.reasons, self.rows);
        let Some(cells) = self.next.slice_mut(rows * width).get_mut(at..at + width) else {
            violation!(clause = "TIME.5", "a row the agenda has not grown to", slot = slot.get(), rows = rows);
        };
        cells
    }

    fn list(&mut self, b: Bucket) -> &mut BlockBag {
        let (lists, i) = match b {
            Bucket::Day(i) => (&mut self.days, i),
            Bucket::Block(i) => (&mut self.blocks, i),
        };
        let Some(list) = lists.get_mut(i) else {
            violation!(clause = "TIME.5", "a wheel bucket past the wheel", bucket = i);
        };
        list
    }

    /// Whether an entry in bucket `b` is its row's live one.
    fn is_live(&self, today: u32, b: Bucket, slot: u32) -> bool {
        let booked = self.booked(Slot::new(slot));
        booked != NEVER && bucket(today, booked) == b
    }
}

/// Books a row's one entry at `day`, its entry at `old` becoming stale unless it lies in the same bucket.
fn book<B: Backing>(t: &mut TableAgenda<B>, pool: &mut BlockPool<B>, today: u32, slot: Slot, day: u32, old: u32) {
    let at = bucket(today, day);
    t.set_booked(slot, day);
    if old == NEVER {
        t.live += 1;
        pool.push(t.list(at), slot.get());
    } else if bucket(today, old) != at {
        t.stale += 1;
        pool.push(t.list(at), slot.get());
        compact_if_due(t, pool, today);
    }
}

/// At a block's first day, moves the entries of its block bucket booked within it into the day buckets, keeps those
/// booked a whole turn of the wheel or more ahead, and drops the stale.
fn cascade<B: Backing>(t: &mut TableAgenda<B>, pool: &mut BlockPool<B>, first_day: u32, scratch: &mut Vec<u32>) {
    let here = Bucket::Block(index((first_day >> WHEEL_BITS) & LOW));
    scratch.clear();
    pool.drain(t.list(here), scratch);
    for &slot in scratch.iter() {
        let booked = t.booked(Slot::new(slot));
        let to = bucket(first_day, booked);
        if booked != NEVER && booked >= first_day && (matches!(to, Bucket::Day(_)) || to == here) {
            pool.push(t.list(to), slot);
        } else {
            t.stale -= 1;
        }
    }
}

/// Drops a table's stale entries once they outnumber its live ones, so they never hold more memory than the live.
fn compact_if_due<B: Backing>(t: &mut TableAgenda<B>, pool: &mut BlockPool<B>, today: u32) {
    if t.stale <= t.live {
        return;
    }
    let (mut entries, mut dropped) = (Vec::new(), 0);
    for i in 0..WHEEL_BUCKETS {
        for b in [Bucket::Day(i), Bucket::Block(i)] {
            entries.clear();
            pool.drain(t.list(b), &mut entries);
            let all = entries.len();
            // A row's entry and a stale one of the same row can share a bucket; one is kept.
            entries.sort_unstable();
            entries.dedup();
            entries.retain(|s| t.is_live(today, b, *s));
            dropped += all - entries.len();
            for &slot in &entries {
                pool.push(t.list(b), slot);
            }
        }
    }
    if count(dropped) != t.stale {
        violation!(clause = "TIME.5", "stale agenda entries miscounted", counted = t.stale, found = dropped);
    }
    t.stale = 0;
}

/// What the agenda holds and has done: entries in the wheel, the stale among them, and rows moved later without
/// acting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgendaCounters {
    pub entries: u64,
    pub stale: u64,
    pub moved: u64,
}

/// One table's rows on today's agenda, in slot order, and for each the reasons it is due for as a mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableToday {
    pub table: TableId,
    pub slots: Vec<Slot>,
    pub reasons: Vec<u16>,
}

/// Which rows act today and why, in table and slot order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodayAgenda {
    pub day: Day,
    pub per_table: Vec<TableToday>,
}

/// Which rows act on which day: each row's next day per reason, and one calendar entry per row at the earliest, kept
/// in a two-level timing wheel of day buckets and 1 024-day block buckets.
#[clause("TIME.5")]
#[derive(Debug)]
pub struct Agenda<B: Backing = SystemBacking> {
    today: u32,
    pool: BlockPool<B>,
    tables: Vec<TableAgenda<B>>,
    moved: u64,
}

impl<B: Backing> Agenda<B> {
    /// An empty agenda on `today` for tables in ascending id order, with room for `max_blocks` blocks of entries.
    ///
    /// # Errors
    /// When a table declares no reasons or more than a cache line holds, or the tables are not in ascending order.
    pub fn new(
        space: &mut AddressSpace,
        today: Day,
        specs: &[AgendaTableSpec],
        max_blocks: u32,
    ) -> Result<Self, String> {
        let mut tables = Vec::with_capacity(specs.len());
        for (i, spec) in specs.iter().enumerate() {
            if spec.reasons == 0 || spec.reasons > MAX_REASONS {
                return Err(format!("table {} declares {} agenda reasons", spec.table.get(), spec.reasons));
            }
            if i > 0 && specs.get(i - 1).is_some_and(|p| p.table >= spec.table) {
                return Err(format!("table {} is out of order", spec.table.get()));
            }
            let cells = index(spec.max_rows) * spec.reasons;
            tables.push(TableAgenda {
                spec: *spec,
                rows: 0,
                next: Region::reserve(space, cells),
                booked: Region::reserve(space, index(spec.max_rows)),
                days: vec![BlockBag::EMPTY; WHEEL_BUCKETS],
                blocks: vec![BlockBag::EMPTY; WHEEL_BUCKETS],
                live: 0,
                stale: 0,
            });
        }
        Ok(Agenda { today: today.get(), pool: BlockPool::new(space, max_blocks), tables, moved: 0 })
    }

    pub fn today(&self) -> Day {
        Day::new(self.today)
    }

    fn table(&self, table: TableId) -> &TableAgenda<B> {
        let Some(t) = self.tables.iter().find(|t| t.spec.table == table) else {
            violation!(clause = "TIME.5", "a table the agenda does not book", table = table.get());
        };
        t
    }

    fn parts(&mut self, table: TableId) -> (&mut TableAgenda<B>, &mut BlockPool<B>) {
        let Some(t) = self.tables.iter_mut().find(|t| t.spec.table == table) else {
            violation!(clause = "TIME.5", "a table the agenda does not book", table = table.get());
        };
        (t, &mut self.pool)
    }

    /// Extends a table's rows to `rows`, each with no reason due and no entry.
    pub fn grow(&mut self, table: TableId, rows: u32) {
        let (t, _) = self.parts(table);
        let rows = index(rows);
        if rows > index(t.spec.max_rows) {
            capacity_exceeded!("agenda rows", t.spec.max_rows, rows);
        }
        if rows <= t.rows {
            return;
        }
        let width = t.spec.reasons;
        t.next.ensure(rows * width);
        t.booked.ensure(rows);
        if let Some(cells) = t.next.slice_mut(rows * width).get_mut(t.rows * width..) {
            cells.fill(NEVER);
        }
        if let Some(cells) = t.booked.slice_mut(rows).get_mut(t.rows..) {
            cells.fill(NEVER);
        }
        t.rows = rows;
    }

    /// The next day a row is due for a reason, or none.
    #[must_use]
    pub fn next(&self, table: TableId, slot: Slot, reason: usize) -> Option<Day> {
        let t = self.table(table);
        let Some(&day) = t.reasons(slot).get(reason) else {
            violation!(clause = "TIME.5", "a reason the table does not declare", reason = reason);
        };
        (day != NEVER).then(|| Day::new(day))
    }

    /// The day of a row's one entry, or none.
    #[must_use]
    pub fn booked(&self, table: TableId, slot: Slot) -> Option<Day> {
        let day = self.table(table).booked(slot);
        (day != NEVER).then(|| Day::new(day))
    }

    /// Sets the next day a row is due for a reason, which must be after today; an earlier day than the row's entry
    /// moves the entry to it.
    pub fn set_next(&mut self, table: TableId, slot: Slot, reason: usize, day: Day) {
        let today = self.today;
        if day.get() <= today || day.get() == NEVER {
            violation!(clause = "TIME.5", "a next day not after today", day = day.get(), today = today);
        }
        let (t, pool) = self.parts(table);
        let Some(cell) = t.reasons_mut(slot).get_mut(reason) else {
            violation!(clause = "TIME.5", "a reason the table does not declare", reason = reason);
        };
        *cell = day.get();
        let old = t.booked(slot);
        if day.get() < old {
            book(t, pool, today, slot, day.get(), old);
        }
    }

    /// Leaves a row with no next day for a reason; its entry stays until its day, when the row is re-booked.
    pub fn clear(&mut self, table: TableId, slot: Slot, reason: usize) {
        let (t, _) = self.parts(table);
        let Some(cell) = t.reasons_mut(slot).get_mut(reason) else {
            violation!(clause = "TIME.5", "a reason the table does not declare", reason = reason);
        };
        *cell = NEVER;
    }

    /// Forgets a row whose slot is released: its reasons are cleared and its entry is stale.
    pub fn release(&mut self, table: TableId, slot: Slot) {
        let today = self.today;
        let (t, pool) = self.parts(table);
        t.reasons_mut(slot).fill(NEVER);
        if t.booked(slot) != NEVER {
            t.set_booked(slot, NEVER);
            t.live -= 1;
            t.stale += 1;
            compact_if_due(t, pool, today);
        }
    }

    /// Moves a row's reasons and entry to another slot, which must hold none, as renumbering does.
    pub fn move_row(&mut self, table: TableId, from: Slot, to: Slot) {
        let today = self.today;
        let (t, pool) = self.parts(table);
        if t.booked(to) != NEVER || t.reasons(to).iter().any(|d| *d != NEVER) {
            violation!(clause = "TIME.5", "a row moved onto a row the agenda books", from = from.get(), to = to.get());
        }
        let reasons: Vec<u32> = t.reasons(from).to_vec();
        t.reasons_mut(to).copy_from_slice(&reasons);
        t.reasons_mut(from).fill(NEVER);
        let booked = t.booked(from);
        if booked != NEVER {
            t.set_booked(from, NEVER);
            t.live -= 1;
            t.stale += 1;
            book(t, pool, today, to, booked, NEVER);
        }
    }

    /// Moves the agenda to `day` and returns the rows due on the days since the last gather, with the reasons each is
    /// due for; each such row is re-booked at its earliest reason after `day`.
    #[clause("TIME.5")]
    pub fn gather(&mut self, day: Day) -> TodayAgenda {
        let (from, to) = (self.today, day.get());
        if to <= from || to == NEVER {
            violation!(clause = "TIME.5", "a gather not after the last", day = to, last = from);
        }
        self.today = to;
        let mut per_table = Vec::with_capacity(self.tables.len());
        let (mut scratch, mut due) = (Vec::new(), Vec::new());
        for t in &mut self.tables {
            due.clear();
            for d in from + 1..=to {
                if d & LOW == 0 {
                    cascade(t, &mut self.pool, d, &mut scratch);
                }
                scratch.clear();
                self.pool.drain(t.list(Bucket::Day(index(d & LOW))), &mut scratch);
                for &slot in &scratch {
                    if t.booked(Slot::new(slot)) == d {
                        due.push(slot);
                    } else {
                        t.stale -= 1;
                    }
                }
            }
            // A row booked again into a bucket that still holds its stale entry appears twice.
            let all = due.len();
            due.sort_unstable();
            due.dedup();
            t.stale -= count(all - due.len());
            // Every due row gives up its entry before any is booked again, so a compaction meets no row booked in
            // the past.
            let mut reasons = Vec::with_capacity(due.len());
            let mut rebook = Vec::with_capacity(due.len());
            for &slot in &due {
                let slot = Slot::new(slot);
                let (mut mask, mut earliest) = (0_u16, NEVER);
                for (r, &next) in t.reasons(slot).iter().enumerate() {
                    if next > from && next <= to {
                        mask |= 1 << r;
                    } else if next > to && next < earliest {
                        earliest = next;
                    }
                }
                if mask == 0 {
                    self.moved += 1;
                }
                reasons.push(mask);
                rebook.push(earliest);
                t.live -= 1;
                t.set_booked(slot, NEVER);
            }
            for (&slot, &earliest) in due.iter().zip(&rebook) {
                if earliest != NEVER {
                    book(t, &mut self.pool, to, Slot::new(slot), earliest, NEVER);
                }
            }
            compact_if_due(t, &mut self.pool, to);
            per_table.push(TableToday {
                table: t.spec.table,
                slots: due.iter().copied().map(Slot::new).collect(),
                reasons,
            });
        }
        TodayAgenda { day, per_table }
    }

    #[must_use]
    pub fn counters(&self) -> AgendaCounters {
        let (live, stale) = self.tables.iter().fold((0, 0), |(l, s), t| (l + t.live, s + t.stale));
        AgendaCounters { entries: live + stale, stale, moved: self.moved }
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.pool.bytes_committed()
            + self.tables.iter().map(|t| t.next.bytes_committed() + t.booked.bytes_committed()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, Slot, TableId};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Agenda, AgendaTableSpec, TodayAgenda};

    type Heap = HeapBacking<4096>;
    const T: TableId = TableId::new(0);

    fn agenda(today: u32, tables: &[(u16, u32, usize)]) -> Agenda<Heap> {
        let specs: Vec<AgendaTableSpec> = tables
            .iter()
            .map(|&(id, max_rows, reasons)| AgendaTableSpec { table: TableId::new(id), max_rows, reasons })
            .collect();
        let mut agenda = Agenda::new(&mut AddressSpace::empty(), Day::new(today), &specs, 1 << 16).unwrap();
        for &(id, max_rows, _) in tables {
            agenda.grow(TableId::new(id), max_rows);
        }
        agenda
    }

    fn slots(today: &TodayAgenda, table: usize) -> Vec<u32> {
        today.per_table[table].slots.iter().map(|s| s.get()).collect()
    }

    /// Random reasons set, cleared and moved, gathered a few days at a time: no reason in a gathered span is missed,
    /// each due row carries exactly its reasons in the span, and every booked row holds one live entry.
    #[test]
    fn agenda_one_live_entry_per_row() {
        let (rows, reasons) = (300_u32, 4_usize);
        let mut a = agenda(0, &[(0, rows, reasons)]);
        let mut d = Draws::new(stream_key(Seed::new(8), "agenda"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut draw = |n: u64| below_u64(&mut d, n);
        let mut model = vec![[u32::MAX; 4]; 300];
        for _ in 0..400 {
            let today = a.today().get();
            for _ in 0..draw(200) {
                let (row, r) = (u32::try_from(draw(u64::from(rows))).unwrap(), usize::try_from(draw(4)).unwrap());
                let ahead = match draw(20) {
                    0 => 1_000 + draw(5_000),
                    1 => 1 << 21,
                    _ => 1 + draw(60),
                };
                if draw(10) == 0 {
                    a.clear(T, Slot::new(row), r);
                    model[usize::try_from(row).unwrap()][r] = u32::MAX;
                } else {
                    let day = today + u32::try_from(ahead).unwrap();
                    a.set_next(T, Slot::new(row), r, Day::new(day));
                    model[usize::try_from(row).unwrap()][r] = day;
                }
            }
            let to = today + 1 + u32::try_from(draw(4)).unwrap();
            let got = a.gather(Day::new(to));
            let mut expected: Vec<(u32, u16)> = Vec::new();
            for (row, m) in model.iter().enumerate() {
                let mask =
                    m.iter().enumerate().filter(|(_, n)| **n > today && **n <= to).fold(0_u16, |k, (r, _)| k | 1 << r);
                if mask != 0 {
                    expected.push((u32::try_from(row).unwrap(), mask));
                }
            }
            let due: Vec<(u32, u16)> = got.per_table[0]
                .slots
                .iter()
                .zip(&got.per_table[0].reasons)
                .map(|(s, m)| (s.get(), *m))
                .filter(|(_, m)| *m != 0)
                .collect();
            assert_eq!(due, expected, "day {to}");
            let mut booked = 0;
            for (row, m) in model.iter().enumerate() {
                let slot = Slot::new(u32::try_from(row).unwrap());
                let b = a.booked(T, slot);
                assert!(b.is_none_or(|b| b.get() > to && m.iter().all(|n| *n <= to || *n >= b.get())), "row {row}");
                booked += u64::from(b.is_some());
            }
            let c = a.counters();
            assert_eq!(c.entries - c.stale, booked);
            assert!(c.stale <= c.entries - c.stale);
        }
    }

    #[test]
    fn agenda_later_moves_entry() {
        let mut a = agenda(0, &[(0, 4, 2)]);
        let row = Slot::new(3);
        a.set_next(T, row, 0, Day::new(10));
        a.set_next(T, row, 0, Day::new(20));
        assert_eq!(a.booked(T, row), Some(Day::new(10)), "a later day writes no entry");
        let day10 = a.gather(Day::new(10));
        assert_eq!((slots(&day10, 0), day10.per_table[0].reasons.clone()), (vec![3], vec![0]));
        assert_eq!(a.counters().moved, 1);
        assert_eq!(a.booked(T, row), Some(Day::new(20)));
        let day20 = a.gather(Day::new(20));
        assert_eq!((slots(&day20, 0), day20.per_table[0].reasons.clone()), (vec![3], vec![1]));
        assert_eq!(a.booked(T, row), None);
    }

    #[test]
    fn agenda_stale_bounded_by_compaction() {
        let rows = 64_u32;
        let mut a = agenda(0, &[(0, rows, 1)]);
        let mut peak = 0;
        // Each row's day falls a block at a time, leaving a stale entry in every block bucket it passed.
        for step in 0..200_u32 {
            for row in 0..rows {
                a.set_next(T, Slot::new(row), 0, Day::new((300 - step) * 1024 + row));
                let c = a.counters();
                assert!(c.stale <= c.entries - c.stale, "step {step} row {row}: {c:?}");
                peak = if c.entries > peak { c.entries } else { peak };
            }
        }
        assert!(peak <= 2 * u64::from(rows) + 1);
    }

    #[test]
    fn agenda_gather_is_canonical() {
        // Each row once, so only the order of the writes differs between the two runs.
        let sets: Vec<(u16, u32, u32)> = (0..200).map(|i| (u16::from(i % 2 == 0), i / 2, 1 + (i * 13) % 40)).collect();
        let run = |order: &[(u16, u32, u32)]| {
            let mut a = agenda(0, &[(0, 100, 1), (1, 100, 1)]);
            // A fixed shuffle of the writes, so neither run writes in slot order.
            for &(table, slot, day) in order.iter().skip(1).step_by(2).chain(order.iter().step_by(2)) {
                a.set_next(TableId::new(table), Slot::new(slot), 0, Day::new(day));
            }
            (1..=40).map(|d| a.gather(Day::new(d))).collect::<Vec<_>>()
        };
        let forward = run(&sets);
        let reversed: Vec<_> = sets.iter().rev().copied().collect();
        assert_eq!(forward, run(&reversed));
        for today in &forward {
            assert_eq!(today.per_table.iter().map(|t| t.table.get()).collect::<Vec<_>>(), vec![0, 1]);
            for t in &today.per_table {
                assert!(t.slots.windows(2).all(|w| w[0] < w[1]));
            }
        }
    }

    #[test]
    fn agenda_far_entries_enter_wheel() {
        let mut a = agenda(700, &[(0, 3, 1)]);
        let far = 700 + (1 << 20) + 5;
        let (near, block) = (Slot::new(0), Slot::new(1));
        a.set_next(T, near, 0, Day::new(900));
        a.set_next(T, block, 0, Day::new(5_000));
        a.set_next(T, Slot::new(2), 0, Day::new(far));
        let mut seen = Vec::new();
        let mut day = 700;
        while day < far + 10 {
            day += 997;
            for t in a.gather(Day::new(day)).per_table {
                seen.extend(t.slots.iter().map(|s| (s.get(), day)));
            }
        }
        assert_eq!(seen.iter().map(|(s, _)| *s).collect::<Vec<_>>(), vec![0, 1, 2]);
        let first_gather_on_or_after = |d: u32| 700 + (d - 700).div_ceil(997) * 997;
        assert_eq!(
            seen,
            vec![
                (0, first_gather_on_or_after(900)),
                (1, first_gather_on_or_after(5_000)),
                (2, first_gather_on_or_after(far))
            ]
        );
        assert_eq!(a.counters().entries, 0);
    }

    #[test]
    fn agenda_release_and_move() {
        let mut a = agenda(0, &[(0, 8, 2)]);
        a.set_next(T, Slot::new(1), 0, Day::new(5));
        a.set_next(T, Slot::new(1), 1, Day::new(3_000));
        a.set_next(T, Slot::new(2), 0, Day::new(7));
        a.release(T, Slot::new(2));
        assert_eq!((a.booked(T, Slot::new(2)), a.next(T, Slot::new(2), 0)), (None, None));
        a.move_row(T, Slot::new(1), Slot::new(6));
        assert_eq!(a.booked(T, Slot::new(1)), None);
        assert_eq!(a.next(T, Slot::new(6), 1), Some(Day::new(3_000)));
        let c = a.counters();
        assert_eq!(c.entries - c.stale, 1);
        let mut due = Vec::new();
        for d in 1..=3_000 {
            for t in a.gather(Day::new(d)).per_table {
                due.extend(t.slots.iter().zip(&t.reasons).map(|(s, m)| (d, s.get(), *m)));
            }
        }
        assert_eq!(due, vec![(5, 6, 0b01), (3_000, 6, 0b10)]);
        assert_eq!(a.counters().entries, 0);
    }

    #[test]
    fn agenda_contracts() {
        let specs = |reasons| [AgendaTableSpec { table: T, max_rows: 1, reasons }];
        assert!(Agenda::<Heap>::new(&mut AddressSpace::empty(), Day::new(0), &specs(17), 16).is_err());
        assert!(Agenda::<Heap>::new(&mut AddressSpace::empty(), Day::new(0), &specs(0), 16).is_err());
        let mut a = agenda(9, &[(0, 1, 1)]);
        let caught =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a.set_next(T, Slot::new(0), 0, Day::new(9))));
        let payload = caught.expect_err("a next day on today stops the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("TIME.5"));
    }
}
