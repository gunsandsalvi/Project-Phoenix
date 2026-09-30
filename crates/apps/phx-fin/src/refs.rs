//! `-F refs`: the party slots of the design point in one table behind their generations, a day's rows begun and ended
//! first in first out, and a day's references resolved in slot order, as a gather meets them.

use std::collections::BTreeMap;

use phx_id::Slot;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, GenRef, Generations, SlotAlloc};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, index, slots, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the references are measured under.
pub const BASE: &str = "refs";

/// The table's rows, the marker of their generations.
#[derive(Debug)]
pub struct Rows;

/// The slots and their generations, the day's references in slot order, the streams the days draw from, and what
/// the last day's resolves found.
#[derive(Debug, Default)]
pub struct Refs {
    table: Option<(SlotAlloc, Generations<Rows>)>,
    held: Vec<GenRef<Rows>>,
    streams: Option<Streams>,
    found: u64,
}

impl Refs {
    /// How many of the day's references resolved, the same for any workers.
    #[must_use]
    pub fn found(&self) -> u64 {
        self.found
    }
}

/// The heaviest of the design point's days by a count.
fn heaviest(design: &Design, key: &str) -> Result<u64, FinError> {
    let mut by_day: Vec<u64> = DayType::ALL.iter().filter_map(|d| design.day(*d)?.get(key).copied()).collect();
    by_day.sort_unstable();
    by_day.last().copied().ok_or_else(|| FinError(format!("the design point has no `{key}` on any day")))
}

impl FinBase for Refs {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] directory_slots` rows begun, and as many references as the heaviest day applies, drawn over them and
    /// sorted by slot.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let rows = count(&design.store, "directory_slots", "store")?;
        let mut space = AddressSpace::empty();
        // The rows a day begins take new slots until the close returns its ended ones, so the table holds a day's
        // rows beyond those filled; `row_life` counts every table's rows, so it is headroom enough for these.
        let max = slots(rows + heaviest(design, "row_life")?)?;
        let mut alloc = SlotAlloc::new(&mut space, max);
        let mut gens = Generations::new(&mut space, max, phx_core::consts::CHUNK_ROWS, phx_id::consts::GENERATION_BITS);
        let begun: Vec<GenRef<Rows>> = (0..rows).map(|_| gens.alloc(&mut alloc)).collect();
        let applies = heaviest(design, "applies")?;
        let mut d = streams.draws(BASE, 0, 0);
        let mut refs = Vec::with_capacity(index(applies)?);
        for _ in 0..applies {
            if let Some(r) = begun.get(index(below_u64(&mut d, rows))?) {
                refs.push(*r);
            }
        }
        refs.sort_unstable_by_key(|r| r.slot());
        (self.table, self.held, self.streams) = (Some((alloc, gens)), refs, Some(*streams));
        Ok(Filled { rows })
    }

    /// Up to half the day's `row_life` rows ended at drawn slots and as many begun, the close returning the ended to
    /// the ring, so the table's rows stay as filled; then the day's `applies` references resolved in slot order.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some((alloc, gens)), Some(streams)) = (self.table.as_mut(), self.streams) else {
            return Err(FinError("the references resolved before their fill".to_owned()));
        };
        if let Some(&life) = counts.get("row_life") {
            let mut d = streams.draws(BASE, 1, crate::kept::day_of(day)?);
            let high = u64::from(alloc.high_water());
            let mut ended: Vec<Slot> =
                (0..life / 2).map(|_| slots(below_u64(&mut d, high)).map(Slot::new)).collect::<Result<_, _>>()?;
            // A slot drawn twice, or whose row ended on an earlier day and waits in the ring, is not ended again.
            ended.sort_unstable();
            ended.dedup();
            ended.retain(|s| alloc.is_live(*s));
            m.read(BASE, "alloc", wide(ended.len()) * 2, || {
                for s in &ended {
                    alloc.release(*s);
                }
                for _ in &ended {
                    let _ = gens.alloc(alloc);
                }
                alloc.close_day();
            });
        }
        // A day that applies nothing, as a closed day may not, resolves nothing.
        let Some(&applies) = counts.get("applies") else { return Ok(()) };
        let Some(refs) = self.held.get(..index(applies)?) else {
            return Err(FinError(format!("{applies} references asked of the {} filled", self.held.len())));
        };
        let (alloc, gens) = (&*alloc, &*gens);
        self.found = m.read(BASE, "resolve", applies, || {
            refs.iter().fold(0_u64, |n, r| n + u64::from(gens.resolve(alloc, *r).is_some()))
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let rows = self.table.as_ref().map_or(0, |(a, g)| wide(a.bytes_committed() + g.bytes_committed()));
        Bytes { rows, resident: 0 }
    }

    /// A slot's generation's bytes.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        self.table
            .as_ref()
            .and_then(|(a, g)| wide(g.bytes_committed()).checked_div(u64::from(a.high_water())))
            .and_then(|b| u32::try_from(b).ok())
            .map(|b| vec![("gen_bytes", f64::from(b))])
            .unwrap_or_default()
    }
}
