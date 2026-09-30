//! `-F daybuf`: a day buffer for each of the design point's daily counts, reserved at its heaviest day, each day
//! cleared and filled to that day's count, so a day after the first of its kind is read committing no page and
//! allocating nothing.

use std::collections::BTreeMap;

use phx_exec::trace::{Reading, Spent};
use phx_store::{AddressSpace, DayBuf};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the day buffers are measured under.
pub const BASE: &str = "daybuf";

/// The pages a day faulted and the allocations it made, each missing where the machine gives none.
type Cost = (Option<u64>, Option<u64>);

/// Each daily count's buffer, and the cost of the last day of each type.
#[derive(Debug, Default)]
pub struct DayBufs {
    bufs: BTreeMap<String, DayBuf<u64>>,
    last: BTreeMap<DayType, Cost>,
}

impl FinBase for DayBufs {
    fn name(&self) -> &'static str {
        BASE
    }

    /// A buffer for each count any day declares, reserved at the heaviest day's count; nothing is written.
    fn fill(&mut self, design: &Design, _streams: &Streams) -> Result<Filled, FinError> {
        let mut heaviest: BTreeMap<String, u64> = BTreeMap::new();
        for d in DayType::ALL {
            for (key, n) in design.day(d).into_iter().flatten() {
                let at = heaviest.entry(key.clone()).or_insert(*n);
                if *n > *at {
                    *at = *n;
                }
            }
        }
        let mut space = AddressSpace::empty();
        for (key, n) in &heaviest {
            self.bufs.insert(key.clone(), DayBuf::new(&mut space, BASE, index(*n)?));
        }
        Ok(Filled { rows: heaviest.values().sum() })
    }

    /// Each buffer cleared and filled to the day's count, a word an item; the day's faults and allocations kept.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let items: u64 = counts.keys().filter_map(|k| self.bufs.get(k).map(|_| counts.get(k))).flatten().sum();
        let bufs = &mut self.bufs;
        let spent = m.read(BASE, "append", items, || {
            let before = Reading::now();
            for (key, b) in bufs.iter_mut() {
                b.clear();
                if let Some(&n) = counts.get(key) {
                    for i in 0..n {
                        b.push(i);
                    }
                }
                b.mark(day.index());
            }
            Spent::between(Some(items), &before, &Reading::now())
        });
        self.last.insert(day, (spent.faults, spent.allocs));
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.bufs.values().map(|b| wide(b.bytes_committed())).sum(), resident: 0 }
    }

    /// The most pages faulted and allocations made by any type of day, each read on its second.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let most = |pick: fn(&Cost) -> Option<u64>| {
            let mut all: Vec<u64> = self.last.values().filter_map(pick).collect();
            all.sort_unstable();
            all.last().and_then(|n| u32::try_from(*n).ok()).map(f64::from)
        };
        [("faults_per_day", most(|l| l.0)), ("allocs_per_day", most(|l| l.1))]
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect()
    }
}
