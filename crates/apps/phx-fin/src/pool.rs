//! `-F pool`: the pool's own cost over a day's dispatches — each handing every one of the most workers four chunks,
//! read on the wall clock — the spin the workers keep between them, and a plan of real work, read as busy cores.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_exec::{ChunkPlan, Pool, PoolSpec};

use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the pool is measured under.
pub const BASE: &str = "pool";

/// Rows of the plan of real work a day runs, each mixed once: enough that its dispatch is a small part of it.
const WORK_ROWS: usize = 1 << 22;

/// A row's mixing, declared in the plan's cost units.
const WORK_COST: u64 = 2;

/// The chunks an empty dispatch hands out: four for each of the eight most workers, as a plan of enough rows has.
const DISPATCH_CHUNKS: usize = 32;

/// Nanoseconds a microsecond.
const NS_PER_US: f64 = 1_000.0;

/// Hundredths of a busy core.
const HUNDREDTHS: f64 = 100.0;

/// The pool, the rows the work mixes, and what the days read: each day type's dispatches, and the sums of the
/// dispatches' wall and spin and of the work's CPU and wall.
#[derive(Debug, Default)]
pub struct PoolBase {
    pool: Option<Pool>,
    rows: Vec<u64>,
    barriers: BTreeMap<&'static str, u64>,
    dispatches: u64,
    dispatch_wall_ns: u64,
    spun_ns: u64,
    work: (u64, u64),
}

/// A measured operation's CPU and wall, where the machine gave them.
fn cpu_and_wall(m: &Measures<'_>, op: &str) -> Option<(u64, u64)> {
    m.iter().find(|((b, o), _)| b == BASE && o == op).and_then(|(_, x)| Some((x.cpu_ns?, x.wall_ns?)))
}

impl FinBase for PoolBase {
    fn name(&self) -> &'static str {
        BASE
    }

    fn fill(&mut self, _: &Design, _: &Streams) -> Result<Filled, FinError> {
        self.pool = Some(Pool::new(&PoolSpec::detect()).map_err(|e| FinError(e.0))?);
        self.rows = (0..WORK_ROWS).map(|i| phx_exec::mix64(u64::try_from(i).unwrap_or(u64::MAX))).collect();
        Ok(Filled { rows: 0 })
    }

    /// The day's dispatches back to back, as a day's sub-steps follow one another, then one plan of real work.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(pool) = self.pool.as_ref() else {
            return Err(FinError("the pool measured before its fill".to_owned()));
        };
        let dispatches = *counts
            .get("barriers")
            .ok_or_else(|| FinError(format!("the design point has no `day.{}.barriers`", day.key())))?;
        self.barriers.insert(day.key(), dispatches);
        let mut chunks = [0_usize; DISPATCH_CHUNKS];
        let spun = phx_exec::pool::usage().spun;
        let wall_before = cpu_and_wall(m, "dispatch").map_or(0, |(_, w)| w);
        m.read(BASE, "dispatch", dispatches, || {
            for _ in 0..dispatches {
                phx_exec::for_each_chunk(Some(pool), &mut chunks, |i, c| *c = black_box(i));
            }
        });
        let wall_after = cpu_and_wall(m, "dispatch").map_or(0, |(_, w)| w);
        self.dispatch_wall_ns += wall_after - wall_before;
        self.dispatches += dispatches;
        self.spun_ns += phx_exec::pool::usage().spun - spun;
        let plan = ChunkPlan::of_rows(self.rows.len(), WORK_COST);
        let mut sums = vec![0_u64; plan.len()];
        let rows = &self.rows;
        let before = cpu_and_wall(m, "work").unwrap_or((0, 0));
        m.read(BASE, "work", u64::try_from(rows.len()).unwrap_or(u64::MAX), || {
            phx_exec::for_plan(Some(pool), &plan, &mut sums, |r, s| {
                if let Some(rows) = rows.get(r) {
                    *s = rows.iter().fold(0, |a, v| a ^ phx_exec::mix64(*v));
                }
            });
        });
        black_box(&sums);
        let after = cpu_and_wall(m, "work").unwrap_or((0, 0));
        self.work = (self.work.0 + after.0 - before.0, self.work.1 + after.1 - before.1);
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes::default()
    }

    /// A dispatch's wall, the spin a worker keeps up after it, the busy cores of the work, and each day's barriers;
    /// a figure whose parts the machine did not give is left out.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let real = |n: u64| n.to_string().parse::<f64>().ok();
        let workers = self.pool.as_ref().and_then(|p| real(u64::try_from(p.workers()).ok()?));
        let mut out = Vec::new();
        if let (Some(wall), Some(n)) = (real(self.dispatch_wall_ns), real(self.dispatches)) {
            out.push(("dispatch_us", wall / n / NS_PER_US));
            if let (Some(spun), Some(w)) = (real(self.spun_ns), workers) {
                out.push(("spin_us", spun / n / w / NS_PER_US));
            }
        }
        if let (Some(cpu), Some(wall)) = (real(self.work.0), real(self.work.1)) {
            out.push(("busy_hundredths", cpu / wall * HUNDREDTHS));
        }
        for (day, key) in [("b", "barriers_b"), ("nb", "barriers_nb"), ("h", "barriers_h")] {
            if let Some(n) = self.barriers.get(day).and_then(|n| real(*n)) {
                out.push((key, n));
            }
        }
        out
    }
}
