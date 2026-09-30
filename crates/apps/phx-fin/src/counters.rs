//! `-F counters`: what the counters themselves cost, a chunk's counting and a span's, over a day's dispatches with
//! every worker's chunk in each, so their share of the day is read rather than assumed.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_exec::PoolSpec;
use phx_exec::pool::counted_chunk;
use phx_exec::trace::{Reading, Spent};

use crate::design::Design;
use crate::fill::Streams;
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The counters' driver: the workers a dispatch spreads its chunks over.
#[derive(Debug, Default)]
pub struct Counters {
    workers: u64,
}

impl FinBase for Counters {
    fn name(&self) -> &'static str {
        "counters"
    }

    fn fill(&mut self, _: &Design, _: &Streams) -> Result<Filled, FinError> {
        self.workers = u64::try_from(PoolSpec::detect().workers()).map_err(|e| FinError(e.to_string()))?;
        Ok(Filled { rows: 0 })
    }

    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let dispatches = *counts
            .get("barriers")
            .ok_or_else(|| FinError(format!("the design point has no `day.{}.barriers`", day.key())))?;
        let chunks = dispatches * self.workers;
        m.read("counters", "chunk", chunks, || {
            for i in 0..chunks {
                counted_chunk(|| {
                    black_box(i);
                });
            }
        });
        m.read("counters", "span", dispatches, || {
            for _ in 0..dispatches {
                let before = Reading::now();
                black_box(Spent::between(None, &before, &Reading::now()).counts());
            }
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes::default()
    }
}
