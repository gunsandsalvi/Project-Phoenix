//! Rolling cursors and sweeps over hand-given high waters: every slot read once a cycle whatever the store's growth,
//! the cursor continued across a save, one result for any workers, reason sweeps on their days only, and the ledger
//! telling a declared sweep from a pass nothing explains.
#![cfg(test)]

use super::{Cursor, DayVisits, SweepDecl, SweepLedger, SweepRun, Visit, Walk, When};
use crate::pool::Pool;
use crate::spec::PoolSpec;

const AUDIT: SweepDecl = SweepDecl { name: "audit", store: 1, slot: 2, when: When::Rolling { cycle: 7 } };
const DEPOSITORS: SweepDecl = SweepDecl { name: "depositors", store: 0, slot: 1, when: When::OnReason };

fn walk(high_water: u32) -> Walk {
    Walk { high_water, rows_per_chunk: 64, cost_per_row: 40_000 }
}

/// Each cycle's reads of every slot, the cycle closing on the day the next begins; the high water a day is `hw(day)`.
fn cycles(cycle: u32, days: u32, hw: impl Fn(u32) -> u32) -> Vec<(Vec<u32>, u32)> {
    let mut cursor = Cursor::starting(0);
    let (mut out, mut reads, mut last_hw) = (Vec::new(), Vec::<u32>::new(), 0);
    for day in 0..days {
        let start = cursor.cycle_start_day();
        let span = cursor.slice(cycle, hw(day), day);
        if cursor.cycle_start_day() != start {
            out.push((std::mem::take(&mut reads), last_hw));
        }
        let len = usize::try_from(hw(day)).unwrap();
        if reads.len() < len {
            reads.resize(len, 0);
        }
        for s in span {
            reads[usize::try_from(s).unwrap()] += 1;
        }
        last_hw = hw(day);
    }
    out
}

#[test]
fn every_row_once_per_cycle() {
    let done = cycles(7, 70, |_| 1_000);
    assert_eq!(done.len(), 9, "a cycle every seven days");
    for (reads, _) in &done {
        assert!(reads.iter().all(|r| *r == 1), "every slot read once a cycle");
    }
    // A cycle's slices are its rows shared over its days: a fixed share a day.
    let mut cursor = Cursor::starting(3);
    let slices: Vec<u32> = (3..10).map(|d| cursor.slice(7, 1_000, d).len().try_into().unwrap()).collect();
    assert_eq!(slices, vec![143, 143, 143, 143, 143, 143, 142]);
}

#[test]
fn every_row_once_per_cycle_with_growth() {
    // Two years of a store growing unevenly, faster each day and by a jump each quarter: each cycle reads every slot below its last day's high water once and
    // closes on its days, the growth lengthening only its slices.
    let hw = |day: u32| 10_000 + day * 37 + day * day / 50 + (day / 90) * 2_000;
    let done = cycles(60, 730, hw);
    assert_eq!(done.len(), 12);
    for (reads, last) in &done {
        assert_eq!(reads.len(), usize::try_from(*last).unwrap());
        assert!(reads.iter().all(|r| *r == 1), "no slot skipped nor read twice");
    }
}

#[test]
fn a_missed_day_is_read_next_cycle() {
    let mut cursor = Cursor::starting(0);
    assert_eq!(cursor.slice(10, 100, 0), 0..10);
    // Days 1–11 not run: the cycle's days are over, so the next begins at the first slot and reads what was missed.
    assert_eq!(cursor.slice(10, 100, 12), 0..10);
    assert_eq!(cursor.cycle_start_day(), 12);
}

#[test]
fn cursor_survives_save() {
    let mut cursor = Cursor::starting(5);
    for day in 5..23 {
        let _ = cursor.slice(30, 50_000 + day, day);
    }
    let (mut back, _) = phx_store::roundtrip(&cursor).unwrap();
    assert_eq!(back, cursor);
    for day in 23..200 {
        assert_eq!(back.slice(30, 50_000 + day, day), cursor.slice(30, 50_000 + day, day));
    }
}

/// Every chunk's sum of its slots' values, over a rolling sweep's first three days.
fn swept(workers: usize, values: &[u64]) -> Vec<Vec<u64>> {
    let pool = (workers > 0).then(|| Pool::new(&PoolSpec::unpinned(workers)).unwrap());
    let (mut run, mut cursor, mut ledger) = (SweepRun::<u64>::default(), Cursor::starting(0), SweepLedger::new(2, 3));
    let high_water = u32::try_from(values.len()).unwrap();
    (0..3)
        .map(|day| {
            let sums = run.rolling(pool.as_ref(), (&AUDIT, &mut cursor, walk(high_water)), day, &mut ledger, |s, o| {
                *o = values[usize::try_from(s.start).unwrap()..usize::try_from(s.end).unwrap()].iter().sum();
            });
            sums.to_vec()
        })
        .collect()
}

