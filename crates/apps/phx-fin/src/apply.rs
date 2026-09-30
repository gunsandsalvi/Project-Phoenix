//! `-F apply`: the design point's accounts, each day's dues' two sides and retail payee credits applied by range —
//! scattered by their accounts' ranges from the producing chunks and swept one job a range — in waves where a heavy
//! day's items pass the dues' lane.

use std::collections::BTreeMap;

use phx_exec::consts::APPLY_RANGE_SHIFT;
use phx_exec::{Item, Pool, PoolSpec, Shards, apply_by_range};
use phx_rand::uniform::below_u64;

use crate::daybuf::day_plan;
use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, index, slots, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the apply is measured under.
pub const BASE: &str = "apply";

/// The day buffer whose lane holds a wave of the dues' shards.
const DUES_LANE: &str = "dues";

/// Dues or sales a producing chunk makes: a chunk of the dues' stream at about 200 µs of its declared cost.
const CHUNK_EVENTS: u64 = 1 << 13;

/// The largest amount a drawn item carries.
const AMOUNT: u64 = 1 << 20;

/// Bytes a MiB.
const MIB: f64 = 1_048_576.0;

/// The accounts' balances, the pool, the shards kept across days, the dues' lane in items, the streams, and each day
/// type's waves.
#[derive(Debug, Default)]
pub struct Apply {
    balances: Vec<i64>,
    pool: Option<Pool>,
    shards: Shards,
    lane: usize,
    streams: Option<Streams>,
    waves: BTreeMap<&'static str, u64>,
    scatter_bytes: u64,
}

impl FinBase for Apply {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] accounts` balances, and the dues' lane the day plan gives at 2b.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let accounts = count(&design.store, "accounts", "store")?;
        self.balances = vec![0; index(accounts)?];
        let declared = design.dayplan()?;
        let plan = day_plan(design)?;
        let at = declared.buffers.iter().position(|(name, _, _)| name == DUES_LANE);
        let lane = at.and_then(|a| plan.placed().get(a)).map(|p| p.bytes);
        let lane = lane.ok_or_else(|| FinError(format!("the day plan has no `{DUES_LANE}` lane")))?;
        self.lane = index(lane / wide(size_of::<Item>()))?;
        self.pool = Some(Pool::new(&PoolSpec::detect()).map_err(|e| FinError(e.0))?);
        self.streams = Some(*streams);
        Ok(Filled { rows: accounts })
    }

    /// The day's dues, each a payer's debit and a payee's credit, and its retail sales' payee credits, made in chunks
    /// and applied by range.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(streams) = self.streams else {
            return Err(FinError("the apply measured before its fill".to_owned()));
        };
        // A day that makes no dues or sales, as the counts may say of a closed day, applies none of them.
        let (dues, retail) = (counts.get("dues").copied().unwrap_or(0), counts.get("retail").copied().unwrap_or(0));
        let accounts = wide(self.balances.len());
        // Each producing chunk draws its own items from its own stream, as a chunk of the dues' traversal would.
        let mut chunks: Vec<Vec<Item>> = Vec::new();
        let total = dues + retail;
        let mut first = 0;
        while first < total {
            // A chunk makes this many sales or dues, each one item or, for a due, two.
            let end = if first + CHUNK_EVENTS < total { first + CHUNK_EVENTS } else { total };
            let mut d = streams.draws(BASE, wide(chunks.len()), day_of(day)?);
            let mut chunk = Vec::new();
            for i in first..end {
                let amount = i64::try_from(1 + i % AMOUNT).map_err(|e| FinError(e.to_string()))?;
                if i < dues {
                    chunk.push(Item { target: slots(below_u64(&mut d, accounts))?, tag: 0, amount: -amount });
                }
                chunk.push(Item { target: slots(below_u64(&mut d, accounts))?, tag: 1, amount });
            }
            chunks.push(chunk);
            first = end;
        }
        let items: u64 = chunks.iter().map(|c| wide(c.len())).sum();
        let inputs: Vec<&[Item]> = chunks.iter().map(Vec::as_slice).collect();
        let shift = APPLY_RANGE_SHIFT;
        let mut ranges: Vec<(&mut [i64], ())> = self.balances.chunks_mut(1 << shift).map(|b| (b, ())).collect();
        let (pool, shards, lane) = (self.pool.as_ref(), &mut self.shards, self.lane);
        m.read(BASE, "item", items, || {
            apply_by_range(pool, (&inputs, shift, lane), &mut ranges, shards, |balances, item| {
                let at = item.target & ((1 << shift) - 1);
                if let Some(b) = usize::try_from(at).ok().and_then(|at| balances.get_mut(at)) {
                    *b += item.amount;
                }
            });
        });
        self.waves.insert(day.key(), self.shards.waves());
        self.scatter_bytes = self.shards.scatter_bytes();
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: wide(self.balances.len() * size_of::<i64>()), resident: self.shards.bytes() }
    }

    /// The heavy day's waves, the scatter's own buffers beside the wave's items, and the dues' lane that holds them.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let real = |n: u64| n.to_string().parse::<f64>().ok();
        let mut out = Vec::new();
        if let Some(w) = self.waves.get("h").and_then(|w| real(*w)) {
            out.push(("waves_h", w));
        }
        if let Some(b) = real(self.scatter_bytes) {
            out.push(("buffer_mb", b / MIB));
        }
        if let Some(lane) = real(wide(self.lane * size_of::<Item>())) {
            out.push(("lane_mb", lane / MIB));
        }
        out
    }
}
