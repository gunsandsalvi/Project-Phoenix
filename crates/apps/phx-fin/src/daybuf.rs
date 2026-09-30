//! `-F daybuf`: a day buffer for each of the design point's daily counts, reserved at its heaviest day, each day
//! cleared and filled to that day's count, so a day after the first of its kind is read committing no page and
//! allocating nothing; and the heaviest day's buffers placed by the day plan, their owners' figures scaled to the design
//! point, each filled in its lane in slot order, so the plan's region is read resident at its largest live set.

use std::collections::BTreeMap;

use phx_exec::trace::{Reading, Spent};
use phx_store::{AddressSpace, BufDecl, DayBuf, DayPlan, DayRegion, Life, Size, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the day buffers are measured under.
pub const BASE: &str = "daybuf";

/// A decimal megabyte, the unit the plan's figures are stated in.
const MB: f64 = 1_000_000.0;

/// A plan's figures in MB, to the nearest tenth the steps state them to: the region's last page's tail is below it.
fn tenths(bytes: u64) -> Result<f64, FinError> {
    let tenths = ((bytes + 50_000) / 100_000).to_string();
    tenths.parse::<f64>().map(|t| t / 10.0).map_err(|e| FinError(e.to_string()))
}

/// The pages a day faulted and the allocations it made, each missing where the machine gives none.
type Cost = (Option<u64>, Option<u64>);

/// Each daily count's buffer, and the cost of the last day of each type.
#[derive(Debug, Default)]
pub struct DayBufs {
    bufs: BTreeMap<String, DayBuf<u64>>,
    last: BTreeMap<DayType, Cost>,
    plan: Option<DayRegion>,
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
        let declared = design.dayplan()?;
        let slot = |name: &str| {
            let at = declared.slots.iter().position(|s| s == name);
            at.and_then(|a| u16::try_from(a).ok()).ok_or_else(|| FinError(format!("the day has no slot `{name}`")))
        };
        let mut decls = Vec::new();
        for (name, mb, (fill, release)) in &declared.buffers {
            let size = match mb {
                Some(mb) => {
                    let bytes = format!("{:.0}", mb * MB * design.point.scale);
                    Size::Bytes(bytes.parse().map_err(|e| FinError(format!("{name}: {e}")))?)
                }
                None => Size::Rest,
            };
            // The declarations live as long as the run: the plan names each by its declared name.
            let name: &'static str = Box::leak(name.clone().into_boxed_str());
            decls.push(BufDecl { name, size, life: Life { fill: slot(fill)?, release: slot(release)? } });
        }
        let slots = u16::try_from(declared.slots.len()).map_err(|e| FinError(e.to_string()))?;
        let plan = DayPlan::plan(&decls, slots).map_err(FinError)?;
        self.plan = Some(DayRegion::new(&mut space, plan));
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
        // The heaviest day's buffers fill their lanes in slot order, each as it is first filled.
        if day == DayType::H
            && let Some(region) = self.plan.as_mut()
        {
            let placed = region.plan().placed().to_vec();
            let last = placed.iter().fold(0, |a, p| if p.life.release > a { p.life.release } else { a });
            for slot in 0..=last {
                for p in placed.iter().filter(|p| p.life.fill == slot) {
                    let words = index(p.bytes.div_ceil(u64::from(u64::BITS / u8::BITS)))?;
                    region.lane_mut(p.decl, slot, words).fill(u64::from(slot));
                }
            }
        }
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let bufs: u64 = self.bufs.values().map(|b| wide(b.bytes_committed())).sum();
        Bytes { rows: bufs + self.plan.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    /// The most pages faulted and allocations made by any type of day, each read on its second.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let most = |pick: fn(&Cost) -> Option<u64>| {
            let mut all: Vec<u64> = self.last.values().filter_map(pick).collect();
            all.sort_unstable();
            all.last().and_then(|n| u32::try_from(*n).ok()).map(f64::from)
        };
        let peak = self.plan.as_ref().and_then(|r| tenths(r.bytes()).ok());
        [("faults_per_day", most(|l| l.0)), ("allocs_per_day", most(|l| l.1)), ("peak_mb", peak)]
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect()
    }
}