#[test]
fn sweep_same_for_any_workers() {
    let values: Vec<u64> = (0..200_000_u64).map(|i| i.wrapping_mul(0x9E37_79B9) >> 7).collect();
    let one = swept(0, &values);
    assert!(one.iter().all(|d| d.len() > 1), "a day's slice runs as several chunks");
    for workers in 1..=8 {
        assert_eq!(swept(workers, &values), one, "{workers} workers");
    }
    let whole: u64 = values[..usize::try_from(3 * 200_000_u32.div_ceil(7)).unwrap()].iter().sum();
    assert_eq!(one.iter().flatten().sum::<u64>(), whole);
}

#[test]
fn reason_sweep_runs_only_on_its_days() {
    let mut ledger = SweepLedger::new(2, 3);
    let mut run = SweepRun::<u64>::default();
    let mut read = Vec::new();
    for day in 0..10_u32 {
        let failed = day == 3 || day == 7;
        let chunks = run
            .on_reason(None, (&DEPOSITORS, walk(5_000)), failed, &mut ledger, |s, o| *o = u64::from(s.end - s.start));
        read.push(chunks.iter().sum::<u64>());
        assert_eq!(ledger.close_day().declared, read[usize::try_from(day).unwrap()]);
    }
    assert_eq!(read, vec![0, 0, 0, 5_000, 0, 0, 0, 5_000, 0, 0]);
}

#[test]
fn a_sweep_run_otherwise_than_declared_stops() {
    let caught = std::panic::catch_unwind(|| {
        let mut run = SweepRun::<u64>::default();
        let _ = run.on_reason(None, (&AUDIT, walk(10)), true, &mut SweepLedger::new(2, 3), |_, _| {});
    });
    assert!(caught.is_err(), "a rolling sweep run on a reason stops the run");
}

#[test]
fn ledger_counts_every_traversal() {
    let mut ledger = SweepLedger::new(2, 3);
    ledger.count((0, 0), Visit::Agenda, 1_200);
    ledger.count((0, 1), Visit::Index, 30);
    ledger.count((1, 1), Visit::Apply, 4_000);
    let mut run = SweepRun::<u64>::default();
    let mut cursor = Cursor::starting(0);
    let _ = run.rolling(None, (&AUDIT, &mut cursor, walk(700)), 0, &mut ledger, |_, _| {});
    // A pass over every row of a store that no declaration names: counted above the agenda, and so seen.
    ledger.count((1, 0), Visit::Pass, 700);
    assert_eq!(ledger.at((1, 2), Visit::Sweep), 100);
    assert_eq!(ledger.day(), DayVisits { following: 5_230, declared: 100, undeclared: 700 });
    assert_eq!(ledger.close_day(), DayVisits { following: 5_230, declared: 100, undeclared: 700 });
    assert_eq!(ledger.day(), DayVisits::default(), "the next day starts empty");
    assert!(std::panic::catch_unwind(move || ledger.count((0, 3), Visit::Agenda, 1)).is_err(), "a slot past the day");
}

#[test]
fn a_writing_sweep_writes_only_its_slice() {
    // A daily sweep over pending amounts: each chunk moves its rows' pending into their balances, the same for any
    // workers, and rows outside the slice untouched.
    let daily = SweepDecl { name: "pending", store: 0, slot: 0, when: When::Rolling { cycle: 1 } };
    let rows: Vec<[i64; 2]> = (0..100_000_i64).map(|i| [i, i % 7 - 3]).collect();
    let mut first = None;
    for workers in 0..=8 {
        let pool = (workers > 0).then(|| Pool::new(&PoolSpec::unpinned(workers)).unwrap());
        let (mut run, mut cursor, mut ledger) =
            (SweepRun::<i64>::default(), Cursor::starting(0), SweepLedger::new(1, 1));
        let mut column = rows.clone();
        let moved = run
            .rolling_mut(
                pool.as_ref(),
                (&daily, &mut cursor, walk(100_000)),
                0,
                (&mut ledger, &mut column),
                |_, run, o| {
                    for [balance, pending] in run {
                        *o += *pending;
                        (*balance, *pending) = (*balance + *pending, 0);
                    }
                },
            )
            .to_vec();
        assert!(column.iter().all(|[_, p]| *p == 0));
        assert_eq!(ledger.day().declared, 100_000);
        match &first {
            None => first = Some((moved, column)),
            Some(f) => assert_eq!(f, &(moved, column), "{workers} workers"),
        }
    }
    let mut half = rows.clone();
    let (mut run, mut cursor, mut ledger) = (SweepRun::<i64>::default(), Cursor::starting(0), SweepLedger::new(1, 1));
    let halves = SweepDecl { when: When::Rolling { cycle: 2 }, ..daily };
    let _ = run.rolling_mut(None, (&halves, &mut cursor, walk(100_000)), 0, (&mut ledger, &mut half), |_, run, _| {
        for r in run {
            r[1] = 0;
        }
    });
    assert!(half[..50_000].iter().all(|r| r[1] == 0) && half[50_000..] == rows[50_000..]);
}
